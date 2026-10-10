//! CHORE-022 review r1 F1/F2: what a CONSUMER sees across cycles on a discovering (keyless, no `models`) local
//! service. After a box that served A goes DOWN, `select` returns nothing for it and `status` shows no `ok` slot for
//! it. After it SWITCHES from A to B, `select` can return B and never A, and `status` shows A not `ok` (or absent).
//! Each case runs end to end through the binary (`probe --force`, `status --json`, `select --json`) on a `[file]`
//! backend, and the store's rows also go through `quotabus::select` in process. The checks hold whichever way the fix
//! goes: non-ok rows written over the stale model keys, or readers that use only the newest cycle.
//!
//! Controls: right after cycle 1 the same predicates DO see A `ok` and selectable, so they can fire. A configured
//! `models` list, put through the same down sequence, reads nothing selectable (today's behaviour, which the
//! discovering case must match). The discovering cases are red against c66572d: the reviewer measured A still
//! selectable there. Loopback stubs only, no key.

mod common;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::Utc;
use common::{rc, run_cli, text};
use quotabus::{Backend, Config, FileBackend, Query, select};
use serde_json::{Value, json};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

const A: &str = "nvidia/Qwen3-32B-NVFP4";
const B: &str = "openai/gpt-oss-120b";
const SVC: &str = "dgx1-qwen";
const T: Duration = Duration::from_secs(60);

type Served = Arc<Mutex<Vec<String>>>;

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

/// 200 for a served model, else vLLM's 404 naming the model.
struct Chat(Served);
impl Respond for Chat {
    fn respond(&self, req: &Request) -> ResponseTemplate {
        let body: Value = serde_json::from_slice(&req.body).unwrap_or(Value::Null);
        let model = body["model"].as_str().unwrap_or("").to_string();
        if self.0.lock().unwrap().contains(&model) {
            ResponseTemplate::new(200).set_body_json(json!({
                "id": "c1", "object": "chat.completion", "model": model,
                "choices": [{"message": {"role": "assistant", "content": "OK"}}]
            }))
        } else {
            ResponseTemplate::new(404).set_body_json(json!({
                "object": "error", "message": format!("The model `{model}` does not exist."),
                "type": "NotFoundError", "code": 404
            }))
        }
    }
}

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

fn down_url() -> String {
    format!("http://127.0.0.1:{}/v1", common::closed_port())
}

/// One test's store and config file. The config is rewritten between cycles (box up, then down); the service id,
/// provider and account stay the same, as they do when a real box goes down.
struct Fixture {
    _tmp: tempfile::TempDir,
    store: PathBuf,
    config: PathBuf,
}

impl Fixture {
    fn new() -> Fixture {
        let tmp = tempfile::tempdir().unwrap();
        let store = tmp.path().join("store");
        std::fs::create_dir_all(&store).unwrap();
        let config = tmp.path().join("quotabus.toml");
        Fixture {
            _tmp: tmp,
            store,
            config,
        }
    }

    fn toml(&self, base_url: &str, models: Option<&[&str]>) -> String {
        let mut s = format!(
            "[file]\ndir = {:?}\n\n[intervals]\napi = \"1m\"\nbalance = \"1m\"\n\n[probe]\nmax_tokens = 20\n\n\
             [[service]]\nid = {SVC:?}\nkind = \"local\"\nprovider = \"qwen\"\nfamily = \"qwen\"\naccount = \"dgx1\"\n\
             base_url = {base_url:?}\nprotocol = \"openai\"\nroles = [\"work\"]\ncost_class = \"local\"\n",
            self.store.display()
        );
        if let Some(ms) = models {
            let ms: Vec<String> = ms.iter().map(|m| format!("{m:?}")).collect();
            s.push_str(&format!("models = [{}]\n", ms.join(", ")));
        }
        s
    }

    fn write(&self, base_url: &str, models: Option<&[&str]>) -> Config {
        let text = self.toml(base_url, models);
        std::fs::write(&self.config, &text).unwrap();
        Config::from_toml_str(&text).unwrap_or_else(|e| panic!("config must parse: {e}\n{text}"))
    }
}

/// Run the binary off the async runtime (the stub keeps serving meanwhile).
async fn cli(config: &Path, args: &[&str]) -> std::process::Output {
    let config = config.to_path_buf();
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    tokio::task::spawn_blocking(move || {
        let a: Vec<&str> = args.iter().map(String::as_str).collect();
        run_cli(&config, &a, &[], T)
    })
    .await
    .unwrap()
}

async fn probe(f: &Fixture) {
    let o = cli(&f.config, &["probe", "--force"]).await;
    assert_eq!(rc(&o), 0, "probe --force: {}", text(&o));
}

/// `status --json`, the entries for SVC: (model slot of the key, verdict).
async fn status(f: &Fixture) -> Vec<(String, String)> {
    let o = cli(&f.config, &["status", "--json"]).await;
    let v: Value = serde_json::from_slice(&o.stdout)
        .unwrap_or_else(|e| panic!("status --json is JSON ({e}): {}", text(&o)));
    v.as_array()
        .unwrap()
        .iter()
        .filter(|e| e["service"] == SVC)
        .map(|e| {
            let model = e["row"]["model"]
                .as_str()
                .map(str::to_string)
                .unwrap_or_else(|| e["key"].as_str().unwrap_or("").to_string());
            (model, e["verdict"].as_str().unwrap_or("").to_string())
        })
        .collect()
}

