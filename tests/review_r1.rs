//! EXP-001 ruled test additions for review r1 (reviews/EXP-001-r1.md) F1-F4. Loopback stubs, fake keys and a
//! throwaway nats-server only.
//!
//! F1 (§6): a row MAY NOT reveal an email or an org id from a provider's error body; the configured account label is
//!     the operator's and stays intact.
//! F2 (§6, §10 Q3, SIG-002 default yes): `publish_balance = false` publishes NO balance; the floor still applies;
//!     absent ⇒ published.
//! F3 (§2): a row that exists but cannot be parsed, or a bus read that fails while listing, reads CANNOT-ASSESS
//!     (rc 2), never UNKNOWN absent.
//! F4 (§2): a file backend whose directory is missing reads CANNOT-ASSESS rc 2 naming the dir; a probe creates it.

mod common;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::Duration;

use chrono::Utc;
use common::{FAKE_SK, NatsServer, rc, run_cli, text};
use quotabus::{Backend, Config, NatsKv, Record, Redactor, Runner, Secret, State};
use serde_json::{Value, json};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const T: Duration = Duration::from_secs(60);
const EMAIL: &str = "jane.doe@example.com";
const ORG: &str = "org-FAKEorg1234567890";
const ORG_FIELD: &str = "acmeOrgId777";

/// Every email address or `org-<id>` in `text`, plus any of the `extra` literals.
fn pii_leaks(text: &str, extra: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    for word in text.split(|c: char| c.is_whitespace() || "\"'(),;:<>[]{}".contains(c)) {
        let w = word.trim_end_matches('.');
        if let Some(at) = w.find('@')
            && at > 0
            && w[at + 1..].contains('.')
            && !w.contains("quotabus@")
        {
            out.push(format!("email {w:?}"));
        }
        if let Some(id) = w.strip_prefix("org-")
            && id.len() >= 6
            && id.chars().all(|c| c.is_ascii_alphanumeric())
            && id.chars().any(|c| c.is_ascii_digit())
        {
            out.push(format!("org id {w:?}"));
        }
    }
    for e in extra {
        if text.contains(e) {
            out.push(format!("literal {e:?}"));
        }
    }
    out
}

fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> + Send + Sync + 'static {
    let m: HashMap<String, String> = pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    move |n: &str| m.get(n).cloned()
}

fn service(
    id: &str,
    account: &str,
    base_url: &str,
    protocol: &str,
    model: &str,
    extra: &str,
) -> String {
    format!(
        "[[service]]\nid = {id:?}\nkind = \"api\"\nprovider = {id:?}\nfamily = {id:?}\naccount = {account:?}\n\
         base_url = {base_url:?}\nprotocol = {protocol:?}\nmodels = [{model:?}]\nsecret = \"QB_KEY\"\n{extra}"
    )
}

fn config(services: &[String]) -> Config {
    let text = format!(
        "[ttl]\napi = \"2m\"\nbalance = \"2m\"\n\n{}",
        services.join("\n")
    );
    Config::from_toml_str(&text).unwrap_or_else(|e| panic!("test config must parse: {e}\n{text}"))
}

// ------------------------------------------------------------------------------------------------- F1

fn pii_body() -> String {
    json!({"error": {
        "message": format!("Incorrect API key provided. Account owner {EMAIL}, organization {ORG}. Contact billing@example.org."),
        "type": "invalid_request_error",
        "organization": ORG_FIELD,
        "code": "invalid_api_key"
    }})
    .to_string()
}

async fn pii_stub() -> MockServer {
    let stub = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(401).set_body_string(pii_body()))
        .mount(&stub)
        .await;
    stub
}

#[test]
fn control_f1_the_checker_catches_an_unscrubbed_email_and_org_id() {
    let mut row = common::record(
        "api.openai.nusy-product-team.gpt-5-6-sol",
        State::AuthFailed,
        Utc::now(),
        60,
    );
    row.error = Some(pii_body());
    let json = serde_json::to_string(&row).unwrap();
    let found = pii_leaks(&json, &[ORG_FIELD]);
    assert!(found.iter().any(|f| f.contains(EMAIL)), "{found:?}");
    assert!(found.iter().any(|f| f.contains(ORG)), "{found:?}");
    assert!(found.iter().any(|f| f.contains(ORG_FIELD)), "{found:?}");
    // and it passes clean text, including the observed_by `host/quotabus@0.1.0` and model ids
    let clean = "auth_failed http_401 invalid_api_key nusy-product-team kimi-k2.5 M5/quotabus@0.1.0 work-org";
    assert_eq!(pii_leaks(clean, &[ORG_FIELD]), Vec::<String>::new());
}

