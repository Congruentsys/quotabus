//! CHORE-011: every subscription window's `used_pct` is rounded to at most 2 decimals WHEN THE ROW IS BUILT, for
//! all three sources: the unified headers (`anthropic-ratelimit-unified-*-utilization` × 100), stream-json's
//! `rate_limit_event … unifiedWindows.*.utilization` (× 100) and Copilot (`100 − percent_remaining`). Seen in
//! HAZ-003's real path: a header `0.14` was stored as `14.000000000000002`.
//!
//! Known answers (exact `==`, and the serialized row shows `14.0`, never `14.000000000000002`): 0.14 → 14.0,
//! 0.005 → 0.5, 0.29 → 29.0, 0.123456 → 12.35; Copilot percent_remaining 33.333 → 66.67, 86.0 → 14.0, and the
//! entitlement fallback 100/300 left → 66.67. The controls show the check can fail: the unrounded product is NOT
//! the known answer, and the "at most 2 decimals" predicate rejects the unrounded value. The mutation is the
//! unrounded build itself (the code before CHORE-011): it turns the 0.14 / 0.29 / 0.123456 / 33.333 cases red.
//! Fakes only: a wiremock stub, fake tokens, a fake `claude` script.

mod exp002;

use exp002::*;
use quotabus::Record;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ── the property and its controls ───────────────────────────────────────────────────────────────────────────────

/// At most 2 decimals: rounding to 2 decimals leaves the value unchanged.
fn at_most_2dp(x: f64) -> bool {
    (x * 100.0).round() / 100.0 == x
}

#[test]
fn control_the_unrounded_product_is_not_the_known_answer() {
    // If these held, the known-answer `==` checks below could not fail on the unrounded build.
    let raw = 0.14_f64 * 100.0;
    assert_ne!(raw, 14.0, "0.14 × 100 must be the noisy f64");
    assert_eq!(serde_json::to_string(&raw).unwrap(), "14.000000000000002");
    assert_ne!(0.29_f64 * 100.0, 29.0);
    assert_ne!(0.123456_f64 * 100.0, 12.35);
    assert_ne!(100.0 - 33.333_f64, 66.67);
}

#[test]
fn control_the_2dp_predicate_rejects_unrounded_values() {
    assert!(
        !at_most_2dp(0.14_f64 * 100.0),
        "14.000000000000002 must fail"
    );
    assert!(!at_most_2dp(0.123456_f64 * 100.0), "12.3456 must fail");
    assert!(!at_most_2dp(100.0 - 33.333_f64), "66.667 must fail");
    // and it accepts the known answers, so it is not false by construction
    for ok in [14.0, 0.5, 29.0, 12.35, 66.67, 0.0, 100.0] {
        assert!(at_most_2dp(ok), "{ok} has at most 2 decimals");
    }
}

/// Every window's `used_pct` (struct and JSON), and `headroom.window_pct`, has at most 2 decimals.
fn assert_rows_rounded(r: &Record) {
    let h = r
        .headroom
        .as_ref()
        .unwrap_or_else(|| panic!("no headroom: {}", json(r)));
    assert!(!h.windows.is_empty(), "no windows: {}", json(r));
    for w in &h.windows {
        assert!(
            at_most_2dp(w.used_pct),
            "{} used_pct {:?} has more than 2 decimals",
            w.window,
            w.used_pct
        );
    }
    if let Some(p) = h.window_pct {
        assert!(at_most_2dp(p), "window_pct {p:?} has more than 2 decimals");
    }
}

/// The exact `used_pct` of window `name`, from the built row's struct, the JSON value and the serialized text.
fn assert_used(r: &Record, name: &str, want: f64) {
    let w = r
        .headroom
        .as_ref()
        .and_then(|h| h.windows.iter().find(|w| w.window == name))
        .unwrap_or_else(|| panic!("no {name} window in {}", json(r)));
    assert_eq!(w.used_pct, want, "{name}: struct used_pct");
    let jw = must_window(r, name);
    assert_eq!(f(&jw["used_pct"]), want, "{name}: JSON used_pct {jw}");
    let text = serde_json::to_string(&jw).unwrap();
    let shown = serde_json::to_string(&want).unwrap();
    assert!(
        text.contains(&format!("\"used_pct\":{shown}")),
        "{name}: serialized window shows {text}, want used_pct {shown}"
    );
}

// ── unified headers ─────────────────────────────────────────────────────────────────────────────────────────────

async fn unified_row(five: f64, seven: f64) -> (MockServer, Record) {
    let s = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(unified(
            200,
            Some((five, RESET_5H, "allowed")),
            Some((seven, RESET_7D, "allowed")),
            Some("allowed"),
        ))
        .mount(&s)
        .await;
    let rows = runner(config(&claude_both(&s.uri())), None)
        .run_due(&[], now(), true)
        .await;
    let r = find(&rows, CLAUDE_KEY).clone();
    (s, r)
}