/// `select --role work --n 50 --json`: the models chosen for SVC, and the rc.
async fn cli_select(f: &Fixture) -> (Vec<String>, i32) {
    let o = cli(
        &f.config,
        &["select", "--role", "work", "--n", "50", "--json"],
    )
    .await;
    let v: Value = serde_json::from_slice(&o.stdout)
        .unwrap_or_else(|e| panic!("select --json is JSON ({e}): {}", text(&o)));
    let models = v
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["service"] == SVC)
        .map(|c| c["model"].as_str().unwrap_or("").to_string())
        .collect();
    (models, rc(&o))
}

/// The store as it holds rows (newest per key) through `quotabus::select`, in process: models chosen for SVC.
async fn lib_select(f: &Fixture, cfg: &Config) -> Vec<String> {
    let rows = FileBackend::new(&f.store)
        .list()
        .await
        .expect("the store reads");
    let q = Query {
        role: "work".into(),
        exclude_families: vec![],
        prefer: None,
        now: Utc::now(),
    };
    select(&rows, &q, cfg)
        .into_iter()
        .filter(|c| c.service == SVC)
        .map(|c| c.model)
        .collect()
}

/// The checked property: the status slots that read `ok`.
fn ok_slots(entries: &[(String, String)]) -> Vec<String> {
    entries
        .iter()
        .filter(|(_, v)| v == "ok")
        .map(|(m, _)| m.clone())
        .collect()
}

// ── controls: the predicates fire on cycle 1 ────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn control_after_cycle_one_a_is_ok_and_selectable() {
    let (stub, _) = spark(&[A]).await;
    let f = Fixture::new();
    let cfg = f.write(&format!("{}/v1", stub.uri()), None);
    probe(&f).await;

    assert_eq!(ok_slots(&status(&f).await), [A], "status sees A ok");
    let (chosen, code) = cli_select(&f).await;
    assert_eq!((chosen, code), (vec![A.to_string()], 0), "select returns A");
    assert_eq!(
        lib_select(&f, &cfg).await,
        [A],
        "quotabus::select returns A"
    );
}

// ── F1 down: up(A) then down ─────────────────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn after_a_discovering_box_goes_down_select_returns_nothing_for_it() {
    let (stub, _) = spark(&[A]).await;
    let f = Fixture::new();
    f.write(&format!("{}/v1", stub.uri()), None);
    probe(&f).await;
    let cfg = f.write(&down_url(), None);
    probe(&f).await;

    let (chosen, code) = cli_select(&f).await;
    assert!(
        chosen.is_empty(),
        "the CLI selects nothing from a down box, got {chosen:?}"
    );
    assert_eq!(code, 3, "nothing qualifies (rc 3)");
    let lib = lib_select(&f, &cfg).await;
    assert!(
        lib.is_empty(),
        "quotabus::select picks nothing from a down box, got {lib:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn after_a_discovering_box_goes_down_status_shows_no_ok_slot_for_it() {
    let (stub, _) = spark(&[A]).await;
    let f = Fixture::new();
    f.write(&format!("{}/v1", stub.uri()), None);
    probe(&f).await;
    f.write(&down_url(), None);
    probe(&f).await;

    let entries = status(&f).await;
    assert!(!entries.is_empty(), "the service is still reported");
    assert!(
        ok_slots(&entries).is_empty(),
        "no slot of a down box reads ok: {entries:?}"
    );
}

// ── F1 switch: up(A) then serving B ─────────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn after_a_switch_select_can_return_b_and_never_a() {
    let (stub, served) = spark(&[A]).await;
    let f = Fixture::new();
    let cfg = f.write(&format!("{}/v1", stub.uri()), None);
    probe(&f).await;
    *served.lock().unwrap() = vec![B.to_string()];
    probe(&f).await;

    let (chosen, code) = cli_select(&f).await;
    assert_eq!(
        (chosen, code),
        (vec![B.to_string()], 0),
        "the CLI selects B, and only B"
    );
    assert_eq!(
        lib_select(&f, &cfg).await,
        [B],
        "quotabus::select returns B, never A"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn after_a_switch_status_shows_a_not_ok_and_b_ok() {
    let (stub, served) = spark(&[A]).await;
    let f = Fixture::new();
    f.write(&format!("{}/v1", stub.uri()), None);
    probe(&f).await;
    *served.lock().unwrap() = vec![B.to_string()];
    probe(&f).await;

    let entries = status(&f).await;
    assert_eq!(
        ok_slots(&entries),
        [B],
        "only B reads ok; A is not ok or absent: {entries:?}"
    );
}

// ── control: a configured models list, same down sequence ──────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn control_a_configured_models_list_after_going_down_selects_nothing() {
    let (stub, _) = spark(&[A]).await;
    let f = Fixture::new();
    f.write(&format!("{}/v1", stub.uri()), Some(&[A]));
    probe(&f).await;
    assert_eq!(
        ok_slots(&status(&f).await),
        [A],
        "cycle 1: A ok, so the predicate can fire"
    );
    let cfg = f.write(&down_url(), Some(&[A]));
    probe(&f).await;

    let (chosen, code) = cli_select(&f).await;
    assert!(chosen.is_empty() && code == 3, "{chosen:?} rc {code}");
    assert!(lib_select(&f, &cfg).await.is_empty());
    let entries = status(&f).await;
    assert!(ok_slots(&entries).is_empty(), "{entries:?}");
}