#[test]
fn f1_the_redactor_scrubs_emails_and_org_ids() {
    let r = Redactor::new(vec![Secret::new(FAKE_SK)]);
    let out = r.redact(&format!(
        "owner {EMAIL}, organization {ORG}; model kimi-k2.5 not found"
    ));
    assert_eq!(pii_leaks(&out, &[]), Vec::<String>::new(), "{out}");
    assert!(
        out.contains("kimi-k2.5 not found"),
        "the rest of the text survives: {out}"
    );
}

#[tokio::test]
async fn f1_an_error_body_with_an_email_and_org_id_is_scrubbed_from_the_row() {
    let stub = pii_stub().await;
    let cfg = config(&[service(
        "openai",
        "nusy-product-team",
        &format!("{}/v1", stub.uri()),
        "openai",
        "gpt-5.6-sol",
        "",
    )]);
    let rows = Runner::new(cfg, env(&[("QB_KEY", FAKE_SK)]))
        .run_once()
        .await;
    assert_eq!(rows.len(), 1);
    let r = &rows[0];
    assert_eq!(r.state, State::AuthFailed, "{r:?}");
    let json = serde_json::to_string(r).unwrap();
    assert_eq!(
        pii_leaks(&json, &[ORG_FIELD, "billing@example.org"]),
        Vec::<String>::new(),
        "{json}"
    );
    assert_eq!(r.account, "nusy-product-team", "the configured label stays");
    assert!(r.key.contains(".nusy-product-team."), "{}", r.key);
}

#[tokio::test]
async fn f1_an_account_label_the_operator_typed_stays_intact() {
    // §6 "unless the operator typed it as the label": the label is config, not provider text
    let stub = pii_stub().await;
    let cfg = config(&[service(
        "openai",
        "ops@example.net",
        &format!("{}/v1", stub.uri()),
        "openai",
        "gpt-5.6-sol",
        "",
    )]);
    let rows = Runner::new(cfg, env(&[("QB_KEY", FAKE_SK)]))
        .run_once()
        .await;
    assert_eq!(rows[0].account, "ops@example.net");
    let err = rows[0].error.clone().unwrap_or_default();
    assert_eq!(pii_leaks(&err, &[ORG_FIELD]), Vec::<String>::new(), "{err}");
}

#[tokio::test(flavor = "multi_thread")]
async fn f1_no_email_or_org_id_reaches_stdout_stderr_or_the_written_row() {
    let stub = pii_stub().await;
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    let cfg = dir.path().join("quotabus.toml");
    let svc = service(
        "openai",
        "nusy-product-team",
        &format!("{}/v1", stub.uri()),
        "openai",
        "gpt-5.6-sol",
        "",
    );
    std::fs::write(
        &cfg,
        format!("[file]\ndir = {:?}\n\n{svc}", state.display().to_string()),
    )
    .unwrap();
    let c = cfg.clone();
    let out = tokio::task::spawn_blocking(move || {
        run_cli(
            &c,
            &["probe"],
            &[("QB_KEY", FAKE_SK), ("RUST_LOG", "trace")],
            T,
        )
    })
    .await
    .unwrap();
    assert_eq!(rc(&out), 0, "{}", text(&out));
    let printed = text(&out);
    assert_eq!(
        pii_leaks(&printed, &[ORG_FIELD]),
        Vec::<String>::new(),
        "stdout/stderr: {printed}"
    );
    let mut written = String::new();
    for f in std::fs::read_dir(&state).unwrap().flatten() {
        written.push_str(&std::fs::read_to_string(f.path()).unwrap());
    }
    assert!(
        written.contains("auth_failed"),
        "control: the row was written: {written}"
    );
    assert_eq!(
        pii_leaks(&written, &[ORG_FIELD]),
        Vec::<String>::new(),
        "{written}"
    );
}

// ------------------------------------------------------------------------------------------------- F2

