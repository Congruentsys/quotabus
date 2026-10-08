//! EXP-003 / DESIGN §3 "selector", §2 (the freshness rule), §9 row E3: `quotabus::select(&rows, &Query, &Config)` is a
//! PURE function over rows plus the static model table in the config (family, cost class, roles, context) — these
//! tests start no bus and touch no file. DoD: the selector refuses a family the author uses; nothing qualifies when
//! every candidate is `unknown` (the CLI turns that into rc 3, `tests/exp003_cli_select.rs`).
//!
//! Seams asserted (EXP-003 test partner): `quotabus::{select, Query, Prefer, Candidate}`; `Query { role,
//! exclude_families, prefer, now }`; `Candidate { service, provider, model, family, key, reason }`, best first; a
//! service's per-model context is `[service.context]` (`"<model>" = <tokens>`). Only a fresh `ok` row qualifies:
//! the design calls the pick "a healthy model" and says nothing that admits `degraded`.

mod common;

use chrono::{DateTime, Duration, Utc};
use quotabus::{Candidate, Config, Kind, Prefer, Query, Record, State, record_key, select};

/// The static model table: five API/local services and two subscription accounts.
const TABLE: &str = r#"
[file]
dir = "/nonexistent/quotabus-exp003-unit"

[[service]]
id = "glm"
kind = "api"
provider = "zhipu"
family = "zhipu"
account = "acct"
models = ["glm-5.3"]
roles = ["review", "work"]
cost_class = "metered"
secret = "QB_UNUSED"
[service.context]
"glm-5.3" = 200000

[[service]]
id = "deepseek"
kind = "api"
provider = "deepseek"
family = "deepseek"
account = "acct"
models = ["deepseek-v4-pro", "deepseek-v4-flash"]
roles = ["review"]
cost_class = "metered"
secret = "QB_UNUSED"
[service.context]
"deepseek-v4-pro" = 128000
"deepseek-v4-flash" = 64000

[[service]]
id = "claude-api"
kind = "api"
provider = "anthropic"
family = "anthropic"
account = "acct"
models = ["claude-opus"]
roles = ["review"]
cost_class = "metered"
secret = "QB_UNUSED"
[service.context]
"claude-opus" = 1000000

[[service]]
id = "qwen"
kind = "local"
provider = "qwen"
family = "qwen"
account = "acct"
models = ["qwen3"]
roles = ["review"]
cost_class = "local"
secret = "QB_UNUSED"
[service.context]
"qwen3" = 32000

[[service]]
id = "kimi"
kind = "api"
provider = "moonshot"
family = "moonshot"
account = "acct"
models = ["kimi-k3"]
roles = ["work"]
cost_class = "metered"
secret = "QB_UNUSED"
[service.context]
"kimi-k3" = 256000

[[service]]
id = "claude-hankh95"
kind = "subscription"
provider = "anthropic"
family = "anthropic"
account = "acct"
models = ["claude-haiku-4-5"]
secret = "QB_UNUSED"

[[service]]
id = "copilot"
kind = "subscription"
provider = "github"
family = "openai"
account = "acct"
roles = ["review"]
secret = "QB_UNUSED"
"#;

fn config() -> Config {
    Config::from_toml_str(TABLE).expect("the static model table parses")
}

/// (kind, provider, model slot, latency ms): every configured slot.
const SLOTS: &[(Kind, &str, &str, u64)] = &[
    (Kind::Api, "zhipu", "glm-5.3", 400),
    (Kind::Api, "deepseek", "deepseek-v4-pro", 300),
    (Kind::Api, "deepseek", "deepseek-v4-flash", 200),
    (Kind::Api, "anthropic", "claude-opus", 50),
    (Kind::Local, "qwen", "qwen3", 900),
    (Kind::Api, "moonshot", "kimi-k3", 100),
    (Kind::Subscription, "anthropic", "claude-hankh95", 80),
    (Kind::Subscription, "github", "copilot", 90),
];

fn row_at(
    kind: Kind,
    provider: &str,
    model: &str,
    state: State,
    checked_at: DateTime<Utc>,
    ttl_s: u64,
    latency: u64,
) -> Record {
    let key = record_key(kind, provider, "acct", model);
    let mut r = common::record(&key, state, checked_at, ttl_s);
    r.model = model.to_string();
    r.family = match provider {
        "github" => "openai".to_string(),
        p => p.to_string(),
    };
    r.latency_ms = Some(latency);
    r
}

