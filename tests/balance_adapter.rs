//! EXP-001 Plan 3 / DESIGN §3 `[service.balance]`, §4: the DeepSeek and Kimi balance adapters (pure parts).

use quotabus::State;
use quotabus::balance::{apply_floor, extract_amount};
use serde_json::json;

fn deepseek(total: &str) -> serde_json::Value {
    json!({"is_available": true, "balance_infos": [
        {"currency": "CNY", "total_balance": total, "granted_balance": "0.00", "topped_up_balance": total}
    ]})
}

fn kimi(avail: f64) -> serde_json::Value {
    json!({"code": 0, "data": {"available_balance": avail, "voucher_balance": 0, "cash_balance": avail},
           "scode": "0x0", "status": true})
}

#[test]
fn deepseek_path_reads_a_numeric_string() {
    assert_eq!(
        extract_amount(&deepseek("110.00"), "balance_infos[0].total_balance"),
        Some(110.0)
    );
    assert_eq!(
        extract_amount(&deepseek("-0.12"), "balance_infos[0].total_balance"),
        Some(-0.12)
    );
}

#[test]
fn kimi_path_reads_a_number() {
    assert_eq!(
        extract_amount(&kimi(49.58), "data.available_balance"),
        Some(49.58)
    );
}

#[test]
fn control_a_missing_or_wrong_path_reads_nothing() {
    assert_eq!(
        extract_amount(&deepseek("110.00"), "balance_infos[1].total_balance"),
        None
    );
    assert_eq!(
        extract_amount(&deepseek("110.00"), "data.available_balance"),
        None
    );
    assert_eq!(
        extract_amount(&kimi(49.58), "balance_infos[0].total_balance"),
        None
    );
    assert_eq!(
        extract_amount(
            &json!({"data": {"available_balance": "lots"}}),
            "data.available_balance"
        ),
        None
    );
}

#[test]
fn a_negative_balance_is_quota_exhausted() {
    // the measured real case: DeepSeek at -0.12 CNY
    assert_eq!(apply_floor(State::Ok, -0.12, 150.0), State::QuotaExhausted);
    assert_eq!(apply_floor(State::Ok, -0.12, 0.0), State::QuotaExhausted);
}

#[test]
fn a_balance_at_or_below_the_floor_is_quota_exhausted() {
    assert_eq!(apply_floor(State::Ok, 150.0, 150.0), State::QuotaExhausted);
    assert_eq!(apply_floor(State::Ok, 49.58, 100.0), State::QuotaExhausted);
}

#[test]
fn control_above_the_floor_keeps_the_probe_state() {
    // the mutation "always quota_exhausted" or "`<` instead of `<=`" turns this or the test above red
    assert_eq!(apply_floor(State::Ok, 150.01, 150.0), State::Ok);
    assert_eq!(
        apply_floor(State::RateLimited, 500.0, 150.0),
        State::RateLimited
    );
    assert_eq!(
        apply_floor(State::AuthFailed, 500.0, 150.0),
        State::AuthFailed
    );
}
