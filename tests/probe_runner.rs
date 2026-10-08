//! EXP-001 Plan 3 / DESIGN §4: the probe runner against loopback stubs. No real provider, no real key.

mod common;

use std::collections::HashMap;

use chrono::{Duration, Utc};
use common::{FAKE_PLAIN, FAKE_SK, leaks};
use quotabus::{Config, Kind, ProbeSource, Record, Runner, State, record_key};
use serde_json::{Value, json};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

fn service(
    id: &str,
    kind: &str,
    provider: &str,
    base_url: &str,
    protocol: &str,
    models: &[&str],
    secret: &str,
) -> String {
    let models: Vec<String> = models.iter().map(|m| format!("{m:?}")).collect();
    format!(
        "[[service]]\nid = {id:?}\nkind = {kind:?}\nprovider = {provider:?}\nfamily = {provider:?}\n\
         account = \"nusy-product-team\"\nbase_url = {base_url:?}\nprotocol = {protocol:?}\nmodels = [{}]\n\
         secret = {secret:?}\n",
        models.join(", ")
    )
}

fn config(services: &[String]) -> Config {
    let text = format!(
        "[intervals]\napi = \"1m\"\nbalance = \"1m\"\n\n[ttl]\napi = \"2m\"\nbalance = \"2m\"\n\n[probe]\nmax_tokens = 20\n\n{}",
        services.join("\n")
    );
    Config::from_toml_str(&text).unwrap_or_else(|e| panic!("test config must parse: {e}\n{text}"))
}

fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> + Send + Sync + 'static {
    let m: HashMap<String, String> = pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    move |name: &str| m.get(name).cloned()
}

fn find<'a>(rows: &'a [Record], model: &str) -> &'a Record {
    rows.iter()
        .find(|r| r.model == model)
        .unwrap_or_else(|| panic!("no row for {model}: {rows:?}"))
}

fn header_values(req: &Request) -> Vec<String> {
    req.headers
        .iter()
        .map(|(_, v)| v.to_str().unwrap_or("").to_string())
        .collect()
}

fn messages_ok() -> ResponseTemplate {
    ResponseTemplate::new(200)
        .insert_header("anthropic-ratelimit-requests-remaining", "49")
        .insert_header("anthropic-ratelimit-tokens-remaining", "39000")
        .set_body_json(
            json!({"id": "msg_1", "type": "message", "content": [{"type": "text", "text": "ok"}]}),
        )
}

fn chat_ok() -> ResponseTemplate {
    ResponseTemplate::new(200)
        .insert_header("x-ratelimit-remaining-requests", "99")
        .insert_header("x-ratelimit-remaining-tokens", "149000")
        .set_body_json(json!({"id": "c1", "object": "chat.completion", "choices": [{"message": {"content": "ok"}}]}))
}

#[tokio::test]
async fn anthropic_probe_posts_v1_messages_with_max_tokens_20_and_the_key_in_a_header() {
    let stub = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/anthropic/v1/messages"))
        .respond_with(messages_ok())
        .mount(&stub)
        .await;
    let base = format!("{}/api/anthropic", stub.uri());
    let cfg = config(&[service(
        "glm",
        "api",
        "zhipu",
        &base,
        "anthropic",
        &["glm-5.3"],
        "QB_TEST_GLM",
    )]);
    let rows = Runner::new(cfg, env(&[("QB_TEST_GLM", FAKE_SK)]))
        .run_once()
        .await;

    assert_eq!(rows.len(), 1);
    let r = &rows[0];
    assert_eq!(r.state, State::Ok, "{r:?}");
    assert_eq!(r.key, "api.zhipu.nusy-product-team.glm-5-3");
    assert_eq!(r.contract, "ai-status/1");
    assert_eq!(r.ttl_s, 120, "ttl_s comes from [ttl] api");
    assert_eq!(r.probe.source, ProbeSource::Official);
    assert_eq!(r.observed_by, quotabus::observed_by());
    assert!(r.latency_ms.is_some());
    assert!(Utc::now() - r.checked_at < Duration::seconds(60) && r.checked_at <= Utc::now());
    let h = r
        .headroom
        .as_ref()
        .expect("headroom from the stub's anthropic-ratelimit-* headers");
    assert_eq!(h.requests_remaining, Some(49));
    assert_eq!(h.tokens_remaining, Some(39000));

    let reqs = stub.received_requests().await.unwrap();
    assert_eq!(reqs.len(), 1, "one call per (service, model)");
    let req = &reqs[0];
    assert_eq!(req.method.as_str(), "POST");
    assert_eq!(req.url.path(), "/api/anthropic/v1/messages");
    assert!(
        !req.url.as_str().contains(FAKE_SK),
        "the key is never in the URL: {}",
        req.url
    );
    assert!(req.url.query().is_none(), "no query string: {}", req.url);
    let body: Value = serde_json::from_slice(&req.body).unwrap();
    assert_eq!(body["max_tokens"], 20);
    assert_eq!(body["model"], "glm-5.3");
    assert!(
        header_values(req)
            .iter()
            .any(|v| v == FAKE_SK || *v == format!("Bearer {FAKE_SK}")),
        "the key goes in a header (x-api-key or Authorization)"
    );
}

