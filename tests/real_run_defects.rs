//! EXP-001 ruled test addition: two defects found by the driver's real run against the providers.
//!
//! D1: the openai-protocol probe sent its JSON body without `Content-Type: application/json` (xAI answered 415,
//! OpenAI 400 "you must provide a model parameter"), so the xAI negative control read `cannot_assess:http_415`
//! instead of `auth_failed`.
//! D2: the balance floor overwrote a non-ok probe state (Kimi `kimi-k2.5` 404 + balance below floor read
//! `quota_exhausted`). Ruling (driver, DESIGN §2): the floor turns a WORKING model (ok, degraded, rate_limited)
//! into `quota_exhausted`; it never replaces auth_failed, model_missing or unknown. The balance is on every row.
//! Loopback stubs and fake keys only.

mod common;

use std::collections::HashMap;

use common::FAKE_SK;
use quotabus::balance::apply_floor;
use quotabus::{Config, Record, Runner, State};
use serde_json::{Value, json};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

fn service(id: &str, provider: &str, base_url: &str, protocol: &str, model: &str) -> String {
    format!(
        "[[service]]\nid = {id:?}\nkind = \"api\"\nprovider = {provider:?}\nfamily = {provider:?}\n\
         account = \"nusy-product-team\"\nbase_url = {base_url:?}\nprotocol = {protocol:?}\nmodels = [{model:?}]\n\
         secret = \"QB_KEY\"\n"
    )
}

fn config(services: &[String]) -> Config {
    let text = format!(
        "[probe]\nttl = \"2m\"\nmax_tokens = 20\n\n{}",
        services.join("\n")
    );
    Config::from_toml_str(&text).unwrap_or_else(|e| panic!("test config must parse: {e}\n{text}"))
}

async fn run(cfg: Config) -> Vec<Record> {
    let m: HashMap<String, String> = [("QB_KEY".to_string(), FAKE_SK.to_string())].into();
    Runner::new(cfg, move |n: &str| m.get(n).cloned())
        .run_once()
        .await
}

/// Exactly ONE `content-type` header, and it is `application/json`. A doubled header (`.json()` plus an explicit
/// `.header(CONTENT_TYPE, ..)`; reqwest's `header` appends) is what a strict server answers 415 to.
fn is_json_content_type(req: &Request) -> bool {
    let values: Vec<String> = req
        .headers
        .get_all("content-type")
        .iter()
        .map(|v| v.to_str().unwrap_or("").trim().to_ascii_lowercase())
        .collect();
    values.len() == 1 && values[0].starts_with("application/json")
}

/// Like the real xAI: 415 unless the body is declared JSON.
fn reject_without_json_ct() -> ResponseTemplate {
    ResponseTemplate::new(415)
        .set_body_string("Expected request with `Content-Type: application/json`")
}

fn assert_json_body_with_model(req: &Request, model: &str) {
    assert!(
        is_json_content_type(req),
        "{} {} must carry exactly one content-type: application/json; got {:?}",
        req.method,
        req.url.path(),
        req.headers
            .get_all("content-type")
            .iter()
            .collect::<Vec<_>>()
    );
    let body: Value = serde_json::from_slice(&req.body).expect("the probe body parses as JSON");
    assert_eq!(body["model"], model, "body: {body}");
}

// ---------------------------------------------------------------- D1

#[tokio::test]
async fn d1_openai_chat_probe_declares_a_json_body() {
    let stub = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(is_json_content_type)
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"choices": [{"message": {"content": "ok"}}]})),
        )
        .mount(&stub)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(reject_without_json_ct())
        .mount(&stub)
        .await;
    let rows = run(config(&[service(
        "openai",
        "openai",
        &format!("{}/v1", stub.uri()),
        "openai",
        "gpt-5.6-sol",
    )]))
    .await;

    let reqs = stub.received_requests().await.unwrap();
    assert_eq!(reqs.len(), 1);
    assert_json_body_with_model(&reqs[0], "gpt-5.6-sol");
    assert_eq!(rows[0].state, State::Ok, "{:?}", rows[0]);
}

