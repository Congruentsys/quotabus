//! HAZ-001: credentials in a `nats://user:pass@host` bus URL (the `[bus] url` or `QUOTABUS_NATS_URL`) reach the
//! connection, and no error, log or row ever contains the password (DESIGN §3 bus config, §6 security).
//!
//! The item's Done-when allows either "credentials reach the connection" or "refused with a clear config error";
//! these tests pin the FIRST: a probe with a creds URL publishes, a reader with it reads.
//!
//! Fixture: a throwaway loopback nats-server (JetStream) whose only user is `qbuser` / `fakePASSnotreal9`, so a
//! connection without those credentials is refused. No real key, no real bus: services are a loopback wiremock
//! stub (fake key) and a service whose secret env is unset (no network at all).
//!
//! Controls:
//! - known answer: the fixture accepts `qbuser`/`PASS` from a direct async-nats connection (so a red CLI test is the
//!   CLI dropping the credentials, not a broken server);
//! - negative control: the same server WITHOUT credentials reads CANNOT-ASSESS rc 2 (must stay so after the fix);
//! - leak-checker control: the checker used on every output fires on text that carries the password, including the
//!   exact shape a naive fix would print (`bus unreachable: nats://qbuser:<pw>@…`);
//! - mutation: dropping the credentials again (today's code) turns the two publish/read tests red.

mod common;

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

use common::{FAKE_PLAIN, NatsServer, closed_port, leaks, rc, run_cli, text};
use quotabus::Record;
use serde_json::Value;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

const T: Duration = Duration::from_secs(60);
const USER: &str = "qbuser";
/// The fixture's password: fake, and shaped like NO redaction pattern, so only a value scrub (or never printing it)
/// keeps it out of the output.
const PASS: &str = "fakePASSnotreal9";
/// A wrong password, equally pattern-free.
const WRONG: &str = "wrongPASSnotreal7";
const BUCKET: &str = "qb_haz001";

/// A service that needs no network: its secret env is never set, so the probe writes `cannot_assess:secret_unset`.
const NOKEY: &str = "api.nokey.acct.m1";
/// A service probed against a loopback stub with a fake key: a fresh `ok` row.
const GOOD: &str = "api.good.acct.m1";

/// A throwaway JetStream nats-server on loopback that requires `qbuser` / `PASS`; killed on drop.
struct AuthNats {
    child: Child,
    port: u16,
    _dir: tempfile::TempDir,
}

impl AuthNats {
    fn start() -> AuthNats {
        drop(NatsServer::start()); // fails with a clear message when nats-server is missing
        let dir = tempfile::tempdir().unwrap();
        let port = common::free_port();
        let conf = dir.path().join("nats.conf");
        std::fs::write(
            &conf,
            format!(
                "listen: 127.0.0.1:{port}\njetstream {{ store_dir: {:?} }}\n\
                 authorization {{ users = [ {{ user: {USER}, password: {PASS} }} ] }}\n",
                dir.path().join("js").display().to_string()
            ),
        )
        .unwrap();
        let child = Command::new("nats-server")
            .arg("-c")
            .arg(&conf)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn nats-server");
        let mut s = AuthNats {
            child,
            port,
            _dir: dir,
        };
        let deadline = Instant::now() + Duration::from_secs(10);
        while std::net::TcpStream::connect(("127.0.0.1", port)).is_err() {
            if let Ok(Some(st)) = s.child.try_wait() {
                panic!("auth nats-server exited: {st}");
            }
            assert!(
                Instant::now() < deadline,
                "auth nats-server did not listen on 127.0.0.1:{port}"
            );
            std::thread::sleep(Duration::from_millis(50));
        }
        std::thread::sleep(Duration::from_millis(200));
        s
    }

    fn plain_url(&self) -> String {
        format!("nats://127.0.0.1:{}", self.port)
    }

    fn creds_url(&self, password: &str) -> String {
        format!("nats://{USER}:{password}@127.0.0.1:{}", self.port)
    }

    /// The row under `key` in `bucket`, read directly as `qbuser` (the test's own view of the bus).
    async fn raw_row(&self, bucket: &str, key: &str) -> Option<String> {
        let client = async_nats::ConnectOptions::with_user_and_password(USER.into(), PASS.into())
            .connect(self.plain_url())
            .await
            .expect("the fixture accepts qbuser/PASS");
        let js = async_nats::jetstream::new(client);
        let kv = js.get_key_value(bucket).await.ok()?;
        kv.get(key)
            .await
            .expect("kv get")
            .map(|b| String::from_utf8_lossy(&b).into_owned())
    }
}