/// Every slot fresh (`checked_at` 30 s ago, ttl 1 h) and `ok`, with its latency — except the Copilot seat, which is
/// `quota_exhausted`: whether a healthy subscription seat is a review candidate is not settled by the design, so no
/// known answer here depends on it.
fn all_ok(now: DateTime<Utc>) -> Vec<Record> {
    let mut rows: Vec<Record> = SLOTS
        .iter()
        .map(|(k, p, m, l)| row_at(*k, p, m, State::Ok, now - Duration::seconds(30), 3600, *l))
        .collect();
    set_state(&mut rows, "copilot", State::QuotaExhausted);
    rows
}

fn set_state(rows: &mut [Record], model: &str, state: State) {
    let r = rows
        .iter_mut()
        .find(|r| r.model == model)
        .expect("a fixture row");
    r.state = state;
    r.reason = (state == State::Unknown).then(|| "cannot_assess:unreachable".to_string());
}

fn query(role: &str, exclude: &[&str], prefer: Option<Prefer>, now: DateTime<Utc>) -> Query {
    Query {
        role: role.to_string(),
        exclude_families: exclude.iter().map(|s| s.to_string()).collect(),
        prefer,
        now,
    }
}

fn models(c: &[Candidate]) -> Vec<&str> {
    c.iter().map(|c| c.model.as_str()).collect()
}

// ---- DoD: the selector refuses a family the author uses ----

#[test]
fn an_excluded_family_is_never_chosen() {
    let now = Utc::now();
    let rows = all_ok(now);
    let got = select(
        &rows,
        &query("review", &["anthropic"], Some(Prefer::Fastest), now),
        &config(),
    );
    assert!(
        got.iter().all(|c| c.family != "anthropic"),
        "anthropic chosen: {got:?}"
    );
    // known answer: claude-opus (50 ms) is out, so the fastest review model is deepseek-v4-flash (200 ms)
    assert_eq!(
        got.first().map(|c| (c.provider.as_str(), c.model.as_str())),
        Some(("deepseek", "deepseek-v4-flash"))
    );
}

#[test]
fn control_without_the_exclusion_the_anthropic_model_wins() {
    // the exclusion test above can fail: with nothing excluded, the 50 ms anthropic model is first
    let now = Utc::now();
    let got = select(
        &all_ok(now),
        &query("review", &[], Some(Prefer::Fastest), now),
        &config(),
    );
    assert_eq!(
        got.first().map(|c| c.model.as_str()),
        Some("claude-opus"),
        "{got:?}"
    );
    assert_eq!(got.first().map(|c| c.family.as_str()), Some("anthropic"));
}

#[test]
fn several_excluded_families_are_all_refused() {
    let now = Utc::now();
    let got = select(
        &all_ok(now),
        &query(
            "review",
            &["anthropic", "deepseek"],
            Some(Prefer::Fastest),
            now,
        ),
        &config(),
    );
    assert!(
        got.iter()
            .all(|c| c.family != "anthropic" && c.family != "deepseek"),
        "{got:?}"
    );
    assert_eq!(
        models(&got),
        ["glm-5.3", "qwen3"],
        "the review models left, fastest first"
    );
}

#[test]
fn only_the_author_family_ok_means_nothing_qualifies() {
    let now = Utc::now();
    let mut rows = all_ok(now);
    for m in [
        "glm-5.3",
        "deepseek-v4-pro",
        "deepseek-v4-flash",
        "qwen3",
        "copilot",
    ] {
        set_state(&mut rows, m, State::QuotaExhausted);
    }
    let got = select(
        &rows,
        &query("review", &["anthropic"], None, now),
        &config(),
    );
    assert!(
        got.is_empty(),
        "a family the author uses is refused even when it is the only healthy one: {got:?}"
    );
}

// ---- DoD: nothing qualifies when every candidate is `unknown` ----

#[test]
fn every_candidate_unknown_means_nothing_qualifies() {
    let now = Utc::now();
    let mut rows = all_ok(now);
    for (_, _, m, _) in SLOTS {
        set_state(&mut rows, m, State::Unknown);
    }
    assert!(
        select(
            &rows,
            &query("review", &["anthropic"], None, now),
            &config()
        )
        .is_empty()
    );
}

