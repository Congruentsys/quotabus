//! EXP-001 / DESIGN §2: the freshness rule as a pure function over (row or none, now).

mod common;

use chrono::{Duration, Utc};
use quotabus::{State, UnknownReason, Verdict, freshness};

const KEY: &str = "api.zhipu.nusy-product-team.glm-5-3";

#[test]
fn no_row_is_unknown_absent() {
    let v = freshness(None, Utc::now());
    assert_eq!(
        v,
        Verdict::Unknown {
            reason: UnknownReason::Absent
        }
    );
    assert_eq!(v.exit_code(), 3);
    assert_eq!(v.label(), "UNKNOWN");
}

#[test]
fn a_row_past_its_ttl_is_unknown_expired() {
    let now = Utc::now();
    let row = common::record(KEY, State::Ok, now - Duration::seconds(61), 60);
    let v = freshness(Some(&row), now);
    assert_eq!(
        v,
        Verdict::Unknown {
            reason: UnknownReason::Expired
        }
    );
    assert_eq!(v.exit_code(), 3);
}

#[test]
fn control_a_row_exactly_at_its_ttl_is_still_fresh() {
    // `checked_at + ttl_s < now` is strict: the mutation `<=` turns this red
    let now = Utc::now();
    let row = common::record(KEY, State::Ok, now - Duration::seconds(60), 60);
    assert_eq!(
        freshness(Some(&row), now),
        Verdict::State {
            state: State::Ok,
            age: Duration::seconds(60)
        }
    );
}

#[test]
fn an_unknown_row_is_cannot_assess_with_its_reason() {
    let now = Utc::now();
    let mut row = common::record(KEY, State::Unknown, now - Duration::seconds(5), 60);
    row.reason = Some("cannot_assess:secret_unset".into());
    let v = freshness(Some(&row), now);
    assert_eq!(
        v,
        Verdict::CannotAssess {
            reason: "cannot_assess:secret_unset".into()
        }
    );
    assert_eq!(v.exit_code(), 2);
    assert_eq!(v.label(), "CANNOT-ASSESS");
}

#[test]
fn expiry_is_checked_before_the_state() {
    // an old CANNOT-ASSESS is no longer a measurement either
    let now = Utc::now();
    let row = common::record(KEY, State::Unknown, now - Duration::seconds(3600), 60);
    assert_eq!(
        freshness(Some(&row), now),
        Verdict::Unknown {
            reason: UnknownReason::Expired
        }
    );
}

#[test]
fn a_fresh_ok_row_is_ok_with_its_age() {
    let now = Utc::now();
    let row = common::record(KEY, State::Ok, now - Duration::seconds(30), 2700);
    let v = freshness(Some(&row), now);
    assert_eq!(
        v,
        Verdict::State {
            state: State::Ok,
            age: Duration::seconds(30)
        }
    );
    assert_eq!(v.exit_code(), 0);
    assert_eq!(v.label(), "ok");
}

#[test]
fn every_measured_non_ok_state_exits_1() {
    let now = Utc::now();
    for (state, label) in [
        (State::AuthFailed, "auth_failed"),
        (State::ModelMissing, "model_missing"),
        (State::QuotaExhausted, "quota_exhausted"),
        (State::RateLimited, "rate_limited"),
        (State::Degraded, "degraded"),
    ] {
        let row = common::record(KEY, state, now - Duration::seconds(10), 2700);
        let v = freshness(Some(&row), now);
        assert_eq!(
            v,
            Verdict::State {
                state,
                age: Duration::seconds(10)
            }
        );
        assert_eq!(v.exit_code(), 1, "{label}");
        assert_eq!(v.label(), label);
    }
}
