//! Shared test helpers for the EXP-001 tests: a sample record, a throwaway loopback nats-server, a CLI runner and
//! a leak checker. No helper ever reads a real key or reaches a non-loopback host.
#![allow(dead_code)]

use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use quotabus::{Kind, Probe, ProbeSource, Record, State};

/// A fake key: shaped like a provider key so the pattern scrubs see it, never a real one.
pub const FAKE_SK: &str = "sk-test-not-a-key-0123456789abcdef";
/// A fake key that matches NO redaction pattern: only the Redactor's value scrub can catch it.
pub const FAKE_PLAIN: &str = "fakeQBKEY7f3a9c0dNOTREAL";

pub fn record(key: &str, state: State, checked_at: DateTime<Utc>, ttl_s: u64) -> Record {
    let parts: Vec<&str> = key.splitn(4, '.').collect();
    assert_eq!(
        parts.len(),
        4,
        "test key must be <kind>.<provider>.<account>.<model>: {key}"
    );
    let kind = match parts[0] {
        "api" => Kind::Api,
        "local" => Kind::Local,
        "subscription" => Kind::Subscription,
        other => panic!("bad kind {other}"),
    };
    Record {
        contract: "ai-status/1".into(),
        key: key.into(),
        kind,
        provider: parts[1].into(),
        account: parts[2].into(),
        model: parts[3].into(),
        family: parts[1].into(),
        state,
        reason: if state == State::Unknown {
            Some("cannot_assess:unreachable".into())
        } else {
            None
        },
        balance: None,
        headroom: None,
        latency_ms: Some(123),
        probe: Probe {
            name: "messages".into(),
            source: ProbeSource::Official,
        },
        error: None,
        checked_at,
        ttl_s,
        observed_by: "testhost/quotabus@0.0.0".into(),
    }
}

/// Every leak of a secret in `text`: a known value, or a `Bearer <x>`, `sk-<x>`, `ghp_<x>` whose `<x>` is not `*`.
pub fn leaks(text: &str, secrets: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    for s in secrets {
        if !s.is_empty() && text.contains(s) {
            out.push(format!("secret value {s:?}"));
        }
    }
    for pat in ["Bearer ", "sk-", "ghp_"] {
        let mut from = 0;
        while let Some(i) = text[from..].find(pat) {
            let at = from + i + pat.len();
            // `sk-` / `ghp_` only at a word start (so `task-1` or `disk-full` is not a leak).
            let word_start = pat == "Bearer "
                || text[..from + i]
                    .chars()
                    .next_back()
                    .is_none_or(|p| !p.is_ascii_alphanumeric());
            if !word_start {
                from = at;
                continue;
            }
            if let Some(c) = text[at..].chars().next()
                && (c.is_ascii_alphanumeric() || c == '-' || c == '_')
            {
                {
                    let end = (at + 24).min(text.len());
                    let end = (at..=end)
                        .rev()
                        .find(|e| text.is_char_boundary(*e))
                        .unwrap_or(at);
                    out.push(format!(
                        "pattern {pat:?} at {i}: {:?}",
                        &text[from + i..end]
                    ));
                }
            }
            from = at;
        }
    }
    out
}

pub fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

/// A loopback port that REFUSES connections, deterministically: port 1 (tcpmux, never served on a dev or CI box)
/// is privileged, so no non-root test process can bind it, and `free_port()` only hands out ephemeral ports. The
/// previous bind-drop-reuse of a free port raced with parallel tests that were handed the same port. The check
/// below proves it is a prompt "connection refused" (not a timeout) every time it is used.
pub fn closed_port() -> u16 {
    const P: u16 = 1;
    let started = Instant::now();
    match TcpStream::connect_timeout(
        &(std::net::Ipv4Addr::LOCALHOST, P).into(),
        Duration::from_secs(2),
    ) {
        Err(e) if e.kind() == std::io::ErrorKind::ConnectionRefused => {}
        Err(e) => panic!("127.0.0.1:{P} must refuse a connection, got {e:?}"),
        Ok(_) => panic!("127.0.0.1:{P} unexpectedly accepts connections: something listens on it"),
    }
    assert!(
        started.elapsed() < Duration::from_secs(1),
        "127.0.0.1:{P} refused too slowly"
    );
    P
}

/// A throwaway `nats-server -js` on loopback, killed on drop.
pub struct NatsServer {
    child: Child,
    pub port: u16,
    _dir: tempfile::TempDir,
}

impl NatsServer {
    pub fn start() -> NatsServer {
        match Command::new("nats-server").arg("--version").output() {
            Ok(o) if o.status.success() => {}
            _ => panic!(
                "nats-server is not on PATH: these tests need a local nats-server (2.11+) binary to start a \
                 throwaway JetStream server on 127.0.0.1. Install it (e.g. `brew install nats-server`)."
            ),
        }
        let dir = tempfile::tempdir().unwrap();
        let port = free_port();
        let child = Command::new("nats-server")
            .args(["-js", "-a", "127.0.0.1", "-p", &port.to_string(), "-sd"])
            .arg(dir.path())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn nats-server");
        let mut srv = NatsServer {
            child,
            port,
            _dir: dir,
        };
        let deadline = Instant::now() + Duration::from_secs(10);
        while TcpStream::connect(("127.0.0.1", port)).is_err() {
            if Instant::now() > deadline {
                let _ = srv.child.kill();
                panic!("throwaway nats-server did not listen on 127.0.0.1:{port} within 10s");
            }
            if let Ok(Some(st)) = srv.child.try_wait() {
                panic!("throwaway nats-server exited early: {st}");
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        std::thread::sleep(Duration::from_millis(200));
        srv
    }

    pub fn url(&self) -> String {
        format!("nats://127.0.0.1:{}", self.port)
    }
}

impl Drop for NatsServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Run the `quotabus` binary with a CLEARED environment plus `envs` (so no real key in the parent env can reach it),
/// failing the test if it runs longer than `timeout`.
pub fn run_cli(config: &Path, args: &[&str], envs: &[(&str, &str)], timeout: Duration) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_quotabus"));
    cmd.env_clear()
        .env("RUST_BACKTRACE", "0")
        .arg("--config")
        .arg(config)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (k, v) in envs {
        cmd.env(k, v);
    }
    let mut child = cmd.spawn().expect("spawn quotabus");
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(_st) = child.try_wait().unwrap() {
            return child.wait_with_output().unwrap();
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let out = child.wait_with_output().unwrap();
            panic!(
                "quotabus {args:?} did not finish within {timeout:?}; stderr: {}",
                String::from_utf8_lossy(&out.stderr)
            );
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}

pub fn rc(out: &Output) -> i32 {
    out.status.code().unwrap_or(-1)
}

pub fn text(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

/// `hostname -s`, independently of the library.
pub fn short_hostname() -> String {
    let o = Command::new("hostname")
        .arg("-s")
        .output()
        .expect("hostname -s");
    String::from_utf8(o.stdout).unwrap().trim().to_string()
}