#[test]
fn every_candidate_absent_means_nothing_qualifies() {
    let now = Utc::now();
    assert!(select(&[], &query("review", &["anthropic"], None, now), &config()).is_empty());
}

#[test]
fn control_one_fresh_ok_row_qualifies() {
    let now = Utc::now();
    let rows = vec![row_at(
        Kind::Api,
        "zhipu",
        "glm-5.3",
        State::Ok,
        now - Duration::seconds(30),
        3600,
        400,
    )];
    let got = select(
        &rows,
        &query("review", &["anthropic"], None, now),
        &config(),
    );
    assert_eq!(models(&got), ["glm-5.3"]);
    let c = &got[0];
    assert_eq!(
        (c.service.as_str(), c.provider.as_str(), c.family.as_str()),
        ("glm", "zhipu", "zhipu")
    );
    assert_eq!(c.key, "api.zhipu.acct.glm-5-3");
    assert!(
        !c.reason.trim().is_empty(),
        "a candidate says why it qualified"
    );
}

// ---- the freshness rule: an expired row is UNKNOWN, never chosen ----

#[test]
fn an_expired_ok_row_is_never_chosen() {
    let now = Utc::now();
    let mut rows = all_ok(now);
    // deepseek-v4-flash is the fastest non-anthropic review model, but its ok is 2 h old on a 60 s ttl
    let flash = rows
        .iter_mut()
        .find(|r| r.model == "deepseek-v4-flash")
        .unwrap();
    flash.checked_at = now - Duration::seconds(7200);
    flash.ttl_s = 60;
    let got = select(
        &rows,
        &query("review", &["anthropic"], Some(Prefer::Fastest), now),
        &config(),
    );
    assert!(
        !models(&got).contains(&"deepseek-v4-flash"),
        "an expired row was chosen: {got:?}"
    );
    assert_eq!(
        got.first().map(|c| c.model.as_str()),
        Some("deepseek-v4-pro")
    );
}

#[test]
fn control_the_same_row_inside_its_ttl_is_chosen() {
    let now = Utc::now();
    let mut rows = all_ok(now);
    let flash = rows
        .iter_mut()
        .find(|r| r.model == "deepseek-v4-flash")
        .unwrap();
    flash.checked_at = now - Duration::seconds(59);
    flash.ttl_s = 60;
    let got = select(
        &rows,
        &query("review", &["anthropic"], Some(Prefer::Fastest), now),
        &config(),
    );
    assert_eq!(
        got.first().map(|c| c.model.as_str()),
        Some("deepseek-v4-flash")
    );
}

#[test]
fn the_rule_is_applied_at_query_now_not_the_wall_clock() {
    // a row fresh by the wall clock but expired at the query's `now` (an hour later) is not chosen
    let now = Utc::now();
    let later = now + Duration::seconds(7200);
    let got = select(
        &all_ok(now),
        &query("review", &["anthropic"], None, later),
        &config(),
    );
    assert!(
        got.is_empty(),
        "rows with a 1 h ttl, read 2 h later: {got:?}"
    );
}

// ---- only a fresh `ok` is chosen ----

#[test]
fn a_state_other_than_ok_is_never_chosen() {
    let now = Utc::now();
    for state in [
        State::AuthFailed,
        State::ModelMissing,
        State::QuotaExhausted,
        State::RateLimited,
        State::Degraded,
        State::Unknown,
    ] {
        let mut rows = all_ok(now);
        set_state(&mut rows, "deepseek-v4-flash", state);
        let got = select(
            &rows,
            &query("review", &["anthropic"], Some(Prefer::Fastest), now),
            &config(),
        );
        assert!(
            !models(&got).contains(&"deepseek-v4-flash"),
            "{} was chosen: {got:?}",
            state.as_str()
        );
        assert_eq!(
            got.first().map(|c| c.model.as_str()),
            Some("deepseek-v4-pro"),
            "{}",
            state.as_str()
        );
    }
}

#[test]
fn an_exhausted_subscription_is_refused() {
    // DESIGN §9: "the selector is only as honest as the subscription rows it refuses on"
    let now = Utc::now();
    let mut rows = all_ok(now);
    set_state(&mut rows, "copilot", State::QuotaExhausted);
    let got = select(
        &rows,
        &query("review", &["anthropic"], None, now),
        &config(),
    );
    assert!(got.iter().all(|c| c.service != "copilot"), "{got:?}");
}

