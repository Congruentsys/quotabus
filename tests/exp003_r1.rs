//! EXP-003 review r1 (`reviews/EXP-003-r1.md`), ruled tests.
//!
//! **F1:** a `kind = "subscription"` row is never a select candidate, even healthy, fresh and listing the role (a
//! Copilot seat with `roles = ["review"]`, `ok`): pending a Captain ruling a seat has no model, so line 1 could not be
//! `provider model`. Tested at the library and at the CLI under every `--prefer` (and none). Control: the same seat
//! filed as `kind = "api"` with a model IS chosen.
//!
//! **F4:** a failed change-subject announce never loses the row. The announce is made to fail deterministically
//! WITHOUT a src seam: the publisher connects as a user whose NATS permissions deny publish on `ai.status.changed.>`
//! (the server rejects the message; the KV write is allowed). Control: the same writes as an unrestricted user ARE
//! announced, so the subscriber can see an announce and its silence under the restricted user is the denial. Not
//! covered: a flush that HANGS (r1's timeout fix) — that needs a server that stalls mid-connection, which a throwaway
//! nats-server cannot do deterministically without a src seam.

mod common;

use std::process::{Child, Command};
use std::time::Duration;

use chrono::{DateTime, Utc};
use common::{rc, run_cli, text};
use futures::StreamExt;
use quotabus::{Backend, Config, Kind, NatsKv, Prefer, Query, Record, State, record_key, select};

const T: Duration = Duration::from_secs(30);

// ---------------------------------------------------------------- F1

/// Two review models and the Copilot seat. `seat_kind` files the seat as `subscription` (no models) or, for the
/// control, as `api` with the model `gpt-5`. The seat is the fastest, cheapest-ranked and largest-context, so under
/// every `--prefer` it would be first if it were a candidate.
fn table(state_dir: &str, seat_kind: &str) -> String {
    let seat = if seat_kind == "subscription" {
        "[[service]]\nid = \"copilot\"\nkind = \"subscription\"\nprovider = \"github\"\nfamily = \"openai\"\n\
         account = \"acct\"\nroles = [\"review\"]\ncost_class = \"local\"\nsecret = \"QB_UNUSED\"\n\
         sources = [\"copilot_internal\"]\n[service.context]\n\"copilot\" = 9000000\n"
            .to_string()
    } else {
        "[[service]]\nid = \"copilot\"\nkind = \"api\"\nprovider = \"github\"\nfamily = \"openai\"\n\
         account = \"acct\"\nmodels = [\"gpt-5\"]\nroles = [\"review\"]\ncost_class = \"local\"\n\
         secret = \"QB_UNUSED\"\n[service.context]\n\"gpt-5\" = 9000000\n"
            .to_string()
    };
    format!(
        "[file]\ndir = {state_dir:?}\n\n\
         [[service]]\nid = \"glm\"\nkind = \"api\"\nprovider = \"zhipu\"\nfamily = \"zhipu\"\naccount = \"acct\"\n\
         models = [\"glm-5.3\"]\nroles = [\"review\"]\ncost_class = \"metered\"\nsecret = \"QB_UNUSED\"\n\
         [service.context]\n\"glm-5.3\" = 200000\n\n\
         [[service]]\nid = \"deepseek\"\nkind = \"api\"\nprovider = \"deepseek\"\nfamily = \"deepseek\"\n\
         account = \"acct\"\nmodels = [\"deepseek-v4-flash\"]\nroles = [\"review\"]\ncost_class = \"metered\"\n\
         secret = \"QB_UNUSED\"\n[service.context]\n\"deepseek-v4-flash\" = 64000\n\n{seat}"
    )
}

fn row(
    kind: Kind,
    provider: &str,
    family: &str,
    model: &str,
    latency: u64,
    now: DateTime<Utc>,
) -> Record {
    let mut r = common::record(
        &record_key(kind, provider, "acct", model),
        State::Ok,
        now - chrono::Duration::seconds(30),
        3600,
    );
    r.model = model.to_string();
    r.family = family.to_string();
    r.latency_ms = Some(latency);
    r
}

/// Every row fresh and `ok`; the seat (5 ms) is the fastest.
fn rows(seat_kind: &str, now: DateTime<Utc>) -> Vec<Record> {
    let seat = if seat_kind == "subscription" {
        row(Kind::Subscription, "github", "openai", "copilot", 5, now)
    } else {
        row(Kind::Api, "github", "openai", "gpt-5", 5, now)
    };
    vec![
        row(Kind::Api, "zhipu", "zhipu", "glm-5.3", 400, now),
        row(
            Kind::Api,
            "deepseek",
            "deepseek",
            "deepseek-v4-flash",
            200,
            now,
        ),
        seat,
    ]
}

