//! HAZ-001 ruled test additions for review r1 (reviews/HAZ-001-r1.md) F1, F2, F4. Fake passwords, loopback stubs and
//! a throwaway nats-server only. (F3, an unencoded `/ # ,` in a password, is a nit left as is: no test.)
//!
//! F1 (should-fix): a malformed config whose `[bus] url` line carries `user:password@` (stray text after the value,
//!     an unterminated string, a wrong type) must not print the password on stdout or stderr from `probe` or
//!     `status`; the URL may be shown as `user:***@`.
//!     Control: the raw `Config::load` error DOES carry the password, and the leak checker sees it there.
//! F2 (nit, ruled in): a comma-separated server list with credentials, a closed port first and the throwaway auth
//!     server second, connects and round-trips.
//!     Control: the same list WITHOUT credentials is refused by the fixture (CANNOT-ASSESS rc 2).
//! F4 (nit): the error text `connect()` returns (the `BackendError` from `NatsKv::connect_reader` /
//!     `connect_publisher`, observed directly, not through the CLI's redacting writer) carries no password.
//!     Mutation (run by hand, reported with the commit): `connect()` putting the raw URL in its error AND `scrub()`
//!     made the identity turns these red.

mod common;

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output};
use std::time::Duration;

use common::{FAKE_PLAIN, closed_port, leaks, rc, run_cli, text};
use quotabus::{BackendError, BusUrl, Config, NatsKv};
use serde_json::Value;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

const T: Duration = Duration::from_secs(60);
const USER: &str = "qbuser";
/// Fake, and shaped like no redaction pattern: only a value scrub (or never printing it) keeps it out.
const PASS: &str = "fakePASSnotreal9";
const WRONG: &str = "wrongPASSnotreal7";
const CFG_PW: &str = "fakeCfgPw9notreal";
const BUCKET: &str = "qb_haz001_r1";
const GOOD: &str = "api.good.acct.m1";

fn assert_no_password(what: &str, s: &str, secrets: &[&str]) {
    let found = leaks(s, secrets);
    assert!(
        found.is_empty(),
        "{what}: a bus password leaked: {found:?}\n---\n{s}"
    );
}

/// A throwaway JetStream nats-server on loopback that requires `qbuser` / `PASS`; killed on drop.
struct AuthNats {
    child: Child,
    port: u16,
    _dir: tempfile::TempDir,
}

impl AuthNats {
    fn start() -> AuthNats {
        common::require_nats_server();
        let dir = tempfile::tempdir().unwrap();
        let conf = dir.path().join("nats.conf");
        std::fs::write(
            &conf,
            format!(
                "jetstream {{ store_dir: {:?} }}\n\
                 authorization {{ users = [ {{ user: {USER}, password: {PASS} }} ] }}\n",
                dir.path().join("js").display().to_string()
            ),
        )
        .unwrap();
        let mut cmd = Command::new("nats-server");
        cmd.arg("-c").arg(&conf);
        let (child, port) = common::spawn_nats(cmd, "auth", dir.path());
        AuthNats {
            child,
            port,
            _dir: dir,
        }
    }

