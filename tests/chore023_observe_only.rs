//! CHORE-023 Definition of Done: observe-only services. A `[[service]]` may say `probe = false` (default true): the
//! service is listed by `status`, `select` and `alert` (reading its rows from the store) but never probed by this
//! process; another process (the Sparks' root daemon) owns it.
//!
//! Asserted (CHORE-023 test partner), each with a control that shows the check can fail:
//! - config: `probe = false` parses, an omitted `probe` reads true, a non-boolean `probe` is refused by a check that
//!   knows the key (not as an unknown field).
//! - `probe --force` (and a plain scheduled `probe`) makes NO request for an observe-only service: a wiremock stub
//!   expecting 0 calls on its path; the control runs the same stub with `probe` left at its default and shows the
//!   stub's call-count check FAILS. It writes no row for it: a row another process wrote stays byte-for-byte.
//! - `status` lists the observe-only service's stored row with its state and age; an absent row reads UNKNOWN
//!   (`absent`), an expired one UNKNOWN (`expired`): the freshness rule is unchanged.
//! - `select` considers it: a stored fresh `ok` row is chosen; a stored non-ok row is refused (rc 3).
//! - `alert` reads it: a stored `auth_failed` row on an observe-only API service files a crossing on stdout.
//! - docs: `examples/quotabus.toml` loads and shows the split (the Sparks `probe = false`, the keyed services probed),
//!   `packaging/README.md` shows the root daemon side, DESIGN.md §3 names `probe = false`.
//!
//! Observe-only services here are an API service with a fake key in the environment (so it WOULD be probed if the
//! setting were ignored) and a `local` service with explicit models (independent of CHORE-022's model discovery).
//! No real key, no real bus, no non-loopback host.

mod common;

use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::Utc;
use common::{FAKE_SK, rc, run_cli, text};
use futures::FutureExt;
use quotabus::{Config, Kind, Record, Runner, State, record_key};
use serde_json::Value;
use wiremock::matchers::{method, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

const T: Duration = Duration::from_secs(60);

// ── fixtures ─────────────────────────────────────────────────────────────────────────────────────────────────────

/// The probed (keyed) API service.
const KEYED_ID: &str = "keyed";
/// An observe-only API service: its key IS in the environment, so only `probe = false` keeps it unprobed.
const OBS_API_ID: &str = "obs-api";
/// An observe-only local service (a Spark), explicit models, no secret.
const OBS_LOCAL_ID: &str = "spark";

fn keyed_key() -> String {
    record_key(Kind::Api, "pkeyed", "acct", "m-keyed")
}
fn obs_api_key() -> String {
    record_key(Kind::Api, "pobs", "acct", "m-obs")
}
fn obs_local_key() -> String {
    record_key(Kind::Local, "qwen", "dgx1", "qwen3")
}

/// The three services; `obs_line` is written into both observe-only services (`"probe = false\n"`, or `""` for the
/// control that leaves `probe` at its default).
fn services(stub: &str, obs_line: &str) -> String {
    format!(
        "[[service]]\nid = {KEYED_ID:?}\nkind = \"api\"\nprovider = \"pkeyed\"\nfamily = \"pkeyed\"\naccount = \"acct\"\n\
         base_url = \"{stub}/keyed\"\nprotocol = \"anthropic\"\nmodels = [\"m-keyed\"]\nroles = [\"review\"]\n\
         cost_class = \"metered\"\nsecret = \"QB_KEYED\"\n\n\
         [[service]]\nid = {OBS_API_ID:?}\nkind = \"api\"\nprovider = \"pobs\"\nfamily = \"pobs\"\naccount = \"acct\"\n\
         base_url = \"{stub}/obs\"\nprotocol = \"anthropic\"\nmodels = [\"m-obs\"]\nroles = [\"review\"]\n\
         cost_class = \"metered\"\nsecret = \"QB_OBS\"\n{obs_line}\n\
         [[service]]\nid = {OBS_LOCAL_ID:?}\nkind = \"local\"\nprovider = \"qwen\"\nfamily = \"qwen\"\naccount = \"dgx1\"\n\
         base_url = \"{stub}/spark/v1\"\nprotocol = \"openai\"\nmodels = [\"qwen3\"]\nroles = [\"work\"]\n\
         cost_class = \"local\"\n{obs_line}"
    )
}

const PROBE_FALSE: &str = "probe = false\n";

fn envs() -> Vec<(&'static str, &'static str)> {
    vec![("QB_KEYED", FAKE_SK), ("QB_OBS", FAKE_SK)]
}

/// A store dir and a config pointing the file backend at it.
struct Fixture {
    _dir: tempfile::TempDir,
    state: PathBuf,
    config: PathBuf,
}

fn fixture(body: &str) -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    std::fs::create_dir(&state).unwrap();
    let config = dir.path().join("quotabus.toml");
    std::fs::write(
        &config,
        format!(
            "[file]\ndir = {:?}\n\n[ttl]\napi = \"2m\"\n\n{body}",
            state.display().to_string()
        ),
    )
    .unwrap();
    Fixture {
        _dir: dir,
        state,
        config,
    }
}