#[tokio::test]
async fn openai_probe_posts_chat_completions() {
    let stub = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(chat_ok())
        .mount(&stub)
        .await;
    let base = format!("{}/v1", stub.uri());
    let model = "meta-llama/Llama-3.3-70B-Instruct-Turbo";
    let cfg = config(&[service(
        "together",
        "api",
        "together",
        &base,
        "openai",
        &[model],
        "QB_TEST_TOGETHER",
    )]);
    let rows = Runner::new(cfg, env(&[("QB_TEST_TOGETHER", FAKE_SK)]))
        .run_once()
        .await;

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].state, State::Ok, "{:?}", rows[0]);
    assert_eq!(rows[0].model, model);
    assert_eq!(
        rows[0].key,
        record_key(Kind::Api, "together", "nusy-product-team", model)
    );
    assert_eq!(
        rows[0].headroom.as_ref().and_then(|h| h.requests_remaining),
        Some(99)
    );

    let reqs = stub.received_requests().await.unwrap();
    assert_eq!(reqs.len(), 1);
    let req = &reqs[0];
    assert_eq!(req.method.as_str(), "POST");
    assert_eq!(req.url.path(), "/v1/chat/completions");
    assert!(!req.url.as_str().contains(FAKE_SK));
    assert_eq!(
        req.headers
            .get("authorization")
            .map(|v| v.to_str().unwrap().to_string()),
        Some(format!("Bearer {FAKE_SK}"))
    );
    let body: Value = serde_json::from_slice(&req.body).unwrap();
    assert_eq!(body["model"], model);
    let ceiling = body
        .get("max_tokens")
        .or_else(|| body.get("max_completion_tokens"));
    assert_eq!(ceiling, Some(&json!(20)), "a 20-token ceiling: {body}");
}

#[tokio::test]
async fn one_row_per_service_and_model() {
    let stub = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/a/v1/messages"))
        .respond_with(messages_ok())
        .mount(&stub)
        .await;
    Mock::given(method("POST"))
        .and(path("/b/chat/completions"))
        .respond_with(chat_ok())
        .mount(&stub)
        .await;
    let cfg = config(&[
        service(
            "glm",
            "api",
            "zhipu",
            &format!("{}/a", stub.uri()),
            "anthropic",
            &["glm-5.3", "glm-5.2"],
            "K1",
        ),
        service(
            "together",
            "api",
            "together",
            &format!("{}/b", stub.uri()),
            "openai",
            &["m1"],
            "K2",
        ),
    ]);
    let rows = Runner::new(cfg, env(&[("K1", FAKE_SK), ("K2", FAKE_SK)]))
        .run_once()
        .await;
    let mut keys: Vec<&str> = rows.iter().map(|r| r.key.as_str()).collect();
    keys.sort();
    assert_eq!(
        keys,
        [
            "api.together.nusy-product-team.m1",
            "api.zhipu.nusy-product-team.glm-5-2",
            "api.zhipu.nusy-product-team.glm-5-3"
        ]
    );
    assert!(rows.iter().all(|r| r.state == State::Ok), "{rows:?}");
}

#[tokio::test]
async fn a_wrong_key_reads_auth_failed_the_xai_negative_control() {
    let stub = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(401)
                .set_body_json(json!({"error": "Incorrect API key provided"})),
        )
        .mount(&stub)
        .await;
    let cfg = config(&[service(
        "xai",
        "api",
        "xai",
        &format!("{}/v1", stub.uri()),
        "openai",
        &["grok-4"],
        "QB_XAI",
    )]);
    let rows = Runner::new(cfg, env(&[("QB_XAI", FAKE_SK)]))
        .run_once()
        .await;
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0].state,
        State::AuthFailed,
        "a disabled key must never read ok: {:?}",
        rows[0]
    );
}

