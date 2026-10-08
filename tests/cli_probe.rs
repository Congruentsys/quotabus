//! EXP-001 Plan 3-4 / DESIGN §2 `observed_by`, §6: `quotabus probe` runs one cycle and writes one row per
//! (service, model) to the backend; nothing it writes or prints carries the key.

mod common;

use std::path::Path;
use std::time::Duration;

use common::{FAKE_PLAIN, FAKE_SK, NatsServer, leaks, rc, run_cli, short_hostname, text};
use quotabus::Record;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const T: Duration = Duration::from_secs(60);

fn services(stub: &str) -> String {
    format!(
        "[ttl]\napi = \"2m\"\nbalance = \"2m\"\n\n\
         [[service]]\nid = \"glm\"\nkind = \"api\"\nprovider = \"zhipu\"\nfamily = \"zhipu\"\naccount = \"nusy-product-team\"\n\
         base_url = \"{stub}/glm\"\nprotocol = \"anthropic\"\nmodels = [\"glm-5.3\", \"glm-5.2\"]\nsecret = \"QB_GLM\"\n\n\
         [[service]]\nid = \"deepseek\"\nkind = \"api\"\nprovider = \"deepseek\"\nfamily = \"deepseek\"\n\
         account = \"nusy-product-team\"\nbase_url = \"{stub}/ds\"\nprotocol = \"anthropic\"\nmodels = [\"deepseek-v4-pro\"]\n\
         secret = \"QB_DS\"\n"
    )
}

async fn ok_stub() -> MockServer {
    let stub = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"content": [{"text": "ok"}]})),
        )
        .mount(&stub)
        .await;
    stub
}

const KEYS: [&str; 3] = [
    "api.deepseek.nusy-product-team.deepseek-v4-pro",
    "api.zhipu.nusy-product-team.glm-5-2",
    "api.zhipu.nusy-product-team.glm-5-3",
];

fn want_observed_by() -> String {
    format!(
        "{}/quotabus@{}",
        short_hostname(),
        env!("CARGO_PKG_VERSION")
    )
}

#[test]
fn host_label_is_the_short_hostname() {
    assert_eq!(quotabus::host_label(), short_hostname());
    assert_eq!(quotabus::observed_by(), want_observed_by());
}

#[tokio::test(flavor = "multi_thread")]
async fn probe_writes_one_row_per_service_model_to_the_file_backend() {
    let stub = ok_stub().await;
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    let cfg = dir.path().join("quotabus.toml");
    std::fs::write(
        &cfg,
        format!(
            "[file]\ndir = {:?}\n\n{}",
            state.display().to_string(),
            services(&stub.uri())
        ),
    )
    .unwrap();
    let c = cfg.clone();
    let out = tokio::task::spawn_blocking(move || {
        run_cli(
            &c,
            &["probe"],
            &[("QB_GLM", FAKE_SK), ("QB_DS", FAKE_SK)],
            T,
        )
    })
    .await
    .unwrap();
    assert_eq!(rc(&out), 0, "{}", text(&out));

    let mut names: Vec<String> = std::fs::read_dir(&state)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    let want: Vec<String> = KEYS.iter().map(|k| format!("{k}.json")).collect();
    assert_eq!(names, want);
    for k in KEYS {
        let r: Record =
            serde_json::from_slice(&std::fs::read(state.join(format!("{k}.json"))).unwrap())
                .unwrap();
        assert_eq!(r.key, k);
        assert_eq!(r.observed_by, want_observed_by());
        assert_eq!(r.ttl_s, 120);
        assert_eq!(r.state, quotabus::State::Ok);
    }
    assert_eq!(stub.received_requests().await.unwrap().len(), 3);
}