/// A row as another process (the root daemon) wrote it, `age_s` ago.
fn daemon_row(key: &str, state: State, age_s: i64, ttl_s: u64) -> Record {
    let mut r = common::record(
        key,
        state,
        Utc::now() - chrono::Duration::seconds(age_s),
        ttl_s,
    );
    r.observed_by = "daemon-host/quotabus@0.0.0".into();
    r
}

fn seed(state_dir: &Path, r: &Record) -> Vec<u8> {
    let bytes = serde_json::to_vec(r).unwrap();
    std::fs::write(state_dir.join(format!("{}.json", r.key)), &bytes).unwrap();
    bytes
}

fn row_file(state_dir: &Path, key: &str) -> PathBuf {
    state_dir.join(format!("{key}.json"))
}

/// A stub that answers every POST 200 (anthropic and openai shapes both read ok), with a 0-call expectation on the
/// observe-only paths `/obs` and `/spark`.
async fn stub_expecting_no_observe_only_calls() -> MockServer {
    let stub = MockServer::start().await;
    let ok = ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "content": [{"type": "text", "text": "OK"}],
        "choices": [{"message": {"role": "assistant", "content": "OK"}}],
        "model": "m",
    }));
    Mock::given(path_regex("^/(obs|spark)(/|$)"))
        .respond_with(ok.clone())
        .expect(0)
        .named("observe-only services")
        .mount(&stub)
        .await;
    Mock::given(method("POST"))
        .and(path_regex("^/keyed/"))
        .respond_with(ok)
        .mount(&stub)
        .await;
    stub
}

/// How many requests the stub saw whose path starts with `prefix`.
async fn hits(stub: &MockServer, prefix: &str) -> usize {
    stub.received_requests()
        .await
        .unwrap()
        .iter()
        .filter(|r| r.url.path().starts_with(prefix))
        .count()
}

/// Whether the stub's call-count expectations hold (`verify` panics when they do not).
async fn verified(stub: &MockServer) -> bool {
    std::panic::AssertUnwindSafe(stub.verify())
        .catch_unwind()
        .await
        .is_ok()
}

async fn cli(cfg: &Path, args: &'static [&'static str]) -> std::process::Output {
    let cfg = cfg.to_path_buf();
    tokio::task::spawn_blocking(move || run_cli(&cfg, args, &envs(), T))
        .await
        .unwrap()
}

fn svc<'a>(c: &'a Config, id: &str) -> &'a quotabus::config::ServiceConfig {
    c.services
        .iter()
        .find(|s| s.id == id)
        .unwrap_or_else(|| panic!("no service {id}"))
}

// ── 1. config ────────────────────────────────────────────────────────────────────────────────────────────────────

#[test]
fn probe_false_parses_and_an_omitted_probe_reads_true() {
    let c = Config::from_toml_str(&services("http://127.0.0.1:9", PROBE_FALSE))
        .unwrap_or_else(|e| panic!("`probe = false` must parse: {e}"));
    assert!(!svc(&c, OBS_API_ID).probe, "probe = false reads false");
    assert!(!svc(&c, OBS_LOCAL_ID).probe, "probe = false reads false");
    assert!(svc(&c, KEYED_ID).probe, "an omitted probe reads true");
}

