//! HAZ-003: the direct subscription read (`unified_headers`) got HTTP 429 `rate_limit_error`, with no
//! `anthropic-ratelimit-unified-*` headers, because it sent no `system`; the same call with
//! `"system": "You are Claude Code, Anthropic's official CLI for Claude."` got 200 with the headers (measured on M5,
//! 2026-10-08T20:21Z, same token, same minute). So every Claude row fell back to `claude -p` (`probe=stream_json`).
//!
//! DoD 1: the direct read's request body carries that `system` string. The stub here MIMICS the measured server: a
//! body with the system string gets the measured 200 + unified headers; any other body gets the measured 429 with
//! none. Controls show the stub (and its matcher) discriminate, so a pass is not by construction.
//! DoD 2: a 429 with no unified headers is never `ok`, never `quota_exhausted` (no header says so), and its reason
//! names the cause (the 429 / rate limit), distinct from a 200 that merely lacked the headers.
//! DoD 4 (light): DESIGN §4's Claude Max row records the requirement and cites this measurement.
//! Fakes only: a loopback wiremock stub, the fake setup-token, a fake `claude` script. No real key, no real bus.

mod exp002;

use exp002::*;
use quotabus::State;
use serde_json::Value;
use wiremock::matchers::{method, path};
use wiremock::{Match, Mock, MockServer, Request, ResponseTemplate};

/// The string Claude Code sends, which the measured server required.
const SYSTEM: &str = "You are Claude Code, Anthropic's official CLI for Claude.";

/// True when a `/v1/messages` body carries the Claude Code system prompt: `"system": SYSTEM`, or (the API's other
/// accepted form) a `system` array whose first text block is SYSTEM.
fn carries_system(body: &Value) -> bool {
    match &body["system"] {
        Value::String(s) => s == SYSTEM,
        Value::Array(blocks) => blocks
            .first()
            .and_then(|b| b["text"].as_str())
            .is_some_and(|t| t == SYSTEM),
        _ => false,
    }
}

struct HasSystem;

impl Match for HasSystem {
    fn matches(&self, request: &Request) -> bool {
        serde_json::from_slice::<Value>(&request.body)
            .map(|b| carries_system(&b))
            .unwrap_or(false)
    }
}

/// The measured refusal: 429 `rate_limit_error`, and no unified headers.
fn refused_429() -> ResponseTemplate {
    ResponseTemplate::new(429).set_body_json(serde_json::json!({
        "type": "error",
        "error": {"type": "rate_limit_error", "message": "Error"}
    }))
}

/// A stub that behaves like the measured server: the system string → 200 with the measured headers; else → 429.
async fn mimic() -> MockServer {
    let s = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(HasSystem)
        .respond_with(measured())
        .with_priority(1)
        .mount(&s)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(refused_429())
        .with_priority(10)
        .mount(&s)
        .await;
    s
}

async fn always(reply: ResponseTemplate) -> MockServer {
    let s = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(reply)
        .mount(&s)
        .await;
    s
}

fn direct_only(base: &str) -> String {
    claude_svc("claude-hankh95", "hankh95", base, &["unified_headers"])
}

fn has_unified(r: &reqwest::Response) -> bool {
    r.headers()
        .keys()
        .any(|k| k.as_str().starts_with("anthropic-ratelimit-unified"))
}

// ── the stub's own controls (they must hold for the DoD 1 tests to mean anything) ─────────────────────────────

#[test]
fn control_the_matcher_accepts_only_the_exact_system_string() {
    let base = serde_json::json!({"model": MODEL, "max_tokens": 1, "messages": []});
    assert!(!carries_system(&base), "no system field must not match");
    let mut wrong = base.clone();
    wrong["system"] = Value::String("You are a helpful assistant.".into());
    assert!(
        !carries_system(&wrong),
        "a different system string must not match"
    );
    let mut near = base.clone();
    near["system"] = Value::String(SYSTEM.trim_end_matches('.').into());
    assert!(
        !carries_system(&near),
        "a near-miss (mutated) string must not match"
    );
    let mut right = base.clone();
    right["system"] = Value::String(SYSTEM.into());
    assert!(carries_system(&right), "the exact string matches");
    let mut blocks = base;
    blocks["system"] = serde_json::json!([{"type": "text", "text": SYSTEM}]);
    assert!(
        carries_system(&blocks),
        "the block form of the same string matches"
    );
}

#[tokio::test]
async fn control_the_mimic_refuses_without_system_and_answers_with_it() {
    let s = mimic().await;
    let c = reqwest::Client::new();
    let url = format!("{}/v1/messages", s.uri());
    let without = c
        .post(&url)
        .json(&serde_json::json!({"model": MODEL, "max_tokens": 1,
            "messages": [{"role": "user", "content": "hi"}]}))
        .send()
        .await
        .unwrap();
    assert_eq!(without.status().as_u16(), 429, "the measured refusal");
    assert!(
        !has_unified(&without),
        "the refusal carries no unified headers"
    );
    let with = c
        .post(&url)
        .json(
            &serde_json::json!({"model": MODEL, "max_tokens": 1, "system": SYSTEM,
            "messages": [{"role": "user", "content": "hi"}]}),
        )
        .send()
        .await
        .unwrap();
    assert_eq!(with.status().as_u16(), 200);
    assert!(has_unified(&with), "the answer carries the unified headers");
}

