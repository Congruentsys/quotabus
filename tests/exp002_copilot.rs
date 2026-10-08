//! EXP-002 Plan 3 / SIG-003: Copilot, read centrally — `GET <base_url>/copilot_internal/user` with the configured
//! token → a `monthly` window (`used_pct = 100 − percent_remaining`, `reset_at` from `quota_reset_date`) and
//! `requests_remaining`. Remaining 0 or "exceeded your monthly quota" → quota_exhausted. Every row is labelled
//! `probe.source = undocumented`; the source runs only when `sources` lists `copilot_internal`.

mod exp002;

use exp002::*;
use quotabus::{ProbeSource, Record, State};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const USER: &str = "/copilot_internal/user";

fn user(remaining: i64, percent_remaining: f64) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "login": "qb-fake-user",
        "copilot_plan": "individual",
        "quota_reset_date": "2026-11-01",
        "quota_snapshots": {
            "chat": {"entitlement": 0, "remaining": 0, "percent_remaining": 100.0, "unlimited": true},
            "premium_interactions": {"entitlement": 300, "remaining": remaining,
                                     "percent_remaining": percent_remaining, "unlimited": false}
        }
    }))
}

async fn read(reply: ResponseTemplate, sources: &[&str]) -> (MockServer, Vec<Record>) {
    let s = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(USER))
        .respond_with(reply)
        .mount(&s)
        .await;
    let rows = runner(config(&copilot_svc(&s.uri(), sources)), None)
        .run_due(&[], now(), true)
        .await;
    (s, rows)
}

#[tokio::test]
async fn known_answer_remaining_and_monthly_window() {
    let (s, rows) = read(user(120, 40.0), &["copilot_internal"]).await;
    let r = find(&rows, COPILOT_KEY);
    assert_eq!(r.state, State::Ok, "{}", json(r));
    assert_eq!(r.account, "hankh95");
    assert_eq!(r.observed_by, quotabus::observed_by());
    assert_eq!(json(r)["headroom"]["requests_remaining"], 120);
    let w = must_window(r, "monthly");
    assert!(approx(f(&w["used_pct"]), 60.0, 1e-9), "{w}");
    assert!(
        w["reset_at"]
            .as_str()
            .is_some_and(|s| s.starts_with("2026-11-01")),
        "{w}"
    );
    let reqs = requests(&s, USER).await;
    assert_eq!(reqs.len(), 1);
    let auth = reqs[0]
        .headers
        .get("authorization")
        .map(|v| v.to_str().unwrap().to_string());
    assert!(
        auth.as_deref()
            .is_some_and(|a| a == format!("Bearer {FAKE_GH}") || a == format!("token {FAKE_GH}")),
        "the configured token is sent: {auth:?}"
    );
}

#[tokio::test]
async fn every_copilot_row_is_labelled_undocumented() {
    let (_s, rows) = read(user(120, 40.0), &["copilot_internal"]).await;
    let r = find(&rows, COPILOT_KEY);
    assert_eq!(r.probe.source, ProbeSource::Undocumented);
    assert_eq!(r.probe.name, "copilot_internal");
    let (_s, rows) = read(ResponseTemplate::new(500), &["copilot_internal"]).await;
    assert_eq!(
        find(&rows, COPILOT_KEY).probe.source,
        ProbeSource::Undocumented,
        "a failed read too"
    );
}

#[tokio::test]
async fn zero_remaining_reads_quota_exhausted() {
    let (_s, rows) = read(user(0, 0.0), &["copilot_internal"]).await;
    assert_eq!(find(&rows, COPILOT_KEY).state, State::QuotaExhausted);
}

#[tokio::test]
async fn control_one_remaining_is_not_exhausted() {
    let (_s, rows) = read(user(1, 0.3), &["copilot_internal"]).await;
    let r = find(&rows, COPILOT_KEY);
    assert_ne!(r.state, State::QuotaExhausted, "{}", json(r));
    assert_ne!(r.state, State::Unknown, "{}", json(r));
}

#[tokio::test]
async fn exceeded_your_monthly_quota_reads_quota_exhausted() {
    let reply = ResponseTemplate::new(429).set_body_json(serde_json::json!(
        {"message": "You have exceeded your monthly quota for premium requests."}
    ));
    let (_s, rows) = read(reply, &["copilot_internal"]).await;
    assert_eq!(find(&rows, COPILOT_KEY).state, State::QuotaExhausted);
}

#[tokio::test]
async fn a_bad_token_reads_auth_failed() {
    let reply =
        ResponseTemplate::new(401).set_body_json(serde_json::json!({"message": "Bad credentials"}));
    let (_s, rows) = read(reply, &["copilot_internal"]).await;
    assert_eq!(find(&rows, COPILOT_KEY).state, State::AuthFailed);
}

#[tokio::test]
async fn an_unparseable_answer_reads_cannot_assess() {
    let (_s, rows) = read(
        ResponseTemplate::new(200).set_body_string("<html>"),
        &["copilot_internal"],
    )
    .await;
    let r = find(&rows, COPILOT_KEY);
    assert_eq!(r.state, State::Unknown);
    assert!(
        r.reason
            .as_deref()
            .unwrap_or("")
            .starts_with("cannot_assess:")
    );
}

#[tokio::test]
async fn off_unless_listed_in_sources() {
    let (s, rows) = read(user(120, 40.0), &[]).await;
    assert!(
        requests(&s, USER).await.is_empty(),
        "copilot_internal ran while unlisted"
    );
    if let Some(r) = rows.iter().find(|r| r.key == COPILOT_KEY) {
        assert_ne!(r.state, State::Ok, "a source that is off measures nothing");
    }
    // control: the same stub is reached once it is listed
    let (s, _) = read(user(120, 40.0), &["copilot_internal"]).await;
    assert_eq!(requests(&s, USER).await.len(), 1);
}