#[test]
fn probe_true_parses_and_the_default_is_true() {
    // known answer for the default, and an explicit `probe = true` (the control: the field is read, not constant)
    let c = Config::from_toml_str(&services("http://127.0.0.1:9", "probe = true\n"))
        .unwrap_or_else(|e| panic!("`probe = true` must parse: {e}"));
    assert!(svc(&c, OBS_API_ID).probe);
    let c = Config::from_toml_str(&services("http://127.0.0.1:9", "")).unwrap();
    assert!(c.services.iter().all(|s| s.probe), "default true");
}

#[test]
fn a_non_boolean_probe_is_refused_by_a_check_that_knows_the_key() {
    for bad in ["probe = \"no\"\n", "probe = 0\n", "probe = \"false\"\n"] {
        let err = Config::from_toml_str(&services("http://127.0.0.1:9", bad))
            .err()
            .unwrap_or_else(|| panic!("{bad:?} must be refused"));
        let msg = err.to_string();
        assert!(msg.contains("probe"), "the error names the key: {msg}");
        // refused as a bad VALUE of a known key, never as an unknown field (which is how it is refused before
        // CHORE-023, and how `probe = false` itself would be refused)
        assert!(
            !msg.contains("unknown field"),
            "{bad:?} must be refused as a bad value of `probe`, not an unknown field: {msg}"
        );
    }
}

