//! CHORE-018 (Captain 2026-10-09 on SIG-010: "SIG-010: go with the recommended default, option 3"): `quotabus alert`
//! files only for a crossing into a state listed in `[alert] states`; the default is the hard failures
//! (`quota_exhausted`, `auth_failed`, `model_missing`, unreachable) PLUS a near-limit window. Latency-degraded and
//! `rate_limited` stay on the bus (and `select` refuses them) but file nothing and do not arm the alert. The
//! crossing-dedup rule is otherwise unchanged (DESIGN §3 alert row): one alert per crossing, only a measured `ok`
//! re-arms.
//!
//! Seams fixed here (test partner, for the implementer):
//! - `[alert] states = [<name>, …]` (a key of the `[alert]` table itself, beside the `[alert.<sink>]` tables). Valid
//!   names: `quota_exhausted`, `auth_failed`, `model_missing`, `rate_limited`, `degraded` (the row state names, every
//!   bad state but `unknown`) plus `unreachable` and `window_near_limit`. Absent → the default
//!   `["quota_exhausted", "auth_failed", "model_missing", "unreachable", "window_near_limit"]`. Any other name is
//!   refused by `Config::from_toml_str` with an error naming the bad name and every valid one.
//! - `Config::alert.states: Vec<String>`: the names in force (the default when absent); compared here as a set.
//! - **unreachable** is how this codebase writes it today: a row with `state = unknown` and
//!   `reason = "cannot_assess:unreachable"` (`classify::classify` on `Outcome::Unreachable`). Under the default that
//!   row files; any OTHER CANNOT-ASSESS (`cannot_assess:not_found`, …) still files nothing and never clears.
//! - **window_near_limit** is a `degraded` row whose `reason` is `"window_near_limit"`, written where the warn rule
//!   (`classify::warn_state`, CHORE-010) degrades an `ok`. A latency-degraded row (a slow 200) does NOT carry that
//!   reason; neither does a worse state the warn rule leaves alone. The alert tells them apart from the row's reason.
//! - `degraded` in `states` files every degraded row (latency or near-limit).
//!
//! Fakes only: a recording fake `nusy-kanban` (no board), the file backend in a temp dir, wiremock stubs, fake keys.

mod common;
mod exp002;

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Output;
use std::time::Duration;

use chrono::Utc;
use common::{FAKE_PLAIN, rc, run_cli, text};
use exp002::*;
use quotabus::{Backend, Config, FileBackend, Headroom, Record, State};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer};

const T: Duration = Duration::from_secs(60);
const KEY: &str = "api.moonshot.nusy-product-team.kimi-k2-5";
const SECRET_ENV: &str = "QB_KIMI";
const NEAR: &str = "window_near_limit";

/// The default, as the ruling names it.
const DEFAULT: [&str; 5] = [
    "quota_exhausted",
    "auth_failed",
    "model_missing",
    "unreachable",
    NEAR,
];
/// Every name `[alert] states` accepts.
const VALID: [&str; 7] = [
    "quota_exhausted",
    "auth_failed",
    "model_missing",
    "rate_limited",
    "degraded",
    "unreachable",
    NEAR,
];

fn set(v: &[impl AsRef<str>]) -> std::collections::BTreeSet<String> {
    v.iter().map(|s| s.as_ref().to_string()).collect()
}

// ── DoD 1: config ───────────────────────────────────────────────────────────────────────────────────────────────

fn parsed(toml: &str) -> Config {
    Config::from_toml_str(toml).unwrap_or_else(|e| panic!("must parse: {e}\n{toml}"))
}

fn refused(toml: &str) -> String {
    match Config::from_toml_str(toml) {
        Ok(c) => panic!(
            "must be refused, but parsed (states {:?}):\n{toml}",
            c.alert.states
        ),
        Err(e) => e.to_string(),
    }
}

#[test]
fn states_default_to_the_hard_failures_plus_the_near_limit_window() {
    for toml in [
        "",
        "[alert.webhook]\nurl = \"http://127.0.0.1:1/hook\"\n",
        "[alert.nusy-kanban]\ncommand = \"/bin/true\"\n",
    ] {
        assert_eq!(
            set(&parsed(toml).alert.states),
            set(&DEFAULT),
            "no `[alert] states`:\n{toml}"
        );
    }
}

