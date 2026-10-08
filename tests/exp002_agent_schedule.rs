//! EXP-002 / SIG-004 Q5 at the agent: `quotabus agent` reads a subscription only when it is DUE on its own
//! `[intervals] subscription` (default 5 min) since that service's last row in the store; `--force` reads every one.
//! The API interval never decides it. And `quotabus status --kind subscription` lists every host's row (the DoD's
//! "five rows from five hosts" read back), each with its age. The Copilot read (a `gh` call) is what is counted.

mod exp002;

use chrono::{Duration, Utc};
use exp002::*;
use quotabus::{Kind, Probe, ProbeSource, Record, State};

const H1: &str = "qbhost1";

fn user_json() -> String {
    r#"{"quota_reset_date":"2026-11-01","quota_snapshots":{"premium_interactions":{"entitlement":300,"remaining":120,"percent_remaining":40.0,"unlimited":false,"reset_date":"2026-11-01"}}}"#.to_string()
}

/// Seed the store with this host's copilot row, `age_min` minutes old.
fn seed_copilot(h: &Host, age_min: i64) {
    let r = Record {
        contract: "ai-status/1".into(),
        key: copilot_key(&h.host),
        kind: Kind::Subscription,
        provider: "github".into(),
        account: h.host.clone(),
        model: "copilot".into(),
        family: "openai".into(),
        state: State::Ok,
        reason: None,
        balance: None,
        headroom: None,
        latency_ms: None,
        probe: Probe {
            name: "copilot_internal".into(),
            source: ProbeSource::Undocumented,
        },
        error: None,
        checked_at: Utc::now() - Duration::minutes(age_min),
        ttl_s: 900,
        observed_by: format!("{}/quotabus@0.0.0", h.host),
    };
    write_json(
        &h.store().join(format!("{}.json", r.key)),
        &serde_json::to_value(&r).unwrap(),
    );
}

#[test]
fn a_second_run_straight_after_the_first_reads_nothing_and_force_reads_again() {
    let h = Host::new(H1);
    h.fake_gh(&user_json(), "", 0);
    let cfg = h.config(COPILOT_SVC);
    h.agent(&cfg, &[]);
    // control: the counter sees a call, so the "nothing" below is a measurement
    assert_eq!(h.gh_calls().len(), 1, "an empty store: due");
    h.agent(&cfg, &[]);
    assert_eq!(h.gh_calls().len(), 1, "within 5 min nothing is due");
    h.agent(&cfg, &["--force"]);
    assert_eq!(h.gh_calls().len(), 2, "--force reads now");
}

#[test]
fn the_default_5m_interval_decides_due_ness() {
    let h = Host::new(H1);
    h.fake_gh(&user_json(), "", 0);
    seed_copilot(&h, 4);
    h.agent(&h.config(COPILOT_SVC), &[]);
    assert!(h.gh_calls().is_empty(), "4 min < 5 min: not due");

    let h = Host::new(H1);
    h.fake_gh(&user_json(), "", 0);
    seed_copilot(&h, 6);
    h.agent(&h.config(COPILOT_SVC), &[]);
    assert_eq!(h.gh_calls().len(), 1, "6 min > 5 min: due");
}

#[test]
fn the_subscription_interval_not_the_api_interval_decides() {
    // a short subscription interval with a long api one: due
    let h = Host::new(H1);
    h.fake_gh(&user_json(), "", 0);
    seed_copilot(&h, 2);
    h.agent(
        &h.config(&format!(
            "[intervals]\napi = \"12h\"\nsubscription = \"1m\"\n\n{COPILOT_SVC}"
        )),
        &[],
    );
    assert_eq!(h.gh_calls().len(), 1, "2 min > subscription 1 min");

    // mutation control: swap them, and an agent reading the api interval would call; this one must not
    let h = Host::new(H1);
    h.fake_gh(&user_json(), "", 0);
    seed_copilot(&h, 2);
    h.agent(
        &h.config(&format!(
            "[intervals]\napi = \"1m\"\nsubscription = \"12h\"\n\n{COPILOT_SVC}"
        )),
        &[],
    );
    assert!(
        h.gh_calls().is_empty(),
        "2 min < subscription 12h, whatever api says"
    );
}

#[test]
fn another_hosts_row_does_not_make_this_host_not_due() {
    // due-ness is per row, and the row is keyed by host: host b's fresh row says nothing about host a
    let a = Host::new("qbhosta");
    let b = Host::new("qbhostb");
    a.fake_gh(&user_json(), "", 0);
    b.fake_gh(&user_json(), "", 0);
    let store = a.store();
    b.agent(&b.config_with_store(&store, COPILOT_SVC), &[]);
    a.agent(&a.config_with_store(&store, COPILOT_SVC), &[]);
    assert_eq!(a.gh_calls().len(), 1, "host a reads its own");
    assert!(row(&store, &copilot_key("qbhosta")).is_some());
    assert!(row(&store, &copilot_key("qbhostb")).is_some());
}

#[test]
fn status_lists_every_hosts_subscription_row_with_its_age() {
    let store_host = Host::new("qbhosta");
    let store = store_host.store();
    let now = now_s();
    for host in ["qbhosta", "qbhostb", "qbhostc"] {
        let h = Host::new(host);
        h.write_cache(&cache_entry(
            "statusline",
            20,
            Some((19.0, now + 9000)),
            None,
        ));
        h.agent(&h.config_with_store(&store, CLAUDE_SVC), &[]);
    }
    // read from a fourth host: the listing is not filtered to the reader's own host
    let reader = Host::new("qbreader");
    let cfg = reader.config_with_store(&store, CLAUDE_SVC);
    let out = reader.quotabus(&cfg, &["status", "--json", "--kind", "subscription"]);
    assert_eq!(rc(&out), 0, "{}", text(&out));
    let arr: serde_json::Value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("status --json: {e}\n{}", text(&out)));
    let arr = arr.as_array().expect("an array");
    for host in ["qbhosta", "qbhostb", "qbhostc"] {
        let e = arr
            .iter()
            .find(|e| e["key"] == claude_key(host))
            .unwrap_or_else(|| panic!("no entry for {host}: {arr:?}"));
        assert_eq!(e["verdict"], "ok", "{e}");
        let age = e["age_s"].as_i64().expect("age_s");
        assert!(
            (15..=120).contains(&age),
            "{host}: age {age} s is the reading's age (~20 s)"
        );
    }
}
