//! CHORE-021 (Captain 2026-10-09: "No alert for local (Recommended)"): a service with `kind = "local"` (the Sparks,
//! DGX1/DGX2 Qwen) files no alert, whatever its state, and its row neither arms nor clears an alert. Its rows are
//! still published, and `select` still refuses a local row that is not `ok`. API (and subscription) rows are
//! unchanged: SIG-010's default `[alert] states` still files for them.
//!
//! Seams fixed here (test partner, for the implementer) — CLI level only, nothing about how it is built:
//! - `quotabus alert` over the file backend, with a recording fake `nusy-kanban` sink (no board).
//! - "Does not arm or clear": after any sequence of local readings no `alert.<local key>` entry is written; an entry
//!   already there (written before this change) is left byte-for-byte as it was, and `alert` prints no `ALERT` or
//!   `CLEARED` line naming the local key.
//!
//! Fakes only: the file backend in a temp dir, a fake sink, a closed loopback port, a fake key. No bus, no real key.

mod common;

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Output;
use std::time::Duration;

use chrono::Utc;
use common::{FAKE_PLAIN, rc, run_cli, text};
use quotabus::{Backend, FileBackend, Headroom, Kind, Record, State, record_key};

const T: Duration = Duration::from_secs(60);
const API_SECRET: &str = "QB_KIMI";
const NEAR: &str = "window_near_limit";
const RULING: &str = "No alert for local (Recommended)";

fn local_key() -> String {
    record_key(Kind::Local, "qwen", "dgx1", "qwen3")
}
fn api_key() -> String {
    record_key(Kind::Api, "moonshot", "nusy-product-team", "kimi-k2.5")
}

// ── the rig: one config with a local service and an API service, one fake sink ─────────────────────────────────

/// A fake `nusy-kanban` that records each call's argv (NUL-separated) as a file under `log`.
struct Fake {
    _dir: tempfile::TempDir,
    bin: PathBuf,
    log: PathBuf,
}

impl Fake {
    fn new() -> Fake {
        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("nusy-kanban");
        let log = dir.path().join("log");
        std::fs::create_dir_all(&log).unwrap();
        let script = format!(
            "#!/bin/sh\nPATH=/usr/bin:/bin; export PATH\n\
             f=$(mktemp '{log}/call.XXXXXX') || exit 99\n\
             printf '%s\\0' \"$@\" > \"$f\"\ncat > /dev/null\necho 'Created SIG-999'\nexit 0\n",
            log = log.display()
        );
        std::fs::write(&bin, script).unwrap();
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
        Fake {
            _dir: dir,
            bin,
            log,
        }
    }

    /// The argv of every call, joined with spaces.
    fn calls(&self) -> Vec<String> {
        std::fs::read_dir(&self.log)
            .unwrap()
            .map(|e| {
                let b = std::fs::read(e.unwrap().path()).unwrap();
                String::from_utf8_lossy(&b).replace('\0', " ")
            })
            .collect()
    }

    /// Calls whose title names service `id` (the title is `provider-status: <id> <model> is <state>`).
    fn filed_for(&self, id: &str) -> usize {
        let needle = format!("provider-status: {id} ");
        self.calls().iter().filter(|c| c.contains(&needle)).count()
    }
}

struct Rig {
    fake: Fake,
    _dir: tempfile::TempDir,
    cfg: PathBuf,
    state_dir: PathBuf,
    store: FileBackend,
}

/// `alert_table` is `""` (no `[alert] states`: SIG-010's default) or an `[alert]` table. `local_url` / `api_url` are
/// the services' base URLs (only `probe` uses them).
fn rig_with(alert_table: &str, local_url: &str, api_url: &str) -> Rig {
    let fake = Fake::new();
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    let cfg = dir.path().join("quotabus.toml");
    std::fs::write(
        &cfg,
        format!(
            "[file]\ndir = {:?}\n\n[ttl]\napi = \"2m\"\nbalance = \"2m\"\n\n\
             [[service]]\nid = \"qwen\"\nkind = \"local\"\nprovider = \"qwen\"\nfamily = \"qwen\"\n\
             account = \"dgx1\"\nbase_url = \"{local_url}\"\nprotocol = \"openai\"\n\
             models = [\"qwen3\"]\nroles = [\"review\"]\ncost_class = \"local\"\n\n\
             [[service]]\nid = \"kimi\"\nkind = \"api\"\nprovider = \"moonshot\"\nfamily = \"moonshot\"\n\
             account = \"nusy-product-team\"\nbase_url = \"{api_url}\"\nprotocol = \"anthropic\"\n\
             models = [\"kimi-k2.5\"]\nroles = [\"review\"]\ncost_class = \"metered\"\nsecret = \"{API_SECRET}\"\n\n\
             {alert_table}\n\
             [alert.nusy-kanban]\ncommand = {:?}\nitem_type = \"signal\"\ntags = [\"provider-status\"]\n",
            state_dir.display().to_string(),
            fake.bin.display().to_string(),
        ),
    )
    .unwrap();
    Rig {
        fake,
        _dir: dir,
        cfg,
        store: FileBackend::new(&state_dir),
        state_dir,
    }
}

