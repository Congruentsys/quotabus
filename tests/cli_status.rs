//! EXP-001 Plan 5, Definition of Done / DESIGN §2-§3, §8: `quotabus status` (table with AGE and SOURCE, `--json`,
//! `--kind`, `--check <service>` exit 0 ok · 1 not ok · 2 CANNOT-ASSESS · 3 UNKNOWN) against the FILE backend and a
//! throwaway NATS bucket; a bus that is down is CANNOT-ASSESS rc 2, never ok.
//!
//! `--json` contract asserted here: an array with one object per configured (service, model):
//! `{"service", "key", "verdict", "reason", "age_s", "row"}` — `verdict` is the state name, `CANNOT-ASSESS` or
//! `UNKNOWN`; `reason` is `absent` / `expired` / the row's `cannot_assess:…` (else null); `age_s` the row's age.

mod common;

use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::Utc;
use common::{NatsServer, rc, run_cli, text};
use quotabus::{Backend, NatsKv, Record, State};
use serde_json::Value;

const T: Duration = Duration::from_secs(30);

/// (service id, kind, provider, model, the row's state or None for no row, age s, ttl s)
type Fixture = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    Option<State>,
    i64,
    u64,
);
const FLEET: &[Fixture] = &[
    ("okapi", "api", "pok", "m-ok", Some(State::Ok), 30, 2700),
    (
        "authbad",
        "api",
        "pauth",
        "m-auth",
        Some(State::AuthFailed),
        30,
        2700,
    ),
    (
        "cannot",
        "api",
        "pcan",
        "m-can",
        Some(State::Unknown),
        30,
        2700,
    ),
    (
        "stale",
        "api",
        "pstale",
        "m-stale",
        Some(State::Ok),
        7200,
        60,
    ),
    ("missing", "api", "pmiss", "m-miss", None, 0, 0),
    ("qwen", "local", "qwen", "qwen3", Some(State::Ok), 30, 2700),
];

fn key(kind: &str, provider: &str, model: &str) -> String {
    format!("{kind}.{provider}.acct.{model}")
}