const PREFERS: &[(Option<&str>, Option<Prefer>)] = &[
    (None, None),
    (Some("fastest"), Some(Prefer::Fastest)),
    (Some("cheapest"), Some(Prefer::Cheapest)),
    (Some("largest-context"), Some(Prefer::LargestContext)),
];

fn query(prefer: Option<Prefer>, now: DateTime<Utc>) -> Query {
    Query {
        role: "review".into(),
        exclude_families: vec!["anthropic".into()],
        prefer,
        now,
    }
}

#[test]
fn f1_a_healthy_subscription_seat_with_the_role_is_never_a_library_candidate() {
    let now = Utc::now();
    let config = Config::from_toml_str(&table("/nonexistent/qb-r1", "subscription")).unwrap();
    let rows = rows("subscription", now);
    for (word, prefer) in PREFERS {
        let got = select(&rows, &query(*prefer, now), &config);
        assert!(
            got.iter()
                .all(|c| c.service != "copilot" && c.key != "subscription.github.acct.copilot"),
            "--prefer {word:?}: the subscription seat is a candidate: {got:?}"
        );
        assert_eq!(
            got.len(),
            2,
            "--prefer {word:?}: the two API models remain: {got:?}"
        );
    }
}

#[test]
fn f1_control_the_same_seat_filed_as_an_api_model_is_chosen() {
    let now = Utc::now();
    let config = Config::from_toml_str(&table("/nonexistent/qb-r1", "api")).unwrap();
    let rows = rows("api", now);
    for (word, prefer) in PREFERS {
        let got = select(&rows, &query(*prefer, now), &config);
        assert!(
            got.iter()
                .any(|c| c.provider == "github" && c.model == "gpt-5"),
            "--prefer {word:?}: the api seat is not chosen: {got:?}"
        );
    }
    let fastest = select(&rows, &query(Some(Prefer::Fastest), now), &config);
    assert_eq!(
        fastest.first().map(|c| c.model.as_str()),
        Some("gpt-5"),
        "5 ms is the fastest"
    );
}

fn cli_fixture(seat_kind: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    std::fs::create_dir(&state).unwrap();
    for r in rows(seat_kind, Utc::now()) {
        std::fs::write(
            state.join(format!("{}.json", r.key)),
            serde_json::to_vec(&r).unwrap(),
        )
        .unwrap();
    }
    let config = dir.path().join("quotabus.toml");
    std::fs::write(&config, table(&state.display().to_string(), seat_kind)).unwrap();
    (dir, config)
}

fn cli_args(word: Option<&str>, json: bool) -> Vec<&str> {
    let mut a = vec![
        "select",
        "--role",
        "review",
        "--exclude-family",
        "anthropic",
    ];
    if let Some(w) = word {
        a.extend(["--prefer", w]);
    }
    if json {
        a.extend(["--n", "10", "--json"]);
    }
    a
}

#[test]
fn f1_the_cli_never_prints_a_subscription_seat() {
    let (_dir, config) = cli_fixture("subscription");
    for (word, _) in PREFERS {
        let o = run_cli(&config, &cli_args(*word, false), &[], T);
        assert_eq!(rc(&o), 0, "--prefer {word:?}: {}", text(&o));
        let line1 = String::from_utf8_lossy(&o.stdout)
            .lines()
            .next()
            .unwrap_or_default()
            .to_string();
        assert!(
            line1 == "deepseek deepseek-v4-flash" || line1 == "zhipu glm-5.3",
            "--prefer {word:?}: line 1 is {line1:?}, not an API model"
        );
        let j = run_cli(&config, &cli_args(*word, true), &[], T);
        assert_eq!(rc(&j), 0, "--prefer {word:?} --json: {}", text(&j));
        let v: serde_json::Value = serde_json::from_slice(&j.stdout).unwrap();
        let arr = v.as_array().expect("a JSON array");
        assert!(
            arr.iter()
                .all(|c| c["provider"] != "github" && c["model"] != "copilot"),
            "--prefer {word:?} --json lists the seat: {v}"
        );
        assert_eq!(arr.len(), 2, "--prefer {word:?}: {v}");
    }
}

