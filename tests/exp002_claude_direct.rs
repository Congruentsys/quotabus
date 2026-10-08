//! EXP-002 Plan 2 (direct read): per Claude account, one 1-token `POST <base>/v1/messages` with the account's
//! setup-token, and the `anthropic-ratelimit-unified-*` headers → one `subscription.anthropic.<account>.<service>`
//! row with its 5 h and 7 d windows (`used_pct`, `reset_at`, `reset_in_s`, `pace`). `allowed` → ok, `rejected` →
//! quota_exhausted, 401/403 → auth_failed (the bad-token control is never ok). An absent window is never 0 %.
//! A loopback stub, fake tokens, a fake clock (`now()`, 9 000 s before the measured 5 h reset).

mod exp002;

use exp002::*;
use quotabus::{Kind, ProbeSource, State};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn stub(reply: ResponseTemplate) -> MockServer {
    let s = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(reply)
        .mount(&s)
        .await;
    s
}

async fn read_once(reply: ResponseTemplate) -> (MockServer, Vec<quotabus::Record>) {
    let s = stub(reply).await;
    let r = runner(config(&claude_both(&s.uri())), None);
    let rows = r.run_due(&[], now(), true).await;
    (s, rows)
}

#[tokio::test]
async fn known_answer_the_measured_headers() {
    let (_s, rows) = read_once(measured()).await;
    let r = find(&rows, CLAUDE_KEY);
    assert_eq!(r.state, State::Ok, "{}", json(r));
    assert_eq!(r.kind, Kind::Subscription);
    assert_eq!(r.provider, "anthropic");
    assert_eq!(r.account, "hankh95");
    assert_eq!(
        r.model, "claude-hankh95",
        "the model slot is the service id"
    );
    assert_eq!(r.checked_at, now());
    assert_eq!(
        r.probe.source,
        ProbeSource::Undocumented,
        "the unified headers are undocumented (SIG-003)"
    );
    assert_eq!(r.probe.name, "unified_headers");
    let w5 = must_window(r, "five_hour");
    assert!(approx(f(&w5["used_pct"]), 30.0, 1e-9), "0.3 → 30 %: {w5}");
    assert_eq!(
        w5["reset_at"]
            .as_str()
            .map(|s| chrono::DateTime::parse_from_rfc3339(s).unwrap().timestamp()),
        Some(RESET_5H)
    );
    let w7 = must_window(r, "seven_day");
    assert!(approx(f(&w7["used_pct"]), 12.0, 1e-9), "0.12 → 12 %: {w7}");
}

#[tokio::test]
async fn reset_in_s_and_pace_per_window() {
    let (_s, rows) = read_once(measured()).await;
    let r = find(&rows, CLAUDE_KEY);
    let at = now().timestamp();
    for (name, pct, reset, len) in [
        ("five_hour", 30.0, RESET_5H, 18_000.0),
        ("seven_day", 12.0, RESET_7D, 604_800.0),
    ] {
        let w = must_window(r, name);
        assert_eq!(w["reset_in_s"].as_i64(), Some(reset - at), "{name}: {w}");
        let pace = (pct / 100.0) / ((len - (reset - at) as f64) / len);
        assert!(
            approx(f(&w["pace"]), pace, 1e-6),
            "{name}: pace {} vs {pace}",
            w["pace"]
        );
    }
    // by hand: 30 % used with half the 5 h window gone is a pace of 0.6
    let w5 = must_window(r, "five_hour");
    assert_eq!(w5["reset_in_s"].as_i64(), Some(9000));
    assert!(approx(f(&w5["pace"]), 0.6, 1e-9));
}

#[tokio::test]
async fn the_row_ttl_is_the_subscription_kinds() {
    let (_s, rows) = read_once(measured()).await;
    assert_eq!(find(&rows, CLAUDE_KEY).ttl_s, 3 * 3600, "default 3 × 1 h");
    let s = stub(measured()).await;
    let cfg = config(&format!(
        "[ttl]\nsubscription = \"7m\"\n\n{}",
        claude_both(&s.uri())
    ));
    let rows = runner(cfg, None).run_due(&[], now(), true).await;
    assert_eq!(find(&rows, CLAUDE_KEY).ttl_s, 420);
}

#[tokio::test]
async fn the_request_is_one_token_with_the_setup_token_as_bearer() {
    let (s, _) = read_once(measured()).await;
    let reqs = requests(&s, "/v1/messages").await;
    assert_eq!(reqs.len(), 1, "one call per account");
    let q = &reqs[0];
    let h = |n: &str| {
        q.headers
            .get(n)
            .map(|v| v.to_str().unwrap_or("").to_string())
    };
    assert_eq!(
        h("authorization").as_deref(),
        Some(format!("Bearer {FAKE_OAT}").as_str())
    );
    assert!(
        h("anthropic-beta").is_some_and(|v| v.contains("oauth-2025-04-20")),
        "{:?}",
        h("anthropic-beta")
    );
    assert!(h("anthropic-version").is_some());
    assert!(h("x-api-key").is_none(), "a setup-token is not an API key");
    let body: serde_json::Value = serde_json::from_slice(&q.body).unwrap();
    assert_eq!(body["max_tokens"], 1, "{body}");
    assert_eq!(body["model"], MODEL);
}

