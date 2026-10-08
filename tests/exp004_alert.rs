//! EXP-004 Plan 1 / Definition of Done: `quotabus alert` files ONE item per crossing (a service going bad), never
//! auto-closed; a model 404 files ONE signal across ten probe cycles; a measured `ok` re-arms it; a CANNOT-ASSESS does
//! not. DESIGN §3 alert: crossing-dedup keyed `alert.<key>` in the same bucket, holding the last alerted state; only a
//! measured `ok` clears it, a CANNOT-ASSESS never does; sinks `nusy-kanban create --tags … --body-file - signal
//! "<title>"` (flags before positionals), `yurtle-kanban create signal "<title>" --push --body-file -`, a JSON
//! webhook, and stdout. §2: every reader applies the freshness rule (absent/expired is UNKNOWN, never a clear; bus
//! unreachable is CANNOT-ASSESS rc 2). §6: no secret leaves the process. §10 Q8: signal, tag `provider-status`.
//!
//! Seams asserted (EXP-004 test partner):
//! - `quotabus [--config <toml>] alert` reads the configured (service, slot) keys from the backend `[bus]` / `[file]`
//!   names, files each crossing to every configured `[alert.*]` sink and prints it on stdout, and exits 0 when every
//!   sink it called succeeded; rc 2 when the store cannot be read.
//! - The dedup state is the store entry `alert.<key>` (file backend: `<dir>/alert.<key>.json`; bus: key `alert.<key>`
//!   in the configured bucket). Its value is a JSON object whose `state` is the last alerted state (snake_case).
//!   After a measured `ok` it is either gone or holds `"state": "ok"`.
//! - A kanban sink's `command` is run as given (the tests give an absolute path to a FAKE that records its argv and
//!   stdin and never touches a board; PATH holds no real kanban binary).
//! - The webhook is a POST of a JSON body naming the key and the state.
//! - A sink that fails leaves the crossing un-alerted, so the next run files again.
//!
//! Not asserted (the design does not say): a crossing from one bad state to another bad state; what a partial failure
//! across several sinks does.

mod common;

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Output;
use std::time::Duration;

use chrono::Utc;
use common::{FAKE_PLAIN, FAKE_SK, NatsServer, leaks, rc, run_cli, text};
use quotabus::{Backend, FileBackend, NatsKv, Record, State};
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

const T: Duration = Duration::from_secs(60);
/// The row key of the kimi service's one model (`kimi-k2.5` slugs to `kimi-k2-5`).
const KEY: &str = "api.moonshot.nusy-product-team.kimi-k2-5";
const SECRET_ENV: &str = "QB_KIMI";

// ── fakes ────────────────────────────────────────────────────────────────────────────────────────────────────────

/// One invocation of a fake kanban binary.
#[derive(Debug, Clone)]
struct Call {
    argv: Vec<String>,
    stdin: String,
}

/// Fake `nusy-kanban` and `yurtle-kanban` executables that record every call (argv NUL-separated, stdin) under `log`
/// and exit 1 while `<log>/<name>.fail` exists. Their paths are baked into the scripts, so they record even when
/// quotabus clears the child's environment.
struct Fakes {
    _dir: tempfile::TempDir,
    bin: PathBuf,
    log: PathBuf,
}