#[tokio::test]
async fn d1_anthropic_messages_probe_declares_a_json_body() {
    let stub = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/anthropic/v1/messages"))
        .and(is_json_content_type)
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"content": [{"type": "text", "text": "ok"}]})),
        )
        .mount(&stub)
        .await;
    Mock::given(method("POST"))
        .and(path("/anthropic/v1/messages"))
        .respond_with(reject_without_json_ct())
        .mount(&stub)
        .await;
    let rows = run(config(&[service(
        "glm",
        "zhipu",
        &format!("{}/anthropic", stub.uri()),
        "anthropic",
        "glm-5.3",
    )]))
    .await;

    let reqs = stub.received_requests().await.unwrap();
    assert_eq!(reqs.len(), 1);
    assert_json_body_with_model(&reqs[0], "glm-5.3");
    assert_eq!(rows[0].state, State::Ok, "{:?}", rows[0]);
}

#[tokio::test]
async fn d1_xai_negative_control_reads_auth_failed_through_the_real_request_path() {
    // the stub behaves like xAI: 415 without a JSON content-type, 401 (disabled key) with one
    let stub = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(is_json_content_type)
        .respond_with(
            ResponseTemplate::new(401)
                .set_body_json(json!({"error": "Incorrect API key provided"})),
        )
        .mount(&stub)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(reject_without_json_ct())
        .mount(&stub)
        .await;
    let rows = run(config(&[service(
        "xai",
        "xai",
        &format!("{}/v1", stub.uri()),
        "openai",
        "grok-4",
    )]))
    .await;
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0].state,
        State::AuthFailed,
        "the disabled key must read auth_failed: {:?}",
        rows[0]
    );
}

#[tokio::test]
async fn control_d1_the_stub_rejects_a_body_without_json_content_type() {
    // the stub above can fail: a raw POST with no content-type gets the 415, so a probe that omits it cannot pass
    let stub = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(is_json_content_type)
        .respond_with(ResponseTemplate::new(401))
        .mount(&stub)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(reject_without_json_ct())
        .mount(&stub)
        .await;
    let client = reqwest::Client::new();
    let url = format!("{}/v1/chat/completions", stub.uri());
    let bare = client
        .post(&url)
        .body(r#"{"model":"grok-4"}"#)
        .send()
        .await
        .unwrap();
    assert_eq!(bare.status().as_u16(), 415);
    let doubled = client
        .post(&url)
        .json(&json!({"model": "grok-4"}))
        .header("content-type", "application/json")
        .send()
        .await
        .unwrap();
    assert_eq!(
        doubled.status().as_u16(),
        415,
        "a doubled content-type is rejected"
    );
    let typed = client
        .post(&url)
        .json(&json!({"model": "grok-4"}))
        .send()
        .await
        .unwrap();
    assert_eq!(typed.status().as_u16(), 401);
}

#[tokio::test]
async fn d1_the_balance_get_is_read_without_a_json_content_type() {
    // the balance GET has no body; whatever its headers, the balance is read
    let stub = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/anthropic/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"content": []})))
        .mount(&stub)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/users/me/balance"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"data": {"available_balance": 500.0}})),
        )
        .mount(&stub)
        .await;
    let mut svc = service(
        "kimi",
        "moonshot",
        &format!("{}/anthropic", stub.uri()),
        "anthropic",
        "kimi-k3",
    );
    svc.push_str(&format!(
        "[service.balance]\nurl = \"{}/v1/users/me/balance\"\npath = \"data.available_balance\"\ncurrency = \"USD\"\nfloor = 100\n",
        stub.uri()
    ));
    let rows = run(config(&[svc])).await;
    assert_eq!(
        rows[0].balance.as_ref().map(|b| b.amount),
        Some(500.0),
        "{:?}",
        rows[0]
    );
    assert_eq!(rows[0].state, State::Ok);
}