#[test]
fn states_are_read_from_the_alert_table_beside_its_sink_tables() {
    let toml = "[alert]\nstates = [\"quota_exhausted\", \"rate_limited\"]\n\n\
                [alert.nusy-kanban]\ncommand = \"/bin/true\"\nitem_type = \"signal\"\n\n\
                [alert.webhook]\nurl = \"http://127.0.0.1:1/hook\"\n";
    let c = parsed(toml);
    assert_eq!(
        set(&c.alert.states),
        set(&["quota_exhausted", "rate_limited"])
    );
    // the sink tables still load alongside it
    assert!(c.alert.nusy_kanban.is_some() && c.alert.webhook.is_some());
}

#[test]
fn every_valid_name_is_accepted_alone() {
    for name in VALID {
        let c = parsed(&format!("[alert]\nstates = [{name:?}]\n"));
        assert_eq!(set(&c.alert.states), set(&[name]), "{name}");
    }
}

#[test]
fn an_unknown_name_is_refused_naming_it_and_the_valid_ones() {
    for bad in ["bogus", "near_limit", "latency_degraded", "quota"] {
        let e = refused(&format!("[alert]\nstates = [\"model_missing\", {bad:?}]\n"));
        assert!(e.contains(bad), "the error names {bad}: {e}");
        for v in VALID {
            assert!(
                e.contains(v),
                "the error for {bad} names the valid {v}: {e}"
            );
        }
    }
}

#[test]
fn control_the_refusal_helper_fails_on_a_good_config() {
    let r = std::panic::catch_unwind(|| refused("[alert]\nstates = [\"model_missing\"]\n"));
    assert!(r.is_err(), "refused() must panic on a config that parses");
    let r = std::panic::catch_unwind(|| refused("[alert]\nstates = [\"bogus\"]\n"));
    assert!(r.is_ok(), "and must not panic on one that is refused");
}

// ── DoD 3: the row tells a near-limit window from latency-degraded ──────────────────────────────────────────────

async fn unified_row(five: f64, overall: &str) -> Record {
    let s = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(unified(
            200,
            Some((five, RESET_5H, overall)),
            Some((0.10, RESET_7D, overall)),
            Some(overall),
        ))
        .mount(&s)
        .await;
    let rows = runner(config(&claude_both(&s.uri())), None)
        .run_due(&[], now(), true)
        .await;
    let r = find(&rows, CLAUDE_KEY).clone();
    assert_eq!(r.probe.name, "unified_headers", "{}", json(&r));
    r
}

#[tokio::test]
async fn a_window_at_97_reads_degraded_with_reason_window_near_limit() {
    let r = unified_row(0.97, "allowed").await;
    assert_eq!(r.state, State::Degraded, "{}", json(&r));
    assert_eq!(r.reason.as_deref(), Some(NEAR), "{}", json(&r));
}

#[tokio::test]
async fn control_a_window_at_89_9_reads_ok_with_no_near_limit_reason() {
    let r = unified_row(0.899, "allowed").await;
    assert_eq!(r.state, State::Ok, "{}", json(&r));
    assert_ne!(r.reason.as_deref(), Some(NEAR), "{}", json(&r));
}

#[tokio::test]
async fn a_worse_state_at_97_is_not_labelled_near_limit() {
    let r = unified_row(0.97, "rejected").await;
    assert_eq!(r.state, State::QuotaExhausted, "{}", json(&r));
    assert_ne!(r.reason.as_deref(), Some(NEAR), "{}", json(&r));
}

/// An API row for a 200 that takes `delay_ms`, with `[probe] degraded_latency_ms = 50`.
async fn api_row(delay_ms: u64) -> Record {
    let s = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(measured().set_delay(Duration::from_millis(delay_ms)))
        .mount(&s)
        .await;
    let toml = format!(
        "[probe]\ndegraded_latency_ms = 50\n\n\
         [[service]]\nid = \"slow\"\nkind = \"api\"\nprovider = \"anthropic\"\nfamily = \"anthropic\"\n\
         account = \"acct\"\nbase_url = {:?}\nprotocol = \"anthropic\"\nmodels = [{MODEL:?}]\n\
         roles = [\"review\"]\nsecret = {CLAUDE_SECRET:?}\n",
        s.uri()
    );
    let rows = runner(config(&toml), None).run_due(&[], now(), true).await;
    rows.iter()
        .find(|r| r.key.starts_with("api.anthropic.acct."))
        .unwrap_or_else(|| panic!("no API row in {rows:?}"))
        .clone()
}

