//! EXP-002 / SIG-004 Q5 (Captain 2026-10-08: "separate times for the different types of queries"): subscription
//! reads are a third query kind, `QueryKind::Subscription`, on their OWN `[intervals] subscription` key (default
//! 5 min) with their own `[ttl] subscription` (default 3 × its own interval = 15 min), in the per-kind shape CHORE-007
//! built. Setting it moves no other kind, and no other kind moves it.

use std::time::Duration;

use quotabus::{Config, QueryKind};

const H: u64 = 3600;
const M: u64 = 60;

fn cfg(text: &str) -> Config {
    Config::from_toml_str(text).unwrap_or_else(|e| panic!("config must parse: {e}\n{text}"))
}

#[test]
fn subscription_defaults_to_5m_with_a_15m_ttl() {
    let c = cfg("");
    assert_eq!(
        c.interval_for(QueryKind::Subscription),
        Duration::from_secs(5 * M)
    );
    assert_eq!(
        c.ttl_for(QueryKind::Subscription),
        Duration::from_secs(15 * M)
    );
}

#[test]
fn control_the_api_and_balance_defaults_are_untouched() {
    // the 12h defaults of CHORE-007 hold beside the new kind: a shared default would fail one side or the other
    let c = cfg("");
    assert_eq!(c.interval_for(QueryKind::Api), Duration::from_secs(12 * H));
    assert_eq!(
        c.interval_for(QueryKind::Balance),
        Duration::from_secs(12 * H)
    );
    assert_ne!(
        c.interval_for(QueryKind::Subscription),
        c.interval_for(QueryKind::Api),
        "subscription has its own default, not the API's"
    );
}

#[test]
fn the_subscription_key_is_accepted_in_intervals_and_ttl() {
    let c = cfg("[intervals]\nsubscription = \"2m\"\n\n[ttl]\nsubscription = \"11m\"\n");
    assert_eq!(
        c.interval_for(QueryKind::Subscription),
        Duration::from_secs(2 * M)
    );
    assert_eq!(
        c.ttl_for(QueryKind::Subscription),
        Duration::from_secs(11 * M)
    );
}

#[test]
fn setting_subscription_moves_no_other_kind_and_vice_versa() {
    let a = cfg("[intervals]\nsubscription = \"1m\"\n");
    assert_eq!(
        a.interval_for(QueryKind::Subscription),
        Duration::from_secs(M)
    );
    assert_eq!(a.interval_for(QueryKind::Api), Duration::from_secs(12 * H));
    assert_eq!(
        a.interval_for(QueryKind::Balance),
        Duration::from_secs(12 * H)
    );
    assert_eq!(a.ttl_for(QueryKind::Api), Duration::from_secs(36 * H));

    let b = cfg("[intervals]\napi = \"1h\"\nbalance = \"2h\"\n");
    assert_eq!(
        b.interval_for(QueryKind::Subscription),
        Duration::from_secs(5 * M),
        "setting api/balance must not move subscription"
    );
    assert_eq!(
        b.ttl_for(QueryKind::Subscription),
        Duration::from_secs(15 * M)
    );
}

#[test]
fn the_default_subscription_ttl_is_three_times_its_own_interval() {
    // 3 × the API interval (3h here) or 3 × any shared value fails: 30m is only 3 × 10m
    let c = cfg("[intervals]\napi = \"1h\"\nsubscription = \"10m\"\n");
    assert_eq!(
        c.ttl_for(QueryKind::Subscription),
        Duration::from_secs(30 * M)
    );
    assert_eq!(c.ttl_for(QueryKind::Api), Duration::from_secs(3 * H));
}

#[test]
fn a_subscription_ttl_moves_no_other_ttl() {
    let c = cfg("[ttl]\nsubscription = \"7m\"\n");
    assert_eq!(
        c.ttl_for(QueryKind::Subscription),
        Duration::from_secs(7 * M)
    );
    assert_eq!(c.ttl_for(QueryKind::Api), Duration::from_secs(36 * H));
    assert_eq!(c.ttl_for(QueryKind::Balance), Duration::from_secs(36 * H));
}

#[test]
fn control_a_bad_subscription_duration_is_refused() {
    for bad in [
        "[intervals]\nsubscription = \"often\"\n",
        "[ttl]\nsubscription = \"5\"\n",
        "[ttl]\nsubscription = \"0s\"\n",
    ] {
        assert!(
            Config::from_toml_str(bad).is_err(),
            "must be refused: {bad}"
        );
    }
}

#[test]
fn control_an_unknown_interval_kind_is_still_refused() {
    // the table stays closed: only api, balance and subscription are kinds
    assert!(Config::from_toml_str("[intervals]\nquota_window = \"5m\"\n").is_err());
}