impl Drop for AuthNats {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
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

/// `[probe]` + the two services; `bus` is a `[bus]` table or empty (then only `QUOTABUS_NATS_URL` names the bus).
fn write_cfg(dir: &Path, bus: &str, stub: &str) -> PathBuf {
    let p = dir.join("quotabus.toml");
    std::fs::write(
        &p,
        format!(
            "{bus}\n[ttl]\napi = \"2m\"\nbalance = \"2m\"\n\n\
             [[service]]\nid = \"nokey\"\nkind = \"api\"\nprovider = \"nokey\"\nfamily = \"nokey\"\naccount = \"acct\"\n\
             base_url = \"{stub}/nokey\"\nprotocol = \"anthropic\"\nmodels = [\"m1\"]\nsecret = \"QB_HAZ_UNSET\"\n\n\
             [[service]]\nid = \"good\"\nkind = \"api\"\nprovider = \"good\"\nfamily = \"good\"\naccount = \"acct\"\n\
             base_url = \"{stub}/good\"\nprotocol = \"anthropic\"\nmodels = [\"m1\"]\nsecret = \"QB_HAZ_KEY\"\n"
        ),
    )
    .unwrap();
    p
}

fn bus_table(url: &str, bucket: &str) -> String {
    format!("[bus]\nurl = {url:?}\nbucket = {bucket:?}\n")
}

/// Run the CLI off the async runtime. `envs` always carries the fake key for `good` and `RUST_LOG=trace`, so every
/// log line a client library emits (including a CONNECT it traces) is in stderr for the leak check.
async fn cli(cfg: &Path, args: &[&str], extra: &[(&str, &str)]) -> Output {
    let cfg = cfg.to_path_buf();
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let mut envs: Vec<(String, String)> = vec![
        ("QB_HAZ_KEY".into(), FAKE_PLAIN.into()),
        ("RUST_LOG".into(), "trace".into()),
    ];
    envs.extend(extra.iter().map(|(k, v)| (k.to_string(), v.to_string())));
    tokio::task::spawn_blocking(move || {
        let a: Vec<&str> = args.iter().map(String::as_str).collect();
        let e: Vec<(&str, &str)> = envs.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        run_cli(&cfg, &a, &e, T)
    })
    .await
    .unwrap()
}

fn assert_no_password(what: &str, s: &str) {
    let found = leaks(s, &[PASS, WRONG, FAKE_PLAIN]);
    assert!(
        found.is_empty(),
        "{what}: the bus password (or the key) leaked: {found:?}\n---\n{s}"
    );
}

fn json_entry(out: &Output, key: &str) -> Value {
    let v: Value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("status --json is not JSON ({e}): {}", text(out)));
    v.as_array()
        .and_then(|a| a.iter().find(|r| r["key"] == key).cloned())
        .unwrap_or_else(|| panic!("no {key} in status --json: {}", text(out)))
}

/// The full round trip with a creds URL: probe publishes both rows, `status` reads them as the rows imply, and no
/// output or row carries the password.
async fn assert_round_trip(srv: &AuthNats, cfg: &Path, extra: &[(&str, &str)], bucket: &str) {
    let probe = cli(cfg, &["probe"], extra).await;
    // the connection first (so a red here names dropped credentials), then the leak check on the same output
    assert_eq!(
        rc(&probe),
        0,
        "a probe whose bus URL carries the server's credentials publishes: {}",
        common::text(&probe)
            .lines()
            .filter(|l| l.starts_with("quotabus:"))
            .collect::<Vec<_>>()
            .join("\n")
    );
    assert_no_password("probe stdout+stderr (RUST_LOG=trace)", &text(&probe));
    assert!(
        String::from_utf8_lossy(&probe.stdout)
            .contains(&format!("published 2 rows to bucket {bucket}")),
        "{}",
        text(&probe)
    );

    for key in [NOKEY, GOOD] {
        let raw = srv
            .raw_row(bucket, key)
            .await
            .unwrap_or_else(|| panic!("the probe published no row {key} to {bucket}"));
        assert_no_password(&format!("row {key}"), &raw);
        let row: Record = serde_json::from_str(&raw).expect("a row");
        assert_eq!(row.key, key);
    }

    // known answers: `good` is a fresh ok row (rc 0); `nokey` is the row's own cannot_assess:secret_unset (rc 2 as
    // the ROW implies, not the bus-unreachable rc 2)
    let good = cli(cfg, &["status", "--check", "good"], extra).await;
    assert_no_password("status --check good", &text(&good));
    assert_eq!(
        rc(&good),
        0,
        "status with the creds URL reads the fresh ok row: {}",
        text(&good)
    );

    let nokey = cli(cfg, &["status", "--check", "nokey"], extra).await;
    assert_no_password("status --check nokey", &text(&nokey));
    assert_eq!(rc(&nokey), 2, "{}", text(&nokey));

    let json = cli(cfg, &["status", "--json"], extra).await;
    assert_no_password("status --json", &text(&json));
    let e = json_entry(&json, NOKEY);
    assert_eq!(
        e["reason"], "cannot_assess:secret_unset",
        "the reason is the row's, not the bus's: {e}"
    );
    assert!(e["row"].is_object(), "the row was read: {e}");
    let e = json_entry(&json, GOOD);
    assert_eq!(e["verdict"], "ok", "{e}");
}

