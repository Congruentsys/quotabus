//! CHORE-018 review r1 (`reviews/CHORE-018-r1.md`).
//!
//! F1: a source's OWN near-limit warning (Anthropic's `allowed_warning`, in the unified headers or in stream-json's
//! `rate_limit_event`) already reads `degraded` before the warn rule runs. That row is a near-limit window too
//! (Captain 2026-10-09 on SIG-010: "SIG-010: go with the recommended default, option 3"), so it carries
//! `reason = "window_near_limit"` and files under the default `[alert] states`, whatever its window's %. Copilot has no
//! native warning word: its near-limit reading is the warn rule's (a window ≥ `warn_pct`), checked here too. A
//! latency-degraded row is untouched.
//!
//! F3: end to end, a row probed through `Runner::run_due` goes through the decide path `quotabus alert` uses
//! (`freshness` → `alert::decide_listed` with the config's `[alert] states`) and files under the default; the control
//! drops the reason and shows the same row then files nothing.
//!
//! Fakes only: wiremock stubs, a fake `claude` script, fake tokens.

mod common;
mod exp002;

use std::time::Duration;

use exp002::*;
use quotabus::alert::{Decision, decide_listed};
use quotabus::{Config, Record, State, freshness};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const NEAR: &str = "window_near_limit";

/// What `quotabus alert` decides for `r` as a first reading (no alert entry), under the default `[alert] states`.
fn decide_default(r: &Record) -> Decision {
    let states = Config::from_toml_str("").unwrap().alert.states;
    let verdict = freshness(Some(r), r.checked_at);
    decide_listed(&verdict, r.reason.as_deref(), None, &states)
}

fn files(r: &Record) -> bool {
    matches!(decide_default(r), Decision::File { .. })
}

/// The Claude subscription row read from the unified headers: the 5 h window at `five`, every status word `status`.
async fn unified_row(five: f64, status: &str) -> Record {
    let s = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(unified(
            200,
            Some((five, RESET_5H, status)),
            Some((0.10, RESET_7D, "allowed")),
            Some(status),
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

/// The Claude subscription row read from stream-json's `rate_limit_event` (no unified headers): 5 h at `five`.
async fn stream_row(five: f64, status: &str) -> Record {
    let s = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(unified(200, None, None, None))
        .mount(&s)
        .await;
    let event = format!(
        r#"{{"type":"rate_limit_event","rate_limit_info":{{"status":"{status}","resetsAt":{RESET_5H},"rateLimitType":"five_hour","unifiedWindows":{{"five_hour":{{"utilization":{five},"resetsAt":{RESET_5H}}},"seven_day":{{"utilization":0.1,"resetsAt":{RESET_7D}}}}}}}}}"#
    );
    let dir = tempfile::tempdir().unwrap();
    let bin = fake_claude(dir.path(), &stream(Some(&event)), 0);
    let rows = runner(config(&claude_both(&s.uri())), Some(&bin))
        .run_due(&[], now(), true)
        .await;
    let r = find(&rows, CLAUDE_KEY).clone();
    assert_eq!(r.probe.name, "stream_json", "{}", json(&r));
    r
}

/// The Copilot row with `percent_remaining` left of 300 premium requests.
async fn copilot_row(percent_remaining: f64) -> Record {
    let s = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/copilot_internal/user"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "login": "qb-fake-user",
            "copilot_plan": "individual",
            "quota_reset_date": "2026-11-01",
            "quota_snapshots": {"premium_interactions": {
                "unlimited": false,
                "entitlement": 300,
                "remaining": 300.0 * percent_remaining / 100.0,
                "percent_remaining": percent_remaining,
            }}
        })))
        .mount(&s)
        .await;
    let rows = runner(config(&copilot_svc(&s.uri(), &["copilot_internal"])), None)
        .run_due(&[], now(), true)
        .await;
    find(&rows, COPILOT_KEY).clone()
}

