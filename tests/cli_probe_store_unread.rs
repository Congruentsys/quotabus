//! CHORE-007 review r1 F3: when the store cannot be read, what is due is unknown, so `quotabus probe` (without
//! `--force`) refuses: rc 1, no call to any provider, and a stderr line that points at `--force` and is not the
//! publish error. The control: `--force` against the same config does make the calls. Loopback stub, fake key.

mod common;

use std::path::{Path, PathBuf};
use std::time::Duration;

use common::{FAKE_SK, rc, run_cli};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const T: Duration = Duration::from_secs(60);
const MESSAGES: &str = "/ds/v1/messages";
const BALANCE: &str = "/bal";

async fn stub() -> MockServer {
    let s = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(MESSAGES))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"content": [{"type": "text", "text": "ok"}]})),
        )
        .mount(&s)
        .await;
    Mock::given(method("GET"))
        .and(path(BALANCE))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"total": 500})))
        .mount(&s)
        .await;
    s
}

/// A config whose `[file] dir` is a REGULAR FILE: the store exists but cannot be read as a directory of rows.
fn unreadable_store_config(dir: &Path, stub: &str) -> PathBuf {
    let not_a_dir = dir.join("state");
    std::fs::write(&not_a_dir, "this is a file, not a row directory\n").unwrap();
    let cfg = dir.join("quotabus.toml");
    std::fs::write(
        &cfg,
        format!(
            "[file]\ndir = {:?}\n\n\
             [[service]]\nid = \"deepseek\"\nkind = \"api\"\nprovider = \"deepseek\"\nfamily = \"deepseek\"\n\
             account = \"nusy-product-team\"\nbase_url = \"{stub}/ds\"\nprotocol = \"anthropic\"\nmodels = [\"m1\"]\n\
             secret = \"QB_DS\"\n\
             [service.balance]\nurl = \"{stub}{BALANCE}\"\npath = \"total\"\ncurrency = \"CNY\"\nfloor = 0\n",
            not_a_dir.display().to_string()
        ),
    )
    .unwrap();
    cfg
}

async fn counts(s: &MockServer) -> (usize, usize) {
    let reqs = s.received_requests().await.unwrap();
    let n = |p: &str| reqs.iter().filter(|r| r.url.path() == p).count();
    (n(MESSAGES), n(BALANCE))
}

async fn probe(cfg: &Path, args: &'static [&'static str]) -> std::process::Output {
    let c = cfg.to_path_buf();
    tokio::task::spawn_blocking(move || run_cli(&c, args, &[("QB_DS", FAKE_SK)], T))
        .await
        .unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn an_unreadable_store_refuses_to_probe_and_points_at_force() {
    let s = stub().await;
    let dir = tempfile::tempdir().unwrap();
    let cfg = unreadable_store_config(dir.path(), &s.uri());

    let out = probe(&cfg, &["probe"]).await;
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        rc(&out),
        1,
        "an unreadable store is a probe failure: {stderr}"
    );
    assert_eq!(
        counts(&s).await,
        (0, 0),
        "what is due is unknown, so nothing is called: {stderr}"
    );
    assert!(
        stderr.contains("--force"),
        "stderr names the --force hint: {stderr}"
    );
    assert!(
        !stderr.contains("cannot publish"),
        "a read failure is not reported as a publish failure: {stderr}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn control_force_against_the_same_unreadable_store_makes_the_calls() {
    // the refusal above is the store read, not a broken stub or config: --force probes the same config
    let s = stub().await;
    let dir = tempfile::tempdir().unwrap();
    let cfg = unreadable_store_config(dir.path(), &s.uri());

    let out = probe(&cfg, &["probe", "--force"]).await;
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        counts(&s).await,
        (1, 1),
        "--force probes every kind without reading the store: {stderr}"
    );
}