#[tokio::test]
async fn a_retired_model_reads_model_missing() {
    let stub = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/anthropic/v1/messages"))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({"error": {
            "message": "Not found the model kimi-k2.5 or Permission denied", "type": "resource_not_found_error"}})))
        .mount(&stub)
        .await;
    let cfg = config(&[service(
        "kimi",
        "api",
        "moonshot",
        &format!("{}/anthropic", stub.uri()),
        "anthropic",
        &["kimi-k2.5"],
        "QB_KIMI",
    )]);
    let rows = Runner::new(cfg, env(&[("QB_KIMI", FAKE_SK)]))
        .run_once()
        .await;
    assert_eq!(rows[0].state, State::ModelMissing, "{:?}", rows[0]);
}

#[tokio::test]
async fn an_unset_or_empty_secret_is_not_probed_and_reads_cannot_assess() {
    let stub = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(messages_ok())
        .mount(&stub)
        .await;
    let cfg = config(&[
        service(
            "set",
            "api",
            "pset",
            &format!("{}/set", stub.uri()),
            "anthropic",
            &["m"],
            "QB_SET",
        ),
        service(
            "unset",
            "api",
            "punset",
            &format!("{}/unset", stub.uri()),
            "anthropic",
            &["m"],
            "QB_UNSET",
        ),
        service(
            "empty",
            "api",
            "pempty",
            &format!("{}/empty", stub.uri()),
            "anthropic",
            &["m"],
            "QB_EMPTY",
        ),
    ]);
    let rows = Runner::new(cfg, env(&[("QB_SET", FAKE_SK), ("QB_EMPTY", "")]))
        .run_once()
        .await;
    assert_eq!(
        rows.len(),
        3,
        "a row for every configured (service, model), probed or not: {rows:?}"
    );
    let by = |p: &str| rows.iter().find(|r| r.provider == p).unwrap();
    assert_eq!(
        by("pset").state,
        State::Ok,
        "control: the service whose secret is set IS probed"
    );
    for p in ["punset", "pempty"] {
        assert_eq!(by(p).state, State::Unknown, "{:?}", by(p));
        assert_eq!(by(p).reason.as_deref(), Some("cannot_assess:secret_unset"));
    }
    let paths: Vec<String> = stub
        .received_requests()
        .await
        .unwrap()
        .iter()
        .map(|r| r.url.path().to_string())
        .collect();
    assert_eq!(
        paths,
        ["/set/v1/messages"],
        "an unset/empty secret means the service is NOT called"
    );
}

#[tokio::test]
async fn connection_refused_reads_cannot_assess_unreachable() {
    let port = common::closed_port();
    let cfg = config(&[service(
        "local-qwen",
        "local",
        "qwen",
        &format!("http://127.0.0.1:{port}/v1"),
        "openai",
        &["qwen3"],
        "QB_QWEN",
    )]);
    let rows = Runner::new(cfg, env(&[("QB_QWEN", FAKE_SK)]))
        .run_once()
        .await;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].kind, Kind::Local);
    assert_eq!(rows[0].state, State::Unknown);
    assert_eq!(rows[0].reason.as_deref(), Some("cannot_assess:unreachable"));
    assert_eq!(rows[0].observed_by, quotabus::observed_by());
}

fn balance_cfg(
    stub: &MockServer,
    balance_path: &str,
    json_path: &str,
    currency: &str,
    floor: f64,
) -> Config {
    let mut svc = service(
        "deepseek",
        "api",
        "deepseek",
        &format!("{}/anthropic", stub.uri()),
        "anthropic",
        &["deepseek-v4-pro"],
        "QB_DS",
    );
    svc.push_str(&format!(
        "[service.balance]\nurl = \"{}{balance_path}\"\npath = {json_path:?}\ncurrency = {currency:?}\nfloor = {floor}\n",
        stub.uri()
    ));
    config(&[svc])
}

async fn balance_stub(balance_path: &str, body: Value) -> MockServer {
    let stub = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/anthropic/v1/messages"))
        .respond_with(messages_ok())
        .mount(&stub)
        .await;
    Mock::given(method("GET"))
        .and(path(balance_path))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&stub)
        .await;
    stub
}