// ---- the role filter ----

#[test]
fn a_model_without_the_role_is_never_chosen() {
    let now = Utc::now();
    // kimi-k3 (100 ms, work only) would beat every non-anthropic review model on speed
    let got = select(
        &all_ok(now),
        &query("review", &["anthropic"], Some(Prefer::Fastest), now),
        &config(),
    );
    assert!(!models(&got).contains(&"kimi-k3"), "{got:?}");
    // the Claude subscription account has no roles: never a review candidate
    assert!(got.iter().all(|c| c.service != "claude-hankh95"), "{got:?}");
}

#[test]
fn control_the_work_role_picks_the_work_models() {
    let now = Utc::now();
    let got = select(
        &all_ok(now),
        &query("work", &[], Some(Prefer::Fastest), now),
        &config(),
    );
    assert_eq!(
        models(&got),
        ["kimi-k3", "glm-5.3"],
        "kimi (100 ms) then glm (400 ms)"
    );
}

// ---- --prefer orderings, known answers ----

#[test]
fn prefer_fastest_orders_by_latency() {
    let now = Utc::now();
    let mut rows = all_ok(now);
    set_state(&mut rows, "copilot", State::QuotaExhausted);
    let got = select(
        &rows,
        &query("review", &["anthropic"], Some(Prefer::Fastest), now),
        &config(),
    );
    assert_eq!(
        models(&got),
        ["deepseek-v4-flash", "deepseek-v4-pro", "glm-5.3", "qwen3"]
    );
}

#[test]
fn prefer_largest_context_orders_by_the_model_table() {
    let now = Utc::now();
    let mut rows = all_ok(now);
    set_state(&mut rows, "copilot", State::QuotaExhausted);
    let got = select(
        &rows,
        &query("review", &["anthropic"], Some(Prefer::LargestContext), now),
        &config(),
    );
    // 200k, 128k, 64k, 32k — and the order is not the latency order (a mutation to "fastest" turns this red)
    assert_eq!(
        models(&got),
        ["glm-5.3", "deepseek-v4-pro", "deepseek-v4-flash", "qwen3"]
    );
}

#[test]
fn prefer_cheapest_puts_a_local_model_before_metered_ones() {
    let now = Utc::now();
    let mut rows = all_ok(now);
    set_state(&mut rows, "copilot", State::QuotaExhausted);
    let got = select(
        &rows,
        &query("review", &["anthropic"], Some(Prefer::Cheapest), now),
        &config(),
    );
    // qwen3 is `cost_class = "local"` and the slowest (900 ms) and smallest: only the cost order puts it first
    assert_eq!(
        got.first().map(|c| c.model.as_str()),
        Some("qwen3"),
        "{got:?}"
    );
    assert_eq!(got.len(), 4);
}

#[test]
fn control_cheapest_without_the_local_model_is_metered() {
    let now = Utc::now();
    let mut rows = all_ok(now);
    set_state(&mut rows, "copilot", State::QuotaExhausted);
    set_state(&mut rows, "qwen3", State::AuthFailed);
    let got = select(
        &rows,
        &query("review", &["anthropic"], Some(Prefer::Cheapest), now),
        &config(),
    );
    assert_ne!(got.first().map(|c| c.model.as_str()), Some("qwen3"));
    assert_eq!(got.len(), 3);
}

#[test]
fn prefer_parses_the_cli_words() {
    assert_eq!("cheapest".parse::<Prefer>(), Ok(Prefer::Cheapest));
    assert_eq!("fastest".parse::<Prefer>(), Ok(Prefer::Fastest));
    assert_eq!(
        "largest-context".parse::<Prefer>(),
        Ok(Prefer::LargestContext)
    );
    assert!("priciest".parse::<Prefer>().is_err());
}

// ---- pure ----

#[test]
fn the_same_input_gives_the_same_answer() {
    let now = Utc::now();
    let rows = all_ok(now);
    let q = query("review", &["anthropic"], Some(Prefer::LargestContext), now);
    assert_eq!(select(&rows, &q, &config()), select(&rows, &q, &config()));
}
