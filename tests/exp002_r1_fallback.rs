//! EXP-002 review r1 findings F1 and F3, on the `claude -p` stream-json fallback.
//! - F1: the fallback child must not inherit the probe's whole environment. Under `doppler run`, that environment
//!   holds every Doppler secret, configured or not. The child gets `CLAUDE_CODE_OAUTH_TOKEN` and `PATH`, but not an
//!   unconfigured variable planted in the probe's environment.
//! - F3: the fallback maps state from `rate_limit_info.status`: `rejected` → quota_exhausted, `allowed` → ok, with the
//!   windows carried through.

mod common;
mod exp002;

use std::collections::BTreeSet;
use std::path::Path;
use std::time::Duration;

use exp002::*;
use quotabus::State;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer};

const PLANTED: &str = "QB_UNCONFIGURED_SECRET";

/// A 200 with NO unified headers, so the stream-json fallback runs.
async fn bare_stub() -> MockServer {
    let s = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(unified(200, None, None, None))
        .mount(&s)
        .await;
    s
}

/// A fake `claude` that writes the NAMES of its environment variables (never their values) to `dir/child.env`,
/// then prints the measured stream.
fn env_dumping_claude(dir: &Path) -> std::path::PathBuf {
    let p = dir.join("claude");
    let script = format!(
        "#!/bin/sh\nenv | cut -d= -f1 > '{dump}'\ncat <<'QB_OUT'\n{out}\nQB_OUT\n",
        dump = dir.join("child.env").display(),
        out = stream(Some(&measured_event()))
    );
    std::fs::write(&p, script).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    p
}

/// The variable names in an `env | cut -d= -f1` dump.
fn names(dump: &str) -> BTreeSet<String> {
    dump.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect()
}

#[test]
fn control_the_env_check_sees_a_variable_that_is_passed() {
    // the check would see the planted variable if the child got it: a dump that holds it reads as holding it
    let n = names(&format!("PATH\n{PLANTED}\nCLAUDE_CODE_OAUTH_TOKEN\n"));
    assert!(n.contains(PLANTED));
    assert!(n.contains("PATH") && n.contains("CLAUDE_CODE_OAUTH_TOKEN"));
    assert!(!names("PATH\nHOME\n").contains(PLANTED));
}

#[tokio::test(flavor = "multi_thread")]
async fn f1_the_fallback_child_does_not_inherit_an_unconfigured_variable() {
    let s = bare_stub().await;
    let dir = tempfile::tempdir().unwrap();
    let bin = env_dumping_claude(dir.path());
    let cfg = dir.path().join("quotabus.toml");
    std::fs::write(
        &cfg,
        format!(
            "[file]\ndir = {:?}\n\n{}",
            dir.path().join("store").display().to_string(),
            claude_both(&s.uri())
        ),
    )
    .unwrap();
    let home = dir.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let (c, b, h) = (
        cfg.clone(),
        bin.display().to_string(),
        home.display().to_string(),
    );
    let out = tokio::task::spawn_blocking(move || {
        common::run_cli(
            &c,
            &["probe"],
            &[
                (CLAUDE_SECRET, FAKE_OAT),
                ("QUOTABUS_CLAUDE_BIN", b.as_str()),
                ("PATH", "/usr/bin:/bin"),
                ("HOME", h.as_str()),
                (PLANTED, "x-unconfigured-doppler-secret"),
            ],
            Duration::from_secs(60),
        )
    })
    .await
    .unwrap();
    assert_eq!(common::rc(&out), 0, "{}", common::text(&out));
    let dump = std::fs::read_to_string(dir.path().join("child.env"))
        .unwrap_or_else(|e| panic!("the fallback did not run: {e}\n{}", common::text(&out)));
    let n = names(&dump);
    // the variables the child must have: these show that the dump is the child's real environment
    assert!(
        n.contains("CLAUDE_CODE_OAUTH_TOKEN"),
        "the child must get the token: {n:?}"
    );
    assert!(n.contains("PATH"), "the child must get PATH: {n:?}");
    assert!(
        !n.contains(PLANTED),
        "the child inherited {PLANTED}, which no service configures: {n:?}"
    );
}

async fn fallback_with_status(status: &str) -> quotabus::Record {
    let s = bare_stub().await;
    let dir = tempfile::tempdir().unwrap();
    let ev = format!(
        r#"{{"type":"rate_limit_event","rate_limit_info":{{"status":"{status}","resetsAt":{RESET_5H},"rateLimitType":"five_hour","unifiedWindows":{{"five_hour":{{"utilization":1.0,"resetsAt":{RESET_5H}}},"seven_day":{{"utilization":0.12,"resetsAt":{RESET_7D}}}}}}}}}"#
    );
    let bin = fake_claude(dir.path(), &stream(Some(&ev)), 0);
    let cfg = config(&claude_svc(
        "claude-hankh95",
        "hankh95",
        &s.uri(),
        &["unified_headers", "stream_json"],
    ));
    let rows = runner(cfg, Some(&bin)).run_due(&[], now(), true).await;
    assert_eq!(claude_runs(dir.path()), 1, "the fallback ran");
    find(&rows, CLAUDE_KEY).clone()
}

#[tokio::test]
async fn f3_a_rejected_rate_limit_event_reads_quota_exhausted() {
    let r = fallback_with_status("rejected").await;
    assert_eq!(r.probe.name, "stream_json");
    assert_eq!(r.state, State::QuotaExhausted, "{}", json(&r));
    assert!(approx(
        f(&must_window(&r, "five_hour")["used_pct"]),
        100.0,
        1e-6
    ));
    assert!(approx(
        f(&must_window(&r, "seven_day")["used_pct"]),
        12.0,
        1e-6
    ));
    assert_eq!(
        must_window(&r, "five_hour")["reset_in_s"].as_i64(),
        Some(9000)
    );
}

#[tokio::test]
async fn f3_control_an_allowed_rate_limit_event_reads_ok() {
    // the same event with only `status` changed: it is the status, not the 100 % window, that exhausts
    let r = fallback_with_status("allowed").await;
    assert_eq!(r.probe.name, "stream_json");
    assert_eq!(r.state, State::Ok, "{}", json(&r));
    assert!(approx(
        f(&must_window(&r, "seven_day")["used_pct"]),
        12.0,
        1e-6
    ));
}