#[tokio::test]
async fn deepseek_negative_balance_reads_quota_exhausted_with_the_balance_on_the_row() {
    let body = json!({"is_available": false, "balance_infos": [{"currency": "CNY", "total_balance": "-0.12",
                      "granted_balance": "0.00", "topped_up_balance": "-0.12"}]});
    let stub = balance_stub("/user/balance", body).await;
    let cfg = balance_cfg(
        &stub,
        "/user/balance",
        "balance_infos[0].total_balance",
        "CNY",
        150.0,
    );
    let rows = Runner::new(cfg, env(&[("QB_DS", FAKE_SK)]))
        .run_once()
        .await;
    assert_eq!(rows.len(), 1);
    let r = find(&rows, "deepseek-v4-pro");
    assert_eq!(r.state, State::QuotaExhausted, "{r:?}");
    let b = r.balance.as_ref().expect("the balance is on the row");
    assert_eq!(b.amount, -0.12);
    assert_eq!(b.currency, "CNY");
    assert!(
        !b.source.is_empty() && !b.source.contains('?'),
        "source names the endpoint, no query: {}",
        b.source
    );

    let reqs = stub.received_requests().await.unwrap();
    let get = reqs
        .iter()
        .find(|q| q.method.as_str() == "GET")
        .expect("the balance was fetched with a GET");
    assert_eq!(get.url.path(), "/user/balance");
    assert!(!get.url.as_str().contains(FAKE_SK));
    assert!(
        header_values(get).iter().any(|v| v.contains(FAKE_SK)),
        "the balance GET carries the key in a header"
    );
}

#[tokio::test]
async fn kimi_balance_at_or_below_floor_reads_quota_exhausted() {
    let body = json!({"code": 0, "data": {"available_balance": 49.58, "voucher_balance": 0, "cash_balance": 49.58}, "status": true});
    let stub = balance_stub("/v1/users/me/balance", body).await;
    let cfg = balance_cfg(
        &stub,
        "/v1/users/me/balance",
        "data.available_balance",
        "USD",
        100.0,
    );
    let rows = Runner::new(cfg, env(&[("QB_DS", FAKE_SK)]))
        .run_once()
        .await;
    assert_eq!(rows[0].state, State::QuotaExhausted, "{:?}", rows[0]);
    assert_eq!(rows[0].balance.as_ref().map(|b| b.amount), Some(49.58));
    assert_eq!(
        rows[0].balance.as_ref().map(|b| b.currency.as_str()),
        Some("USD")
    );
}

#[tokio::test]
async fn control_a_balance_above_floor_keeps_the_probe_state() {
    let body = json!({"is_available": true, "balance_infos": [{"currency": "CNY", "total_balance": "500.00"}]});
    let stub = balance_stub("/user/balance", body).await;
    let cfg = balance_cfg(
        &stub,
        "/user/balance",
        "balance_infos[0].total_balance",
        "CNY",
        150.0,
    );
    let rows = Runner::new(cfg, env(&[("QB_DS", FAKE_SK)]))
        .run_once()
        .await;
    assert_eq!(rows[0].state, State::Ok, "{:?}", rows[0]);
    assert_eq!(rows[0].balance.as_ref().map(|b| b.amount), Some(500.0));
}

async fn echo_stub(key: &str) -> MockServer {
    let stub = MockServer::start().await;
    let body = format!(
        "{{\"error\":\"invalid key {key}; you sent Authorization: Bearer {key}; other keys seen: \
         sk-live-abcdefghijklmnop1234 ghp_abcdefghijklmnopqrstuvwxyz0123456789; {}\"}}",
        "padding ".repeat(120)
    );
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(401).set_body_string(body))
        .mount(&stub)
        .await;
    stub
}

#[tokio::test]
async fn an_echoed_key_never_reaches_the_row_and_the_error_is_capped() {
    for key in [FAKE_PLAIN, FAKE_SK] {
        let stub = echo_stub(key).await;
        let cfg = config(&[service(
            "glm",
            "api",
            "zhipu",
            &format!("{}/x", stub.uri()),
            "anthropic",
            &["glm-5.3"],
            "QB_ECHO",
        )]);
        let rows = Runner::new(cfg, env(&[("QB_ECHO", key)])).run_once().await;
        assert_eq!(rows[0].state, State::AuthFailed);
        let json = serde_json::to_string(&rows).unwrap();
        let dbg = format!("{rows:?}");
        assert_eq!(
            leaks(&json, &[key]),
            Vec::<String>::new(),
            "row JSON leaks: {json}"
        );
        assert_eq!(leaks(&dbg, &[key]), Vec::<String>::new(), "row Debug leaks");
        let err = rows[0].error.as_deref().expect("a redacted error is kept");
        assert!(
            err.chars().count() <= 200,
            "error is {} chars",
            err.chars().count()
        );
    }
}