impl Fakes {
    fn new() -> Fakes {
        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("bin");
        let log = dir.path().join("log");
        std::fs::create_dir_all(&bin).unwrap();
        std::fs::create_dir_all(&log).unwrap();
        for name in ["nusy-kanban", "yurtle-kanban"] {
            let script = format!(
                "#!/bin/sh\n\
                 PATH=/usr/bin:/bin; export PATH\n\
                 f=$(mktemp '{log}/{name}.XXXXXX') || exit 99\n\
                 printf '%s\\0' \"$@\" > \"$f.argv\"\n\
                 cat > \"$f.stdin\"\n\
                 if [ -e '{log}/{name}.fail' ]; then echo 'fake {name}: refused' >&2; exit 1; fi\n\
                 echo 'Created SIG-999'\n\
                 exit 0\n",
                log = log.display(),
            );
            let p = bin.join(name);
            std::fs::write(&p, script).unwrap();
            std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        Fakes {
            _dir: dir,
            bin,
            log,
        }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.bin.join(name)
    }

    fn calls(&self, name: &str) -> Vec<Call> {
        let mut out = Vec::new();
        for e in std::fs::read_dir(&self.log).unwrap() {
            let p = e.unwrap().path();
            let n = p.file_name().unwrap().to_string_lossy().into_owned();
            if n.starts_with(&format!("{name}.")) && n.ends_with(".argv") {
                let raw = std::fs::read(&p).unwrap();
                let argv = raw
                    .split(|b| *b == 0)
                    .filter(|a| !a.is_empty())
                    .map(|a| String::from_utf8_lossy(a).into_owned())
                    .collect();
                let stdin = std::fs::read_to_string(p.with_extension("stdin")).unwrap_or_default();
                out.push(Call { argv, stdin });
            }
        }
        out
    }

    fn set_fail(&self, name: &str, fail: bool) {
        let p = self.log.join(format!("{name}.fail"));
        if fail {
            std::fs::write(p, "").unwrap();
        } else {
            let _ = std::fs::remove_file(p);
        }
    }
}

#[test]
fn control_the_fakes_record_argv_and_stdin_and_can_fail() {
    // the harness can see a call, its argv and stdin, and a failure — so a count of 0 or 1 below means something
    let f = Fakes::new();
    for (fail, want) in [(false, 0), (true, 1)] {
        f.set_fail("nusy-kanban", fail);
        let mut child = std::process::Command::new(f.path("nusy-kanban"))
            .args(["create", "--tags", "a,b", "signal", "a title with spaces"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .env_clear()
            .spawn()
            .unwrap();
        use std::io::Write;
        child.stdin.take().unwrap().write_all(b"body\n").unwrap();
        assert_eq!(child.wait().unwrap().code(), Some(want));
    }
    let calls = f.calls("nusy-kanban");
    assert_eq!(calls.len(), 2);
    assert!(calls.iter().all(|c| c.argv
        == ["create", "--tags", "a,b", "signal", "a title with spaces"]
        && c.stdin == "body\n"));
    assert!(f.calls("yurtle-kanban").is_empty());
}

// ── config and runs ──────────────────────────────────────────────────────────────────────────────────────────────

/// The kimi service (probed at `<stub>/kimi`), `[ttl] api = 2m`, the store section, and the sinks.
fn write_config(dir: &Path, store: &str, sinks: &str, stub: &str) -> PathBuf {
    let cfg = dir.join("quotabus.toml");
    std::fs::write(
        &cfg,
        format!(
            "{store}\n[ttl]\napi = \"2m\"\nbalance = \"2m\"\n\n\
             [[service]]\nid = \"kimi\"\nkind = \"api\"\nprovider = \"moonshot\"\nfamily = \"moonshot\"\n\
             account = \"nusy-product-team\"\nbase_url = \"{stub}/kimi\"\nprotocol = \"anthropic\"\n\
             models = [\"kimi-k2.5\"]\nroles = [\"review\"]\nsecret = \"{SECRET_ENV}\"\n\n{sinks}"
        ),
    )
    .unwrap();
    cfg
}

fn kanban_sinks(f: &Fakes) -> String {
    format!(
        "[alert.nusy-kanban]\ncommand = {:?}\nitem_type = \"signal\"\ntags = [\"provider-status\", \"infra\"]\n\n\
         [alert.yurtle-kanban]\ncommand = {:?}\nitem_type = \"signal\"\ntags = [\"provider-status\"]\n",
        f.path("nusy-kanban").display().to_string(),
        f.path("yurtle-kanban").display().to_string(),
    )
}

fn webhook_sink(stub: &str) -> String {
    format!("[alert.webhook]\nurl = \"{stub}/hook\"\n")
}

fn file_store(dir: &Path) -> (String, PathBuf) {
    let state = dir.join("state");
    (
        format!("[file]\ndir = {:?}\n", state.display().to_string()),
        state,
    )
}

/// The environment of every run: no real kanban on PATH, a fake key for the probe and the redactor.
fn envs() -> Vec<(&'static str, &'static str)> {
    vec![("PATH", "/usr/bin:/bin"), (SECRET_ENV, FAKE_PLAIN)]
}

async fn cli(cfg: &Path, args: &'static [&'static str]) -> Output {
    let c = cfg.to_path_buf();
    tokio::task::spawn_blocking(move || run_cli(&c, args, &envs(), T))
        .await
        .unwrap()
}

async fn alert(cfg: &Path) -> Output {
    let out = cli(cfg, &["alert"]).await;
    let all = text(&out);
    assert!(
        !all.contains("not yet implemented") || rc(&out) == 0,
        "quotabus alert is still the stub: {all}"
    );
    out
}

/// A fresh row for KEY in `state` (CANNOT-ASSESS when `State::Unknown`).
fn row(state: State) -> Record {
    common::record(KEY, state, Utc::now(), 600)
}

/// An expired row: checked 2 h ago with a 60 s TTL.
fn expired(state: State) -> Record {
    common::record(KEY, state, Utc::now() - chrono::Duration::hours(2), 60)
}

fn alert_file(state_dir: &Path) -> PathBuf {
    state_dir.join(format!("alert.{KEY}.json"))
}

/// The `state` held at `alert.<KEY>` in the file backend, or None when there is no entry.
fn alerted_state_file(state_dir: &Path) -> Option<String> {
    let bytes = std::fs::read(alert_file(state_dir)).ok()?;
    let v: serde_json::Value =
        serde_json::from_slice(&bytes).unwrap_or_else(|e| panic!("alert.{KEY} is not JSON: {e}"));
    Some(
        v.get("state")
            .and_then(|s| s.as_str())
            .unwrap_or_else(|| panic!("alert.{KEY} holds no `state`: {v}"))
            .to_string(),
    )
}

/// The entry is cleared: gone, or holding `ok`.
fn is_cleared(state: Option<&str>) -> bool {
    matches!(state, None | Some("ok"))
}

fn total(f: &Fakes) -> usize {
    f.calls("nusy-kanban").len() + f.calls("yurtle-kanban").len()
}

// ── the Definition of Done, through real probe cycles ────────────────────────────────────────────────────────────

/// Answer every probe with `status` and `body`.
async fn answer(stub: &MockServer, status: u16, body: serde_json::Value) {
    stub.reset().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(status).set_body_json(body))
        .mount(stub)
        .await;
}

async fn model_404(stub: &MockServer) {
    answer(
        stub,
        404,
        serde_json::json!({"error": {"type": "not_found_error", "message": "model kimi-k2.5 not found"}}),
    )
    .await;
}

async fn model_ok(stub: &MockServer) {
    answer(stub, 200, serde_json::json!({"content": [{"text": "ok"}]})).await;
}

/// A 404 that does not name the model: the probe writes CANNOT-ASSESS (`cannot_assess:not_found`).
async fn cannot_assess(stub: &MockServer) {
    answer(stub, 404, serde_json::json!({"error": "page not found"})).await;
}

/// One probe cycle and the alert after it; returns the alert's output.
async fn cycle(cfg: &Path, state_dir: &Path, want: State) -> Output {
    let p = cli(cfg, &["probe", "--force"]).await;
    assert_eq!(rc(&p), 0, "probe: {}", text(&p));
    let r = FileBackend::new(state_dir)
        .get(KEY)
        .await
        .unwrap()
        .expect("the probe wrote the row");
    assert_eq!(r.state, want, "the probe measured {want:?}");
    let a = alert(cfg).await;
    assert_eq!(rc(&a), 0, "alert: {}", text(&a));
    a
}

#[tokio::test(flavor = "multi_thread")]
async fn a_model_404_files_one_signal_across_ten_probe_cycles() {
    let f = Fakes::new();
    let probe_stub = MockServer::start().await;
    let hook = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&hook)
        .await;
    model_404(&probe_stub).await;
    let dir = tempfile::tempdir().unwrap();
    let (store, state_dir) = file_store(dir.path());
    let sinks = format!("{}\n{}", kanban_sinks(&f), webhook_sink(&hook.uri()));
    let cfg = write_config(dir.path(), &store, &sinks, &probe_stub.uri());

