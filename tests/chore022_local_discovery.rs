//! CHORE-022 DoD 1 and 3: a `kind = "local"` service with no `models` lists `GET <base>/models` (base_url ends in
//! `/v1`, so the request is `/v1/models`) and probes what is served, one row per served model keyed by its id. A
//! configured `models` keeps today's behaviour. An unreachable box is ONE `cannot_assess:unreachable` row per service.
//! No request to a keyless local service carries an Authorization header. Every check carries a control that shows it
//! can fail. Loopback stubs only; no real key (the one fake key is `common::FAKE_SK`).

mod common;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use common::FAKE_SK;
use quotabus::{Config, Kind, Record, Runner, State, record_key};
use serde_json::{Value, json};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

/// What the Spark measured on 2026-10-09 served (the item body: `["nvidia/Qwen3-32B-NVFP4"]`).
const QWEN: &str = "nvidia/Qwen3-32B-NVFP4";
/// The model after a `spark-model switch`.
const SWITCHED: &str = "openai/gpt-oss-120b";

/// A local service; `models = None` omits the line (discovery), `secret = None` omits it (no key).
fn local(
    id: &str,
    account: &str,
    base_url: &str,
    models: Option<&[&str]>,
    secret: Option<&str>,
) -> String {
    let mut s = format!(
        "[[service]]\nid = {id:?}\nkind = \"local\"\nprovider = \"qwen\"\nfamily = \"qwen\"\naccount = {account:?}\n\
         base_url = {base_url:?}\nprotocol = \"openai\"\nroles = [\"work\"]\ncost_class = \"local\"\n"
    );
    if let Some(ms) = models {
        let ms: Vec<String> = ms.iter().map(|m| format!("{m:?}")).collect();
        s.push_str(&format!("models = [{}]\n", ms.join(", ")));
    }
    if let Some(name) = secret {
        s.push_str(&format!("secret = {name:?}\n"));
    }
    s
}