#[tokio::test(flavor = "multi_thread")]
async fn probe_writes_to_the_configured_nats_bucket_with_per_key_ttl() {
    let srv = NatsServer::start();
    let stub = ok_stub().await;
    let dir = tempfile::tempdir().unwrap();
    let cfg = dir.path().join("quotabus.toml");
    std::fs::write(
        &cfg,
        format!(
            "[bus]\nurl = {:?}\nbucket = \"qb_probe_bucket\"\n\n{}",
            srv.url(),
            services(&stub.uri())
        ),
    )
    .unwrap();
    let c = cfg.clone();
    let out = tokio::task::spawn_blocking(move || {
        run_cli(
            &c,
            &["probe"],
            &[("QB_GLM", FAKE_SK), ("QB_DS", FAKE_SK)],
            T,
        )
    })
    .await
    .unwrap();
    assert_eq!(rc(&out), 0, "{}", text(&out));

    let js = async_nats::jetstream::new(async_nats::connect(srv.url()).await.unwrap());
    let mut stream = js
        .get_stream("KV_qb_probe_bucket")
        .await
        .expect("probe created the configured bucket");
    assert!(
        stream.info().await.unwrap().config.allow_message_ttl,
        "created with per-key TTL"
    );
    let kv = js.get_key_value("qb_probe_bucket").await.unwrap();
    for k in KEYS {
        let bytes = kv
            .get(k)
            .await
            .unwrap()
            .unwrap_or_else(|| panic!("no row {k} on the bus"));
        let r: Record = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(r.observed_by, want_observed_by());
    }
    assert!(
        js.get_stream("KV_ai_status").await.is_err(),
        "the default bucket was not touched"
    );
}

fn all_files(dir: &Path) -> String {
    let mut s = String::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            s.push_str(&String::from_utf8_lossy(
                &std::fs::read(e.path()).unwrap_or_default(),
            ));
        }
    }
    s
}

#[tokio::test(flavor = "multi_thread")]
async fn an_echoed_key_reaches_no_row_no_stdout_and_no_log() {
    for key in [FAKE_PLAIN, FAKE_SK] {
        let stub = MockServer::start().await;
        let body = format!(
            "{{\"error\":\"bad key {key}; Authorization: Bearer {key}; also sk-live-abcdefghijklmnop1234 \
             ghp_abcdefghijklmnopqrstuvwxyz0123456789 {}\"}}",
            "pad ".repeat(200)
        );
        Mock::given(method("POST"))
            .and(path("/glm/v1/messages"))
            .respond_with(ResponseTemplate::new(401).set_body_string(body.clone()))
            .mount(&stub)
            .await;
        Mock::given(method("POST"))
            .and(path("/ds/v1/messages"))
            .respond_with(ResponseTemplate::new(500).set_body_string(body))
            .mount(&stub)
            .await;
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        let cfg = dir.path().join("quotabus.toml");
        std::fs::write(
            &cfg,
            format!(
                "[file]\ndir = {:?}\n\n{}",
                state.display().to_string(),
                services(&stub.uri())
            ),
        )
        .unwrap();
        let c = cfg.clone();
        let k = key.to_string();
        let out = tokio::task::spawn_blocking(move || {
            run_cli(
                &c,
                &["probe"],
                &[("QB_GLM", &k), ("QB_DS", &k), ("RUST_LOG", "trace")],
                T,
            )
        })
        .await
        .unwrap();
        let printed = text(&out);
        assert_eq!(
            leaks(&printed, &[key]),
            Vec::<String>::new(),
            "stdout/stderr leak: {printed}"
        );
        let written = all_files(&state);
        assert!(
            written.contains("auth_failed"),
            "control: the rows were written ({written})"
        );
        assert_eq!(
            leaks(&written, &[key]),
            Vec::<String>::new(),
            "a row leaks: {written}"
        );
        for f in std::fs::read_dir(&state).unwrap().flatten() {
            let r: Record = serde_json::from_slice(&std::fs::read(f.path()).unwrap()).unwrap();
            if let Some(e) = &r.error {
                assert!(
                    e.chars().count() <= 200,
                    "error {} chars",
                    e.chars().count()
                );
            }
        }
    }
}