fn services_toml() -> String {
    FLEET
        .iter()
        .map(|(id, kind, provider, model, ..)| {
            format!(
                "[[service]]\nid = {id:?}\nkind = {kind:?}\nprovider = {provider:?}\nfamily = {provider:?}\n\
                 account = \"acct\"\nmodels = [{model:?}]\nsecret = \"QB_UNUSED\"\n"
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn rows() -> Vec<Record> {
    FLEET
        .iter()
        .filter_map(|(_, kind, provider, model, state, age, ttl)| {
            state.map(|s| {
                let mut r = common::record(
                    &key(kind, provider, model),
                    s,
                    Utc::now() - chrono::Duration::seconds(*age),
                    *ttl,
                );
                r.family = provider.to_string();
                r
            })
        })
        .collect()
}

struct FileFixture {
    _dir: tempfile::TempDir,
    config: PathBuf,
}

fn file_fixture() -> FileFixture {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    std::fs::create_dir(&state).unwrap();
    // DESIGN §8: the file backend is `<dir>/<key>.json`
    for r in rows() {
        std::fs::write(
            state.join(format!("{}.json", r.key)),
            serde_json::to_vec(&r).unwrap(),
        )
        .unwrap();
    }
    let config = dir.path().join("quotabus.toml");
    std::fs::write(
        &config,
        format!(
            "[file]\ndir = {:?}\n\n{}",
            state.display().to_string(),
            services_toml()
        ),
    )
    .unwrap();
    FileFixture { _dir: dir, config }
}

fn nats_config(dir: &Path, url: &str, bucket: &str) -> PathBuf {
    let p = dir.join("quotabus.toml");
    std::fs::write(
        &p,
        format!(
            "[bus]\nurl = {url:?}\nbucket = {bucket:?}\n\n{}",
            services_toml()
        ),
    )
    .unwrap();
    p
}

fn check(config: &Path, service: &str) -> i32 {
    rc(&run_cli(config, &["status", "--check", service], &[], T))
}

fn json(config: &Path, extra: &[&str]) -> Vec<Value> {
    let mut args = vec!["status", "--json"];
    args.extend_from_slice(extra);
    let out = run_cli(config, &args, &[], T);
    let v: Value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("status --json is not JSON ({e}): {}", text(&out)));
    v.as_array().expect("status --json prints an array").clone()
}

fn entry<'a>(rows: &'a [Value], key: &str) -> &'a Value {
    rows.iter()
        .find(|r| r["key"] == key)
        .unwrap_or_else(|| panic!("no {key} in {rows:?}"))
}

fn assert_exit_codes(config: &Path) {
    assert_eq!(check(config, "okapi"), 0, "fresh ok ⇒ 0");
    assert_eq!(check(config, "authbad"), 1, "fresh auth_failed ⇒ 1");
    assert_eq!(
        check(config, "cannot"),
        2,
        "a row reading unknown ⇒ CANNOT-ASSESS 2"
    );
    assert_eq!(
        check(config, "stale"),
        3,
        "an expired row ⇒ UNKNOWN 3, even though it says ok"
    );
    assert_eq!(check(config, "missing"), 3, "no row ⇒ UNKNOWN 3");
}

fn assert_json(config: &Path) {
    let all = json(config, &[]);
    assert_eq!(
        all.len(),
        FLEET.len(),
        "one entry per configured (service, model): {all:?}"
    );
    let ok = entry(&all, &key("api", "pok", "m-ok"));
    assert_eq!(ok["verdict"], "ok");
    assert_eq!(ok["service"], "okapi");
    let age = ok["age_s"].as_i64().expect("age_s");
    assert!((25..=120).contains(&age), "age_s {age}");
    assert_eq!(ok["row"]["contract"], "ai-status/1");
    assert_eq!(
        entry(&all, &key("api", "pauth", "m-auth"))["verdict"],
        "auth_failed"
    );
    let can = entry(&all, &key("api", "pcan", "m-can"));
    assert_eq!(can["verdict"], "CANNOT-ASSESS");
    assert_eq!(can["reason"], "cannot_assess:unreachable");
    let stale = entry(&all, &key("api", "pstale", "m-stale"));
    assert_eq!(stale["verdict"], "UNKNOWN");
    assert_eq!(stale["reason"], "expired");
    let miss = entry(&all, &key("api", "pmiss", "m-miss"));
    assert_eq!(miss["verdict"], "UNKNOWN");
    assert_eq!(miss["reason"], "absent");
    assert!(miss["row"].is_null());

    let api = json(config, &["--kind", "api"]);
    assert_eq!(api.len(), FLEET.len() - 1, "--kind api drops the local row");
    assert!(
        api.iter()
            .all(|r| r["key"].as_str().unwrap().starts_with("api.")),
        "{api:?}"
    );
}

fn assert_table(config: &Path) {
    let out = run_cli(config, &["status"], &[], T);
    let s = String::from_utf8_lossy(&out.stdout).to_string();
    let header = s.lines().next().unwrap_or("");
    assert!(
        header.contains("AGE") && header.contains("SOURCE"),
        "the table has AGE and SOURCE columns: {s}"
    );
    for needle in [
        "okapi",
        "auth_failed",
        "CANNOT-ASSESS",
        "UNKNOWN",
        "official",
    ] {
        assert!(s.contains(needle), "table lacks {needle:?}: {s}");
    }
}

#[test]
fn file_backend_check_exit_codes() {
    assert_exit_codes(&file_fixture().config);
}

#[test]
fn file_backend_status_json() {
    assert_json(&file_fixture().config);
}

#[test]
fn file_backend_status_table_has_age_and_source() {
    assert_table(&file_fixture().config);
}

async fn nats_fixture(srv: &NatsServer, bucket: &str) {
    let kv = NatsKv::connect_publisher(&srv.url(), bucket).await.unwrap();
    for r in rows() {
        kv.put(&r).await.unwrap();
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn nats_bucket_check_exit_codes_json_and_table() {
    let srv = NatsServer::start();
    nats_fixture(&srv, "qb_status").await;
    let dir = tempfile::tempdir().unwrap();
    let cfg = nats_config(dir.path(), &srv.url(), "qb_status");
    tokio::task::spawn_blocking(move || {
        assert_exit_codes(&cfg);
        assert_json(&cfg);
        assert_table(&cfg);
    })
    .await
    .unwrap();
}

#[test]
fn bus_down_is_cannot_assess_rc_2_never_ok() {
    let dir = tempfile::tempdir().unwrap();
    let port = common::closed_port();
    let cfg = nats_config(dir.path(), &format!("nats://127.0.0.1:{port}"), "ai_status");
    for args in [
        &["status", "--check", "okapi"][..],
        &["status"][..],
        &["status", "--json"][..],
    ] {
        let out = run_cli(&cfg, args, &[], T);
        assert_eq!(rc(&out), 2, "{args:?} with the bus down: {}", text(&out));
        assert!(
            text(&out).contains("CANNOT-ASSESS"),
            "{args:?} must say CANNOT-ASSESS: {}",
            text(&out)
        );
        assert!(
            !String::from_utf8_lossy(&out.stdout).contains("\"ok\""),
            "{args:?} printed an ok"
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_missing_bucket_is_cannot_assess_rc_2() {
    let srv = NatsServer::start();
    let dir = tempfile::tempdir().unwrap();
    let cfg = nats_config(dir.path(), &srv.url(), "qb_absent_bucket");
    let out =
        tokio::task::spawn_blocking(move || run_cli(&cfg, &["status", "--check", "okapi"], &[], T))
            .await
            .unwrap();
    assert_eq!(rc(&out), 2, "{}", text(&out));
    assert!(text(&out).contains("CANNOT-ASSESS"));
}