async fn balance_rows(publish: Option<bool>, amount: f64) -> (Record, usize) {
    let stub = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/anthropic/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"content": []})))
        .mount(&stub)
        .await;
    Mock::given(method("GET"))
        .and(path("/user/balance"))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            json!({"balance_infos": [{"currency": "CNY", "total_balance": format!("{amount}")}]}),
        ))
        .mount(&stub)
        .await;
    let switch = publish
        .map(|p| format!("publish_balance = {p}\n"))
        .unwrap_or_default();
    let extra = format!(
        "{switch}[service.balance]\nurl = \"{}/user/balance\"\npath = \"balance_infos[0].total_balance\"\ncurrency = \"CNY\"\nfloor = 150\n",
        stub.uri()
    );
    let cfg = config(&[service(
        "deepseek",
        "nusy-product-team",
        &format!("{}/anthropic", stub.uri()),
        "anthropic",
        "deepseek-v4-pro",
        &extra,
    )]);
    let rows = Runner::new(cfg, env(&[("QB_KEY", FAKE_SK)]))
        .run_once()
        .await;
    assert_eq!(rows.len(), 1);
    let gets = stub
        .received_requests()
        .await
        .unwrap()
        .iter()
        .filter(|q| q.method.as_str() == "GET")
        .count();
    (rows.into_iter().next().unwrap(), gets)
}

#[tokio::test]
async fn f2_publish_balance_false_puts_no_balance_on_the_row_but_the_floor_still_applies() {
    let (r, gets) = balance_rows(Some(false), 12.5).await;
    assert_eq!(gets, 1, "the probe still reads the balance");
    assert_eq!(
        r.state,
        State::QuotaExhausted,
        "the floor still applies to the state: {r:?}"
    );
    assert_eq!(
        r.balance, None,
        "publish_balance = false ⇒ no balance on the row"
    );
    let json = serde_json::to_string(&r).unwrap();
    assert!(
        !json.contains("12.5"),
        "the amount is nowhere in the row: {json}"
    );
}

#[tokio::test]
async fn f2_publish_balance_false_above_floor_is_ok_without_a_balance() {
    let (r, _) = balance_rows(Some(false), 500.0).await;
    assert_eq!(r.state, State::Ok);
    assert_eq!(r.balance, None);
}

#[tokio::test]
async fn control_f2_the_balance_is_published_by_default_and_when_true() {
    for publish in [None, Some(true)] {
        let (r, _) = balance_rows(publish, 12.5).await;
        assert_eq!(r.state, State::QuotaExhausted, "{publish:?}");
        assert_eq!(
            r.balance.as_ref().map(|b| b.amount),
            Some(12.5),
            "{publish:?}: {r:?}"
        );
    }
}

// ------------------------------------------------------------------------------------------------- F3, F4

