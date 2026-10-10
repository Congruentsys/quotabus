//! CHORE-024 (CHORE-021 r1 nits F1, F2).
//!
//! DoD 1: a SUBSCRIPTION service driven through `quotabus alert` (fake nusy-kanban, file backend) over the same
//! crossings as tests/chore021_local_no_alert.rs files per SIG-010's default `[alert] states` (Captain 2026-10-09:
//! "SIG-010: go with the recommended default, option 3"): the hard failures (quota_exhausted, auth_failed,
//! model_missing, unreachable) plus a near-limit window; latency-degraded and rate_limited file nothing. The control
//! goes red if subscription were skipped like local: the checker rejects a "no filings" outcome, and the local
//! service fed the SAME readings in the SAME run (which IS skipped) is rejected by the same checker.
//!
//! DoD 2: the summary line reports skipped local slots separately. Format pinned here (test partner, for the
//! implementer), the LAST line of `alert`'s stdout, always printed, every field always present:
//!
//! ```text
//! alert: <N> keys checked, <M> local skipped, <F> crossings filed, <C> re-armed
//! ```
//!
//! `N` counts only the slots that alert actually checks (api + subscription); `M` counts the slots of
//! `kind = "local"` services that were skipped (each listed model is one slot); `N + M` is every configured slot.
//!
//! Fakes only: the file backend in a temp dir, a recording fake sink, a fake key. No bus, no real key.

mod common;

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Output;
use std::time::Duration;

use chrono::Utc;
use common::{FAKE_PLAIN, rc, run_cli, text};
use quotabus::{Backend, FileBackend, Headroom, Kind, Record, State, record_key};

const T: Duration = Duration::from_secs(60);
const API_SECRET: &str = "QB_KIMI";
const SUB_SECRET: &str = "QB_GH_TOKEN";
const NEAR: &str = "window_near_limit";

fn sub_key() -> String {
    record_key(Kind::Subscription, "github", "hankh95", "copilot")
}
fn local_key(model: &str) -> String {
    record_key(Kind::Local, "qwen", "dgx1", model)
}
fn api_key() -> String {
    record_key(Kind::Api, "moonshot", "nusy-product-team", "kimi-k2.5")
}

// ── the rig ───────────────────────────────────────────────────────────────────────────────────────────────────────

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

/// Services: a local `qwen` with `local_models` (omitted when empty), optionally the API `kimi`, optionally the
/// subscription `copilot`. `alert_table` is `""` (SIG-010's default) or an `[alert]` table.
fn rig_full(alert_table: &str, local_models: &[&str], api: bool, sub: bool) -> Rig {
    let fake = Fake::new();
    let dir = tempfile::tempdir().unwrap();
    let state_dir = dir.path().join("state");
    std::fs::create_dir_all(&state_dir).unwrap();
    let cfg = dir.path().join("quotabus.toml");
    let mut s = format!(
        "[file]\ndir = {:?}\n\n[ttl]\napi = \"2m\"\nbalance = \"2m\"\nsubscription = \"2m\"\n\n",
        state_dir.display().to_string()
    );
    if !local_models.is_empty() {
        let models: Vec<String> = local_models.iter().map(|m| format!("{m:?}")).collect();
        s.push_str(&format!(
            "[[service]]\nid = \"qwen\"\nkind = \"local\"\nprovider = \"qwen\"\nfamily = \"qwen\"\n\
             account = \"dgx1\"\nbase_url = \"http://127.0.0.1:1/v1\"\nprotocol = \"openai\"\n\
             models = [{}]\nroles = [\"review\"]\ncost_class = \"local\"\n\n",
            models.join(", ")
        ));
    }
    if api {
        s.push_str(&format!(
            "[[service]]\nid = \"kimi\"\nkind = \"api\"\nprovider = \"moonshot\"\nfamily = \"moonshot\"\n\
             account = \"nusy-product-team\"\nbase_url = \"http://127.0.0.1:1/kimi\"\nprotocol = \"anthropic\"\n\
             models = [\"kimi-k2.5\"]\nroles = [\"review\"]\ncost_class = \"metered\"\nsecret = \"{API_SECRET}\"\n\n"
        ));
    }
    if sub {
        s.push_str(&format!(
            "[[service]]\nid = \"copilot\"\nkind = \"subscription\"\nprovider = \"github\"\nfamily = \"openai\"\n\
             account = \"hankh95\"\nsecret = \"{SUB_SECRET}\"\nsources = [\"copilot_internal\"]\n\n"
        ));
    }
    s.push_str(&format!(
        "{alert_table}\n[alert.nusy-kanban]\ncommand = {:?}\nitem_type = \"signal\"\ntags = [\"provider-status\"]\n",
        fake.bin.display().to_string()
    ));
    std::fs::write(&cfg, s).unwrap();
    Rig {
        fake,
        _dir: dir,
        cfg,
        store: FileBackend::new(&state_dir),
        state_dir,
    }
}

