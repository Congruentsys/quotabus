//! HAZ-002: the throwaway nats-server fixture under concurrency. Many servers start at once through the shared
//! helper; each is connected to IMMEDIATELY (no retry) with async-nats and given a JetStream call. Every first
//! connection must succeed, and every server must be the test's OWN: a stream created on it is the only stream it
//! holds (two servers handed one port would share, or one would die and its client reach a stranger). Servers are
//! dropped at staggered times, so a client that reached someone else's server sees it vanish. Loopback only.

mod common;

use std::sync::{Arc, Barrier};
use std::time::Duration;

use common::NatsServer;

const THREADS: usize = 24;
const ROUNDS: usize = 6;

async fn first_contact(srv: &NatsServer, tag: &str) -> Result<(), String> {
    let client = async_nats::ConnectOptions::new()
        .connection_timeout(Duration::from_secs(5))
        .connect(srv.url())
        .await
        .map_err(|e| format!("first connect to {} failed: {e}", srv.url()))?;
    let js = async_nats::jetstream::new(client.clone());
    let name = format!("STRESS_{tag}");
    tokio::time::timeout(
        Duration::from_secs(10),
        js.create_stream(async_nats::jetstream::stream::Config {
            name: name.clone(),
            subjects: vec![format!("stress.{tag}")],
            ..Default::default()
        }),
    )
    .await
    .map_err(|_| format!("create_stream on {} timed out", srv.url()))?
    .map_err(|e| format!("create_stream on {} failed: {e}", srv.url()))?;
    // Let the other servers of this round come and go, then prove this server is still ours and still up.
    tokio::time::sleep(Duration::from_millis(fastrand_ms(tag))).await;
    let names: Vec<String> = {
        use futures::TryStreamExt;
        tokio::time::timeout(Duration::from_secs(10), js.stream_names().try_collect())
            .await
            .map_err(|_| format!("stream_names on {} timed out", srv.url()))?
            .map_err(|e| format!("stream_names on {} failed: {e}", srv.url()))?
    };
    if names != vec![name.clone()] {
        return Err(format!(
            "server at {} is not this test's own: streams {names:?}, expected only {name}",
            srv.url()
        ));
    }
    client
        .flush()
        .await
        .map_err(|e| format!("flush on {} failed: {e}", srv.url()))?;
    Ok(())
}

/// A cheap per-thread spread (0..300 ms) so drops are staggered; no RNG crate needed.
fn fastrand_ms(tag: &str) -> u64 {
    tag.bytes().fold(1469598103934665603u64, |h, b| {
        (h ^ b as u64).wrapping_mul(1099511628211)
    }) % 300
}

#[test]
fn many_concurrent_throwaway_servers_each_accept_their_first_connection() {
    let mut failures = Vec::new();
    for round in 0..ROUNDS {
        let gate = Arc::new(Barrier::new(THREADS));
        let handles: Vec<_> = (0..THREADS)
            .map(|i| {
                let gate = gate.clone();
                std::thread::spawn(move || -> Result<(), String> {
                    gate.wait();
                    let srv = NatsServer::start();
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .unwrap();
                    rt.block_on(first_contact(&srv, &format!("r{round}t{i}")))
                })
            })
            .collect();
        for (i, h) in handles.into_iter().enumerate() {
            match h.join() {
                Ok(Ok(())) => {}
                Ok(Err(e)) => failures.push(format!("round {round} thread {i}: {e}")),
                Err(p) => failures.push(format!(
                    "round {round} thread {i} panicked: {}",
                    p.downcast_ref::<String>()
                        .cloned()
                        .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
                        .unwrap_or_default()
                )),
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} throwaway servers failed first contact:\n{}",
        failures.len(),
        ROUNDS * THREADS,
        failures.join("\n")
    );
}
