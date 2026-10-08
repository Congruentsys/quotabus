//! CHORE-007 DoD 2-3: a probe run makes a kind's calls only when that kind is DUE (its own interval has elapsed since
//! that kind's last row in the store; no row = due); a due API probe never reads the balance and vice versa;
//! `force` runs every kind; each row's `ttl_s` is its own kind's TTL. A fake clock (`now`) and a file-backend store
//! drive `Runner::run_due`; a loopback stub counts the calls. No real provider, no real key.

mod common;

use chrono::{DateTime, Duration, TimeZone, Utc};
use common::FAKE_SK;
use quotabus::{Backend, Config, FileBackend, Kind, Record, Runner, record_key};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const MESSAGES: &str = "/ds/v1/messages";
const BALANCE: &str = "/bal";

/// A fake clock far from the real one: rows stamped with the real time would read as years old.
fn t0() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2031, 1, 1, 0, 0, 0).unwrap()
}

fn api_key() -> String {
    record_key(Kind::Api, "deepseek", "nusy-product-team", "m1")
}

async fn stub() -> MockServer {
    let s = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(MESSAGES))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"content": [{"type": "text", "text": "ok"}]})),
        )
        .mount(&s)
        .await;
    Mock::given(method("GET"))
        .and(path(BALANCE))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"total": 500})))
        .mount(&s)
        .await;
    s
}

fn config(stub: &str, tables: &str) -> Config {
    let text = format!(
        "{tables}\n\n\
         [[service]]\nid = \"deepseek\"\nkind = \"api\"\nprovider = \"deepseek\"\nfamily = \"deepseek\"\n\
         account = \"nusy-product-team\"\nbase_url = \"{stub}/ds\"\nprotocol = \"anthropic\"\nmodels = [\"m1\"]\n\
         secret = \"QB_DS\"\n\
         [service.balance]\nurl = \"{stub}{BALANCE}\"\npath = \"total\"\ncurrency = \"CNY\"\nfloor = 0\n"
    );
    Config::from_toml_str(&text).unwrap_or_else(|e| panic!("test config must parse: {e}\n{text}"))
}

fn runner(cfg: Config) -> Runner {
    Runner::new(cfg, |n: &str| (n == "QB_DS").then(|| FAKE_SK.to_string()))
}

/// (messages calls, balance reads) the stub has seen.
async fn counts(s: &MockServer) -> (usize, usize) {
    let reqs = s.received_requests().await.unwrap();
    let n = |p: &str| reqs.iter().filter(|r| r.url.path() == p).count();
    (n(MESSAGES), n(BALANCE))
}

struct Store {
    _dir: tempfile::TempDir,
    fb: FileBackend,
}

fn store() -> Store {
    let dir = tempfile::tempdir().unwrap();
    let fb = FileBackend::new(dir.path().join("rows"));
    std::fs::create_dir_all(&fb.dir).unwrap();
    Store { _dir: dir, fb }
}

/// One tick: read the store, run what is due at `now`, write the rows back (as `quotabus probe` does).
async fn tick(r: &Runner, st: &Store, now: DateTime<Utc>, force: bool) -> Vec<Record> {
    let existing = st.fb.list().await.unwrap();
    let rows = r.run_due(&existing, now, force).await;
    for row in &rows {
        assert_eq!(
            row.checked_at, now,
            "a row is stamped with the injected clock: {row:?}"
        );
        st.fb.put(row).await.unwrap();
    }
    rows
}

const STEP_MIN: i64 = 25;
const LAST_MIN: i64 = 400;

/// Ticks every 25 min over 0..=400 min; the minutes at which each kind made a call, and the rows of each tick.
async fn simulate(tables: &str) -> (Vec<i64>, Vec<i64>, Vec<(i64, Vec<Record>)>) {
    let s = stub().await;
    let r = runner(config(&s.uri(), tables));
    let st = store();
    let (mut api, mut bal, mut all) = (Vec::new(), Vec::new(), Vec::new());
    let mut before = counts(&s).await;
    let mut m = 0;
    while m <= LAST_MIN {
        let rows = tick(&r, &st, t0() + Duration::minutes(m), false).await;
        let after = counts(&s).await;
        assert!(
            after.0 - before.0 <= 1 && after.1 - before.1 <= 1,
            "at most one call per kind per tick (one model): {before:?} -> {after:?} at {m} min"
        );
        if after.0 > before.0 {
            api.push(m);
        }
        if after.1 > before.1 {
            bal.push(m);
        }
        all.push((m, rows));
        before = after;
        m += STEP_MIN;
    }
    (api, bal, all)
}

/// The reference schedule for ONE kind with interval `every` minutes: due when no run yet, or `every` has elapsed
/// since its own last run. (The 25-min step never lands exactly on 60 or 180, so `>=` vs `>` cannot matter.)
fn reference(every: i64) -> Vec<i64> {
    let mut out = Vec::new();
    let mut last: Option<i64> = None;
    let mut m = 0;
    while m <= LAST_MIN {
        if last.is_none_or(|l| m - l >= every) {
            out.push(m);
            last = Some(m);
        }
        m += STEP_MIN;
    }
    out
}

#[test]
fn control_the_reference_schedule_is_a_known_answer_and_tells_the_intervals_apart() {
    assert_eq!(reference(60), [0, 75, 150, 225, 300, 375]);
    assert_eq!(reference(180), [0, 200, 400]);
    // the control that fails if the intervals are shared: one shared interval gives both kinds ONE schedule, which
    // cannot equal both of these at once
    assert_ne!(reference(60), reference(180));
}