// ------------------------------------------------------------------------------------------- controls

#[tokio::test(flavor = "multi_thread")]
async fn control_known_answer_the_fixture_accepts_qbuser_and_refuses_anonymous() {
    let srv = AuthNats::start();
    // accepts the credentials (raw_row panics if the connection is refused); no bucket yet
    assert!(srv.raw_row(BUCKET, NOKEY).await.is_none());
    // refuses a connection without them, and one with the wrong password
    let anon = async_nats::ConnectOptions::new()
        .connection_timeout(Duration::from_secs(5))
        .connect(srv.plain_url())
        .await;
    assert!(anon.is_err(), "the fixture must require credentials");
    let wrong = async_nats::ConnectOptions::with_user_and_password(USER.into(), WRONG.into())
        .connection_timeout(Duration::from_secs(5))
        .connect(srv.plain_url())
        .await;
    assert!(wrong.is_err(), "the fixture must refuse a wrong password");
}

#[test]
fn control_the_leak_checker_catches_the_password() {
    let url = format!("nats://{USER}:{PASS}@127.0.0.1:4222");
    // the shapes an unredacted error or log would take
    for s in [
        format!("quotabus: cannot publish: bus unreachable: {url}"),
        format!("CANNOT-ASSESS: bus unreachable: failed to connect to {url}"),
        format!(r#"CONNECT {{"user":"{USER}","pass":"{PASS}"}}"#),
        PASS.to_string(),
    ] {
        assert!(
            !leaks(&s, &[PASS, WRONG, FAKE_PLAIN]).is_empty(),
            "the checker must flag {s:?}"
        );
        let r = std::panic::catch_unwind(|| assert_no_password("control", &s));
        assert!(r.is_err(), "assert_no_password must fail on {s:?}");
    }
    let wrong = format!("bus unreachable: nats://{USER}:{WRONG}@127.0.0.1:1");
    assert!(!leaks(&wrong, &[PASS, WRONG]).is_empty());
    // and stays quiet on a scrubbed / credential-free text
    for s in [
        "bus unreachable: nats://127.0.0.1:4222: authorization violation".to_string(),
        "bus unreachable: nats://qbuser:***@127.0.0.1:4222".to_string(),
    ] {
        assert!(leaks(&s, &[PASS, WRONG, FAKE_PLAIN]).is_empty(), "{s:?}");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn control_without_credentials_the_same_server_is_cannot_assess_rc_2() {
    let srv = AuthNats::start();
    let stub = ok_stub().await;
    let dir = tempfile::tempdir().unwrap();
    let cfg = write_cfg(
        dir.path(),
        &bus_table(&srv.plain_url(), BUCKET),
        &stub.uri(),
    );

    let probe = cli(&cfg, &["probe"], &[]).await;
    assert_eq!(
        rc(&probe),
        1,
        "a probe that cannot reach the bus fails: {}",
        text(&probe)
    );
    assert!(
        srv.raw_row(BUCKET, GOOD).await.is_none(),
        "nothing was published"
    );

    let st = cli(&cfg, &["status", "--check", "good"], &[]).await;
    assert_eq!(
        rc(&st),
        2,
        "no credentials: CANNOT-ASSESS rc 2: {}",
        text(&st)
    );
    let json = cli(&cfg, &["status", "--json"], &[]).await;
    let e = json_entry(&json, GOOD);
    assert_eq!(e["verdict"], "CANNOT-ASSESS", "{e}");
    assert_ne!(e["reason"], "absent", "{e}");
    assert!(e["row"].is_null(), "{e}");
}

// ------------------------------------------------------------------------------------------- HAZ-001

#[tokio::test(flavor = "multi_thread")]
async fn creds_in_the_bus_table_url_reach_the_connection_probe_publishes_and_status_reads() {
    let srv = AuthNats::start();
    let stub = ok_stub().await;
    let dir = tempfile::tempdir().unwrap();
    let cfg = write_cfg(
        dir.path(),
        &bus_table(&srv.creds_url(PASS), BUCKET),
        &stub.uri(),
    );
    assert_round_trip(&srv, &cfg, &[], BUCKET).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn creds_in_quotabus_nats_url_reach_the_connection_probe_publishes_and_status_reads() {
    let srv = AuthNats::start();
    let stub = ok_stub().await;
    let dir = tempfile::tempdir().unwrap();
    // no [bus] table, no [file] table: the env var alone names the bus (default bucket)
    let cfg = write_cfg(dir.path(), "", &stub.uri());
    let url = srv.creds_url(PASS);
    assert_round_trip(
        &srv,
        &cfg,
        &[("QUOTABUS_NATS_URL", url.as_str())],
        quotabus::config::DEFAULT_BUCKET,
    )
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_wrong_password_is_cannot_assess_rc_2_and_no_output_contains_it() {
    let srv = AuthNats::start();
    let stub = ok_stub().await;
    let dir = tempfile::tempdir().unwrap();
    let url = srv.creds_url(WRONG);
    for (label, bus, extra) in [
        ("[bus] url", bus_table(&url, BUCKET), vec![]),
        (
            "QUOTABUS_NATS_URL",
            String::new(),
            vec![("QUOTABUS_NATS_URL", url.as_str())],
        ),
    ] {
        let cfg = write_cfg(dir.path(), &bus, &stub.uri());
        let probe = cli(&cfg, &["probe"], &extra).await;
        assert_no_password(&format!("{label}: probe"), &text(&probe));
        assert_eq!(rc(&probe), 1, "{label}: {}", text(&probe));

        let st = cli(&cfg, &["status", "--check", "good"], &extra).await;
        assert_no_password(&format!("{label}: status --check"), &text(&st));
        assert_eq!(
            rc(&st),
            2,
            "{label}: a wrong password is CANNOT-ASSESS rc 2: {}",
            text(&st)
        );

        let json = cli(&cfg, &["status", "--json"], &extra).await;
        assert_no_password(&format!("{label}: status --json"), &text(&json));
        let e = json_entry(&json, GOOD);
        assert_eq!(e["verdict"], "CANNOT-ASSESS", "{label}: {e}");
        assert!(e["row"].is_null(), "{label}: {e}");
    }
    assert!(
        srv.raw_row(BUCKET, GOOD).await.is_none(),
        "nothing was published"
    );
    assert!(
        srv.raw_row(quotabus::config::DEFAULT_BUCKET, GOOD)
            .await
            .is_none(),
        "nothing was published"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_down_bus_with_a_creds_url_leaks_no_password() {
    let stub = ok_stub().await;
    let dir = tempfile::tempdir().unwrap();
    let url = format!("nats://{USER}:{PASS}@127.0.0.1:{}", closed_port());
    for (label, bus, extra) in [
        ("[bus] url", bus_table(&url, BUCKET), vec![]),
        (
            "QUOTABUS_NATS_URL",
            String::new(),
            vec![("QUOTABUS_NATS_URL", url.as_str())],
        ),
    ] {
        let cfg = write_cfg(dir.path(), &bus, &stub.uri());
        let probe = cli(&cfg, &["probe"], &extra).await;
        assert_no_password(&format!("{label}: probe"), &text(&probe));
        assert_eq!(rc(&probe), 1, "{label}: {}", text(&probe));

        let st = cli(&cfg, &["status", "--check", "good"], &extra).await;
        assert_no_password(&format!("{label}: status --check"), &text(&st));
        assert_eq!(rc(&st), 2, "{label}: {}", text(&st));

        let json = cli(&cfg, &["status", "--json"], &extra).await;
        assert_no_password(&format!("{label}: status --json"), &text(&json));
        let e = json_entry(&json, GOOD);
        assert_eq!(e["reason"], "cannot_assess:bus_unreachable", "{label}: {e}");
    }
}