// ── DoD 1: the direct read sends the system string ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn the_direct_read_body_carries_the_claude_code_system_prompt() {
    let s = always(measured()).await;
    let _ = runner(config(&direct_only(&s.uri())), None)
        .run_due(&[], now(), true)
        .await;
    let reqs = requests(&s, "/v1/messages").await;
    assert_eq!(reqs.len(), 1, "one call per account");
    let body: Value = serde_json::from_slice(&reqs[0].body).unwrap();
    assert!(
        carries_system(&body),
        "the direct read must send \"system\": {SYSTEM:?}; body was {body}"
    );
    // the rest of the request is unchanged (EXP-002)
    assert_eq!(body["max_tokens"], 1, "{body}");
    assert_eq!(body["model"], MODEL, "{body}");
}

#[tokio::test]
async fn against_a_server_that_requires_it_the_direct_read_reads_ok_with_both_windows() {
    let s = mimic().await;
    let rows = runner(config(&direct_only(&s.uri())), None)
        .run_due(&[], now(), true)
        .await;
    let r = find(&rows, CLAUDE_KEY);
    assert_eq!(r.state, State::Ok, "{}", json(r));
    assert_eq!(r.probe.name, "unified_headers", "{}", json(r));
    assert!(approx(
        f(&must_window(r, "five_hour")["used_pct"]),
        30.0,
        1e-9
    ));
    assert!(approx(
        f(&must_window(r, "seven_day")["used_pct"]),
        12.0,
        1e-9
    ));
}

#[tokio::test]
async fn with_the_fallback_listed_the_direct_read_answers_and_claude_never_runs() {
    let s = mimic().await;
    let dir = tempfile::tempdir().unwrap();
    let bin = fake_claude(dir.path(), &stream(Some(&measured_event())), 0);
    let rows = runner(config(&claude_both(&s.uri())), Some(&bin))
        .run_due(&[], now(), true)
        .await;
    let r = find(&rows, CLAUDE_KEY);
    assert_eq!(
        r.probe.name,
        "unified_headers",
        "the row fell back to stream_json: {}",
        json(r)
    );
    assert_eq!(r.state, State::Ok, "{}", json(r));
    assert_eq!(
        claude_runs(dir.path()),
        0,
        "no `claude -p` start was needed"
    );
}

// ── DoD 2: a 429 with no unified headers is classified honestly ───────────────────────────────────────────────

async fn direct_row(reply: ResponseTemplate) -> quotabus::Record {
    let s = always(reply).await;
    let rows = runner(config(&direct_only(&s.uri())), None)
        .run_due(&[], now(), true)
        .await;
    find(&rows, CLAUDE_KEY).clone()
}

fn names_the_429(reason: &str) -> bool {
    let r = reason.to_ascii_lowercase();
    r.contains("429") || r.contains("rate_limit") || r.contains("rate-limit")
}

#[tokio::test]
async fn a_429_without_unified_headers_is_neither_ok_nor_exhausted_and_its_reason_names_the_429() {
    for reply in [
        refused_429(),
        refused_429().insert_header("retry-after", "30"),
    ] {
        let r = direct_row(reply).await;
        let j = json(&r);
        assert_ne!(r.state, State::Ok, "a refusal is not a reading: {j}");
        assert_ne!(
            r.state,
            State::QuotaExhausted,
            "no unified header said `rejected`: {j}"
        );
        assert!(
            window(&r, "five_hour").is_none() && window(&r, "seven_day").is_none(),
            "{j}"
        );
        let reason = r.reason.as_deref().unwrap_or("");
        assert!(
            names_the_429(reason),
            "the reason must name the cause (the HTTP 429 rate_limit_error), got {reason:?}: {j}"
        );
        assert!(
            !j.to_string().contains(FAKE_OAT),
            "the token leaked into the row"
        );
    }
}

#[tokio::test]
async fn control_a_200_without_unified_headers_does_not_claim_a_429() {
    // the reason test above must not pass by a constant: a 200 that lacked the headers is a different cause
    let r = direct_row(ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "id": "msg_fake", "type": "message", "role": "assistant", "model": MODEL,
        "content": [{"type": "text", "text": "o"}], "stop_reason": "max_tokens",
        "usage": {"input_tokens": 35, "output_tokens": 1}
    })))
    .await;
    assert_eq!(r.state, State::Unknown, "{}", json(&r));
    let reason = r.reason.as_deref().unwrap_or("");
    assert!(
        !names_the_429(reason),
        "a 200 reason named a 429: {reason:?}"
    );
}

#[tokio::test]
async fn control_a_429_whose_headers_say_rejected_is_exhausted() {
    // the negative control for "never quota_exhausted": when a header DOES say so, it is
    let r = direct_row(
        unified(
            429,
            Some((1.0, RESET_5H, "rejected")),
            Some((0.4, RESET_7D, "allowed")),
            Some("rejected"),
        )
        .insert_header("retry-after", "9000"),
    )
    .await;
    assert_eq!(r.state, State::QuotaExhausted, "{}", json(&r));
}

// ── DoD 4 (light): DESIGN §4 records the requirement ──────────────────────────────────────────────────────────

#[test]
fn design_s4_claude_row_records_the_system_prompt_requirement() {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/DESIGN.md");
    let d = std::fs::read_to_string(&p).unwrap();
    let s4 = d
        .split("## 4. Probe catalogue")
        .nth(1)
        .and_then(|t| t.split("\n## 5.").next())
        .expect("DESIGN has §4");
    let row = s4
        .lines()
        .find(|l| l.starts_with("| Claude Max"))
        .expect("§4 has the Claude Max row");
    assert!(
        row.contains(SYSTEM),
        "the Claude row must name the system string the direct read needs"
    );
    assert!(
        row.contains("429"),
        "the row must record the 429 seen without it"
    );
    assert!(
        row.contains("HAZ-003"),
        "the row must cite this measurement (HAZ-003)"
    );
}