fn rig(alert_table: &str) -> Rig {
    rig_with(
        alert_table,
        "http://127.0.0.1:1/v1",
        "http://127.0.0.1:1/kimi",
    )
}

async fn cli(rig: &Rig, args: &'static [&'static str]) -> Output {
    let c = rig.cfg.clone();
    tokio::task::spawn_blocking(move || {
        run_cli(
            &c,
            args,
            &[("PATH", "/usr/bin:/bin"), (API_SECRET, FAKE_PLAIN)],
            T,
        )
    })
    .await
    .unwrap()
}

/// A fresh reading.
#[derive(Debug, Clone, Copy, PartialEq)]
enum R {
    Ok,
    QuotaExhausted,
    AuthFailed,
    ModelMissing,
    RateLimited,
    /// `unknown` + `cannot_assess:unreachable`.
    Unreachable,
    /// `degraded`, a slow 200.
    Latency,
    /// `degraded` + reason `window_near_limit`.
    Near,
}

const EVERY_BAD: [R; 7] = [
    R::QuotaExhausted,
    R::AuthFailed,
    R::ModelMissing,
    R::RateLimited,
    R::Unreachable,
    R::Latency,
    R::Near,
];

fn row(key: &str, r: R) -> Record {
    let state = match r {
        R::Ok => State::Ok,
        R::QuotaExhausted => State::QuotaExhausted,
        R::AuthFailed => State::AuthFailed,
        R::ModelMissing => State::ModelMissing,
        R::RateLimited => State::RateLimited,
        R::Unreachable => State::Unknown,
        R::Latency | R::Near => State::Degraded,
    };
    let mut rec = common::record(key, state, Utc::now(), 600);
    rec.reason = match r {
        R::Unreachable => Some("cannot_assess:unreachable".into()),
        R::Near => Some(NEAR.into()),
        _ => None,
    };
    match r {
        R::Latency => rec.latency_ms = Some(20_000),
        R::Near => {
            rec.headroom = Some(Headroom {
                window_pct: Some(97.0),
                window: Some("five_hour".into()),
                ..Headroom::default()
            })
        }
        _ => {}
    }
    rec
}

/// Put a fresh row for `key` reading `r`, then run `alert` once; returns its output.
async fn put_and_alert(rig: &Rig, key: &str, r: R) -> Output {
    rig.store.put(&row(key, r)).await.unwrap();
    let out = cli(rig, &["alert"]).await;
    assert_eq!(rc(&out), 0, "alert after {key} {r:?}: {}", text(&out));
    out
}

/// Feed the same sequence to the local and the API service, one `alert` per step; return (local filed, api filed).
async fn both(alert_table: &str, seq: &[R]) -> (usize, usize) {
    let rig = rig(alert_table);
    for r in seq {
        rig.store.put(&row(&local_key(), *r)).await.unwrap();
        rig.store.put(&row(&api_key(), *r)).await.unwrap();
        let out = cli(&rig, &["alert"]).await;
        assert_eq!(rc(&out), 0, "alert after {r:?}: {}", text(&out));
    }
    (rig.fake.filed_for("qwen"), rig.fake.filed_for("kimi"))
}

/// Feed `seq` to the local service only; return (filed for qwen, every stdout joined).
async fn local_only(alert_table: &str, seq: &[R]) -> (Rig, String) {
    let rig = rig(alert_table);
    let mut all = String::new();
    for r in seq {
        let out = put_and_alert(&rig, &local_key(), *r).await;
        all.push_str(&String::from_utf8_lossy(&out.stdout));
    }
    (rig, all)
}

fn alert_entry_path(rig: &Rig, key: &str) -> PathBuf {
    rig.state_dir.join(format!("alert.{key}.json"))
}

/// ok → bad → ok → bad … for `cycles` round trips, ending on ok.
fn flapping(bad: R, cycles: usize) -> Vec<R> {
    let mut v = vec![R::Ok];
    for _ in 0..cycles {
        v.push(bad);
        v.push(R::Ok);
    }
    v
}

