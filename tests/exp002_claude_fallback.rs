//! EXP-002 Plan 2 (fallback, the Captain's "Direct, stream-json fallback"): when the direct 200 carries no unified
//! headers, `claude -p --output-format stream-json` runs ONCE for that account with the token only in the child's
//! env (`CLAUDE_CODE_OAUTH_TOKEN`, never argv); its `rate_limit_event.rate_limit_info.unifiedWindows` gives the
//! windows (`source = official`, `probe.name = stream_json`). No `claude` binary, a failed run or no event →
//! `unknown` with a `cannot_assess:` reason, never 0 % and never ok. The binary is a fake script, injected as
//! `QUOTABUS_CLAUDE_BIN` through the Runner's env lookup.

mod exp002;

use exp002::*;
use quotabus::{ProbeSource, Record, State};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// A 200 with NO unified headers.
async fn bare_stub() -> MockServer {
    let s = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(unified(200, None, None, None))
        .mount(&s)
        .await;
    s
}

async fn run(stub: &MockServer, bin: Option<&std::path::Path>, sources: &[&str]) -> Vec<Record> {
    let cfg = config(&claude_svc(
        "claude-hankh95",
        "hankh95",
        &stub.uri(),
        sources,
    ));
    runner(cfg, bin).run_due(&[], now(), true).await
}

#[tokio::test]
async fn missing_headers_fall_back_to_stream_json() {
    let s = bare_stub().await;
    let dir = tempfile::tempdir().unwrap();
    let bin = fake_claude(dir.path(), &stream(Some(&measured_event())), 0);
    let rows = run(&s, Some(&bin), &["unified_headers", "stream_json"]).await;
    let r = find(&rows, CLAUDE_KEY);
    assert_eq!(r.state, State::Ok, "{}", json(r));
    assert_eq!(
        r.probe.source,
        ProbeSource::Official,
        "stream-json is Claude Code's own output"
    );
    assert_eq!(r.probe.name, "stream_json");
    assert!(approx(
        f(&must_window(r, "five_hour")["used_pct"]),
        30.0,
        1e-6
    ));
    assert!(approx(
        f(&must_window(r, "seven_day")["used_pct"]),
        12.0,
        1e-6
    ));
    assert_eq!(
        must_window(r, "five_hour")["reset_in_s"].as_i64(),
        Some(9000)
    );
    assert_eq!(
        claude_runs(dir.path()),
        1,
        "claude runs once for the account"
    );
}

#[tokio::test]
async fn the_token_goes_in_the_childs_env_never_argv() {
    let s = bare_stub().await;
    let dir = tempfile::tempdir().unwrap();
    let bin = fake_claude(dir.path(), &stream(Some(&measured_event())), 0);
    run(&s, Some(&bin), &["unified_headers", "stream_json"]).await;
    let log = claude_log(dir.path());
    assert!(
        log.iter().any(|l| l == "env:token-ok"),
        "CLAUDE_CODE_OAUTH_TOKEN was not the token: {log:?}"
    );
    let argv = log
        .iter()
        .find(|l| l.starts_with("argv:"))
        .expect("an argv line");
    assert!(
        !argv.contains(FAKE_OAT) && !argv.contains("sk-ant"),
        "the token is on argv: {argv}"
    );
    assert!(
        argv.contains("-p") && argv.contains("stream-json"),
        "{argv}"
    );
}

#[tokio::test]
async fn control_headers_present_means_no_fallback() {
    let s = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(measured())
        .mount(&s)
        .await;
    let dir = tempfile::tempdir().unwrap();
    let bin = fake_claude(dir.path(), &stream(Some(&measured_event())), 0);
    let rows = run(&s, Some(&bin), &["unified_headers", "stream_json"]).await;
    assert_eq!(claude_runs(dir.path()), 0, "the direct read sufficed");
    assert_eq!(find(&rows, CLAUDE_KEY).probe.name, "unified_headers");
}

