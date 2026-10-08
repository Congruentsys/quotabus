//! EXP-002 Plan 1 / the Captain's "Hourly" ruling (2026-10-08) / §10 Q5: subscription reads are a third query kind,
//! `QueryKind::Subscription`, on their OWN `[intervals] subscription` key (default 1 h) with their own
//! `[ttl] subscription` (default 3 × its own interval = 3 h). Setting it moves no other kind, and vice versa.

use std::time::Duration;

use quotabus::{Config, QueryKind};

const H: u64 = 3600;
const M: u64 = 60;

fn cfg(text: &str) -> Config {
    Config::from_toml_str(text).unwrap_or_else(|e| panic!("config must parse: {e}\n{text}"))
}

#[test]
fn subscription_defaults_to_1h_with_a_3h_ttl() {
    let c = cfg("");
    assert_eq!(
        c.interval_for(QueryKind::Subscription),
        Duration::from_secs(H)
    );
    assert_eq!(
        c.ttl_for(QueryKind::Subscription),
        Duration::from_secs(3 * H)
    );
}

#[test]
fn control_api_and_balance_keep_their_12h_defaults() {
    let c = cfg("");
    assert_eq!(c.interval_for(QueryKind::Api), Duration::from_secs(12 * H));
    assert_eq!(
        c.interval_for(QueryKind::Balance),
        Duration::from_secs(12 * H)
    );
    assert_ne!(
        c.interval_for(QueryKind::Subscription),
        c.interval_for(QueryKind::Api),
        "subscription has its own default"
    );
}

#[test]
fn the_subscription_key_is_accepted_in_intervals_and_ttl() {
    let c = cfg("[intervals]\nsubscription = \"20m\"\n\n[ttl]\nsubscription = \"50m\"\n");
    assert_eq!(
        c.interval_for(QueryKind::Subscription),
        Duration::from_secs(20 * M)
    );
    assert_eq!(
        c.ttl_for(QueryKind::Subscription),
        Duration::from_secs(50 * M)
    );
}

#[test]
fn setting_subscription_moves_no_other_kind_and_vice_versa() {
    let a = cfg("[intervals]\nsubscription = \"1m\"\n");
    assert_eq!(a.interval_for(QueryKind::Api), Duration::from_secs(12 * H));
    assert_eq!(
        a.interval_for(QueryKind::Balance),
        Duration::from_secs(12 * H)
    );
    assert_eq!(a.ttl_for(QueryKind::Api), Duration::from_secs(36 * H));
    let b = cfg("[intervals]\napi = \"1m\"\nbalance = \"2m\"\n");
    assert_eq!(
        b.interval_for(QueryKind::Subscription),
        Duration::from_secs(H)
    );
    assert_eq!(
        b.ttl_for(QueryKind::Subscription),
        Duration::from_secs(3 * H)
    );
}

#[test]
fn the_default_subscription_ttl_is_three_times_its_own_interval() {
    // 3 × the API interval would be 3 h here; 3 × 10 min is 30 min
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
fn control_bad_subscription_durations_are_refused() {
    for bad in [
        "[intervals]\nsubscription = \"hourly\"\n",
        "[ttl]\nsubscription = \"5\"\n",
        "[ttl]\nsubscription = \"0s\"\n",
    ] {
        assert!(
            Config::from_toml_str(bad).is_err(),
            "must be refused: {bad}"
        );
    }
    assert!(Config::from_toml_str("[intervals]\nquota_window = \"1h\"\n").is_err());
}
