//! Shared test helpers for the EXP-001 tests: a sample record, a throwaway loopback nats-server, a CLI runner and
//! a leak checker. No helper ever reads a real key or reaches a non-loopback host.
#![allow(dead_code)]

use std::net::TcpStream;
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

/// A loopback port that REFUSES connections, deterministically: port 1 (tcpmux, never served on a dev or CI box)
/// is privileged, so no non-root test process can bind it, and every throwaway nats-server gets a port nats-server itself chose (`-p -1`). The
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

/// Fail with a clear message when no `nats-server` binary is on PATH.
pub fn require_nats_server() {
    match Command::new("nats-server").arg("--version").output() {
        Ok(o) if o.status.success() => {}
        _ => panic!(
            "nats-server is not on PATH: these tests need a local nats-server (2.11+) binary to start a \
             throwaway JetStream server on 127.0.0.1. Install it (e.g. `brew install nats-server`)."
        ),
    }
}

/// How long a throwaway nats-server may take to bind and greet a client before the test fails.
pub const NATS_READY_BOUND: Duration = Duration::from_secs(10);

/// Spawn `cmd` (a `nats-server` command carrying its own options, e.g. `-js -sd <dir>` or `-c <conf>`) on loopback
/// and return it only once it ACCEPTS a client: a TCP connect to its port is answered with the NATS `INFO` line.
///
/// HAZ-002: the port is chosen by nats-server itself (`-p -1`, read back from `--ports_file_dir`), never by a
/// bind-drop-reuse of a "free" port, so two servers are never handed one port and a port a test holds is the one
/// its own server listens on. These flags override any `listen:` in a config file. Within `NATS_READY_BOUND` the
/// server is ready, or the test panics with what the server logged.
pub fn spawn_nats(mut cmd: Command, what: &str, dir: &Path) -> (Child, u16) {
    let ports_dir = dir.join("ports");
    std::fs::create_dir_all(&ports_dir).unwrap();
    let log_path = dir.join("nats-server.log");
    let log = std::fs::File::create(&log_path).unwrap();
    let mut child = cmd
        .args(["-a", "127.0.0.1", "-p", "-1", "--ports_file_dir"])
        .arg(&ports_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(log)
        .spawn()
        .expect("spawn nats-server");
    let deadline = Instant::now() + NATS_READY_BOUND;
    let fail = |child: &mut Child, why: String| -> ! {
        let _ = child.kill();
        let _ = child.wait();
        let logged = std::fs::read_to_string(&log_path).unwrap_or_default();
        panic!("{what} nats-server not ready: {why}\n--- its log ---\n{logged}");
    };
    let ports_file = ports_dir.join(format!("nats-server_{}.ports", child.id()));
    let mut port = None;
    let mut last = String::from("no ports file yet");
    while Instant::now() < deadline {
        if let Ok(Some(st)) = child.try_wait() {
            fail(&mut child, format!("it exited early: {st}"));
        }
        if port.is_none() {
            port = read_ports_file(&ports_file);
        }
        if let Some(p) = port {
            match greets(p) {
                Ok(()) => return (child, p),
                Err(e) => last = format!("127.0.0.1:{p}: {e}"),
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    fail(
        &mut child,
        format!("not within {NATS_READY_BOUND:?} ({last})"),
    )
}

/// The client port in a nats-server ports file (`{"nats":["nats://127.0.0.1:<port>"]}`), once it is fully written.
fn read_ports_file(path: &Path) -> Option<u16> {
    let text = std::fs::read_to_string(path).ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    let url = v.get("nats")?.as_array()?.first()?.as_str()?;
    url.rsplit(':').next()?.parse().ok()
}

/// A TCP connect to `127.0.0.1:port` is answered with the NATS `INFO {...}` greeting.
fn greets(port: u16) -> std::io::Result<()> {
    use std::io::{BufRead, BufReader};
    let s = TcpStream::connect_timeout(
        &(std::net::Ipv4Addr::LOCALHOST, port).into(),
        Duration::from_secs(1),
    )?;
    s.set_read_timeout(Some(Duration::from_secs(2)))?;
    let mut line = String::new();
    BufReader::new(s).read_line(&mut line)?;
    if line.starts_with("INFO ") {
        Ok(())
    } else {
        Err(std::io::Error::other(format!(
            "no INFO greeting, got {line:?}"
        )))
    }
}

/// A throwaway `nats-server -js` on loopback, killed on drop.
pub struct NatsServer {
    child: Child,
    pub port: u16,
    _dir: tempfile::TempDir,
}

impl NatsServer {
    pub fn start() -> NatsServer {
        require_nats_server();
        let dir = tempfile::tempdir().unwrap();
        let mut cmd = Command::new("nats-server");
        cmd.arg("-js").arg("-sd").arg(dir.path().join("js"));
        let (child, port) = spawn_nats(cmd, "throwaway", dir.path());
        NatsServer {
            child,
            port,
            _dir: dir,
        }
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
