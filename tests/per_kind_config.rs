//! CHORE-007 DoD 1: an `[intervals]` table with one key per query kind (`api`, `balance`), defaults 12h each; each
//! kind's TTL defaults to 3 × its OWN interval and is set per kind in `[ttl]`; the old `[probe] interval` / `ttl` keys
//! are refused with an error naming the new keys; `max_tokens` and `degraded_latency_ms` stay in `[probe]`.

use std::time::Duration;

use quotabus::{Config, QueryKind};

const H: u64 = 3600;

fn cfg(text: &str) -> Config {
    Config::from_toml_str(text).unwrap_or_else(|e| panic!("config must parse: {e}\n{text}"))
}

fn refused(text: &str) -> String {
    match Config::from_toml_str(text) {
        Ok(c) => panic!("config must be refused, but parsed: {c:?}\n{text}"),
        Err(e) => e.to_string(),
    }
}

#[test]
fn defaults_are_api_12h_and_balance_12h_with_ttl_3x() {
    let c = cfg("");
    assert_eq!(c.interval_for(QueryKind::Api), Duration::from_secs(12 * H));
    assert_eq!(
        c.interval_for(QueryKind::Balance),
        Duration::from_secs(12 * H)
    );
    assert_eq!(c.ttl_for(QueryKind::Api), Duration::from_secs(36 * H));
    assert_eq!(c.ttl_for(QueryKind::Balance), Duration::from_secs(36 * H));
}

#[test]
fn each_kind_has_its_own_interval() {
    let c = cfg("[intervals]\napi = \"1h\"\nbalance = \"6h\"\n");
    assert_eq!(c.interval_for(QueryKind::Api), Duration::from_secs(H));
    assert_eq!(
        c.interval_for(QueryKind::Balance),
        Duration::from_secs(6 * H)
    );
}

#[test]
fn setting_one_kind_leaves_the_other_at_its_default() {
    // control against a shared interval: setting `api` must not move `balance`, and vice versa
    let a = cfg("[intervals]\napi = \"1h\"\n");
    assert_eq!(a.interval_for(QueryKind::Api), Duration::from_secs(H));
    assert_eq!(
        a.interval_for(QueryKind::Balance),
        Duration::from_secs(12 * H)
    );
    let b = cfg("[intervals]\nbalance = \"30m\"\n");
    assert_eq!(b.interval_for(QueryKind::Api), Duration::from_secs(12 * H));
    assert_eq!(
        b.interval_for(QueryKind::Balance),
        Duration::from_secs(30 * 60)
    );
}

#[test]
fn default_ttl_is_three_times_the_kinds_own_interval() {
    // a TTL of 3 × the API interval for both kinds (or 3 × any one shared value) fails here: 3h vs 18h
    let c = cfg("[intervals]\napi = \"1h\"\nbalance = \"6h\"\n");
    assert_eq!(c.ttl_for(QueryKind::Api), Duration::from_secs(3 * H));
    assert_eq!(c.ttl_for(QueryKind::Balance), Duration::from_secs(18 * H));
}

#[test]
fn ttl_is_overridable_per_kind() {
    let c = cfg("[intervals]\napi = \"1h\"\nbalance = \"6h\"\n\n[ttl]\nbalance = \"7h\"\n");
    assert_eq!(
        c.ttl_for(QueryKind::Balance),
        Duration::from_secs(7 * H),
        "[ttl] balance is used as written"
    );
    assert_eq!(
        c.ttl_for(QueryKind::Api),
        Duration::from_secs(3 * H),
        "[ttl] balance does not move the api TTL"
    );
    let d = cfg("[ttl]\napi = \"90m\"\n");
    assert_eq!(d.ttl_for(QueryKind::Api), Duration::from_secs(90 * 60));
    assert_eq!(d.ttl_for(QueryKind::Balance), Duration::from_secs(36 * H));
}

#[test]
fn the_old_probe_interval_key_is_refused_naming_the_new_keys() {
    let e = refused("[probe]\ninterval = \"15m\"\n");
    assert!(
        e.contains("[intervals]") && e.contains("api") && e.contains("balance"),
        "the error must name the new keys ([intervals] api / balance): {e}"
    );
}

#[test]
fn the_old_probe_ttl_key_is_refused_naming_the_new_keys() {
    let e = refused("[probe]\nttl = \"45m\"\n");
    assert!(
        e.contains("[ttl]"),
        "the error must name the new [ttl] table: {e}"
    );
}

#[test]
fn control_max_tokens_and_degraded_latency_stay_in_probe() {
    // the positive control for the refusals above: `[probe]` itself is still accepted
    let c =
        cfg("[probe]\nmax_tokens = 7\ndegraded_latency_ms = 4321\n\n[intervals]\napi = \"2h\"\n");
    assert_eq!(c.probe.max_tokens, 7);
    assert_eq!(c.probe.degraded_latency_ms, 4321);
    assert_eq!(c.interval_for(QueryKind::Api), Duration::from_secs(2 * H));
}

#[test]
fn control_a_bad_interval_or_ttl_duration_is_refused() {
    refused("[intervals]\napi = \"often\"\n");
    refused("[ttl]\nbalance = \"12\"\n");
}