#[tokio::test]
async fn rejected_reads_quota_exhausted() {
    let reply = unified(
        429,
        Some((1.0, RESET_5H, "rejected")),
        Some((0.4, RESET_7D, "allowed")),
        Some("rejected"),
    )
    .insert_header("retry-after", "9000");
    let (_s, rows) = read_once(reply).await;
    let r = find(&rows, CLAUDE_KEY);
    assert_eq!(r.state, State::QuotaExhausted, "{}", json(r));
    assert!(approx(
        f(&must_window(r, "five_hour")["used_pct"]),
        100.0,
        1e-9
    ));
}

#[tokio::test]
async fn control_allowed_with_high_usage_is_not_exhausted() {
    let (_s, rows) = read_once(unified(
        200,
        Some((0.97, RESET_5H, "allowed")),
        Some((0.4, RESET_7D, "allowed")),
        Some("allowed"),
    ))
    .await;
    let r = find(&rows, CLAUDE_KEY);
    assert_ne!(
        r.state,
        State::QuotaExhausted,
        "only `rejected` exhausts: {}",
        json(r)
    );
    assert_ne!(r.state, State::Unknown, "{}", json(r));
}

#[tokio::test]
async fn a_bad_token_reads_auth_failed_never_ok() {
    for code in [401u16, 403] {
        let reply = ResponseTemplate::new(code).set_body_json(serde_json::json!(
            {"type": "error", "error": {"type": "authentication_error", "message": "invalid x-api-key"}}
        ));
        let (_s, rows) = read_once(reply).await;
        let r = find(&rows, CLAUDE_KEY);
        assert_eq!(r.state, State::AuthFailed, "{code}: {}", json(r));
    }
}

#[tokio::test]
async fn an_absent_window_is_never_written_as_zero() {
    let (_s, rows) = read_once(unified(
        200,
        None,
        Some((0.12, RESET_7D, "allowed")),
        Some("allowed"),
    ))
    .await;
    let r = find(&rows, CLAUDE_KEY);
    assert!(
        window(r, "five_hour").is_none(),
        "a 5 h window the headers did not carry: {}",
        json(r)["headroom"]
    );
    assert!(approx(
        f(&must_window(r, "seven_day")["used_pct"]),
        12.0,
        1e-9
    ));
}

#[tokio::test]
async fn a_measured_zero_is_kept() {
    // the negative control for "never 0": a window the headers REPORT as 0 is a measurement
    let (_s, rows) = read_once(unified(
        200,
        Some((0.0, RESET_5H, "allowed")),
        Some((0.12, RESET_7D, "allowed")),
        Some("allowed"),
    ))
    .await;
    assert_eq!(
        f(&must_window(find(&rows, CLAUDE_KEY), "five_hour")["used_pct"]),
        0.0
    );
}

#[tokio::test]
async fn observed_by_is_the_probe_host_and_one_row_per_account() {
    let s = stub(measured()).await;
    let text = format!(
        "{}\n{}",
        claude_svc("claude-a", "acct-a", &s.uri(), &["unified_headers"]),
        claude_svc("claude-b", "acct-b", &s.uri(), &["unified_headers"])
    );
    let rows = runner(config(&text), None).run_due(&[], now(), true).await;
    let a = find(&rows, "subscription.anthropic.acct-a.claude-a");
    let b = find(&rows, "subscription.anthropic.acct-b.claude-b");
    for r in [a, b] {
        assert_eq!(
            r.observed_by,
            quotabus::observed_by(),
            "read centrally, observed by the probe host"
        );
    }
    assert_eq!(
        requests(&s, "/v1/messages").await.len(),
        2,
        "one call per account"
    );
}

#[tokio::test]
async fn the_direct_read_is_off_unless_listed_in_sources() {
    // SIG-003: the unified headers are undocumented — opt-in; with only stream_json listed, no HTTP call is made
    let s = stub(measured()).await;
    let dir = tempfile::tempdir().unwrap();
    let bin = fake_claude(dir.path(), &stream(Some(&measured_event())), 0);
    let cfg = config(&claude_svc(
        "claude-hankh95",
        "hankh95",
        &s.uri(),
        &["stream_json"],
    ));
    let rows = runner(cfg, Some(&bin)).run_due(&[], now(), true).await;
    assert!(
        requests(&s, "/v1/messages").await.is_empty(),
        "the undocumented route ran while unlisted"
    );
    assert_eq!(find(&rows, CLAUDE_KEY).probe.source, ProbeSource::Official);
}

#[tokio::test]
async fn an_unset_token_makes_no_call_and_reads_unknown() {
    let s = stub(measured()).await;
    let r = quotabus::Runner::new(config(&claude_both(&s.uri())), |_: &str| None);
    let rows = r.run_due(&[], now(), true).await;
    let row = find(&rows, CLAUDE_KEY);
    assert_eq!(row.state, State::Unknown);
    assert_eq!(row.reason.as_deref(), Some("cannot_assess:secret_unset"));
    assert!(requests(&s, "/v1/messages").await.is_empty());
}
