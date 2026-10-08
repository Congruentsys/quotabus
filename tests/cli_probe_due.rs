//! CHORE-007 DoD 1-2 at the CLI: `quotabus probe` calls only what is due (defaults 12h, so a second run straight
//! after the first calls nothing); `quotabus probe --force` runs every kind; a config with the old `[probe]
//! interval` is refused naming `[intervals]`. File backend, loopback stub, fake key.

mod common;

use std::path::Path;
use std::time::Duration;

use common::{FAKE_SK, rc, run_cli, text};
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

fn write_config(dir: &Path, stub: &str, extra: &str) -> std::path::PathBuf {
    let state = dir.join("state");
    let cfg = dir.join("quotabus.toml");
    std::fs::write(
        &cfg,
        format!(
            "[file]\ndir = {:?}\n\n{extra}\n\n\
             [[service]]\nid = \"deepseek\"\nkind = \"api\"\nprovider = \"deepseek\"\nfamily = \"deepseek\"\n\
             account = \"nusy-product-team\"\nbase_url = \"{stub}/ds\"\nprotocol = \"anthropic\"\nmodels = [\"m1\"]\n\
             secret = \"QB_DS\"\n\
             [service.balance]\nurl = \"{stub}{BALANCE}\"\npath = \"total\"\ncurrency = \"CNY\"\nfloor = 0\n",
            state.display().to_string()
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
async fn a_second_probe_straight_after_the_first_calls_nothing_and_force_calls_every_kind() {
    let s = stub().await;
    let dir = tempfile::tempdir().unwrap();
    let cfg = write_config(dir.path(), &s.uri(), "");

    let out = probe(&cfg, &["probe"]).await;
    assert_eq!(rc(&out), 0, "{}", text(&out));
    // control: the counting can see calls, so the "nothing" below is a measurement, not a blind spot
    assert_eq!(
        counts(&s).await,
        (1, 1),
        "an empty store: every kind is due"
    );

    let out = probe(&cfg, &["probe"]).await;
    assert_eq!(rc(&out), 0, "{}", text(&out));
    assert_eq!(
        counts(&s).await,
        (1, 1),
        "within the 12h default intervals nothing is due: {}",
        text(&out)
    );

    let out = probe(&cfg, &["probe", "--force"]).await;
    assert_eq!(rc(&out), 0, "{}", text(&out));
    assert_eq!(counts(&s).await, (2, 2), "--force runs every kind");
}

#[tokio::test(flavor = "multi_thread")]
async fn probe_refuses_the_old_probe_interval_key_naming_intervals() {
    let s = stub().await;
    let dir = tempfile::tempdir().unwrap();
    let cfg = write_config(dir.path(), &s.uri(), "[probe]\ninterval = \"15m\"\n");
    let out = probe(&cfg, &["probe"]).await;
    assert_eq!(
        rc(&out),
        1,
        "a bad config is a probe failure: {}",
        text(&out)
    );
    assert!(
        text(&out).contains("[intervals]"),
        "the error names the new table: {}",
        text(&out)
    );
    assert_eq!(counts(&s).await, (0, 0), "a refused config calls nothing");
}