// ---------------------------------------------------------------- D2

async fn kimi_with_balance(probe: ResponseTemplate, balance: f64, model: &str) -> Record {
    let stub = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/anthropic/v1/messages"))
        .respond_with(probe)
        .mount(&stub)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/users/me/balance"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"code": 0, "data": {"available_balance": balance}})),
        )
        .mount(&stub)
        .await;
    let mut svc = service(
        "kimi",
        "moonshot",
        &format!("{}/anthropic", stub.uri()),
        "anthropic",
        model,
    );
    svc.push_str(&format!(
        "[service.balance]\nurl = \"{}/v1/users/me/balance\"\npath = \"data.available_balance\"\ncurrency = \"USD\"\nfloor = 100\n",
        stub.uri()
    ));
    let rows = run(config(&[svc])).await;
    assert_eq!(rows.len(), 1);
    rows.into_iter().next().unwrap()
}

fn assert_balance(r: &Record, amount: f64) {
    let b = r
        .balance
        .as_ref()
        .unwrap_or_else(|| panic!("the balance is recorded on every row: {r:?}"));
    assert_eq!(b.amount, amount);
    assert_eq!(b.currency, "USD");
}

#[tokio::test]
async fn d2_a_missing_model_stays_model_missing_below_the_floor() {
    let not_found = ResponseTemplate::new(404).set_body_json(json!({"error": {
        "type": "resource_not_found_error", "message": "Not found the model kimi-k2.5 or Permission denied"}}));
    let r = kimi_with_balance(not_found, 17.42, "kimi-k2.5").await;
    assert_eq!(r.state, State::ModelMissing, "{r:?}");
    assert_balance(&r, 17.42);
}

#[tokio::test]
async fn d2_a_wrong_key_stays_auth_failed_below_the_floor() {
    let r = kimi_with_balance(
        ResponseTemplate::new(401).set_body_json(json!({"error": "invalid key"})),
        17.42,
        "kimi-k3",
    )
    .await;
    assert_eq!(r.state, State::AuthFailed, "{r:?}");
    assert_balance(&r, 17.42);
}

#[tokio::test]
async fn control_d2_a_working_model_below_the_floor_is_quota_exhausted() {
    let r = kimi_with_balance(
        ResponseTemplate::new(200).set_body_json(json!({"content": []})),
        17.42,
        "kimi-k3",
    )
    .await;
    assert_eq!(r.state, State::QuotaExhausted, "{r:?}");
    assert_balance(&r, 17.42);
}

#[tokio::test]
async fn control_d2_a_working_model_above_the_floor_is_ok() {
    let r = kimi_with_balance(
        ResponseTemplate::new(200).set_body_json(json!({"content": []})),
        250.0,
        "kimi-k3",
    )
    .await;
    assert_eq!(r.state, State::Ok, "{r:?}");
    assert_balance(&r, 250.0);
}

#[test]
fn d2_the_floor_never_replaces_a_measured_failure() {
    for s in [State::AuthFailed, State::ModelMissing, State::Unknown] {
        assert_eq!(apply_floor(s, 17.42, 100.0), s, "{s:?} below the floor");
        assert_eq!(
            apply_floor(s, -0.12, 100.0),
            s,
            "{s:?} at a negative balance"
        );
    }
}

#[test]
fn d2_the_floor_turns_a_working_model_into_quota_exhausted() {
    for s in [State::Ok, State::Degraded, State::RateLimited] {
        assert_eq!(
            apply_floor(s, 17.42, 100.0),
            State::QuotaExhausted,
            "{s:?} below the floor"
        );
        assert_eq!(
            apply_floor(s, 100.0, 100.0),
            State::QuotaExhausted,
            "{s:?} at the floor"
        );
        assert_eq!(apply_floor(s, 100.01, 100.0), s, "{s:?} above the floor");
    }
    assert_eq!(
        apply_floor(State::QuotaExhausted, 500.0, 100.0),
        State::QuotaExhausted
    );
}