fn services_toml() -> String {
    ["good", "bad"]
        .iter()
        .map(|id| {
            format!(
                "[[service]]\nid = {id:?}\nkind = \"api\"\nprovider = {id:?}\nfamily = {id:?}\naccount = \"acct\"\n\
                 models = [\"m1\"]\nsecret = \"QB_UNUSED\"\n"
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

const GOOD: &str = "api.good.acct.m1";
const BAD: &str = "api.bad.acct.m1";

fn good_row() -> Record {
    common::record(
        GOOD,
        State::Ok,
        Utc::now() - chrono::Duration::seconds(10),
        2700,
    )
}

fn json_entries(cfg: &Path) -> (i32, Vec<Value>, String) {
    let out = run_cli(cfg, &["status", "--json"], &[], T);
    let v: Value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("not JSON ({e}): {}", text(&out)));
    (
        rc(&out),
        v.as_array().cloned().unwrap_or_default(),
        text(&out),
    )
}

fn entry<'a>(rows: &'a [Value], key: &str) -> &'a Value {
    rows.iter()
        .find(|r| r["key"] == key)
        .unwrap_or_else(|| panic!("no {key} in {rows:?}"))
}

fn assert_unreadable_is_cannot_assess(cfg: &Path) {
    let out = run_cli(cfg, &["status", "--check", "bad"], &[], T);
    assert_eq!(
        rc(&out),
        2,
        "an unreadable row is CANNOT-ASSESS (rc 2), never UNKNOWN absent: {}",
        text(&out)
    );
    let (_, entries, all) = json_entries(cfg);
    let bad = entry(&entries, BAD);
    assert_eq!(bad["verdict"], "CANNOT-ASSESS", "{all}");
    assert_ne!(bad["reason"], "absent");
    assert!(
        bad["reason"]
            .as_str()
            .unwrap_or("")
            .starts_with("cannot_assess:"),
        "{bad}"
    );
}

fn file_fixture(dir: &Path) -> PathBuf {
    let state = dir.join("state");
    std::fs::create_dir(&state).unwrap();
    std::fs::write(
        state.join(format!("{GOOD}.json")),
        serde_json::to_vec(&good_row()).unwrap(),
    )
    .unwrap();
    std::fs::write(state.join(format!("{BAD}.json")), b"not json").unwrap();
    let cfg = dir.join("quotabus.toml");
    std::fs::write(
        &cfg,
        format!(
            "[file]\ndir = {:?}\n\n{}",
            state.display().to_string(),
            services_toml()
        ),
    )
    .unwrap();
    cfg
}

#[test]
fn f3_a_corrupt_row_file_reads_cannot_assess() {
    let dir = tempfile::tempdir().unwrap();
    assert_unreadable_is_cannot_assess(&file_fixture(dir.path()));
}

#[test]
fn control_f3_a_readable_row_beside_a_corrupt_one_still_reads_ok() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = file_fixture(dir.path());
    assert_eq!(
        rc(&run_cli(&cfg, &["status", "--check", "good"], &[], T)),
        0
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn f3_a_corrupt_row_on_the_bus_reads_cannot_assess() {
    let srv = NatsServer::start();
    let kv = NatsKv::connect_publisher(&srv.url(), "qb_r1_corrupt")
        .await
        .unwrap();
    kv.put(&good_row()).await.unwrap();
    let js = async_nats::jetstream::new(async_nats::connect(srv.url()).await.unwrap());
    js.get_key_value("qb_r1_corrupt")
        .await
        .unwrap()
        .put(BAD, "not json".into())
        .await
        .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let cfg = dir.path().join("quotabus.toml");
    std::fs::write(
        &cfg,
        format!(
            "[bus]\nurl = {:?}\nbucket = \"qb_r1_corrupt\"\n\n{}",
            srv.url(),
            services_toml()
        ),
    )
    .unwrap();
    tokio::task::spawn_blocking(move || {
        assert_unreadable_is_cannot_assess(&cfg);
        assert_eq!(
            rc(&run_cli(&cfg, &["status", "--check", "good"], &[], T)),
            0,
            "control: the good row reads ok"
        );
    })
    .await
    .unwrap();
}

/// A throwaway nats-server with two users: `admin` (everything, used by the test to set up) and `lister`, the
/// identity of any connection WITHOUT credentials (`no_auth_user`), which is how the CLI connects. When `deny_reads`,
/// lister can read stream info and create consumers (list keys) but its KV value reads (direct get / msg get) are
/// DENIED: a bus read that fails part-way through a listing. Without it, lister may do everything (the control).
struct RestrictedNats {
    child: Child,
    port: u16,
    _dir: tempfile::TempDir,
}

impl RestrictedNats {
    fn start(deny_reads: bool) -> RestrictedNats {
        common::require_nats_server();
        let dir = tempfile::tempdir().unwrap();
        let deny = if deny_reads {
            r#", deny: ["$JS.API.DIRECT.GET.>", "$JS.API.STREAM.MSG.GET.>"]"#
        } else {
            ""
        };
        let conf = dir.path().join("nats.conf");
        std::fs::write(
            &conf,
            format!(
                "jetstream {{ store_dir: {:?} }}\nno_auth_user: lister\n\
                 authorization {{ users = [\n\
                   {{ user: admin, password: adminpw }}\n\
                   {{ user: lister, password: listerpw, permissions: {{\n\
                       publish: {{ allow: [\">\"]{deny} }}\n\
                       subscribe: {{ allow: [\">\"] }} }} }}\n\
                 ] }}\n",
                dir.path().join("js").display().to_string()
            ),
        )
        .unwrap();
        let mut cmd = Command::new("nats-server");
        cmd.arg("-c").arg(&conf);
        let (child, port) = common::spawn_nats(cmd, "restricted", dir.path());
        RestrictedNats {
            child,
            port,
            _dir: dir,
        }
    }

    fn url(&self) -> String {
        format!("nats://127.0.0.1:{}", self.port)
    }

    /// As admin: a per-key-TTL bucket holding a good row for both services.
    async fn seed(&self, bucket: &str) {
        let client =
            async_nats::ConnectOptions::with_user_and_password("admin".into(), "adminpw".into())
                .connect(self.url())
                .await
                .expect("admin connects");
        let js = async_nats::jetstream::new(client);
        let kv = js
            .create_key_value(async_nats::jetstream::kv::Config {
                bucket: bucket.into(),
                history: 1,
                limit_markers: Some(Duration::from_secs(60)),
                ..Default::default()
            })
            .await
            .unwrap();
        let mut bad = good_row();
        bad.key = BAD.into();
        bad.provider = "bad".into();
        for r in [good_row(), bad] {
            kv.put(&r.key, serde_json::to_vec(&r).unwrap().into())
                .await
                .unwrap();
        }
    }
}

impl Drop for RestrictedNats {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn bus_cfg(dir: &Path, url: &str, bucket: &str) -> PathBuf {
    let p = dir.join("quotabus.toml");
    std::fs::write(
        &p,
        format!(
            "[bus]\nurl = {url:?}\nbucket = {bucket:?}\n\n{}",
            services_toml()
        ),
    )
    .unwrap();
    p
}

#[tokio::test(flavor = "multi_thread")]
async fn control_f3_the_same_bus_without_the_read_deny_reads_ok() {
    let srv = RestrictedNats::start(false);
    srv.seed("qb_r1_partial").await;
    let dir = tempfile::tempdir().unwrap();
    let cfg = bus_cfg(dir.path(), &srv.url(), "qb_r1_partial");
    let out =
        tokio::task::spawn_blocking(move || run_cli(&cfg, &["status", "--check", "bad"], &[], T))
            .await
            .unwrap();
    assert_eq!(
        rc(&out),
        0,
        "the fixture is sound; only the deny makes the read fail: {}",
        text(&out)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn f3_a_value_read_that_fails_while_listing_reads_cannot_assess_not_absent() {
    let srv = RestrictedNats::start(true);
    srv.seed("qb_r1_partial").await;
    let dir = tempfile::tempdir().unwrap();
    let cfg = bus_cfg(dir.path(), &srv.url(), "qb_r1_partial");
    tokio::task::spawn_blocking(move || {
        let out = run_cli(&cfg, &["status", "--check", "bad"], &[], T);
        assert_eq!(
            rc(&out),
            2,
            "a failed read is CANNOT-ASSESS rc 2: {}",
            text(&out)
        );
        let out = run_cli(&cfg, &["status", "--json"], &[], T);
        let s = text(&out);
        assert!(
            !s.contains("\"absent\""),
            "a failed read is not `absent`: {s}"
        );
        assert!(
            !s.contains("\"verdict\":\"ok\"") && !s.contains("\"verdict\": \"ok\""),
            "{s}"
        );
        assert_eq!(rc(&out), 2, "{s}");
    })
    .await
    .unwrap();
}

#[test]
fn f4_a_missing_file_backend_dir_is_cannot_assess_rc_2_naming_the_dir() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("no-such-rows-dir");
    let cfg = dir.path().join("quotabus.toml");
    std::fs::write(
        &cfg,
        format!(
            "[file]\ndir = {:?}\n\n{}",
            missing.display().to_string(),
            services_toml()
        ),
    )
    .unwrap();
    for args in [
        &["status"][..],
        &["status", "--check", "good"][..],
        &["status", "--json"][..],
    ] {
        let out = run_cli(&cfg, args, &[], T);
        assert_eq!(rc(&out), 2, "{args:?}: {}", text(&out));
        assert!(
            text(&out).contains("CANNOT-ASSESS"),
            "{args:?}: {}",
            text(&out)
        );
        assert!(
            text(&out).contains("no-such-rows-dir"),
            "{args:?} names the missing dir: {}",
            text(&out)
        );
    }
    assert!(!missing.exists(), "a reader does not create the directory");
}

#[test]
fn control_f4_an_existing_empty_dir_is_unknown_absent_and_a_probe_creates_a_missing_one() {
    let dir = tempfile::tempdir().unwrap();
    let rows = dir.path().join("rows");
    let cfg = dir.path().join("quotabus.toml");
    std::fs::write(
        &cfg,
        format!(
            "[file]\ndir = {:?}\n\n{}",
            rows.display().to_string(),
            services_toml()
        ),
    )
    .unwrap();
    // a probe (secret unset ⇒ cannot_assess rows) creates the directory when it writes
    let out = run_cli(&cfg, &["probe"], &[], T);
    assert_eq!(rc(&out), 0, "{}", text(&out));
    assert!(rows.is_dir(), "the probe creates the row directory");
    for f in std::fs::read_dir(&rows).unwrap().flatten() {
        std::fs::remove_file(f.path()).unwrap();
    }
    // an existing, empty directory is "never probed": UNKNOWN absent, rc 3 (not CANNOT-ASSESS)
    assert_eq!(
        rc(&run_cli(&cfg, &["status", "--check", "good"], &[], T)),
        3
    );
}
