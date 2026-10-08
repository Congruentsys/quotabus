//! EXP-002 Plan 4: subscription reads run on their OWN due-ness (CHORE-007's scheduler): due when `[intervals]
//! subscription` (default 1 h) has elapsed since that account's last row; the API interval never decides it;
//! `--force` reads now. At the CLI (`quotabus probe`, file backend) a second run straight after the first reads
//! nothing, `--force` reads again, rows are `observed_by` this (probe) host, and `status --check <service>` finds the
//! subscription row.

mod common;
mod exp002;

use std::time::Duration;

use chrono::Duration as D;
use exp002::*;
use quotabus::{Kind, Probe, ProbeSource, Record, State};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer};

async fn stub() -> MockServer {
    let s = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(measured())
        .mount(&s)
        .await;
    s
}

/// The account's last row, `age_min` minutes before `now()`.
fn last_row(age_min: i64) -> Record {
    Record {
        contract: "ai-status/1".into(),
        key: CLAUDE_KEY.into(),
        kind: Kind::Subscription,
        provider: "anthropic".into(),
        account: "hankh95".into(),
        model: "claude-hankh95".into(),
        family: "anthropic".into(),
        state: State::Ok,
        reason: None,
        balance: None,
        headroom: None,
        latency_ms: Some(100),
        probe: Probe {
            name: "unified_headers".into(),
            source: ProbeSource::Undocumented,
        },
        error: None,
        checked_at: now() - D::minutes(age_min),
        ttl_s: 10_800,
        observed_by: quotabus::observed_by(),
    }
}

async fn calls_after(tables: &str, age_min: i64, force: bool) -> usize {
    let s = stub().await;
    let cfg = config(&format!("{tables}\n\n{}", claude_both(&s.uri())));
    runner(cfg, None)
        .run_due(&[last_row(age_min)], now(), force)
        .await;
    requests(&s, "/v1/messages").await.len()
}

#[tokio::test]
async fn the_1h_default_decides_due_ness() {
    assert_eq!(calls_after("", 59, false).await, 0, "59 min < 1 h: not due");
    assert_eq!(calls_after("", 61, false).await, 1, "61 min > 1 h: due");
}

#[tokio::test]
async fn an_empty_store_is_due() {
    let s = stub().await;
    runner(config(&claude_both(&s.uri())), None)
        .run_due(&[], now(), false)
        .await;
    assert_eq!(requests(&s, "/v1/messages").await.len(), 1);
}

#[tokio::test]
async fn the_subscription_interval_not_the_api_interval_decides() {
    assert_eq!(
        calls_after(
            "[intervals]\napi = \"12h\"\nsubscription = \"1m\"\n",
            2,
            false
        )
        .await,
        1,
        "2 min > subscription 1 min"
    );
    // mutation control: swapped, a reader of the api interval would call; this must not
    assert_eq!(
        calls_after(
            "[intervals]\napi = \"1m\"\nsubscription = \"12h\"\n",
            2,
            false
        )
        .await,
        0,
        "2 min < subscription 12 h, whatever api says"
    );
}

#[tokio::test]
async fn force_reads_a_fresh_account() {
    assert_eq!(calls_after("", 1, true).await, 1);
}

// ── the CLI ──────────────────────────────────────────────────────────────────────────────────────────────────────

const T: Duration = Duration::from_secs(60);

fn write_cfg(dir: &std::path::Path, base: &str) -> std::path::PathBuf {
    let p = dir.join("quotabus.toml");
    std::fs::write(
        &p,
        format!(
            "[file]\ndir = {:?}\n\n{}",
            dir.join("store").display().to_string(),
            claude_both(base)
        ),
    )
    .unwrap();
    p
}

async fn cli(cfg: &std::path::Path, args: &'static [&'static str]) -> std::process::Output {
    let c = cfg.to_path_buf();
    tokio::task::spawn_blocking(move || {
        common::run_cli(
            &c,
            args,
            &[(CLAUDE_SECRET, FAKE_OAT), ("PATH", "/usr/bin:/bin")],
            T,
        )
    })
    .await
    .unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn probe_reads_once_then_nothing_then_force_reads_again() {
    let s = stub().await;
    let dir = tempfile::tempdir().unwrap();
    let cfg = write_cfg(dir.path(), &s.uri());
    let out = cli(&cfg, &["probe"]).await;
    assert_eq!(common::rc(&out), 0, "{}", common::text(&out));
    assert_eq!(
        requests(&s, "/v1/messages").await.len(),
        1,
        "empty store: due"
    );
    let out = cli(&cfg, &["probe"]).await;
    assert_eq!(common::rc(&out), 0, "{}", common::text(&out));
    assert_eq!(
        requests(&s, "/v1/messages").await.len(),
        1,
        "within 1 h nothing is due"
    );
    let out = cli(&cfg, &["probe", "--force"]).await;
    assert_eq!(common::rc(&out), 0, "{}", common::text(&out));
    assert_eq!(
        requests(&s, "/v1/messages").await.len(),
        2,
        "--force reads now"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn the_cli_row_is_observed_by_the_probe_host() {
    let s = stub().await;
    let dir = tempfile::tempdir().unwrap();
    let cfg = write_cfg(dir.path(), &s.uri());
    let out = cli(&cfg, &["probe"]).await;
    assert_eq!(common::rc(&out), 0, "{}", common::text(&out));
    let p = dir.path().join("store").join(format!("{CLAUDE_KEY}.json"));
    let row: serde_json::Value = serde_json::from_slice(
        &std::fs::read(&p)
            .unwrap_or_else(|e| panic!("{}: {e}\n{}", p.display(), common::text(&out))),
    )
    .unwrap();
    assert_eq!(
        row["observed_by"],
        format!(
            "{}/quotabus@{}",
            common::short_hostname(),
            quotabus::VERSION
        ),
        "{row}"
    );
    assert_eq!(row["account"], "hankh95", "the account label, not the host");
}

#[tokio::test(flavor = "multi_thread")]
async fn status_check_finds_the_subscription_row() {
    let s = stub().await;
    let dir = tempfile::tempdir().unwrap();
    let cfg = write_cfg(dir.path(), &s.uri());
    assert_eq!(common::rc(&cli(&cfg, &["probe"]).await), 0);
    let out = cli(&cfg, &["status", "--json", "--kind", "subscription"]).await;
    assert_eq!(common::rc(&out), 0, "{}", common::text(&out));
    let arr: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let e = arr
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["key"] == CLAUDE_KEY)
        .unwrap_or_else(|| panic!("no {CLAUDE_KEY} entry: {arr}"));
    assert_eq!(e["verdict"], "ok", "{e}");
    assert!(e["age_s"].as_i64().is_some(), "{e}");
    let out = cli(&cfg, &["status", "--check", "claude-hankh95"]).await;
    assert_eq!(
        common::rc(&out),
        0,
        "--check of an ok account exits 0: {}",
        common::text(&out)
    );
}