#[tokio::test(flavor = "multi_thread")]
async fn each_kind_runs_on_its_own_interval() {
    let (api, bal, _) = simulate("[intervals]\napi = \"1h\"\nbalance = \"3h\"\n").await;
    assert_eq!(api, reference(60), "api runs every 1h of its own");
    assert_eq!(bal, reference(180), "balance runs every 3h of its own");
}

#[tokio::test(flavor = "multi_thread")]
async fn swapped_intervals_swap_the_schedules() {
    // mutation guard: a runner that reads the balance on the API interval (or the reverse) passes at most one of
    // this test and the one above
    let (api, bal, _) = simulate("[intervals]\napi = \"3h\"\nbalance = \"1h\"\n").await;
    assert_eq!(api, reference(180));
    assert_eq!(bal, reference(60));
}

#[tokio::test(flavor = "multi_thread")]
async fn an_empty_store_makes_every_kind_due() {
    let s = stub().await;
    let r = runner(config(
        &s.uri(),
        "[intervals]\napi = \"1h\"\nbalance = \"3h\"\n",
    ));
    let st = store();
    assert_eq!(counts(&s).await, (0, 0));
    tick(&r, &st, t0(), false).await;
    assert_eq!(counts(&s).await, (1, 1), "no row = due, for each kind");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_due_api_probe_never_reads_the_balance() {
    let s = stub().await;
    let r = runner(config(
        &s.uri(),
        "[intervals]\napi = \"1h\"\nbalance = \"3h\"\n",
    ));
    let st = store();
    tick(&r, &st, t0(), false).await;
    assert_eq!(counts(&s).await, (1, 1));
    tick(&r, &st, t0() + Duration::minutes(90), false).await;
    assert_eq!(
        counts(&s).await,
        (2, 1),
        "at 90 min only the api probe is due"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_due_balance_read_never_calls_the_model() {
    let s = stub().await;
    let r = runner(config(
        &s.uri(),
        "[intervals]\napi = \"3h\"\nbalance = \"1h\"\n",
    ));
    let st = store();
    tick(&r, &st, t0(), false).await;
    assert_eq!(counts(&s).await, (1, 1));
    tick(&r, &st, t0() + Duration::minutes(90), false).await;
    assert_eq!(
        counts(&s).await,
        (1, 2),
        "at 90 min only the balance read is due"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn force_runs_every_kind_even_when_none_is_due() {
    let s = stub().await;
    let r = runner(config(
        &s.uri(),
        "[intervals]\napi = \"1h\"\nbalance = \"3h\"\n",
    ));
    let st = store();
    tick(&r, &st, t0(), false).await;
    assert_eq!(counts(&s).await, (1, 1));
    // control: one minute later nothing is due, so an unforced run calls nothing
    let rows = tick(&r, &st, t0() + Duration::minutes(1), false).await;
    assert_eq!(counts(&s).await, (1, 1), "nothing is due after 1 min");
    assert!(rows.is_empty(), "nothing due ⇒ no rows: {rows:?}");
    tick(&r, &st, t0() + Duration::minutes(2), true).await;
    assert_eq!(counts(&s).await, (2, 2), "force runs every kind");
}

#[tokio::test(flavor = "multi_thread")]
async fn each_row_carries_its_own_kinds_ttl() {
    // default TTLs: 3 × own interval ⇒ api 3h = 10800 s, balance 9h = 32400 s
    let (_, _, all) = simulate("[intervals]\napi = \"1h\"\nbalance = \"3h\"\n").await;
    let at = |m: i64| &all.iter().find(|(t, _)| *t == m).unwrap().1;

    let api_only = at(75);
    assert!(!api_only.is_empty(), "the api-only tick writes rows");
    for r in api_only {
        assert_eq!(r.ttl_s, 3 * 3600, "an api row carries the api TTL: {r:?}");
    }
    let balance_only = at(200);
    assert!(
        !balance_only.is_empty(),
        "the balance-only tick writes a row: the store must hold the balance's last read"
    );
    for r in balance_only {
        assert_eq!(
            r.ttl_s,
            9 * 3600,
            "a balance row carries the balance TTL: {r:?}"
        );
    }
    let mut both: Vec<u64> = at(0).iter().map(|r| r.ttl_s).collect();
    both.sort();
    both.dedup();
    assert_eq!(
        both,
        [3 * 3600, 9 * 3600],
        "tick 0 writes rows of both kinds"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_overridden_ttl_is_the_rows_ttl_per_kind() {
    let s = stub().await;
    let r = runner(config(
        &s.uri(),
        "[intervals]\napi = \"1h\"\nbalance = \"3h\"\n\n[ttl]\napi = \"5h\"\nbalance = \"7h\"\n",
    ));
    let st = store();
    let rows = tick(&r, &st, t0(), true).await;
    let api: Vec<&Record> = rows.iter().filter(|r| r.key == api_key()).collect();
    let other: Vec<&Record> = rows.iter().filter(|r| r.key != api_key()).collect();
    assert_eq!(api.len(), 1, "one api row for (deepseek, m1): {rows:?}");
    assert_eq!(api[0].ttl_s, 5 * 3600, "[ttl] api");
    assert!(
        !other.is_empty(),
        "the balance read writes its own row: {rows:?}"
    );
    for r in other {
        assert_eq!(r.ttl_s, 7 * 3600, "[ttl] balance: {r:?}");
    }
}
