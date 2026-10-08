//! EXP-001 Plan 4 / DESIGN §3 Outputs, docs/findings/EXP-001-per-key-ttl.md: the KV publisher creates its bucket WITH
//! per-key TTL, and a row written with a short TTL is ABSENT after it expires. Throwaway loopback nats-server only.

mod common;

use std::time::Duration;

use chrono::Utc;
use common::NatsServer;
use quotabus::{Backend, BackendError, NatsKv, State};

const LIMIT: Duration = Duration::from_secs(20);

async fn stream_allows_msg_ttl(url: &str, bucket: &str) -> bool {
    let client = async_nats::connect(url).await.unwrap();
    let js = async_nats::jetstream::new(client);
    let mut stream = js
        .get_stream(format!("KV_{bucket}"))
        .await
        .expect("the bucket's stream exists");
    stream.info().await.unwrap().config.allow_message_ttl
}

async fn stream_exists(url: &str, bucket: &str) -> bool {
    let client = async_nats::connect(url).await.unwrap();
    async_nats::jetstream::new(client)
        .get_stream(format!("KV_{bucket}"))
        .await
        .is_ok()
}

#[tokio::test(flavor = "multi_thread")]
async fn publisher_creates_the_bucket_with_per_key_ttl() {
    let srv = NatsServer::start();
    let kv = tokio::time::timeout(LIMIT, NatsKv::connect_publisher(&srv.url(), "qb_test_ttl"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(kv.bucket, "qb_test_ttl");
    assert!(
        stream_allows_msg_ttl(&srv.url(), "qb_test_ttl").await,
        "bucket must be created with per-key TTL (allow_msg_ttl)"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn control_a_plain_bucket_reports_no_per_key_ttl() {
    // the check above can fail: a bucket made without limit markers (every pre-existing bucket on Mini) reads false
    let srv = NatsServer::start();
    let js = async_nats::jetstream::new(async_nats::connect(srv.url()).await.unwrap());
    js.create_key_value(async_nats::jetstream::kv::Config {
        bucket: "qb_plain".into(),
        history: 1,
        ..Default::default()
    })
    .await
    .unwrap();
    assert!(!stream_allows_msg_ttl(&srv.url(), "qb_plain").await);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_short_ttl_row_is_absent_after_expiry_and_a_long_one_survives() {
    let srv = NatsServer::start();
    let kv = tokio::time::timeout(
        LIMIT,
        NatsKv::connect_publisher(&srv.url(), "qb_test_expiry"),
    )
    .await
    .unwrap()
    .unwrap();
    let short = common::record(
        "api.zhipu.nusy-product-team.short",
        State::Ok,
        Utc::now(),
        1,
    );
    let long = common::record(
        "api.zhipu.nusy-product-team.long",
        State::Ok,
        Utc::now(),
        600,
    );
    kv.put(&short).await.unwrap();
    kv.put(&long).await.unwrap();

    // known answer: both readable at once, and the value round-trips
    assert_eq!(kv.get(&short.key).await.unwrap().as_ref(), Some(&short));
    assert_eq!(kv.get(&long.key).await.unwrap().as_ref(), Some(&long));

    tokio::time::sleep(Duration::from_secs(4)).await;
    assert_eq!(
        kv.get(&short.key).await.unwrap(),
        None,
        "a 1 s per-key TTL row must be ABSENT at +4 s"
    );
    // control: absence is the TTL's doing, not a reader that returns nothing
    assert_eq!(kv.get(&long.key).await.unwrap().as_ref(), Some(&long));
    let keys: Vec<String> = kv
        .list()
        .await
        .unwrap()
        .into_iter()
        .map(|r| r.key)
        .collect();
    assert_eq!(keys, std::slice::from_ref(&long.key));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_reader_never_creates_a_missing_bucket() {
    let srv = NatsServer::start();
    let r = tokio::time::timeout(LIMIT, NatsKv::connect_reader(&srv.url(), "qb_never_made"))
        .await
        .unwrap();
    assert!(
        matches!(r, Err(BackendError::BucketMissing(_))),
        "got {:?}",
        r.err()
    );
    assert!(
        !stream_exists(&srv.url(), "qb_never_made").await,
        "the reader created the bucket"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_unreachable_bus_is_an_error_not_an_empty_bucket() {
    let port = common::closed_port();
    let r = tokio::time::timeout(
        LIMIT,
        NatsKv::connect_reader(&format!("nats://127.0.0.1:{port}"), "ai_status"),
    )
    .await
    .expect("an unreachable bus must fail promptly, not hang");
    assert!(
        matches!(r, Err(BackendError::Unreachable(_))),
        "got {:?}",
        r.err()
    );
}
