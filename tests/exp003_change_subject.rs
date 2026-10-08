//! EXP-003 Plan "change subjects on the bus" / DESIGN §3 Outputs: `ai.status.changed.<key>` is published only when
//! `state` differs from the previous row (a flapping latency does not spam the bus). Throwaway loopback nats-server.
//!
//! Seam asserted (EXP-003 test partner): the publisher's write, `NatsKv::put` (what `quotabus probe` writes every row
//! through), publishes the change; the message's payload is the new row as JSON. The very first row for a key is not
//! asserted either way (the design does not say whether "no previous row" is a change).

mod common;

use std::time::Duration;

use chrono::Utc;
use common::NatsServer;
use futures::StreamExt;
use quotabus::{Backend, NatsKv, Record, State};

const LIMIT: Duration = Duration::from_secs(20);
const KEY: &str = "api.zhipu.acct.glm-5-3";
const OTHER: &str = "api.deepseek.acct.deepseek-v4-pro";

/// Every message that arrives within `wait`.
async fn drain(sub: &mut async_nats::Subscriber, wait: Duration) -> Vec<async_nats::Message> {
    let mut got = Vec::new();
    let deadline = tokio::time::Instant::now() + wait;
    while let Ok(Some(m)) = tokio::time::timeout_at(deadline, sub.next()).await {
        got.push(m);
    }
    got
}

fn row(key: &str, state: State, latency: u64) -> Record {
    let mut r = common::record(key, state, Utc::now(), 600);
    r.latency_ms = Some(latency);
    r
}

#[tokio::test(flavor = "multi_thread")]
async fn a_state_change_publishes_the_change_subject_and_an_unchanged_state_does_not() {
    let srv = NatsServer::start();
    let kv = tokio::time::timeout(
        LIMIT,
        NatsKv::connect_publisher(&srv.url(), "qb_exp003_changes"),
    )
    .await
    .unwrap()
    .unwrap();
    let client = async_nats::connect(srv.url()).await.unwrap();
    let mut sub = client.subscribe("ai.status.changed.>").await.unwrap();
    client.flush().await.unwrap();

    // the first rows: whatever they announce is not asserted
    kv.put(&row(KEY, State::Ok, 100)).await.unwrap();
    kv.put(&row(OTHER, State::Ok, 100)).await.unwrap();
    let _ = drain(&mut sub, Duration::from_millis(700)).await;

    // the same state again, with a different latency: no change, nothing published
    kv.put(&row(KEY, State::Ok, 4000)).await.unwrap();
    kv.put(&row(OTHER, State::Ok, 9)).await.unwrap();
    let quiet = drain(&mut sub, Duration::from_millis(1500)).await;
    assert!(
        quiet.is_empty(),
        "an unchanged state published: {:?}",
        quiet
            .iter()
            .map(|m| m.subject.to_string())
            .collect::<Vec<_>>()
    );

    // a state change: exactly one message, on ai.status.changed.<key>, carrying the new row
    kv.put(&row(KEY, State::QuotaExhausted, 100)).await.unwrap();
    let got = drain(&mut sub, Duration::from_secs(3)).await;
    let subjects: Vec<String> = got.iter().map(|m| m.subject.to_string()).collect();
    assert_eq!(
        subjects,
        [format!("ai.status.changed.{KEY}")],
        "the change of {KEY} only"
    );
    let carried: Record = serde_json::from_slice(&got[0].payload).expect("the payload is the row");
    assert_eq!(
        (carried.key.as_str(), carried.state),
        (KEY, State::QuotaExhausted)
    );

    // and back: ok after quota_exhausted is a change too
    kv.put(&row(KEY, State::Ok, 100)).await.unwrap();
    let back = drain(&mut sub, Duration::from_secs(3)).await;
    assert_eq!(back.len(), 1, "the recovery is a change");
    assert_eq!(back[0].subject.as_str(), format!("ai.status.changed.{KEY}"));
}