#[tokio::test]
async fn unified_headers_0_14_is_stored_as_14() {
    let (_s, r) = unified_row(0.005, 0.14).await;
    assert_eq!(r.probe.name, "unified_headers");
    assert_used(&r, "seven_day", 14.0);
    assert_used(&r, "five_hour", 0.5);
    assert_rows_rounded(&r);
    let text = serde_json::to_string(&r).unwrap();
    assert!(
        !text.contains("14.000000000000002"),
        "the row still carries the f64 noise: {text}"
    );
    // the most-used window is the seven-day one, and its headline pct is rounded too
    assert_eq!(r.headroom.as_ref().unwrap().window_pct, Some(14.0));
}

#[tokio::test]
async fn unified_headers_round_to_two_decimals() {
    let (_s, r) = unified_row(0.123456, 0.29).await;
    assert_used(&r, "five_hour", 12.35);
    assert_used(&r, "seven_day", 29.0);
    assert_rows_rounded(&r);
}

// ── stream-json fallback ────────────────────────────────────────────────────────────────────────────────────────

fn event(five: f64, seven: f64) -> String {
    format!(
        r#"{{"type":"rate_limit_event","rate_limit_info":{{"status":"allowed","resetsAt":{RESET_5H},"rateLimitType":"five_hour","unifiedWindows":{{"five_hour":{{"utilization":{five},"resetsAt":{RESET_5H}}},"seven_day":{{"utilization":{seven},"resetsAt":{RESET_7D}}}}}}}}}"#
    )
}

async fn stream_row(five: f64, seven: f64) -> Record {
    let s = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(unified(200, None, None, None))
        .mount(&s)
        .await;
    let dir = tempfile::tempdir().unwrap();
    let bin = fake_claude(dir.path(), &stream(Some(&event(five, seven))), 0);
    let rows = runner(config(&claude_both(&s.uri())), Some(&bin))
        .run_due(&[], now(), true)
        .await;
    let r = find(&rows, CLAUDE_KEY).clone();
    assert_eq!(r.probe.name, "stream_json", "{}", json(&r));
    r
}

#[tokio::test]
async fn stream_json_0_14_is_stored_as_14() {
    let r = stream_row(0.005, 0.14).await;
    assert_used(&r, "seven_day", 14.0);
    assert_used(&r, "five_hour", 0.5);
    assert_rows_rounded(&r);
    assert!(
        !serde_json::to_string(&r)
            .unwrap()
            .contains("14.000000000000002")
    );
}

#[tokio::test]
async fn stream_json_rounds_to_two_decimals() {
    let r = stream_row(0.123456, 0.29).await;
    assert_used(&r, "five_hour", 12.35);
    assert_used(&r, "seven_day", 29.0);
    assert_rows_rounded(&r);
}

// ── Copilot ─────────────────────────────────────────────────────────────────────────────────────────────────────

const USER: &str = "/copilot_internal/user";

fn copilot_reply(premium: serde_json::Value) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "login": "qb-fake-user",
        "copilot_plan": "individual",
        "quota_reset_date": "2026-11-01",
        "quota_snapshots": {
            "chat": {"entitlement": 0, "remaining": 0, "percent_remaining": 100.0, "unlimited": true},
            "premium_interactions": premium
        }
    }))
}

async fn copilot_row(premium: serde_json::Value) -> Record {
    let s = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(USER))
        .respond_with(copilot_reply(premium))
        .mount(&s)
        .await;
    let rows = runner(config(&copilot_svc(&s.uri(), &["copilot_internal"])), None)
        .run_due(&[], now(), true)
        .await;
    find(&rows, COPILOT_KEY).clone()
}

#[tokio::test]
async fn copilot_percent_remaining_33_333_is_66_67() {
    let r = copilot_row(serde_json::json!({
        "entitlement": 300, "remaining": 100, "percent_remaining": 33.333, "unlimited": false
    }))
    .await;
    assert_used(&r, "monthly", 66.67);
    assert_rows_rounded(&r);
    assert_eq!(r.headroom.as_ref().unwrap().window_pct, Some(66.67));
}

#[tokio::test]
async fn copilot_percent_remaining_86_is_14() {
    let r = copilot_row(serde_json::json!({
        "entitlement": 300, "remaining": 258, "percent_remaining": 86.0, "unlimited": false
    }))
    .await;
    assert_used(&r, "monthly", 14.0);
    assert_rows_rounded(&r);
}

#[tokio::test]
async fn copilot_entitlement_fallback_is_rounded() {
    // no percent_remaining: 100 of 300 left is 33.33…% left, 66.666…% used → 66.67
    let r = copilot_row(serde_json::json!({
        "entitlement": 300, "remaining": 100, "unlimited": false
    }))
    .await;
    assert_used(&r, "monthly", 66.67);
    assert_rows_rounded(&r);
}