/// The DoD 1 rig: one local service (one model), one subscription service.
fn rig(alert_table: &str) -> Rig {
    rig_full(alert_table, &["qwen3"], false, true)
}

async fn alert(rig: &Rig) -> Output {
    let c = rig.cfg.clone();
    let out = tokio::task::spawn_blocking(move || {
        run_cli(
            &c,
            &["alert"],
            &[
                ("PATH", "/usr/bin:/bin"),
                (API_SECRET, FAKE_PLAIN),
                (SUB_SECRET, FAKE_PLAIN),
            ],
            T,
        )
    })
    .await
    .unwrap();
    assert_eq!(rc(&out), 0, "alert: {}", text(&out));
    out
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
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
    /// `degraded` + reason `window_near_limit` (a subscription window at 97 %).
    Near,
}

/// SIG-010's default `[alert] states`: these file.
const FILES_BY_DEFAULT: [R; 5] = [
    R::QuotaExhausted,
    R::AuthFailed,
    R::ModelMissing,
    R::Unreachable,
    R::Near,
];
/// Not in the default: these stay on the bus and file nothing.
const QUIET_BY_DEFAULT: [R; 2] = [R::Latency, R::RateLimited];

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

/// ok → bad → ok → bad … for `cycles` round trips, ending on ok.
fn flapping(bad: R, cycles: usize) -> Vec<R> {
    let mut v = vec![R::Ok];
    for _ in 0..cycles {
        v.push(bad);
        v.push(R::Ok);
    }
    v
}

/// SIG-010 under the crossing rule: the number of signals `seq` must file for an alerting service. A crossing is a
/// reading in `listed` whose previous ALERTED state is not bad (only a measured `ok` re-arms; a first reading
/// counts as a crossing from nothing).
fn expected_filings(seq: &[R], listed: &[R]) -> usize {
    let mut armed = true;
    let mut n = 0;
    for r in seq {
        if *r == R::Ok {
            armed = true;
        } else if listed.contains(r) && armed {
            n += 1;
            armed = false;
        }
    }
    n
}

/// The DoD 1 checker: an observed filing count for an alerting service over `seq` matches SIG-010's default.
fn files_per_sig010(observed: usize, seq: &[R]) -> Result<(), String> {
    let want = expected_filings(seq, &FILES_BY_DEFAULT);
    if observed == want {
        Ok(())
    } else {
        Err(format!(
            "filed {observed}, SIG-010's default files {want} for {seq:?}"
        ))
    }
}

/// Feed `seq` to the subscription AND the local service, one `alert` per step; (subscription filed, local filed).
async fn sub_and_local(alert_table: &str, seq: &[R]) -> (Rig, usize, usize) {
    let rig = rig(alert_table);
    for r in seq {
        rig.store.put(&row(&sub_key(), *r)).await.unwrap();
        rig.store.put(&row(&local_key("qwen3"), *r)).await.unwrap();
        alert(&rig).await;
    }
    let (s, l) = (rig.fake.filed_for("copilot"), rig.fake.filed_for("qwen"));
    (rig, s, l)
}

fn alert_entry_path(rig: &Rig, key: &str) -> PathBuf {
    rig.state_dir.join(format!("alert.{key}.json"))
}

// ── controls on the fixtures and the checker ──────────────────────────────────────────────────────────────────────

#[test]
fn control_the_fixtures_are_what_they_say() {
    assert_eq!(sub_key(), "subscription.github.hankh95.copilot");
    let u = row(&sub_key(), R::Unreachable);
    assert_eq!(u.kind, Kind::Subscription);
    assert_eq!(u.state, State::Unknown);
    assert_eq!(u.reason.as_deref(), Some("cannot_assess:unreachable"));
    assert_eq!(row(&sub_key(), R::Near).reason.as_deref(), Some(NEAR));
    assert_eq!(row(&local_key("qwen3"), R::Ok).kind, Kind::Local);
    assert_eq!(flapping(R::ModelMissing, 4).len(), 9);
}

#[test]
fn control_the_sig010_checker_has_known_answers_and_can_fail() {
    // known answers
    assert_eq!(
        expected_filings(&flapping(R::Unreachable, 4), &FILES_BY_DEFAULT),
        4
    );
    assert_eq!(
        expected_filings(&flapping(R::Latency, 4), &FILES_BY_DEFAULT),
        0
    );
    assert_eq!(
        expected_filings(
            &[R::Ok, R::Unreachable, R::ModelMissing, R::Ok, R::AuthFailed],
            &FILES_BY_DEFAULT
        ),
        2,
        "bad → bad is the same outage; only ok re-arms"
    );
    assert!(files_per_sig010(4, &flapping(R::Unreachable, 4)).is_ok());
    // negative control: the outcome "subscription skipped like local" (no filings at all) is rejected
    assert!(files_per_sig010(0, &flapping(R::Unreachable, 4)).is_err());
    assert!(files_per_sig010(0, &flapping(R::QuotaExhausted, 4)).is_err());
    // and so is "every bad state files" (SIG-010 option 1, not the default)
    assert!(files_per_sig010(4, &flapping(R::Latency, 4)).is_err());
}