fn config(services: &[String]) -> Config {
    let text = format!(
        "[intervals]\napi = \"1m\"\nbalance = \"1m\"\n\n[probe]\nmax_tokens = 20\n\n{}",
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

fn no_env() -> impl Fn(&str) -> Option<String> + Send + Sync + 'static {
    env(&[])
}

/// The models the stub Spark serves right now; a test changes it between cycles (a `spark-model switch`).
type Served = Arc<Mutex<Vec<String>>>;

/// `GET /v1/models`: the OpenAI-shaped list of what is served.
struct ModelsList(Served);
impl Respond for ModelsList {
    fn respond(&self, _: &Request) -> ResponseTemplate {
        let data: Vec<Value> = self
            .0
            .lock()
            .unwrap()
            .iter()
            .map(|id| json!({"id": id, "object": "model", "owned_by": "vllm"}))
            .collect();
        ResponseTemplate::new(200).set_body_json(json!({"object": "list", "data": data}))
    }
}

/// `POST /v1/chat/completions`: 200 for a served model, else vLLM's 404 naming the model.
struct Chat(Served);
impl Respond for Chat {
    fn respond(&self, req: &Request) -> ResponseTemplate {
        let body: Value = serde_json::from_slice(&req.body).unwrap_or(Value::Null);
        let model = body["model"].as_str().unwrap_or("").to_string();
        if self.0.lock().unwrap().contains(&model) {
            ResponseTemplate::new(200).set_body_json(
                json!({"id": "c1", "object": "chat.completion", "model": model,
                       "choices": [{"message": {"role": "assistant", "content": "OK"}}]}),
            )
        } else {
            ResponseTemplate::new(404).set_body_json(json!({
                "object": "error",
                "message": format!("The model `{model}` does not exist."),
                "type": "NotFoundError",
                "code": 404
            }))
        }
    }
}

/// A stub Spark at `<uri>/v1` serving `served`.
async fn spark(served: &[&str]) -> (MockServer, Served) {
    let stub = MockServer::start().await;
    let s: Served = Arc::new(Mutex::new(served.iter().map(|m| m.to_string()).collect()));
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(ModelsList(s.clone()))
        .mount(&stub)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(Chat(s.clone()))
        .mount(&stub)
        .await;
    (stub, s)
}

fn base(stub: &MockServer) -> String {
    format!("{}/v1", stub.uri())
}

fn models_of(rows: &[Record]) -> Vec<String> {
    let mut m: Vec<String> = rows.iter().map(|r| r.model.clone()).collect();
    m.sort();
    m
}

/// The checked property: rows in `model_missing`.
fn model_missing(rows: &[Record]) -> Vec<&Record> {
    rows.iter()
        .filter(|r| r.state == State::ModelMissing)
        .collect()
}

/// The checked property: requests that carried an Authorization header (any case; HTTP header names are
/// case-insensitive and wiremock's map is too).
fn with_authorization(reqs: &[Request]) -> Vec<String> {
    reqs.iter()
        .filter(|r| r.headers.get("authorization").is_some())
        .map(|r| format!("{} {}", r.method, r.url.path()))
        .collect()
}

fn chat_models(reqs: &[Request]) -> Vec<String> {
    reqs.iter()
        .filter(|r| r.method.as_str() == "POST" && r.url.path() == "/v1/chat/completions")
        .map(|r| {
            let b: Value = serde_json::from_slice(&r.body).unwrap();
            b["model"].as_str().unwrap_or("").to_string()
        })
        .collect()
}

// ── DoD 1 / 3: the served model is discovered and probed ────────────────────────────────────────────────────────

#[tokio::test]
async fn a_local_service_without_models_probes_the_model_v1_models_serves() {
    let (stub, _) = spark(&[QWEN]).await;
    let cfg = config(&[local("dgx1", "dgx1", &base(&stub), None, None)]);
    assert!(
        cfg.services[0].models.is_empty(),
        "the config under test omits models"
    );

    let rows = Runner::new(cfg, no_env()).run_once().await;

    assert_eq!(rows.len(), 1, "one row per served model: {rows:?}");
    let r = &rows[0];
    assert_eq!(r.model, QWEN, "the row is keyed by the served id");
    assert_eq!(r.key, record_key(Kind::Local, "qwen", "dgx1", QWEN));
    assert_eq!(r.kind, Kind::Local);
    assert_eq!(r.state, State::Ok, "the served model answers 200: {r:?}");

    let reqs = stub.received_requests().await.unwrap();
    assert!(
        reqs.iter()
            .any(|q| q.method.as_str() == "GET" && q.url.path() == "/v1/models"),
        "the probe lists GET <base>/models: {:?}",
        reqs.iter()
            .map(|q| format!("{} {}", q.method, q.url.path()))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        chat_models(&reqs),
        [QWEN],
        "then one completion, on the served model"
    );
    let chat = reqs.iter().find(|q| q.method.as_str() == "POST").unwrap();
    let body: Value = serde_json::from_slice(&chat.body).unwrap();
    assert_eq!(
        body.get("max_tokens")
            .or_else(|| body.get("max_completion_tokens")),
        Some(&json!(20)),
        "a 20-token completion (DESIGN §4 local row): {body}"
    );
}

#[tokio::test]
async fn every_served_model_gets_its_own_row() {
    let (stub, _) = spark(&[QWEN, SWITCHED]).await;
    let cfg = config(&[local("dgx1", "dgx1", &base(&stub), None, None)]);
    let rows = Runner::new(cfg, no_env()).run_once().await;

    let mut want = vec![QWEN.to_string(), SWITCHED.to_string()];
    want.sort();
    assert_eq!(models_of(&rows), want, "one row per served model: {rows:?}");
    for r in &rows {
        assert_eq!(r.key, record_key(Kind::Local, "qwen", "dgx1", &r.model));
        assert_eq!(r.state, State::Ok, "{r:?}");
    }
    let mut probed = chat_models(&stub.received_requests().await.unwrap());
    probed.sort();
    assert_eq!(probed, want, "each served model is probed once");
}

// ── DoD 1 / 3: a switch is a row for the new id, not model_missing ──────────────────────────────────────────────

#[tokio::test]
async fn a_switched_served_model_reads_the_new_id_and_no_model_missing() {
    let (stub, served) = spark(&[QWEN]).await;
    let cfg = config(&[local("dgx1", "dgx1", &base(&stub), None, None)]);
    let runner = Runner::new(cfg, no_env());

    let before = runner.run_once().await;
    assert_eq!(
        models_of(&before),
        [QWEN],
        "cycle 1 reads the first model: {before:?}"
    );

    // `spark-model switch`: the box now serves a different model
    *served.lock().unwrap() = vec![SWITCHED.to_string()];
    let after = runner.run_once().await;

    assert_eq!(
        models_of(&after),
        [SWITCHED],
        "cycle 2 has a row for the new id, and only that: {after:?}"
    );
    assert_eq!(after[0].state, State::Ok, "{:?}", after[0]);
    assert_eq!(
        after[0].key,
        record_key(Kind::Local, "qwen", "dgx1", SWITCHED)
    );
    assert!(
        model_missing(&after).is_empty(),
        "a switch is not model_missing: {after:?}"
    );
}

#[tokio::test]
async fn control_a_fixed_models_list_after_a_switch_does_read_model_missing() {
    // the model_missing check can fail: today's fixed list, after the same switch, reads model_missing
    let (stub, _) = spark(&[SWITCHED]).await;
    let cfg = config(&[local("dgx1", "dgx1", &base(&stub), Some(&[QWEN]), None)]);
    let rows = Runner::new(cfg, no_env()).run_once().await;
    assert_eq!(models_of(&rows), [QWEN], "{rows:?}");
    assert_eq!(
        model_missing(&rows).len(),
        1,
        "the predicate sees model_missing: {rows:?}"
    );
}

// ── DoD 1: a configured models list keeps today's behaviour ─────────────────────────────────────────────────────

#[tokio::test]
async fn a_configured_models_list_is_probed_as_configured_not_discovered() {
    // the box serves two models; the config names one: exactly that one is probed and rowed
    let (stub, _) = spark(&[QWEN, SWITCHED]).await;
    let cfg = config(&[local("dgx1", "dgx1", &base(&stub), Some(&[QWEN]), None)]);
    let rows = Runner::new(cfg, no_env()).run_once().await;
    assert_eq!(
        models_of(&rows),
        [QWEN],
        "a configured list is not widened by discovery: {rows:?}"
    );
    assert_eq!(rows[0].state, State::Ok, "{:?}", rows[0]);
    assert_eq!(
        chat_models(&stub.received_requests().await.unwrap()),
        [QWEN]
    );
}

#[tokio::test]
async fn a_configured_local_secret_that_is_unset_still_reads_secret_unset() {
    // today's behaviour for a configured secret is kept: named but unset ⇒ secret_unset, no call
    let (stub, _) = spark(&[QWEN]).await;
    let cfg = config(&[local(
        "dgx1",
        "dgx1",
        &base(&stub),
        Some(&[QWEN]),
        Some("QB_TEST_LOCAL_UNSET"),
    )]);
    let rows = Runner::new(cfg, no_env()).run_once().await;
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(
        rows[0].reason.as_deref(),
        Some("cannot_assess:secret_unset")
    );
    assert!(
        stub.received_requests().await.unwrap().is_empty(),
        "not called"
    );
}

// ── DoD 3: no request carries an Authorization header ────────────────────────────────────────────────────────────

#[tokio::test]
async fn no_request_to_a_keyless_local_service_carries_authorization() {
    let (stub, _) = spark(&[QWEN]).await;
    let cfg = config(&[local("dgx1", "dgx1", &base(&stub), None, None)]);
    let rows = Runner::new(cfg, no_env()).run_once().await;
    assert_eq!(
        rows.len(),
        1,
        "the discovered model was probed (so requests were made): {rows:?}"
    );

    let reqs = stub.received_requests().await.unwrap();
    assert!(
        reqs.iter().any(|q| q.method.as_str() == "GET")
            && reqs.iter().any(|q| q.method.as_str() == "POST"),
        "both the list and the completion were made, so both are checked: {}",
        reqs.len()
    );
    assert!(
        with_authorization(&reqs).is_empty(),
        "no Authorization header: {:?}",
        with_authorization(&reqs)
    );
}

#[tokio::test]
async fn control_a_local_service_with_a_key_does_send_authorization() {
    // the Authorization check can fail: give the same service a (fake) key and the completion carries one
    let (stub, _) = spark(&[QWEN]).await;
    let cfg = config(&[local(
        "dgx1",
        "dgx1",
        &base(&stub),
        Some(&[QWEN]),
        Some("QB_TEST_LOCAL_KEY"),
    )]);
    let rows = Runner::new(cfg, env(&[("QB_TEST_LOCAL_KEY", FAKE_SK)]))
        .run_once()
        .await;
    assert_eq!(rows.len(), 1, "{rows:?}");
    let reqs = stub.received_requests().await.unwrap();
    assert!(
        !with_authorization(&reqs).is_empty(),
        "the predicate sees the header: {reqs:?}"
    );
}

// ── DoD 1 / 3: an unreachable box is one cannot_assess:unreachable row per service ──────────────────────────────

#[tokio::test]
async fn an_unreachable_box_without_models_is_one_unreachable_row() {
    let port = common::closed_port();
    let cfg = config(&[local(
        "dgx2",
        "dgx2",
        &format!("http://127.0.0.1:{port}/v1"),
        None,
        None,
    )]);
    let rows = Runner::new(cfg, no_env()).run_once().await;

    assert_eq!(
        rows.len(),
        1,
        "one row for the service, no model row invented: {rows:?}"
    );
    let r = &rows[0];
    assert_eq!(r.kind, Kind::Local);
    assert_eq!(r.account, "dgx2");
    assert_eq!(r.state, State::Unknown, "{r:?}");
    assert_eq!(r.reason.as_deref(), Some("cannot_assess:unreachable"));
    assert!(
        r.key.starts_with("local.qwen.dgx2."),
        "the row is the service's: {}",
        r.key
    );
}

#[tokio::test]
async fn two_unreachable_boxes_are_two_rows_with_distinct_keys() {
    let port = common::closed_port();
    let url = format!("http://127.0.0.1:{port}/v1");
    let cfg = config(&[
        local("dgx1", "dgx1", &url, None, None),
        local("dgx2", "dgx2", &url, None, None),
    ]);
    let rows = Runner::new(cfg, no_env()).run_once().await;

    assert_eq!(rows.len(), 2, "one row per service: {rows:?}");
    assert_ne!(rows[0].key, rows[1].key, "each service has its own row");
    for r in &rows {
        assert_eq!(
            r.reason.as_deref(),
            Some("cannot_assess:unreachable"),
            "{r:?}"
        );
    }
}

#[tokio::test]
async fn one_reachable_and_one_unreachable_box_read_independently() {
    let (stub, _) = spark(&[QWEN]).await;
    let port = common::closed_port();
    let cfg = config(&[
        local("dgx1", "dgx1", &base(&stub), None, None),
        local(
            "dgx2",
            "dgx2",
            &format!("http://127.0.0.1:{port}/v1"),
            None,
            None,
        ),
    ]);
    let rows = Runner::new(cfg, no_env()).run_once().await;
    assert_eq!(rows.len(), 2, "{rows:?}");
    let dgx1 = rows
        .iter()
        .find(|r| r.account == "dgx1")
        .expect("a dgx1 row");
    let dgx2 = rows
        .iter()
        .find(|r| r.account == "dgx2")
        .expect("a dgx2 row");
    assert_eq!((dgx1.model.as_str(), dgx1.state), (QWEN, State::Ok));
    assert_eq!(dgx2.reason.as_deref(), Some("cannot_assess:unreachable"));
}

#[tokio::test]
async fn control_an_unreachable_box_with_two_configured_models_is_two_rows() {
    // the one-row check can fail: with a configured list, today's per-model rows are two, not one
    let port = common::closed_port();
    let cfg = config(&[local(
        "dgx2",
        "dgx2",
        &format!("http://127.0.0.1:{port}/v1"),
        Some(&["a", "b"]),
        None,
    )]);
    let rows = Runner::new(cfg, no_env()).run_once().await;
    assert_eq!(rows.len(), 2, "{rows:?}");
    assert!(
        rows.iter()
            .all(|r| r.reason.as_deref() == Some("cannot_assess:unreachable"))
    );
}