#[tokio::test]
async fn a_latency_degraded_row_does_not_carry_the_near_limit_reason() {
    let r = api_row(400).await;
    assert_eq!(r.state, State::Degraded, "a slow 200: {}", json(&r));
    assert_ne!(r.reason.as_deref(), Some(NEAR), "{}", json(&r));
    // control: the same stub, fast, reads ok — the degraded above is the latency, not the fixture
    let r = api_row(0).await;
    assert_eq!(r.state, State::Ok, "{}", json(&r));
}

// ── DoD 2/4: `quotabus alert` under the default and a configured list ─────────────────────────────────────────

/// A fake `nusy-kanban` that records each call as a file under `log`; never touches a board.
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

    fn filed(&self) -> usize {
        std::fs::read_dir(&self.log).unwrap().count()
    }
}

struct Rig {
    fake: Fake,
    _dir: tempfile::TempDir,
    cfg: PathBuf,
    state_dir: PathBuf,
    store: FileBackend,
}

/// The kimi service, the file backend, the fake sink and `alert_table` (`""` = no `[alert] states`, the default).
fn rig_with(alert_table: &str, base_url: &str) -> Rig {
    let fake = Fake::new();
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    let cfg = dir.path().join("quotabus.toml");
    std::fs::write(
        &cfg,
        format!(
            "[file]\ndir = {:?}\n\n[ttl]\napi = \"2m\"\nbalance = \"2m\"\n\n\
             [[service]]\nid = \"kimi\"\nkind = \"api\"\nprovider = \"moonshot\"\nfamily = \"moonshot\"\n\
             account = \"nusy-product-team\"\nbase_url = \"{base_url}\"\nprotocol = \"anthropic\"\n\
             models = [\"kimi-k2.5\"]\nroles = [\"review\"]\nsecret = \"{SECRET_ENV}\"\n\n\
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
    rig_with(alert_table, "http://127.0.0.1:1/kimi")
}

async fn cli(rig: &Rig, args: &'static [&'static str]) -> Output {
    let c = rig.cfg.clone();
    tokio::task::spawn_blocking(move || {
        run_cli(
            &c,
            args,
            &[("PATH", "/usr/bin:/bin"), (SECRET_ENV, FAKE_PLAIN)],
            T,
        )
    })
    .await
    .unwrap()
}

/// What a fresh row reading `r` is.
#[derive(Debug, Clone, Copy, PartialEq)]
enum R {
    Ok,
    QuotaExhausted,
    AuthFailed,
    ModelMissing,
    RateLimited,
    /// `unknown` + `cannot_assess:unreachable`.
    Unreachable,
    /// `unknown` + `cannot_assess:not_found`: a CANNOT-ASSESS that is not unreachable.
    NotFound,
    /// `degraded`, a slow 200: no reason, no headroom.
    Latency,
    /// `degraded` + reason `window_near_limit`, a window at 97 %.
    Near,
}

fn row(r: R) -> Record {
    let state = match r {
        R::Ok => State::Ok,
        R::QuotaExhausted => State::QuotaExhausted,
        R::AuthFailed => State::AuthFailed,
        R::ModelMissing => State::ModelMissing,
        R::RateLimited => State::RateLimited,
        R::Unreachable | R::NotFound => State::Unknown,
        R::Latency | R::Near => State::Degraded,
    };
    let mut rec = common::record(KEY, state, Utc::now(), 600);
    rec.reason = match r {
        R::Unreachable => Some("cannot_assess:unreachable".into()),
        R::NotFound => Some("cannot_assess:not_found".into()),
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

async fn put_and_alert(rig: &Rig, r: R) {
    rig.store.put(&row(r)).await.unwrap();
    let out = cli(rig, &["alert"]).await;
    assert_eq!(rc(&out), 0, "alert after {r:?}: {}", text(&out));
}

/// Run `seq` (one fresh row then one `alert` each) on a fresh rig; return how many items were filed.
async fn filed(alert_table: &str, seq: &[R]) -> usize {
    let rig = rig(alert_table);
    for r in seq {
        put_and_alert(&rig, *r).await;
    }
    rig.fake.filed()
}

/// The `state` held at `alert.<KEY>`, or None.
fn alerted(rig: &Rig) -> Option<String> {
    let bytes = std::fs::read(rig.state_dir.join(format!("alert.{KEY}.json"))).ok()?;
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    v.get("state").and_then(|s| s.as_str()).map(str::to_string)
}

#[test]
fn control_the_row_fixtures_are_what_they_say() {
    assert_eq!(row(R::Unreachable).state, State::Unknown);
    assert_eq!(
        row(R::Unreachable).reason.as_deref(),
        Some("cannot_assess:unreachable")
    );
    assert_eq!(
        row(R::NotFound).reason.as_deref(),
        Some("cannot_assess:not_found")
    );
    assert_eq!(row(R::Near).state, State::Degraded);
    assert_eq!(row(R::Near).reason.as_deref(), Some(NEAR));
    assert_eq!(row(R::Latency).state, State::Degraded);
    assert_eq!(row(R::Latency).reason, None);
    assert_eq!(row(R::RateLimited).reason, None);
}

#[tokio::test(flavor = "multi_thread")]
async fn under_the_default_each_hard_failure_and_a_near_limit_window_files() {
    for r in [
        R::QuotaExhausted,
        R::AuthFailed,
        R::ModelMissing,
        R::Unreachable,
        R::Near,
    ] {
        assert_eq!(filed("", &[R::Ok, r]).await, 1, "ok -> {r:?} files ONE");
        assert_eq!(filed("", &[r]).await, 1, "a first reading of {r:?} files");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn under_the_default_latency_degraded_and_rate_limited_file_nothing() {
    for r in [R::Latency, R::RateLimited] {
        assert_eq!(filed("", &[R::Ok, r, r, R::Ok, r]).await, 0, "{r:?}");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn latency_degraded_and_rate_limited_do_not_arm_the_alert() {
    for r in [R::Latency, R::RateLimited] {
        let rig = rig("");
        put_and_alert(&rig, R::Ok).await;
        put_and_alert(&rig, r).await;
        assert!(
            matches!(alerted(&rig).as_deref(), None | Some("ok")),
            "{r:?} left the key alerted: {:?}",
            alerted(&rig)
        );
        // had it armed (recorded as the alerted state), this would be the same outage and file nothing
        put_and_alert(&rig, R::ModelMissing).await;
        assert_eq!(
            rig.fake.filed(),
            1,
            "{r:?} then model_missing files the model_missing"
        );
        // and with no ok in between, from the very first reading
        assert_eq!(filed("", &[r, R::QuotaExhausted]).await, 1, "{r:?} first");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_cannot_assess_that_is_not_unreachable_still_files_nothing_and_never_clears() {
    assert_eq!(filed("", &[R::Ok, R::NotFound, R::NotFound]).await, 0);
    // after an alert, neither kind of CANNOT-ASSESS clears it: the next hard failure is the same outage
    assert_eq!(
        filed("", &[R::ModelMissing, R::NotFound, R::ModelMissing]).await,
        1
    );
    assert_eq!(
        filed("", &[R::ModelMissing, R::Unreachable, R::ModelMissing]).await,
        1
    );
    // an unreachable crossing is one outage with the hard failure that follows it
    assert_eq!(
        filed("", &[R::Unreachable, R::ModelMissing, R::Unreachable]).await,
        1
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn exp004_flapping_ok_and_latency_degraded_ten_cycles_files_zero() {
    let seq: Vec<R> = (0..10)
        .map(|i| if i % 2 == 0 { R::Ok } else { R::Latency })
        .collect();
    assert_eq!(filed("", &seq).await, 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn control_ok_and_near_limit_ten_cycles_still_files_once_per_crossing() {
    // the zero above can fail: the same flapping with a near-limit window files on every return to it
    let seq: Vec<R> = (0..10)
        .map(|i| if i % 2 == 0 { R::Ok } else { R::Near })
        .collect();
    assert_eq!(filed("", &seq).await, 5);
    // and a near-limit window held for ten cycles is ONE crossing
    assert_eq!(filed("", &[R::Near; 10]).await, 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_configured_list_with_rate_limited_files_it() {
    let with_rl = "[alert]\nstates = [\"quota_exhausted\", \"auth_failed\", \"model_missing\", \"unreachable\", \
                   \"window_near_limit\", \"rate_limited\"]\n";
    assert_eq!(filed(with_rl, &[R::Ok, R::RateLimited]).await, 1);
    assert_eq!(
        filed(with_rl, &[R::Ok, R::RateLimited, R::Ok, R::RateLimited]).await,
        2
    );
    // control: the default rig files nothing for the same sequence
    assert_eq!(filed("", &[R::Ok, R::RateLimited]).await, 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_configured_list_is_the_whole_list_not_added_to_the_default() {
    let only_rl = "[alert]\nstates = [\"rate_limited\"]\n";
    assert_eq!(filed(only_rl, &[R::Ok, R::RateLimited]).await, 1);
    for r in [R::ModelMissing, R::Unreachable, R::Near, R::Latency] {
        assert_eq!(filed(only_rl, &[R::Ok, r]).await, 0, "{r:?} is not listed");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn degraded_in_the_list_files_a_latency_degraded_row() {
    let deg = "[alert]\nstates = [\"degraded\"]\n";
    assert_eq!(filed(deg, &[R::Ok, R::Latency]).await, 1);
    assert_eq!(filed(deg, &[R::Ok, R::Near]).await, 1);
    // window_near_limit alone does not take a slow 200
    let near = "[alert]\nstates = [\"window_near_limit\"]\n";
    assert_eq!(filed(near, &[R::Ok, R::Latency]).await, 0);
    assert_eq!(filed(near, &[R::Ok, R::Near]).await, 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn an_unreachable_service_measured_by_the_probe_files_under_the_default() {
    // end to end: the probe writes what "unreachable" is in this codebase, and the alert files it
    let port = common::closed_port();
    let rig = rig_with("", &format!("http://127.0.0.1:{port}/kimi"));
    let p = cli(&rig, &["probe", "--force"]).await;
    assert_eq!(rc(&p), 0, "probe: {}", text(&p));
    let r = rig
        .store
        .get(KEY)
        .await
        .unwrap()
        .expect("the probe wrote the row");
    assert_eq!(r.state, State::Unknown, "{r:?}");
    assert_eq!(
        r.reason.as_deref(),
        Some("cannot_assess:unreachable"),
        "{r:?}"
    );
    let a = cli(&rig, &["alert"]).await;
    assert_eq!(rc(&a), 0, "alert: {}", text(&a));
    assert_eq!(rig.fake.filed(), 1, "unreachable files: {}", text(&a));
}

// ── DoD 5: the docs ─────────────────────────────────────────────────────────────────────────────────────────────

const RULING: &str = "SIG-010: go with the recommended default, option 3";

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

/// §3's table row for `alert` (the line starting `| alert |`).
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

#[test]
fn design_3_alert_row_quotes_the_ruling_with_its_signal() {
    let row = alert_row(&read("docs/DESIGN.md"));
    assert!(row.contains(RULING), "§3 alert row lacks the ruling: {row}");
    assert!(
        row.contains("states"),
        "§3 alert row names `[alert] states`: {row}"
    );
}

#[test]
fn design_10_q8_quotes_the_ruling_with_its_signal() {
    let q = q8(&read("docs/DESIGN.md"));
    assert!(q.contains(RULING), "§10 Q8 lacks the ruling: {q}");
    assert!(
        q.contains("kanban-work/signals/SIG-010"),
        "§10 Q8 cites the signal file as the other rulings do: {q}"
    );
}

#[test]
fn control_the_design_readers_can_fail() {
    let d = read("docs/DESIGN.md");
    let q = q8(&d);
    assert!(q.contains("SIG-006"), "Q8 is the alert item type question");
    assert!(!q.contains("statusLine"), "Q8 stops before Q9");
    let row = alert_row(&d);
    assert!(row.contains("crossing-dedup"));
    assert!(!row.contains("| `select` |") && !row.contains("not-a-word-in-design-xyzzy"));
}

#[test]
fn the_example_config_shows_alert_states_with_the_default_and_loads() {
    let text = read("examples/quotabus.toml");
    let v: toml::Value = toml::from_str(&text).expect("examples/quotabus.toml is TOML");
    let states = v
        .get("alert")
        .and_then(|a| a.get("states"))
        .and_then(|s| s.as_array())
        .expect("examples/quotabus.toml has `[alert] states = [...]`");
    let names: Vec<&str> = states.iter().map(|s| s.as_str().unwrap()).collect();
    assert_eq!(set(&names), set(&DEFAULT), "the example shows the default");
    let c = Config::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/quotabus.toml"))
        .expect("examples/quotabus.toml loads");
    assert_eq!(set(&c.alert.states), set(&DEFAULT));
}