    let mut outs = Vec::new();
    for _ in 0..10 {
        outs.push(cycle(&cfg, &state_dir, State::ModelMissing).await);
    }
    assert_eq!(
        f.calls("nusy-kanban").len(),
        1,
        "ONE nusy-kanban signal across ten cycles"
    );
    assert_eq!(
        f.calls("yurtle-kanban").len(),
        1,
        "ONE yurtle-kanban signal across ten cycles"
    );
    assert_eq!(
        hook.received_requests().await.unwrap().len(),
        1,
        "ONE webhook POST across ten cycles"
    );
    assert_eq!(
        alerted_state_file(&state_dir).as_deref(),
        Some("model_missing")
    );

    // stdout is a sink too: the first run names the crossing, in plain text
    let first = String::from_utf8_lossy(&outs[0].stdout).into_owned();
    assert!(
        first.contains(KEY),
        "stdout names the crossing's key: {first:?}"
    );
    assert!(
        first.contains("model_missing"),
        "stdout names the state: {first:?}"
    );
    assert!(
        !first.contains('\u{1b}'),
        "stdout is plain (no escapes): {first:?}"
    );
    assert!(
        !first.trim_start().starts_with('{') && !first.trim_start().starts_with('['),
        "stdout is plain text, not JSON: {first:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_measured_ok_re_arms_it_and_the_next_crossing_files_again() {
    let f = Fakes::new();
    let probe_stub = MockServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let (store, state_dir) = file_store(dir.path());
    let cfg = write_config(dir.path(), &store, &kanban_sinks(&f), &probe_stub.uri());

    model_404(&probe_stub).await;
    cycle(&cfg, &state_dir, State::ModelMissing).await;
    cycle(&cfg, &state_dir, State::ModelMissing).await;
    assert_eq!(f.calls("nusy-kanban").len(), 1);

    model_ok(&probe_stub).await;
    cycle(&cfg, &state_dir, State::Ok).await;
    assert_eq!(
        f.calls("nusy-kanban").len(),
        1,
        "a recovery files nothing (and closes nothing)"
    );
    assert!(
        is_cleared(alerted_state_file(&state_dir).as_deref()),
        "a measured ok clears alert.<key>: {:?}",
        alerted_state_file(&state_dir)
    );

    model_404(&probe_stub).await;
    cycle(&cfg, &state_dir, State::ModelMissing).await;
    assert_eq!(
        f.calls("nusy-kanban").len(),
        2,
        "the next crossing files again"
    );
    assert_eq!(f.calls("yurtle-kanban").len(), 2);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_cannot_assess_does_not_clear_it() {
    let f = Fakes::new();
    let probe_stub = MockServer::start().await;
    let dir = tempfile::tempdir().unwrap();
    let (store, state_dir) = file_store(dir.path());
    let cfg = write_config(dir.path(), &store, &kanban_sinks(&f), &probe_stub.uri());

    model_404(&probe_stub).await;
    cycle(&cfg, &state_dir, State::ModelMissing).await;
    cannot_assess(&probe_stub).await;
    for _ in 0..3 {
        cycle(&cfg, &state_dir, State::Unknown).await;
    }
    assert_eq!(
        alerted_state_file(&state_dir).as_deref(),
        Some("model_missing"),
        "a CANNOT-ASSESS leaves the alerted state as it was"
    );
    model_404(&probe_stub).await;
    cycle(&cfg, &state_dir, State::ModelMissing).await;
    assert_eq!(
        f.calls("nusy-kanban").len(),
        1,
        "the 404 after a CANNOT-ASSESS is the SAME crossing: nothing new is filed"
    );
    assert_eq!(f.calls("yurtle-kanban").len(), 1);
}

// ── the same rules on rows written directly (file backend) ──────────────────────────────────────────────────────

struct FileRig {
    f: Fakes,
    _dir: tempfile::TempDir,
    cfg: PathBuf,
    state_dir: PathBuf,
    store: FileBackend,
}

fn file_rig(extra_sinks: &str) -> FileRig {
    let f = Fakes::new();
    let dir = tempfile::tempdir().unwrap();
    let (store, state_dir) = file_store(dir.path());
    let sinks = format!("{}\n{extra_sinks}", kanban_sinks(&f));
    let cfg = write_config(dir.path(), &store, &sinks, "http://127.0.0.1:1");
    FileRig {
        f,
        _dir: dir,
        cfg,
        store: FileBackend::new(&state_dir),
        state_dir,
    }
}

async fn put_and_alert(rig: &FileRig, r: &Record) -> Output {
    rig.store.put(r).await.unwrap();
    let out = alert(&rig.cfg).await;
    assert_eq!(rc(&out), 0, "alert: {}", text(&out));
    out
}

#[tokio::test(flavor = "multi_thread")]
async fn control_removing_the_alert_entry_between_cycles_files_again() {
    // the ONE-signal count can fail, and the dedup is held at alert.<key> in the store (not anywhere else)
    let rig = file_rig("");
    put_and_alert(&rig, &row(State::ModelMissing)).await;
    assert!(
        alert_file(&rig.state_dir).exists(),
        "the dedup state is at alert.<key>"
    );
    std::fs::remove_file(alert_file(&rig.state_dir)).unwrap();
    put_and_alert(&rig, &row(State::ModelMissing)).await;
    assert_eq!(
        rig.f.calls("nusy-kanban").len(),
        2,
        "without alert.<key> the crossing is filed again"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_expired_or_absent_row_is_unknown_and_does_not_clear_it() {
    let rig = file_rig("");
    put_and_alert(&rig, &row(State::ModelMissing)).await;
    // expired, even with state ok on it: the freshness rule makes it UNKNOWN, not a measured ok
    put_and_alert(&rig, &expired(State::Ok)).await;
    assert_eq!(
        alerted_state_file(&rig.state_dir).as_deref(),
        Some("model_missing"),
        "an expired ok is not a measured ok"
    );
    // absent
    std::fs::remove_file(rig.state_dir.join(format!("{KEY}.json"))).unwrap();
    let out = alert(&rig.cfg).await;
    assert_eq!(rc(&out), 0, "{}", text(&out));
    assert_eq!(
        alerted_state_file(&rig.state_dir).as_deref(),
        Some("model_missing")
    );
    put_and_alert(&rig, &row(State::ModelMissing)).await;
    assert_eq!(rig.f.calls("nusy-kanban").len(), 1, "UNKNOWN never re-arms");

    // control: the same sequence with a FRESH ok in the middle does re-arm
    let rig = file_rig("");
    put_and_alert(&rig, &row(State::ModelMissing)).await;
    put_and_alert(&rig, &row(State::Ok)).await;
    put_and_alert(&rig, &row(State::ModelMissing)).await;
    assert_eq!(rig.f.calls("nusy-kanban").len(), 2);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_healthy_or_cannot_assess_first_row_files_nothing() {
    let rig = file_rig("");
    put_and_alert(&rig, &row(State::Ok)).await;
    put_and_alert(&rig, &row(State::Unknown)).await;
    assert_eq!(total(&rig.f), 0, "only a service going bad is a crossing");
    // control: the same rig files the moment the row goes bad
    put_and_alert(&rig, &row(State::QuotaExhausted)).await;
    assert_eq!(rig.f.calls("nusy-kanban").len(), 1);
}

// ── the sinks' shape ─────────────────────────────────────────────────────────────────────────────────────────────

/// The value of `--flag v` or `--flag=v`.
fn flag_value<'a>(argv: &'a [String], flag: &str) -> Option<&'a str> {
    argv.iter().enumerate().find_map(|(i, a)| {
        if a == flag {
            argv.get(i + 1).map(String::as_str)
        } else {
            a.strip_prefix(&format!("{flag}="))
        }
    })
}

fn tags_of(argv: &[String]) -> Vec<String> {
    flag_value(argv, "--tags")
        .map(|v| v.split(',').map(|t| t.trim().to_string()).collect())
        .unwrap_or_default()
}

#[tokio::test(flavor = "multi_thread")]
async fn each_kanban_sink_creates_a_signal_tagged_provider_status_with_the_body_on_stdin() {
    let rig = file_rig("");
    put_and_alert(&rig, &row(State::ModelMissing)).await;

    let nk = rig.f.calls("nusy-kanban");
    assert_eq!(nk.len(), 1);
    let a = &nk[0].argv;
    assert_eq!(
        a.first().map(String::as_str),
        Some("create"),
        "nusy-kanban create …: {a:?}"
    );
    let ty = a
        .iter()
        .position(|x| x == "signal")
        .unwrap_or_else(|| panic!("item type signal: {a:?}"));
    // flags before positionals (`nusy-kanban create --help`): every --flag precedes `signal "<title>"`
    let last_flag = a.iter().rposition(|x| x.starts_with("--")).unwrap_or(0);
    assert!(
        last_flag < ty,
        "nusy-kanban flags come before the positionals: {a:?}"
    );
    assert_eq!(a.len(), ty + 2, "`signal` then the title, last: {a:?}");
    assert!(!a[ty + 1].trim().is_empty(), "a title");
    assert!(
        tags_of(a).contains(&"provider-status".to_string()),
        "--tags carries provider-status: {a:?}"
    );
    assert_eq!(
        flag_value(a, "--body-file"),
        Some("-"),
        "the body on stdin: {a:?}"
    );

    let yk = rig.f.calls("yurtle-kanban");
    assert_eq!(yk.len(), 1);
    let a = &yk[0].argv;
    assert_eq!(
        a.first().map(String::as_str),
        Some("create"),
        "yurtle-kanban create …: {a:?}"
    );
    assert_eq!(
        a.get(1).map(String::as_str),
        Some("signal"),
        "create signal \"<title>\": {a:?}"
    );
    assert!(
        a.get(2)
            .is_some_and(|t| !t.starts_with("--") && !t.trim().is_empty()),
        "a title: {a:?}"
    );
    assert!(
        a.iter().any(|x| x == "--push"),
        "yurtle-kanban create takes --push: {a:?}"
    );
    assert!(
        !a.iter()
            .any(|x| x == "--no-push" || x == "--no-commit" || x == "--assign"),
        "never a local-only or assigned write: {a:?}"
    );
    assert!(tags_of(a).contains(&"provider-status".to_string()), "{a:?}");
    assert_eq!(flag_value(a, "--body-file"), Some("-"), "{a:?}");

    for c in nk.iter().chain(&yk) {
        assert!(
            c.stdin.contains(KEY),
            "the body names the key: {:?}",
            c.stdin
        );
        assert!(
            c.stdin.contains("model_missing"),
            "the body names the state: {:?}",
            c.stdin
        );
        assert!(
            c.argv.iter().any(|x| x.contains("kimi")),
            "the title names the model or service: {:?}",
            c.argv
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn the_webhook_gets_one_json_post_naming_the_key_and_state() {
    let hook = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&hook)
        .await;
    let rig = file_rig(&webhook_sink(&hook.uri()));
    for _ in 0..3 {
        put_and_alert(&rig, &row(State::ModelMissing)).await;
    }
    let got = hook.received_requests().await.unwrap();
    assert_eq!(got.len(), 1, "one POST per crossing");
    assert_eq!(got[0].method.as_str(), "POST");
    assert_eq!(got[0].url.path(), "/hook");
    let body: serde_json::Value =
        serde_json::from_slice(&got[0].body).expect("the webhook body is JSON");
    let s = body.to_string();
    assert!(
        s.contains(KEY) && s.contains("model_missing"),
        "names key and state: {s}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn the_alert_never_auto_closes_anything() {
    let rig = file_rig("");
    for st in [
        State::ModelMissing,
        State::ModelMissing,
        State::Ok,
        State::Ok,
        State::Unknown,
        State::ModelMissing,
        State::Ok,
    ] {
        put_and_alert(&rig, &row(st)).await;
    }
    let calls: Vec<Call> = rig
        .f
        .calls("nusy-kanban")
        .into_iter()
        .chain(rig.f.calls("yurtle-kanban"))
        .collect();
    // two crossings, two boards: four creates, and nothing that is not a create (no close, move, done, update)
    assert_eq!(calls.len(), 4, "{calls:?}");
    for c in &calls {
        assert_eq!(
            c.argv.first().map(String::as_str),
            Some("create"),
            "only create, never close/move: {:?}",
            c.argv
        );
    }
}

// ── failure, secrets, the store ──────────────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn a_failed_kanban_sink_leaves_the_crossing_unalerted_so_it_retries() {
    let rig = file_rig("");
    rig.f.set_fail("nusy-kanban", true);
    rig.f.set_fail("yurtle-kanban", true);
    rig.store.put(&row(State::ModelMissing)).await.unwrap();
    let _ = alert(&rig.cfg).await;
    assert_eq!(rig.f.calls("nusy-kanban").len(), 1, "it tried");
    assert!(
        alerted_state_file(&rig.state_dir).as_deref() != Some("model_missing"),
        "a failed sink must not mark the crossing as alerted"
    );

    rig.f.set_fail("nusy-kanban", false);
    rig.f.set_fail("yurtle-kanban", false);
    put_and_alert(&rig, &row(State::ModelMissing)).await;
    assert_eq!(
        rig.f.calls("nusy-kanban").len(),
        2,
        "the next run retries the crossing"
    );
    assert_eq!(
        alerted_state_file(&rig.state_dir).as_deref(),
        Some("model_missing")
    );
    put_and_alert(&rig, &row(State::ModelMissing)).await;
    assert_eq!(
        rig.f.calls("nusy-kanban").len(),
        2,
        "once filed, it is not filed again"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_failed_webhook_leaves_the_crossing_unalerted_so_it_retries() {
    let hook = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(500))
        .up_to_n_times(1)
        .with_priority(1)
        .mount(&hook)
        .await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .with_priority(2)
        .mount(&hook)
        .await;
    let dir = tempfile::tempdir().unwrap();
    let (store, state_dir) = file_store(dir.path());
    let cfg = write_config(
        dir.path(),
        &store,
        &webhook_sink(&hook.uri()),
        "http://127.0.0.1:1",
    );
    let backend = FileBackend::new(&state_dir);

    backend.put(&row(State::ModelMissing)).await.unwrap();
    let _ = alert(&cfg).await;
    assert_eq!(hook.received_requests().await.unwrap().len(), 1, "it tried");
    assert!(
        alerted_state_file(&state_dir).as_deref() != Some("model_missing"),
        "a 500 from the webhook must not mark the crossing as alerted"
    );
    for _ in 0..2 {
        backend.put(&row(State::ModelMissing)).await.unwrap();
        let out = alert(&cfg).await;
        assert_eq!(rc(&out), 0, "{}", text(&out));
    }
    assert_eq!(
        hook.received_requests().await.unwrap().len(),
        2,
        "retried once, then deduped"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn no_secret_reaches_argv_a_body_the_webhook_or_the_output() {
    let hook = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&hook)
        .await;
    let rig = file_rig(&webhook_sink(&hook.uri()));
    // a row carrying the configured key's value (QB_KIMI=FAKE_PLAIN, which matches no pattern) and an sk- key in its
    // error, as a buggy or older writer might leave it: the alert redacts before anything leaves the process
    let mut r = row(State::ModelMissing);
    r.error = Some(format!(
        "404 for key {FAKE_PLAIN} (Authorization: Bearer {FAKE_SK})"
    ));
    let out = put_and_alert(&rig, &r).await;
    let mut surfaces = vec![text(&out)];
    for c in rig
        .f
        .calls("nusy-kanban")
        .into_iter()
        .chain(rig.f.calls("yurtle-kanban"))
    {
        surfaces.push(c.argv.join(" "));
        surfaces.push(c.stdin);
    }
    for req in hook.received_requests().await.unwrap() {
        surfaces.push(String::from_utf8_lossy(&req.body).into_owned());
        surfaces.push(req.url.to_string());
    }
    assert_eq!(surfaces.len(), 7, "output + 2 calls × 2 + one webhook × 2");
    for s in &surfaces {
        let l = leaks(s, &[FAKE_PLAIN, FAKE_SK]);
        assert!(l.is_empty(), "a secret left the process: {l:?} in {s:?}");
    }
    // control: the leak check sees the planted value
    assert!(!leaks(r.error.as_deref().unwrap(), &[FAKE_PLAIN, FAKE_SK]).is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn the_alert_entry_does_not_read_as_a_broken_row() {
    // the dedup state shares the store with the rows: a reader's listing must not report alert.<key> as unreadable
    let rig = file_rig("");
    put_and_alert(&rig, &row(State::ModelMissing)).await;
    assert!(alert_file(&rig.state_dir).exists());
    let listing = rig.store.scan().await.unwrap();
    assert!(
        listing
            .unreadable
            .iter()
            .all(|(k, _)| !k.starts_with("alert.")),
        "alert entries are not broken rows: {:?}",
        listing.unreadable
    );
    let keys: Vec<&str> = listing.rows.iter().map(|r| r.key.as_str()).collect();
    assert_eq!(keys, [KEY], "only the status row is a row");
}

#[tokio::test(flavor = "multi_thread")]
async fn an_unreadable_store_is_cannot_assess_rc_2_and_files_nothing() {
    let f = Fakes::new();
    let dir = tempfile::tempdir().unwrap();
    let port = common::closed_port();
    let store = format!("[bus]\nurl = \"nats://127.0.0.1:{port}\"\nbucket = \"ai_status\"\n");
    let cfg = write_config(dir.path(), &store, &kanban_sinks(&f), "http://127.0.0.1:1");
    let out = alert(&cfg).await;
    assert_eq!(
        rc(&out),
        2,
        "bus down is CANNOT-ASSESS rc 2: {}",
        text(&out)
    );
    assert_eq!(total(&f), 0, "nothing is filed when nothing could be read");
}

// ── the bus backend: a throwaway nats-server ─────────────────────────────────────────────────────────────────────

const BUCKET: &str = "qb_exp004_alert";

async fn alerted_state_bus(url: &str) -> Option<String> {
    let js = async_nats::jetstream::new(async_nats::connect(url).await.unwrap());
    let kv = js.get_key_value(BUCKET).await.unwrap();
    let bytes = kv.get(format!("alert.{KEY}")).await.unwrap()?;
    let v: serde_json::Value = serde_json::from_slice(&bytes)
        .unwrap_or_else(|e| panic!("alert.{KEY} on the bus is not JSON: {e}"));
    Some(v["state"].as_str().expect("a `state` field").to_string())
}

async fn delete_alert_bus(url: &str) {
    let js = async_nats::jetstream::new(async_nats::connect(url).await.unwrap());
    let kv = js.get_key_value(BUCKET).await.unwrap();
    kv.purge(format!("alert.{KEY}")).await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn on_the_bus_the_dedup_state_is_alert_key_in_the_same_bucket() {
    let srv = NatsServer::start();
    let f = Fakes::new();
    let dir = tempfile::tempdir().unwrap();
    let store = format!("[bus]\nurl = {:?}\nbucket = \"{BUCKET}\"\n", srv.url());
    let cfg = write_config(dir.path(), &store, &kanban_sinks(&f), "http://127.0.0.1:1");
    let kv = tokio::time::timeout(T, NatsKv::connect_publisher(&srv.url(), BUCKET))
        .await
        .unwrap()
        .unwrap();

    for _ in 0..10 {
        kv.put(&row(State::ModelMissing)).await.unwrap();
        let out = alert(&cfg).await;
        assert_eq!(rc(&out), 0, "{}", text(&out));
    }
    assert_eq!(
        f.calls("nusy-kanban").len(),
        1,
        "ONE signal across ten cycles, on the bus"
    );
    assert_eq!(
        alerted_state_bus(&srv.url()).await.as_deref(),
        Some("model_missing")
    );

    // CANNOT-ASSESS does not clear it
    kv.put(&row(State::Unknown)).await.unwrap();
    alert(&cfg).await;
    assert_eq!(
        alerted_state_bus(&srv.url()).await.as_deref(),
        Some("model_missing")
    );
    kv.put(&row(State::ModelMissing)).await.unwrap();
    alert(&cfg).await;
    assert_eq!(f.calls("nusy-kanban").len(), 1);

    // a measured ok re-arms it
    kv.put(&row(State::Ok)).await.unwrap();
    alert(&cfg).await;
    assert!(is_cleared(alerted_state_bus(&srv.url()).await.as_deref()));
    kv.put(&row(State::ModelMissing)).await.unwrap();
    alert(&cfg).await;
    assert_eq!(f.calls("nusy-kanban").len(), 2, "re-armed, filed again");
}

#[tokio::test(flavor = "multi_thread")]
async fn control_on_the_bus_purging_alert_key_files_again() {
    let srv = NatsServer::start();
    let f = Fakes::new();
    let dir = tempfile::tempdir().unwrap();
    let store = format!("[bus]\nurl = {:?}\nbucket = \"{BUCKET}\"\n", srv.url());
    let cfg = write_config(dir.path(), &store, &kanban_sinks(&f), "http://127.0.0.1:1");
    let kv = tokio::time::timeout(T, NatsKv::connect_publisher(&srv.url(), BUCKET))
        .await
        .unwrap()
        .unwrap();
    kv.put(&row(State::ModelMissing)).await.unwrap();
    assert_eq!(rc(&alert(&cfg).await), 0);
    assert!(alerted_state_bus(&srv.url()).await.is_some());
    delete_alert_bus(&srv.url()).await;
    kv.put(&row(State::ModelMissing)).await.unwrap();
    assert_eq!(rc(&alert(&cfg).await), 0);
    assert_eq!(
        f.calls("nusy-kanban").len(),
        2,
        "the dedup is the bucket's alert.<key>, nothing else"
    );
}
