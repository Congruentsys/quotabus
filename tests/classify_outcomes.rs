//! EXP-001 Plan 2-3 / DESIGN §2 `state`, §4: a pure map from one HTTP outcome to a state and headroom.

use quotabus::classify::headroom_from_headers;
use quotabus::{HttpOutcome, Outcome, State, Thresholds, classify};

const T: Thresholds = Thresholds {
    degraded_latency_ms: 5000,
};

fn http(status: u16, headers: &[(&str, &str)], body: &str, latency_ms: u64) -> Outcome {
    Outcome::Http(HttpOutcome {
        status,
        headers: headers
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        body: body.into(),
        latency_ms,
    })
}

fn state(o: &Outcome) -> State {
    classify(o, "glm-5.3", &T).state
}

#[test]
fn ok_200() {
    let c = classify(
        &http(200, &[], r#"{"content":[{"text":"hi"}]}"#, 300),
        "glm-5.3",
        &T,
    );
    assert_eq!(c.state, State::Ok);
    assert_eq!(c.reason, None);
    assert_eq!(c.latency_ms, Some(300));
}

#[test]
fn auth_failed_401_and_403() {
    assert_eq!(
        state(&http(401, &[], r#"{"error":"invalid x-api-key"}"#, 50)),
        State::AuthFailed
    );
    assert_eq!(
        state(&http(403, &[], r#"{"error":"forbidden"}"#, 50)),
        State::AuthFailed
    );
}

#[test]
fn model_missing_404_naming_the_model() {
    let body = r#"{"error":{"message":"Not found the model kimi-k2.5 or Permission denied","type":"resource_not_found_error"}}"#;
    assert_eq!(
        classify(&http(404, &[], body, 40), "kimi-k2.5", &T).state,
        State::ModelMissing
    );
}

#[test]
fn quota_exhausted_402() {
    assert_eq!(
        state(&http(402, &[], r#"{"error":"Insufficient Balance"}"#, 40)),
        State::QuotaExhausted
    );
}

#[test]
fn rate_limited_429_with_retry_after() {
    assert_eq!(
        state(&http(429, &[("retry-after", "30")], "slow down", 40)),
        State::RateLimited
    );
    assert_eq!(
        state(&http(429, &[("Retry-After", "5")], "slow down", 40)),
        State::RateLimited
    );
}

#[test]
fn rate_limited_429_with_a_reset_header() {
    assert_eq!(
        state(&http(
            429,
            &[("x-ratelimit-reset-requests", "2s")],
            "slow down",
            40
        )),
        State::RateLimited
    );
}

#[test]
fn quota_exhausted_429_without_a_retry_window() {
    // the mutation "every 429 is rate_limited" turns this red
    assert_eq!(
        state(&http(429, &[], r#"{"error":"too many requests"}"#, 40)),
        State::QuotaExhausted
    );
}

#[test]
fn quota_exhausted_when_the_body_says_monthly_quota() {
    let body = r#"{"error":{"message":"You have exceeded your monthly quota"}}"#;
    assert_eq!(
        state(&http(429, &[("retry-after", "30")], body, 40)),
        State::QuotaExhausted
    );
    assert_eq!(state(&http(403, &[], body, 40)), State::QuotaExhausted);
}

#[test]
fn quota_exhausted_on_anthropic_spend_cap() {
    let body = r#"{"type":"error","error":{"type":"rate_limit_error","message":"enforced_spend_limit_reached"}}"#;
    assert_eq!(
        state(&http(429, &[("retry-after", "60")], body, 40)),
        State::QuotaExhausted
    );
}

#[test]
fn degraded_when_latency_exceeds_the_threshold() {
    assert_eq!(state(&http(200, &[], "{}", 5001)), State::Degraded);
    assert_eq!(state(&http(200, &[], "{}", 9000)), State::Degraded);
}

#[test]
fn control_latency_at_the_threshold_is_still_ok() {
    // boundary known answer: "latency > threshold"; the mutation `>=` turns this red
    assert_eq!(state(&http(200, &[], "{}", 5000)), State::Ok);
    assert_eq!(state(&http(200, &[], "{}", 1)), State::Ok);
}

#[test]
fn connection_refused_is_unknown_cannot_assess_unreachable() {
    let c = classify(&Outcome::Unreachable, "glm-5.3", &T);
    assert_eq!(c.state, State::Unknown);
    assert_eq!(c.reason.as_deref(), Some("cannot_assess:unreachable"));
}

#[test]
fn control_a_server_error_is_never_ok() {
    // negative control: a 5xx is not a measured ok
    for status in [500, 502, 503] {
        let c = classify(&http(status, &[], "upstream error", 40), "glm-5.3", &T);
        assert_ne!(c.state, State::Ok, "status {status}");
    }
    assert_ne!(state(&http(404, &[], "no route", 40)), State::Ok);
}

#[test]
fn headroom_from_anthropic_headers() {
    let o = http(
        200,
        &[
            ("anthropic-ratelimit-requests-limit", "50"),
            ("Anthropic-RateLimit-Requests-Remaining", "49"),
            ("anthropic-ratelimit-requests-reset", "2026-10-08T18:00:00Z"),
            ("anthropic-ratelimit-tokens-limit", "40000"),
            ("anthropic-ratelimit-tokens-remaining", "39000"),
        ],
        "{}",
        200,
    );
    let h = classify(&o, "glm-5.3", &T)
        .headroom
        .expect("headroom from anthropic-ratelimit-* headers");
    assert_eq!(h.requests_remaining, Some(49));
    assert_eq!(h.tokens_remaining, Some(39000));
    assert_eq!(h.reset_at.as_deref(), Some("2026-10-08T18:00:00Z"));
}

#[test]
fn headroom_from_x_ratelimit_headers() {
    let headers: Vec<(String, String)> = [
        ("x-ratelimit-limit-requests", "100"),
        ("x-ratelimit-remaining-requests", "99"),
        ("x-ratelimit-limit-tokens", "150000"),
        ("X-RateLimit-Remaining-Tokens", "149000"),
        ("x-ratelimit-reset-requests", "600ms"),
    ]
    .iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect();
    let h = headroom_from_headers(&headers).expect("headroom from x-ratelimit-* headers");
    assert_eq!(h.requests_remaining, Some(99));
    assert_eq!(h.tokens_remaining, Some(149000));
}

#[test]
fn control_no_ratelimit_headers_means_no_headroom() {
    let headers = vec![("content-type".to_string(), "application/json".to_string())];
    assert_eq!(headroom_from_headers(&headers), None);
}