// ── F1: the source's own near-limit warning ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn f1_unified_allowed_warning_at_97_is_near_limit_and_files_under_the_default() {
    let r = unified_row(0.97, "allowed_warning").await;
    assert_eq!(r.state, State::Degraded, "{}", json(&r));
    assert_eq!(r.reason.as_deref(), Some(NEAR), "{}", json(&r));
    assert!(files(&r), "{:?}", decide_default(&r));
}

#[tokio::test]
async fn f1_unified_allowed_warning_below_warn_pct_is_still_near_limit() {
    // the provider says the window is near its limit; that is the near-limit window whatever quotabus's warn_pct
    let r = unified_row(0.50, "allowed_warning").await;
    assert_eq!(r.state, State::Degraded, "{}", json(&r));
    assert_eq!(r.reason.as_deref(), Some(NEAR), "{}", json(&r));
    assert!(files(&r), "{:?}", decide_default(&r));
}

#[tokio::test]
async fn f1_stream_json_allowed_warning_is_near_limit_and_files() {
    for five in [0.50, 0.97] {
        let r = stream_row(five, "allowed_warning").await;
        assert_eq!(r.state, State::Degraded, "{five}: {}", json(&r));
        assert_eq!(r.reason.as_deref(), Some(NEAR), "{five}: {}", json(&r));
        assert!(files(&r), "{five}: {:?}", decide_default(&r));
    }
}

#[tokio::test]
async fn f1_stream_json_allowed_at_97_is_near_limit_through_the_warn_rule() {
    let r = stream_row(0.97, "allowed").await;
    assert_eq!(r.reason.as_deref(), Some(NEAR), "{}", json(&r));
    assert!(files(&r));
}

#[tokio::test]
async fn f1_copilot_at_97_used_is_near_limit_and_files() {
    let r = copilot_row(3.0).await;
    assert_eq!(r.state, State::Degraded, "{}", json(&r));
    assert_eq!(r.reason.as_deref(), Some(NEAR), "{}", json(&r));
    assert!(files(&r));
    // control: half used is ok, no reason, files nothing
    let r = copilot_row(50.0).await;
    assert_eq!(r.state, State::Ok, "{}", json(&r));
    assert_eq!(r.reason, None, "{}", json(&r));
    assert!(!files(&r));
}

#[tokio::test]
async fn f1_control_rejected_and_allowed_are_not_labelled_near_limit() {
    let r = unified_row(0.97, "rejected").await;
    assert_eq!(r.state, State::QuotaExhausted, "{}", json(&r));
    assert_ne!(r.reason.as_deref(), Some(NEAR), "{}", json(&r));
    let r = unified_row(0.50, "allowed").await;
    assert_eq!(r.state, State::Ok, "{}", json(&r));
    assert_eq!(r.reason, None, "{}", json(&r));
    assert!(!files(&r));
}

#[tokio::test]
async fn f1_a_latency_degraded_row_is_untouched_and_files_nothing() {
    let s = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(measured().set_delay(Duration::from_millis(400)))
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
    let r = rows
        .iter()
        .find(|r| r.key.starts_with("api.anthropic.acct."))
        .unwrap_or_else(|| panic!("no API row in {rows:?}"))
        .clone();
    assert_eq!(r.state, State::Degraded, "{}", json(&r));
    assert_ne!(r.reason.as_deref(), Some(NEAR), "{}", json(&r));
    assert!(!files(&r), "{:?}", decide_default(&r));
}

// ── F3: probe → alert, end to end ──────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn f3_a_probed_near_limit_window_files_under_the_default() {
    let r = unified_row(0.97, "allowed").await;
    assert_eq!(r.state, State::Degraded, "{}", json(&r));
    assert_eq!(
        decide_default(&r),
        Decision::File {
            state: State::Degraded,
            skip: vec![]
        }
    );
    // control: the same probed row with its reason dropped files nothing — the reason is what files it
    let mut bare = r.clone();
    bare.reason = None;
    assert_eq!(decide_default(&bare), Decision::Nothing);
}