#[tokio::test]
async fn an_auth_failure_is_not_retried_through_claude() {
    let s = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(401)
                .set_body_string("{\"error\":{\"type\":\"authentication_error\"}}"),
        )
        .mount(&s)
        .await;
    let dir = tempfile::tempdir().unwrap();
    let bin = fake_claude(dir.path(), &stream(Some(&measured_event())), 0);
    let rows = run(&s, Some(&bin), &["unified_headers", "stream_json"]).await;
    assert_eq!(find(&rows, CLAUDE_KEY).state, State::AuthFailed);
    assert_eq!(
        claude_runs(dir.path()),
        0,
        "a 401 is the answer; the fallback is for missing headers"
    );
}

#[tokio::test]
async fn no_claude_binary_reads_cannot_assess() {
    let s = bare_stub().await;
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("no-such-claude");
    let rows = run(&s, Some(&missing), &["unified_headers", "stream_json"]).await;
    let r = find(&rows, CLAUDE_KEY);
    assert_eq!(r.state, State::Unknown, "{}", json(r));
    assert!(
        r.reason
            .as_deref()
            .unwrap_or("")
            .starts_with("cannot_assess:"),
        "{:?}",
        r.reason
    );
    assert!(
        window(r, "five_hour").is_none(),
        "no reading, no window (never 0 %)"
    );
}

#[tokio::test]
async fn missing_headers_without_the_fallback_listed_read_cannot_assess() {
    let s = bare_stub().await;
    let dir = tempfile::tempdir().unwrap();
    let bin = fake_claude(dir.path(), &stream(Some(&measured_event())), 0);
    let rows = run(&s, Some(&bin), &["unified_headers"]).await;
    let r = find(&rows, CLAUDE_KEY);
    assert_eq!(
        r.state,
        State::Unknown,
        "a 200 with no usage headers is not a usage reading: {}",
        json(r)
    );
    assert!(
        r.reason
            .as_deref()
            .unwrap_or("")
            .starts_with("cannot_assess:")
    );
    assert_eq!(claude_runs(dir.path()), 0, "stream_json is not listed");
}

#[tokio::test]
async fn a_failed_claude_run_reads_cannot_assess() {
    let s = bare_stub().await;
    let dir = tempfile::tempdir().unwrap();
    let bin = fake_claude(dir.path(), "Invalid API key · Please run /login", 1);
    let rows = run(&s, Some(&bin), &["unified_headers", "stream_json"]).await;
    let r = find(&rows, CLAUDE_KEY);
    assert_eq!(r.state, State::Unknown, "{}", json(r));
    assert!(
        r.reason
            .as_deref()
            .unwrap_or("")
            .starts_with("cannot_assess:")
    );
}

#[tokio::test]
async fn a_stream_without_the_event_reads_cannot_assess() {
    let s = bare_stub().await;
    let dir = tempfile::tempdir().unwrap();
    let bin = fake_claude(dir.path(), &stream(None), 0);
    let rows = run(&s, Some(&bin), &["unified_headers", "stream_json"]).await;
    let r = find(&rows, CLAUDE_KEY);
    assert_eq!(r.state, State::Unknown, "{}", json(r));
    assert!(window(r, "five_hour").is_none() && window(r, "seven_day").is_none());
}

#[tokio::test]
async fn an_event_with_one_window_writes_only_that_window() {
    let s = bare_stub().await;
    let dir = tempfile::tempdir().unwrap();
    let ev = format!(
        r#"{{"type":"rate_limit_event","rate_limit_info":{{"status":"allowed","unifiedWindows":{{"seven_day":{{"utilization":0.12,"resetsAt":{RESET_7D}}}}}}}}}"#
    );
    let bin = fake_claude(dir.path(), &stream(Some(&ev)), 0);
    let rows = run(&s, Some(&bin), &["unified_headers", "stream_json"]).await;
    let r = find(&rows, CLAUDE_KEY);
    assert!(
        window(r, "five_hour").is_none(),
        "absent, not 0 %: {}",
        json(r)["headroom"]
    );
    assert!(approx(
        f(&must_window(r, "seven_day")["used_pct"]),
        12.0,
        1e-6
    ));
}