#[test]
fn control_the_fixtures_are_what_they_say() {
    assert_eq!(local_key(), "local.qwen.dgx1.qwen3");
    assert!(api_key().starts_with("api.moonshot."));
    let u = row(&local_key(), R::Unreachable);
    assert_eq!(u.kind, Kind::Local);
    assert_eq!(u.state, State::Unknown);
    assert_eq!(u.reason.as_deref(), Some("cannot_assess:unreachable"));
    assert_eq!(row(&local_key(), R::Near).reason.as_deref(), Some(NEAR));
    assert_eq!(row(&api_key(), R::ModelMissing).kind, Kind::Api);
    assert_eq!(flapping(R::ModelMissing, 2).len(), 5);
}

// ── DoD 1 + 3: local files nothing; API unchanged ───────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn a_local_service_flapping_ok_unreachable_over_several_cycles_files_zero() {
    let (local, api) = both("", &flapping(R::Unreachable, 4)).await;
    assert_eq!(local, 0, "local ok <-> unreachable files nothing");
    // control in the same run: the API service fed the SAME crossings files once per crossing (SIG-010)
    assert_eq!(api, 4, "api ok <-> unreachable still files per crossing");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_local_service_flapping_ok_model_missing_over_several_cycles_files_zero() {
    let (local, api) = both("", &flapping(R::ModelMissing, 4)).await;
    assert_eq!(
        local, 0,
        "local ok <-> model_missing (a Spark switched models) files nothing"
    );
    assert_eq!(api, 4, "api ok <-> model_missing still files per crossing");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_local_service_mixing_unreachable_and_model_missing_files_zero() {
    let seq = [
        R::Ok,
        R::Unreachable,
        R::Ok,
        R::ModelMissing,
        R::Ok,
        R::Unreachable,
        R::ModelMissing,
        R::Ok,
        R::ModelMissing,
        R::Ok,
    ];
    let (local, api) = both("", &seq).await;
    assert_eq!(local, 0);
    // api: crossings at steps 2, 4, 6 (6→7 is the same outage), 9
    assert_eq!(api, 4);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_local_service_files_nothing_whatever_its_state_under_the_default() {
    for bad in EVERY_BAD {
        let (local, _) = both("", &[R::Ok, bad, R::Ok, bad]).await;
        assert_eq!(local, 0, "local {bad:?} filed under the default");
        let (local, _) = both("", &[bad]).await;
        assert_eq!(local, 0, "a first local reading of {bad:?} filed");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_local_service_files_nothing_even_when_every_state_is_listed() {
    let all = "[alert]\nstates = [\"quota_exhausted\", \"auth_failed\", \"model_missing\", \"rate_limited\", \
               \"degraded\", \"unreachable\", \"window_near_limit\"]\n";
    for bad in EVERY_BAD {
        let (local, api) = both(all, &[R::Ok, bad, R::Ok, bad]).await;
        assert_eq!(local, 0, "local {bad:?} filed with every state listed");
        // control: the API service with every state listed files each crossing
        assert_eq!(api, 2, "api {bad:?} with every state listed");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn api_rows_keep_sig_010s_default() {
    // what files for an API service is exactly CHORE-018's default, unchanged by this chore
    for bad in [
        R::QuotaExhausted,
        R::AuthFailed,
        R::ModelMissing,
        R::Unreachable,
        R::Near,
    ] {
        let (_, api) = both("", &[R::Ok, bad]).await;
        assert_eq!(api, 1, "api ok -> {bad:?} files ONE under the default");
    }
    for bad in [R::Latency, R::RateLimited] {
        let (_, api) = both("", &[R::Ok, bad, R::Ok, bad]).await;
        assert_eq!(
            api, 0,
            "api {bad:?} is not in the default and files nothing"
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_local_row_never_arms_an_alert_entry() {
    for bad in EVERY_BAD {
        let (rig, out) = local_only("", &[R::Ok, bad, bad, R::Ok, bad]).await;
        assert!(
            !alert_entry_path(&rig, &local_key()).exists(),
            "{bad:?}: alert.{} was written",
            local_key()
        );
        let lk = local_key();
        assert!(
            !out.lines()
                .any(|l| (l.starts_with("ALERT") || l.starts_with("CLEARED")) && l.contains(&lk)),
            "{bad:?}: alert printed a crossing or a clear for the local key:\n{out}"
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn control_an_api_row_does_arm_its_alert_entry() {
    // the absence above can fail: the same readings on the API key write `alert.<api key>`
    let rig = rig("");
    put_and_alert(&rig, &api_key(), R::Ok).await;
    put_and_alert(&rig, &api_key(), R::ModelMissing).await;
    assert!(alert_entry_path(&rig, &api_key()).exists());
    assert_eq!(rig.fake.filed_for("kimi"), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_local_ok_does_not_clear_an_entry_left_from_before() {
    // an `alert.<local key>` written before this change (the key alerted as model_missing) is left exactly as it was
    let rig = rig("");
    std::fs::create_dir_all(&rig.state_dir).unwrap();
    let entry = quotabus::alert::AlertEntry::new(
        &local_key(),
        State::ModelMissing,
        None,
        Utc::now() - chrono::Duration::hours(1),
    );
    let bytes = serde_json::to_vec(&entry).unwrap();
    let p = alert_entry_path(&rig, &local_key());
    std::fs::write(&p, &bytes).unwrap();
    for r in [R::Ok, R::ModelMissing, R::Ok, R::Unreachable, R::Ok] {
        let out = put_and_alert(&rig, &local_key(), r).await;
        let s = String::from_utf8_lossy(&out.stdout).into_owned();
        assert!(
            !s.lines()
                .any(|l| l.starts_with("CLEARED") && l.contains(&local_key())),
            "after {r:?}: a local row cleared the entry:\n{s}"
        );
    }
    assert_eq!(
        std::fs::read(&p).unwrap(),
        bytes,
        "the pre-existing local entry was rewritten"
    );
    assert_eq!(rig.fake.filed_for("qwen"), 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn control_an_api_ok_does_clear_an_entry_left_from_before() {
    // the "left as it was" above can fail: the same seeded entry on the API key is rewritten by a measured ok
    let rig = rig("");
    std::fs::create_dir_all(&rig.state_dir).unwrap();
    let entry = quotabus::alert::AlertEntry::new(
        &api_key(),
        State::ModelMissing,
        None,
        Utc::now() - chrono::Duration::hours(1),
    );
    let bytes = serde_json::to_vec(&entry).unwrap();
    let p = alert_entry_path(&rig, &api_key());
    std::fs::write(&p, &bytes).unwrap();
    let out = put_and_alert(&rig, &api_key(), R::Ok).await;
    assert_ne!(std::fs::read(&p).unwrap(), bytes, "{}", text(&out));
    assert!(String::from_utf8_lossy(&out.stdout).contains("CLEARED"));
}

// ── DoD 2: still published; select still refuses a local row that is not ok ──────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn an_unreachable_spark_is_still_published_and_files_nothing() {
    // end to end: the probe writes the local row (a closed port: unreachable), and alert files nothing for it
    let port = common::closed_port();
    let rig = rig_with(
        "",
        &format!("http://127.0.0.1:{port}/v1"),
        &format!("http://127.0.0.1:{port}/kimi"),
    );
    let p = cli(&rig, &["probe", "--force"]).await;
    assert_eq!(rc(&p), 0, "probe: {}", text(&p));
    let r = rig
        .store
        .get(&local_key())
        .await
        .unwrap()
        .expect("the local row is still published");
    assert_eq!(r.kind, Kind::Local, "{r:?}");
    assert_eq!(r.state, State::Unknown, "{r:?}");
    assert_eq!(
        r.reason.as_deref(),
        Some("cannot_assess:unreachable"),
        "{r:?}"
    );
    let a = cli(&rig, &["alert"]).await;
    assert_eq!(rc(&a), 0, "alert: {}", text(&a));
    assert_eq!(rig.fake.filed_for("qwen"), 0, "local filed: {}", text(&a));
    // control, same run: the API service at the same closed port is unreachable too and files
    assert_eq!(rig.fake.filed_for("kimi"), 1, "api: {}", text(&a));
    // and status still shows the local row
    let s = cli(&rig, &["status"]).await;
    assert!(
        String::from_utf8_lossy(&s.stdout).contains("qwen3"),
        "status lists the local row: {}",
        text(&s)
    );
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

/// `select --role review --prefer cheapest` with the local row reading `r` and the API row ok.
async fn select_with_local(r: R) -> Output {
    let rig = rig("");
    let mut api = row(&api_key(), R::Ok);
    api.model = "kimi-k2.5".into();
    rig.store.put(&api).await.unwrap();
    let mut local = row(&local_key(), r);
    local.model = "qwen3".into();
    rig.store.put(&local).await.unwrap();
    // `alert` runs between cycles; it must not change what select sees
    let a = cli(&rig, &["alert"]).await;
    assert_eq!(rc(&a), 0, "{}", text(&a));
    cli(
        &rig,
        &[
            "select", "--role", "review", "--prefer", "cheapest", "--n", "5",
        ],
    )
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn control_select_picks_the_local_row_first_when_it_is_ok() {
    // known answer: cheapest puts `local` before `metered`
    let o = select_with_local(R::Ok).await;
    assert_eq!(rc(&o), 0, "{}", text(&o));
    assert_eq!(
        stdout(&o).lines().next(),
        Some("qwen qwen3"),
        "{}",
        text(&o)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn select_still_refuses_a_local_row_that_is_not_ok() {
    for bad in EVERY_BAD {
        let o = select_with_local(bad).await;
        let out = stdout(&o);
        assert!(
            !out.contains("qwen3"),
            "select offered the local row reading {bad:?}:\n{}",
            text(&o)
        );
        // the API row is still offered
        assert_eq!(rc(&o), 0, "{bad:?}: {}", text(&o));
        assert_eq!(
            out.lines().next(),
            Some("moonshot kimi-k2.5"),
            "{bad:?}: {}",
            text(&o)
        );
    }
}

// ── DoD 4: DESIGN records the ruling ──────────────────────────────────────────────────────────────────────────

fn read(rel: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

/// DESIGN's `## <n>.` section, up to the next `## `.
fn section(design: &str, n: u32) -> String {
    let head = format!("\n## {n}. ");
    let start = design
        .find(&head)
        .unwrap_or_else(|| panic!("DESIGN has no §{n}"));
    let rest = &design[start + 1..];
    let end = rest[3..].find("\n## ").map(|i| i + 3).unwrap_or(rest.len());
    rest[..end].to_string()
}

/// §3's table row for `alert`.
fn alert_row(design: &str) -> String {
    section(design, 3)
        .lines()
        .find(|l| l.starts_with("| alert |"))
        .expect("DESIGN §3 has an `| alert |` row")
        .to_string()
}

/// §10 Q8: from `8. **` to `9. **`.
fn q8(design: &str) -> String {
    let s = section(design, 10);
    let start = s.find("\n8. **").expect("§10 has Q8");
    let rest = &s[start + 1..];
    let end = rest.find("\n9. **").expect("§10 has Q9 after Q8");
    rest[..end].to_string()
}

fn records_the_ruling(text: &str) -> Result<(), String> {
    if !text.contains(RULING) {
        return Err(format!("lacks the verbatim ruling {RULING:?}"));
    }
    if !text.contains("Captain 2026-10-09") {
        return Err("lacks the name and date `Captain 2026-10-09`".into());
    }
    if !text.contains("kind = \"local\"") && !text.contains("`local`") {
        return Err("does not name the local kind".into());
    }
    // it says it was the recommended option, in words beyond the quoted label
    let outside = text.replace(RULING, "");
    if !outside.to_lowercase().contains("recommended") {
        return Err("does not say the ruling was the recommended option".into());
    }
    Ok(())
}

#[test]
fn design_3_alert_row_records_the_local_ruling() {
    let row = alert_row(&read("docs/DESIGN.md"));
    if let Err(e) = records_the_ruling(&row) {
        panic!("§3 alert row {e}: {row}");
    }
}

#[test]
fn design_10_q8_records_the_local_ruling() {
    let q = q8(&read("docs/DESIGN.md"));
    if let Err(e) = records_the_ruling(&q) {
        panic!("§10 Q8 {e}: {q}");
    }
}

#[test]
fn control_the_ruling_check_can_fail_and_can_pass() {
    // known answer: a text that records it passes
    let good = format!(
        "Captain 2026-10-09: \"{RULING}\", the recommended option: a `local` row files no signal."
    );
    assert!(
        records_the_ruling(&good).is_ok(),
        "{:?}",
        records_the_ruling(&good)
    );
    // negative controls: each missing part fails
    assert!(records_the_ruling(&good.replace(RULING, "No alert for local")).is_err());
    assert!(records_the_ruling(&good.replace("Captain 2026-10-09", "Captain")).is_err());
    assert!(records_the_ruling(&good.replace("the recommended option", "the option")).is_err());
    assert!(records_the_ruling(&good.replace("`local`", "Spark")).is_err());
    // and the readers bound their regions: §10 Q8 is the alert question and stops before Q9
    let d = read("docs/DESIGN.md");
    let q = q8(&d);
    assert!(q.contains("SIG-006") && !q.contains("statusLine"));
    assert!(alert_row(&d).contains("crossing-dedup"));
}