// ── DoD 1: a subscription service files per SIG-010's default, end to end ───────────────────────────────────────

#[tokio::test(flavor = "multi_thread")]
async fn a_subscription_flapping_ok_unreachable_files_once_per_crossing() {
    let seq = flapping(R::Unreachable, 4);
    let (_rig, sub, local) = sub_and_local("", &seq).await;
    files_per_sig010(sub, &seq).unwrap_or_else(|e| panic!("subscription: {e}"));
    assert_eq!(sub, 4);
    // control, same run: the local service (skipped, CHORE-021) fed the same readings fails the same checker,
    // so the check above would go red if subscription were skipped like local
    assert_eq!(local, 0);
    assert!(files_per_sig010(local, &seq).is_err());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_subscription_flapping_ok_model_missing_files_once_per_crossing() {
    let seq = flapping(R::ModelMissing, 4);
    let (_rig, sub, local) = sub_and_local("", &seq).await;
    files_per_sig010(sub, &seq).unwrap_or_else(|e| panic!("subscription: {e}"));
    assert!(
        files_per_sig010(local, &seq).is_err(),
        "control: local skipped"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_subscription_mixing_unreachable_and_model_missing_files_per_crossing() {
    // the same sequence as chore021's mixed test: crossings at steps 2, 4, 6 (6→7 is one outage), 9
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
    let (_rig, sub, local) = sub_and_local("", &seq).await;
    assert_eq!(sub, 4);
    files_per_sig010(sub, &seq).unwrap();
    assert!(
        files_per_sig010(local, &seq).is_err(),
        "control: local skipped"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_subscription_files_each_default_state_and_not_the_others() {
    for bad in FILES_BY_DEFAULT {
        let seq = [R::Ok, bad, R::Ok, bad];
        let (_rig, sub, local) = sub_and_local("", &seq).await;
        assert_eq!(
            sub, 2,
            "subscription ok <-> {bad:?} files per crossing under the default"
        );
        assert!(
            files_per_sig010(local, &seq).is_err(),
            "control: local {bad:?} skipped"
        );
    }
    for bad in QUIET_BY_DEFAULT {
        let seq = [R::Ok, bad, R::Ok, bad];
        let (_rig, sub, _) = sub_and_local("", &seq).await;
        assert_eq!(
            sub, 0,
            "subscription {bad:?} is not in the default and files nothing"
        );
        files_per_sig010(sub, &seq).unwrap();
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_subscription_honours_a_configured_alert_states() {
    // `[alert] states` governs subscription rows too: listing rate_limited makes it file
    let t = "[alert]\nstates = [\"rate_limited\"]\n";
    let (_rig, sub, local) =
        sub_and_local(t, &[R::Ok, R::RateLimited, R::Ok, R::RateLimited]).await;
    assert_eq!(sub, 2);
    assert_eq!(local, 0);
    // and an unlisted hard failure then files nothing
    let (_rig, sub, _) = sub_and_local(t, &[R::Ok, R::QuotaExhausted]).await;
    assert_eq!(sub, 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_subscription_row_arms_its_entry_and_a_measured_ok_clears_it() {
    let rig = rig("");
    rig.store.put(&row(&sub_key(), R::Ok)).await.unwrap();
    alert(&rig).await;
    rig.store
        .put(&row(&sub_key(), R::QuotaExhausted))
        .await
        .unwrap();
    let a = alert(&rig).await;
    let p = alert_entry_path(&rig, &sub_key());
    assert!(
        p.exists(),
        "alert.{} was not written: {}",
        sub_key(),
        text(&a)
    );
    assert!(
        stdout(&a)
            .lines()
            .any(|l| l.starts_with("ALERT") && l.contains(&sub_key())),
        "no ALERT line for the subscription key:\n{}",
        text(&a)
    );
    let armed = std::fs::read(&p).unwrap();
    rig.store.put(&row(&sub_key(), R::Ok)).await.unwrap();
    let c = alert(&rig).await;
    assert!(
        stdout(&c)
            .lines()
            .any(|l| l.starts_with("CLEARED") && l.contains(&sub_key())),
        "a measured ok did not clear the subscription key:\n{}",
        text(&c)
    );
    assert_ne!(
        std::fs::read(&p).unwrap(),
        armed,
        "the entry was not rewritten by the clear"
    );
    assert_eq!(rig.fake.filed_for("copilot"), 1);
    // control, same rig: the local key with the same readings writes no entry
    assert!(!alert_entry_path(&rig, &local_key("qwen3")).exists());
}

// ── DoD 2: the summary line reports skipped local slots separately ────────────────────────────────────────────────

/// (checked, local skipped, filed, re-armed) from `alert`'s last stdout line, if it has the pinned format.
fn summary(out: &str) -> Option<(usize, usize, usize, usize)> {
    let last = out.lines().last()?;
    let rest = last.strip_prefix("alert: ")?;
    let parts: Vec<&str> = rest.split(", ").collect();
    if parts.len() != 4 {
        return None;
    }
    let num = |p: &str, suffix: &str| -> Option<usize> { p.strip_suffix(suffix)?.parse().ok() };
    Some((
        num(parts[0], " keys checked")?,
        num(parts[1], " local skipped")?,
        num(parts[2], " crossings filed")?,
        num(parts[3], " re-armed")?,
    ))
}

#[test]
fn control_the_summary_parser_has_known_answers_and_can_fail() {
    assert_eq!(
        summary("ALERT x\nalert: 2 keys checked, 3 local skipped, 1 crossings filed, 0 re-armed"),
        Some((2, 3, 1, 0))
    );
    // negative control: today's line (no local count) does not parse
    assert_eq!(
        summary("alert: 4 keys checked, 0 crossings filed, 0 re-armed"),
        None
    );
    // it must be the LAST line
    assert_eq!(
        summary("alert: 2 keys checked, 3 local skipped, 1 crossings filed, 0 re-armed\nmore"),
        None
    );
    assert_eq!(
        summary("alert: x keys checked, 3 local skipped, 1 crossings filed, 0 re-armed"),
        None
    );
}

async fn summary_of(rig: &Rig) -> (usize, usize, usize, usize) {
    let a = alert(rig).await;
    summary(&stdout(&a)).unwrap_or_else(|| panic!("no pinned summary line:\n{}", text(&a)))
}

#[tokio::test(flavor = "multi_thread")]
async fn the_summary_counts_local_slots_as_skipped_not_checked() {
    // 2 local slots, 1 api, 1 subscription; every row fresh and ok
    let rig = rig_full("", &["qwen3", "qwen3-coder"], true, true);
    for k in [
        local_key("qwen3"),
        local_key("qwen3-coder"),
        api_key(),
        sub_key(),
    ] {
        rig.store.put(&row(&k, R::Ok)).await.unwrap();
    }
    assert_eq!(summary_of(&rig).await, (2, 2, 0, 0));
}

#[tokio::test(flavor = "multi_thread")]
async fn the_summary_checked_count_does_not_move_with_the_local_slot_count() {
    // mutation: counting local slots into `checked` (today's behaviour) turns this red — N stays 2 while M is 0, 1, 3
    for (models, m) in [
        (&[][..], 0usize),
        (&["qwen3"][..], 1),
        (&["a", "b", "c"][..], 3),
    ] {
        let rig = rig_full("", models, true, true);
        let (n, skipped, _, _) = summary_of(&rig).await;
        assert_eq!((n, skipped), (2, m), "local models {models:?}");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn the_summary_counts_filings_and_re_arms_with_local_skipped() {
    // the local slot reads bad too: it is skipped, and its filing is not counted
    let rig = rig_full("", &["qwen3"], true, true);
    for k in [local_key("qwen3"), api_key(), sub_key()] {
        rig.store.put(&row(&k, R::Ok)).await.unwrap();
    }
    assert_eq!(summary_of(&rig).await, (2, 1, 0, 0));
    for k in [local_key("qwen3"), api_key(), sub_key()] {
        rig.store.put(&row(&k, R::QuotaExhausted)).await.unwrap();
    }
    assert_eq!(
        summary_of(&rig).await,
        (2, 1, 2, 0),
        "api + subscription filed; local skipped"
    );
    for k in [local_key("qwen3"), api_key(), sub_key()] {
        rig.store.put(&row(&k, R::Ok)).await.unwrap();
    }
    assert_eq!(
        summary_of(&rig).await,
        (2, 1, 0, 2),
        "api + subscription re-armed"
    );
    assert_eq!(rig.fake.filed_for("qwen"), 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_summary_with_only_local_services_checks_nothing() {
    let rig = rig_full("", &["qwen3", "qwen3-coder"], false, false);
    for k in [local_key("qwen3"), local_key("qwen3-coder")] {
        rig.store.put(&row(&k, R::Unreachable)).await.unwrap();
    }
    assert_eq!(summary_of(&rig).await, (0, 2, 0, 0));
}