// ── 2. probe makes no call and writes no row ─────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn probe_force_makes_no_call_and_writes_no_row_for_an_observe_only_service() {
    let stub = stub_expecting_no_observe_only_calls().await;
    let f = fixture(&services(&stub.uri(), PROBE_FALSE));
    // the root daemon's rows, fresh: a write by this process would replace them
    let api_before = seed(&f.state, &daemon_row(&obs_api_key(), State::Ok, 60, 3600));
    let local_before = seed(&f.state, &daemon_row(&obs_local_key(), State::Ok, 60, 3600));

    let out = cli(&f.config, &["probe", "--force"]).await;
    assert_eq!(rc(&out), 0, "{}", text(&out));

    assert_eq!(
        hits(&stub, "/obs").await,
        0,
        "a request to the observe-only API service"
    );
    assert_eq!(
        hits(&stub, "/spark").await,
        0,
        "a request to the observe-only local service"
    );
    assert!(
        verified(&stub).await,
        "the stub's 0-call expectation failed"
    );
    assert_eq!(
        hits(&stub, "/keyed").await,
        1,
        "the probed service is still probed"
    );

    assert_eq!(
        std::fs::read(row_file(&f.state, &obs_api_key())).unwrap(),
        api_before,
        "the daemon's row for the observe-only API service was rewritten"
    );
    assert_eq!(
        std::fs::read(row_file(&f.state, &obs_local_key())).unwrap(),
        local_before,
        "the daemon's row for the observe-only local service was rewritten"
    );
    assert!(
        row_file(&f.state, &keyed_key()).exists(),
        "the probed service's row is written"
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    for k in [obs_api_key(), obs_local_key()] {
        assert!(
            !stdout.contains(&k),
            "probe reported a row for {k}: {stdout}"
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn probe_force_writes_no_row_when_the_store_has_none_for_it() {
    let stub = stub_expecting_no_observe_only_calls().await;
    let f = fixture(&services(&stub.uri(), PROBE_FALSE));
    let out = cli(&f.config, &["probe", "--force"]).await;
    assert_eq!(rc(&out), 0, "{}", text(&out));
    assert!(
        verified(&stub).await,
        "the stub's 0-call expectation failed"
    );
    for k in [obs_api_key(), obs_local_key()] {
        assert!(
            !row_file(&f.state, &k).exists(),
            "probe wrote a row for the observe-only {k}"
        );
    }
    assert!(row_file(&f.state, &keyed_key()).exists());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_scheduled_probe_makes_no_call_for_an_observe_only_service_either() {
    // no row at all reads due: only `probe = false` keeps it unprobed
    let stub = stub_expecting_no_observe_only_calls().await;
    let f = fixture(&services(&stub.uri(), PROBE_FALSE));
    let out = cli(&f.config, &["probe"]).await;
    assert_eq!(rc(&out), 0, "{}", text(&out));
    assert!(
        verified(&stub).await,
        "the stub's 0-call expectation failed"
    );
    assert_eq!(hits(&stub, "/keyed").await, 1);
    for k in [obs_api_key(), obs_local_key()] {
        assert!(!row_file(&f.state, &k).exists(), "row written for {k}");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn control_with_probe_left_at_its_default_the_stub_call_count_check_fails() {
    // the same stub and config, `probe` omitted: both services are probed, so the 0-call check must FAIL and the
    // daemon's row must be overwritten. This shows the checks above can fail.
    let stub = stub_expecting_no_observe_only_calls().await;
    let f = fixture(&services(&stub.uri(), ""));
    let api_before = seed(&f.state, &daemon_row(&obs_api_key(), State::Ok, 60, 3600));
    let out = cli(&f.config, &["probe", "--force"]).await;
    assert_eq!(rc(&out), 0, "{}", text(&out));
    assert!(
        hits(&stub, "/obs").await >= 1,
        "the API service was not probed"
    );
    assert!(
        hits(&stub, "/spark").await >= 1,
        "the local service was not probed"
    );
    assert!(
        !verified(&stub).await,
        "the 0-call expectation must fail when the service is probed"
    );
    assert_ne!(
        std::fs::read(row_file(&f.state, &obs_api_key())).unwrap(),
        api_before,
        "a probed service's row is rewritten"
    );
    // clear the failed expectation so the stub's drop does not panic
    stub.reset().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn the_runner_returns_no_row_for_an_observe_only_service() {
    let stub = stub_expecting_no_observe_only_calls().await;
    let c = Config::from_toml_str(&services(&stub.uri(), PROBE_FALSE))
        .unwrap_or_else(|e| panic!("`probe = false` must parse: {e}"));
    let runner = Runner::new(c, |_| Some(FAKE_SK.to_string()));
    let rows = runner.run_due(&[], Utc::now(), true).await;
    let keys: Vec<&str> = rows.iter().map(|r| r.key.as_str()).collect();
    assert!(keys.contains(&keyed_key().as_str()), "{keys:?}");
    for k in [obs_api_key(), obs_local_key()] {
        assert!(
            !keys.contains(&k.as_str()),
            "run_due returned {k}: {keys:?}"
        );
    }
    assert!(
        verified(&stub).await,
        "the stub's 0-call expectation failed"
    );
}

// ── 3. status lists it from the store ────────────────────────────────────────────────────────────────────────────

fn status_json(cfg: &Path) -> Vec<Value> {
    let out = run_cli(cfg, &["status", "--json"], &[], T);
    assert_eq!(rc(&out), 0, "{}", text(&out));
    let v: Value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("status --json is not JSON ({e}): {}", text(&out)));
    v.as_array().expect("an array").clone()
}

fn entry<'a>(all: &'a [Value], key: &str) -> Option<&'a Value> {
    all.iter().find(|e| e["key"] == key)
}

#[test]
fn status_lists_the_observe_only_row_from_the_store_with_state_and_age() {
    let f = fixture(&services("http://127.0.0.1:9", PROBE_FALSE));
    seed(
        &f.state,
        &daemon_row(&obs_local_key(), State::Ok, 300, 3600),
    );
    seed(
        &f.state,
        &daemon_row(&obs_api_key(), State::AuthFailed, 30, 3600),
    );

    let all = status_json(&f.config);
    let local =
        entry(&all, &obs_local_key()).unwrap_or_else(|| panic!("no observe-only row in {all:?}"));
    assert_eq!(local["service"], OBS_LOCAL_ID);
    assert_eq!(local["verdict"], "ok");
    let age = local["age_s"].as_i64().expect("age_s");
    assert!((295..=400).contains(&age), "age_s {age}");
    assert_eq!(local["row"]["observed_by"], "daemon-host/quotabus@0.0.0");
    // the state is the stored one, not a constant
    let api =
        entry(&all, &obs_api_key()).unwrap_or_else(|| panic!("no observe-only row in {all:?}"));
    assert_eq!(api["verdict"], "auth_failed");

    let table = run_cli(&f.config, &["status"], &[], T);
    let s = String::from_utf8_lossy(&table.stdout).to_string();
    let line = s
        .lines()
        .find(|l| l.starts_with(OBS_LOCAL_ID))
        .unwrap_or_else(|| panic!("the table lacks {OBS_LOCAL_ID}: {s}"));
    assert!(
        line.contains("ok") && line.contains("5m"),
        "state and age: {line}"
    );

    assert_eq!(
        rc(&run_cli(
            &f.config,
            &["status", "--check", OBS_LOCAL_ID],
            &[],
            T
        )),
        0
    );
    assert_eq!(
        rc(&run_cli(
            &f.config,
            &["status", "--check", OBS_API_ID],
            &[],
            T
        )),
        1
    );
}

#[test]
fn status_reads_an_absent_observe_only_row_unknown_and_an_expired_one_unknown() {
    let f = fixture(&services("http://127.0.0.1:9", PROBE_FALSE));
    // the local row expired (age 7200 s > ttl 60 s); the API row is absent
    seed(&f.state, &daemon_row(&obs_local_key(), State::Ok, 7200, 60));
    let all = status_json(&f.config);
    let absent =
        entry(&all, &obs_api_key()).unwrap_or_else(|| panic!("absent row not listed: {all:?}"));
    assert_eq!(absent["verdict"], "UNKNOWN");
    assert_eq!(absent["reason"], "absent");
    assert!(absent["row"].is_null());
    let expired =
        entry(&all, &obs_local_key()).unwrap_or_else(|| panic!("expired row not listed: {all:?}"));
    assert_eq!(expired["verdict"], "UNKNOWN");
    assert_eq!(expired["reason"], "expired");
    assert_eq!(
        rc(&run_cli(
            &f.config,
            &["status", "--check", OBS_API_ID],
            &[],
            T
        )),
        3
    );
    assert_eq!(
        rc(&run_cli(
            &f.config,
            &["status", "--check", OBS_LOCAL_ID],
            &[],
            T
        )),
        3
    );
}

#[test]
fn control_status_lists_no_entry_for_a_service_not_in_the_config() {
    // the row is in the store but its service is not configured: no entry. So the lookup above is not vacuous.
    let body = services("http://127.0.0.1:9", "");
    let without_local = body
        .split("\n[[service]]")
        .filter(|s| !s.contains(&format!("id = {OBS_LOCAL_ID:?}")))
        .collect::<Vec<_>>()
        .join("\n[[service]]");
    assert!(
        !without_local.contains(OBS_LOCAL_ID),
        "the control removed the service"
    );
    let f = fixture(&without_local);
    seed(&f.state, &daemon_row(&obs_local_key(), State::Ok, 60, 3600));
    let all = status_json(&f.config);
    assert!(
        entry(&all, &obs_local_key()).is_none(),
        "an unconfigured service's row must not be listed: {all:?}"
    );
}

// ── 4. select considers it ───────────────────────────────────────────────────────────────────────────────────────

/// Only the observe-only API service carries role `review` here; the keyed one is removed from the role.
fn select_fixture(obs_api_state: Option<State>) -> Fixture {
    let body = services("http://127.0.0.1:9", PROBE_FALSE).replacen(
        "roles = [\"review\"]",
        "roles = [\"work\"]",
        1,
    );
    let f = fixture(&body);
    if let Some(s) = obs_api_state {
        seed(&f.state, &daemon_row(&obs_api_key(), s, 60, 3600));
    }
    f
}

#[test]
fn select_chooses_an_observe_only_service_with_a_fresh_ok_row() {
    let f = select_fixture(Some(State::Ok));
    let out = run_cli(&f.config, &["select", "--role", "review"], &[], T);
    assert_eq!(rc(&out), 0, "{}", text(&out));
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).lines().next(),
        Some("pobs m-obs"),
        "{}",
        text(&out)
    );
}

#[test]
fn select_refuses_an_observe_only_service_with_a_non_ok_or_absent_row() {
    for state in [Some(State::AuthFailed), Some(State::Unknown), None] {
        let f = select_fixture(state);
        let out = run_cli(&f.config, &["select", "--role", "review"], &[], T);
        assert_eq!(rc(&out), 3, "{state:?}: {}", text(&out));
        assert!(
            !String::from_utf8_lossy(&out.stdout).contains("m-obs"),
            "{state:?} was chosen: {}",
            text(&out)
        );
    }
}

// ── 5. alert reads it ────────────────────────────────────────────────────────────────────────────────────────────

#[test]
fn alert_reads_an_observe_only_services_row() {
    let f = fixture(&services("http://127.0.0.1:9", PROBE_FALSE));
    seed(
        &f.state,
        &daemon_row(&obs_api_key(), State::AuthFailed, 30, 3600),
    );
    seed(&f.state, &daemon_row(&keyed_key(), State::Ok, 30, 3600));
    let out = run_cli(&f.config, &["alert"], &[], T);
    assert_eq!(rc(&out), 0, "{}", text(&out));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains(&format!("ALERT {} auth_failed", obs_api_key())),
        "alert did not file the observe-only service's crossing: {}",
        text(&out)
    );
    assert!(
        !stdout.contains(&format!("ALERT {}", keyed_key())),
        "an ok row filed: {stdout}"
    );
}

#[test]
fn control_alert_files_nothing_for_an_observe_only_services_ok_row() {
    let f = fixture(&services("http://127.0.0.1:9", PROBE_FALSE));
    seed(&f.state, &daemon_row(&obs_api_key(), State::Ok, 30, 3600));
    let out = run_cli(&f.config, &["alert"], &[], T);
    assert_eq!(rc(&out), 0, "{}", text(&out));
    assert!(
        !String::from_utf8_lossy(&out.stdout).contains("ALERT "),
        "{}",
        text(&out)
    );
}

// ── 6. docs ──────────────────────────────────────────────────────────────────────────────────────────────────────

fn repo(p: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(p)
}

/// Whitespace removed, so `probe      = false` (aligned) matches `probe=false`.
fn squash(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

/// The observe-only services of a config.
fn observe_only(c: &Config) -> Vec<&quotabus::config::ServiceConfig> {
    c.services.iter().filter(|s| !s.probe).collect()
}

#[test]
fn example_config_loads_and_shows_the_split() {
    let c = Config::load(&repo("examples/quotabus.toml"))
        .unwrap_or_else(|e| panic!("examples/quotabus.toml must load: {e}"));
    let obs = observe_only(&c);
    assert!(
        !obs.is_empty(),
        "examples/quotabus.toml has no `probe = false` service (the Sparks, owned by the root daemon)"
    );
    for s in &obs {
        assert_eq!(
            s.kind,
            Kind::Local,
            "{} is observe-only but not a local Spark",
            s.id
        );
    }
    assert!(
        obs.iter().any(|s| s.id == "local-qwen"),
        "local-qwen (the Spark) is observe-only in the agent's config"
    );
    // the keyed services stay probed by the agent
    for s in c.services.iter().filter(|s| s.kind == Kind::Api) {
        assert!(s.probe, "keyed API service {} must stay probed", s.id);
    }
}

#[test]
fn control_a_config_without_probe_false_has_no_observe_only_service() {
    let c = Config::from_toml_str(&services("http://127.0.0.1:9", "")).unwrap();
    assert!(observe_only(&c).is_empty());
}

#[test]
fn packaging_readme_shows_the_root_daemon_side_of_the_split() {
    let text = std::fs::read_to_string(repo("packaging/README.md")).unwrap();
    assert!(
        squash(&text).contains("probe=false"),
        "packaging/README.md does not name `probe = false`"
    );
    assert!(
        text.to_lowercase().contains("daemon"),
        "packaging/README.md does not describe the root daemon that probes the Sparks"
    );
}

#[test]
fn design_section_3_names_probe_false() {
    let design = std::fs::read_to_string(repo("docs/DESIGN.md")).unwrap();
    let start = design.find("\n## 3.").expect("DESIGN.md has §3");
    let end = design[start + 1..]
        .find("\n## 4.")
        .map(|i| start + 1 + i)
        .expect("DESIGN.md has §4");
    let s3 = &design[start..end];
    assert!(
        squash(s3).contains("probe=false"),
        "DESIGN.md §3 does not name `probe = false`"
    );
    // control: the section cut is not the whole document
    assert!(!s3.contains("\n## 4."));
}