    async fn raw_row(&self, bucket: &str, key: &str) -> Option<String> {
        let client = async_nats::ConnectOptions::with_user_and_password(USER.into(), PASS.into())
            .connect(format!("nats://127.0.0.1:{}", self.port))
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

fn services(stub: &str) -> String {
    format!(
        "[probe]\nttl = \"2m\"\n\n\
         [[service]]\nid = \"good\"\nkind = \"api\"\nprovider = \"good\"\nfamily = \"good\"\naccount = \"acct\"\n\
         base_url = \"{stub}/good\"\nprotocol = \"anthropic\"\nmodels = [\"m1\"]\nsecret = \"QB_HAZ_KEY\"\n"
    )
}

async fn cli(cfg: &Path, args: &[&str]) -> Output {
    let cfg = cfg.to_path_buf();
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    tokio::task::spawn_blocking(move || {
        let a: Vec<&str> = args.iter().map(String::as_str).collect();
        run_cli(
            &cfg,
            &a,
            &[("QB_HAZ_KEY", FAKE_PLAIN), ("RUST_LOG", "trace")],
            T,
        )
    })
    .await
    .unwrap()
}

// ------------------------------------------------------------------------------------------------- F1

/// The reviewer's three repros: stray text after the value, an unterminated string, a wrong type.
fn malformed_bus_tables() -> Vec<(&'static str, String)> {
    let url = format!("nats://{USER}:{CFG_PW}@127.0.0.1:4222");
    vec![
        (
            "stray text after the url",
            format!("[bus]\nurl = \"{url}\" junk\nbucket = \"{BUCKET}\"\n"),
        ),
        (
            "unterminated string",
            format!("[bus]\nurl = \"{url}\nbucket = \"{BUCKET}\"\n"),
        ),
        (
            "wrong type (array)",
            format!("[bus]\nurl = [\"{url}\"]\nbucket = \"{BUCKET}\"\n"),
        ),
    ]
}

fn write_malformed(dir: &Path, i: usize, bus: &str) -> PathBuf {
    let p = dir.join(format!("bad{i}.toml"));
    std::fs::write(&p, format!("{bus}\n{}", services("http://127.0.0.1:9"))).unwrap();
    p
}

#[test]
fn control_f1_the_raw_config_error_carries_the_password_and_the_checker_sees_it() {
    let dir = tempfile::tempdir().unwrap();
    for (i, (label, bus)) in malformed_bus_tables().into_iter().enumerate() {
        let p = write_malformed(dir.path(), i, &bus);
        let err = match Config::load(&p) {
            Ok(_) => panic!("{label}: the fixture must not parse"),
            Err(e) => e.to_string(),
        };
        assert!(
            !leaks(&err, &[CFG_PW]).is_empty(),
            "{label}: the unredacted Config::load error is expected to quote the password (else this fixture \
             tests nothing): {err}"
        );
        let r = std::panic::catch_unwind(|| assert_no_password("control", &err, &[CFG_PW]));
        assert!(
            r.is_err(),
            "{label}: assert_no_password must fail on the raw error"
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn f1_a_malformed_bus_url_line_never_prints_its_password_from_probe_or_status() {
    let dir = tempfile::tempdir().unwrap();
    for (i, (label, bus)) in malformed_bus_tables().into_iter().enumerate() {
        let p = write_malformed(dir.path(), i, &bus);
        let probe = cli(&p, &["probe"]).await;
        assert_no_password(&format!("{label}: probe"), &text(&probe), &[CFG_PW]);
        assert_eq!(
            rc(&probe),
            1,
            "{label}: a bad config fails the probe: {}",
            text(&probe)
        );
        assert!(
            text(&probe).contains(&format!("bad{i}.toml")),
            "{label}: the error still names the config: {}",
            text(&probe)
        );

        for args in [
            &["status"][..],
            &["status", "--json"],
            &["status", "--check", "good"],
        ] {
            let st = cli(&p, args).await;
            assert_no_password(&format!("{label}: {args:?}"), &text(&st), &[CFG_PW]);
            assert_eq!(
                rc(&st),
                2,
                "{label}: {args:?} is CANNOT-ASSESS rc 2: {}",
                text(&st)
            );
        }
    }
}

// ------------------------------------------------------------------------------------------------- F2

fn list_cfg(dir: &Path, url: &str, stub: &str) -> PathBuf {
    let p = dir.join("quotabus.toml");
    std::fs::write(
        &p,
        format!(
            "[bus]\nurl = {url:?}\nbucket = {BUCKET:?}\n\n{}",
            services(stub)
        ),
    )
    .unwrap();
    p
}

#[tokio::test(flavor = "multi_thread")]
async fn control_f2_the_server_list_without_credentials_is_refused_rc_2() {
    let srv = AuthNats::start();
    let stub = ok_stub().await;
    let dir = tempfile::tempdir().unwrap();
    let url = format!(
        "nats://127.0.0.1:{},nats://127.0.0.1:{}",
        closed_port(),
        srv.port
    );
    let cfg = list_cfg(dir.path(), &url, &stub.uri());
    let probe = cli(&cfg, &["probe"]).await;
    assert_eq!(rc(&probe), 1, "{}", text(&probe));
    assert!(srv.raw_row(BUCKET, GOOD).await.is_none());
    let st = cli(&cfg, &["status", "--check", "good"]).await;
    assert_eq!(rc(&st), 2, "{}", text(&st));
}

#[tokio::test(flavor = "multi_thread")]
async fn f2_a_server_list_with_credentials_connects_and_round_trips() {
    let srv = AuthNats::start();
    let stub = ok_stub().await;
    let dir = tempfile::tempdir().unwrap();
    let url = format!(
        "nats://{USER}:{PASS}@127.0.0.1:{},nats://{USER}:{PASS}@127.0.0.1:{}",
        closed_port(),
        srv.port
    );
    let cfg = list_cfg(dir.path(), &url, &stub.uri());

    let probe = cli(&cfg, &["probe"]).await;
    assert_eq!(
        rc(&probe),
        0,
        "a creds server list (closed port first) publishes: {}",
        text(&probe)
            .lines()
            .filter(|l| l.starts_with("quotabus:"))
            .collect::<Vec<_>>()
            .join("\n")
    );
    assert_no_password("probe", &text(&probe), &[PASS]);
    let raw = srv
        .raw_row(BUCKET, GOOD)
        .await
        .expect("the probe published the row");
    assert_no_password("row", &raw, &[PASS]);

    let st = cli(&cfg, &["status", "--check", "good"]).await;
    assert_no_password("status --check", &text(&st), &[PASS]);
    assert_eq!(rc(&st), 0, "status reads the fresh ok row: {}", text(&st));
    let json = cli(&cfg, &["status", "--json"]).await;
    assert_no_password("status --json", &text(&json), &[PASS]);
    let v: Value = serde_json::from_slice(&json.stdout).expect("json");
    let e = v
        .as_array()
        .and_then(|a| a.iter().find(|r| r["key"] == GOOD).cloned())
        .expect("good entry");
    assert_eq!(e["verdict"], "ok", "{e}");
}

// ------------------------------------------------------------------------------------------------- F4

fn err_of<T>(r: Result<T, BackendError>, what: &str) -> BackendError {
    match r {
        Ok(_) => panic!("{what}: expected a connection error"),
        Err(e) => e,
    }
}

fn assert_clean_error(what: &str, e: &BackendError, secrets: &[&str]) {
    let shown = format!("{e} | {e:?}");
    assert_no_password(what, &shown, secrets);
    assert_eq!(
        e.reason(),
        "cannot_assess:bus_unreachable",
        "{what}: a failed connect is bus_unreachable: {shown}"
    );
}

#[test]
fn control_f4_the_checker_sees_a_password_in_a_backend_error() {
    let e = BackendError::Unreachable(format!(
        "failed to connect to nats://{USER}:{PASS}@127.0.0.1:1: connection refused"
    ));
    let shown = format!("{e} | {e:?}");
    assert!(!leaks(&shown, &[PASS]).is_empty());
    let r = std::panic::catch_unwind(|| assert_no_password("control", &shown, &[PASS]));
    assert!(
        r.is_err(),
        "assert_no_password must fail on a BackendError carrying the password"
    );
    // BusUrl's own Debug is the masked form
    let dbg = format!(
        "{:?}",
        BusUrl::parse(&format!("nats://{USER}:{PASS}@127.0.0.1:1"))
    );
    assert_no_password("BusUrl Debug", &dbg, &[PASS]);
}

#[tokio::test(flavor = "multi_thread")]
async fn f4_connect_errors_to_a_down_bus_carry_no_password() {
    let port = closed_port();
    for url in [
        format!("nats://{USER}:{PASS}@127.0.0.1:{port}"),
        format!("nats://{PASS}@127.0.0.1:{port}"), // token form
        format!("nats://{USER}:{PASS}@127.0.0.1:{port},nats://{USER}:{PASS}@127.0.0.1:{port}"),
    ] {
        let e = err_of(NatsKv::connect_reader(&url, BUCKET).await, "connect_reader");
        assert_clean_error("connect_reader (down bus)", &e, &[PASS]);
        let e = err_of(
            NatsKv::connect_publisher(&url, BUCKET).await,
            "connect_publisher",
        );
        assert_clean_error("connect_publisher (down bus)", &e, &[PASS]);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn f4_connect_errors_for_a_wrong_password_carry_no_password() {
    let srv = AuthNats::start();
    let url = format!("nats://{USER}:{WRONG}@127.0.0.1:{}", srv.port);
    let e = err_of(NatsKv::connect_reader(&url, BUCKET).await, "connect_reader");
    assert_clean_error("connect_reader (wrong password)", &e, &[WRONG, PASS]);
    let e = err_of(
        NatsKv::connect_publisher(&url, BUCKET).await,
        "connect_publisher",
    );
    assert_clean_error("connect_publisher (wrong password)", &e, &[WRONG, PASS]);
}