#[test]
fn f1_control_the_cli_prints_the_api_seat_first_under_fastest() {
    let (_dir, config) = cli_fixture("api");
    let o = run_cli(&config, &cli_args(Some("fastest"), false), &[], T);
    assert_eq!(rc(&o), 0, "{}", text(&o));
    assert_eq!(String::from_utf8_lossy(&o.stdout), "github gpt-5\n");
}

// ---------------------------------------------------------------- F4

/// A loopback JetStream nats-server with two users: `admin` (everything) and `writer`, who may write KV but whose
/// publish on `ai.status.changed.>` the server denies.
struct DenyingNats {
    child: Child,
    port: u16,
    _dir: tempfile::TempDir,
}

impl DenyingNats {
    fn start() -> DenyingNats {
        common::require_nats_server();
        let dir = tempfile::tempdir().unwrap();
        let conf = dir.path().join("nats.conf");
        std::fs::write(
            &conf,
            format!(
                "jetstream {{ store_dir: {:?} }}\n\
                 authorization {{ users = [\n\
                   {{ user: admin, password: adminpw }}\n\
                   {{ user: writer, password: writerpw, permissions: {{\n\
                       publish: {{ allow: [\">\"], deny: [\"ai.status.changed.>\"] }}\n\
                       subscribe: {{ allow: [\">\"] }} }} }}\n\
                 ] }}\n",
                dir.path().join("js").display().to_string()
            ),
        )
        .unwrap();
        let mut cmd = Command::new("nats-server");
        cmd.arg("-c").arg(&conf);
        let (child, port) = common::spawn_nats(cmd, "denying", dir.path());
        DenyingNats {
            child,
            port,
            _dir: dir,
        }
    }

    fn url(&self, user: &str, pass: &str) -> String {
        format!("nats://{user}:{pass}@127.0.0.1:{}", self.port)
    }
}

impl Drop for DenyingNats {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

async fn drain(sub: &mut async_nats::Subscriber, wait: Duration) -> Vec<async_nats::Message> {
    let mut got = Vec::new();
    let deadline = tokio::time::Instant::now() + wait;
    while let Ok(Some(m)) = tokio::time::timeout_at(deadline, sub.next()).await {
        got.push(m);
    }
    got
}

fn rec(key: &str, state: State) -> Record {
    common::record(key, state, Utc::now(), 600)
}

/// Write ok then quota_exhausted for `key` as (user, pass); return what the admin subscriber saw, after checking
/// that each put returned Ok and the row reads back with its new state.
async fn write_two_states(
    srv: &DenyingNats,
    user: &str,
    pass: &str,
    bucket: &str,
    key: &str,
) -> usize {
    let admin =
        async_nats::ConnectOptions::with_user_and_password("admin".into(), "adminpw".into())
            .connect(format!("nats://127.0.0.1:{}", srv.port))
            .await
            .expect("admin connects");
    let mut sub = admin.subscribe("ai.status.changed.>").await.unwrap();
    admin.flush().await.unwrap();
    let kv = tokio::time::timeout(T, NatsKv::connect_publisher(&srv.url(user, pass), bucket))
        .await
        .unwrap()
        .unwrap_or_else(|e| panic!("{user} connects as publisher: {e}"));
    for state in [State::Ok, State::QuotaExhausted] {
        let r = rec(key, state);
        tokio::time::timeout(T, kv.put(&r))
            .await
            .expect("put returns promptly")
            .unwrap_or_else(|e| panic!("{user}: put {} failed: {e}", state.as_str()));
        let back = kv.get(key).await.unwrap();
        assert_eq!(
            back.as_ref().map(|b| b.state),
            Some(state),
            "{user}: the row after the put"
        );
        assert_eq!(back.as_ref(), Some(&r), "{user}: the row round-trips");
    }
    drain(&mut sub, Duration::from_secs(2)).await.len()
}

#[tokio::test(flavor = "multi_thread")]
async fn f4_a_denied_announce_never_loses_the_row() {
    let srv = DenyingNats::start();
    let seen = write_two_states(
        &srv,
        "writer",
        "writerpw",
        "qb_r1_denied",
        "api.zhipu.acct.glm-5-3",
    )
    .await;
    assert_eq!(
        seen, 0,
        "the server must have denied every announce (else this test proves nothing)"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn f4_control_the_same_writes_unrestricted_are_announced() {
    let srv = DenyingNats::start();
    let seen = write_two_states(
        &srv,
        "admin",
        "adminpw",
        "qb_r1_allowed",
        "api.zhipu.acct.glm-5-3",
    )
    .await;
    assert!(
        seen >= 1,
        "the ok -> quota_exhausted change must be announced to an unrestricted publisher"
    );
}
