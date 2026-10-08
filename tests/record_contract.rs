//! EXP-001 Plan 2 / DESIGN §2: the status record and its serialised form.

mod common;

use chrono::{DateTime, Duration, SubsecRound, Utc};
use quotabus::{Balance, Headroom, Kind, Record, State, record_key, slug};

fn full_row() -> Record {
    let mut r = common::record(
        "api.deepseek.nusy-product-team.deepseek-v4-pro",
        State::QuotaExhausted,
        Utc::now(),
        2700,
    );
    r.family = "deepseek".into();
    r.balance = Some(Balance {
        amount: -0.12,
        currency: "CNY".into(),
        source: "https://api.deepseek.com/user/balance".into(),
    });
    r.headroom = Some(Headroom {
        requests_remaining: Some(49),
        tokens_remaining: Some(39000),
        ..Default::default()
    });
    r.error = Some("insufficient balance".into());
    r
}

#[test]
fn record_serialises_with_the_design_fields() {
    let row = full_row();
    let v = serde_json::to_value(&row).unwrap();
    for field in [
        "contract",
        "key",
        "kind",
        "provider",
        "account",
        "model",
        "family",
        "state",
        "balance",
        "headroom",
        "latency_ms",
        "probe",
        "error",
        "checked_at",
        "ttl_s",
        "observed_by",
    ] {
        assert!(
            v.get(field).is_some(),
            "serialised record lacks `{field}`: {v}"
        );
    }
    assert_eq!(v["contract"], "ai-status/1");
    assert_eq!(quotabus::CONTRACT, "ai-status/1");
    assert_eq!(v["kind"], "api");
    assert_eq!(v["state"], "quota_exhausted");
    assert_eq!(v["ttl_s"], 2700);
    assert_eq!(v["probe"]["source"], "official");
    assert_eq!(v["balance"]["currency"], "CNY");
    assert_eq!(v["balance"]["amount"].as_f64(), Some(-0.12));
    assert_eq!(v["headroom"]["requests_remaining"], 49);
}

#[test]
fn checked_at_is_rfc3339_and_round_trips() {
    let at = Utc::now().trunc_subsecs(0) - Duration::seconds(17);
    let mut row = full_row();
    row.checked_at = at;
    let v = serde_json::to_value(&row).unwrap();
    let s = v["checked_at"].as_str().expect("checked_at is a string");
    let parsed = DateTime::parse_from_rfc3339(s)
        .unwrap_or_else(|e| panic!("checked_at {s:?} is not RFC 3339: {e}"));
    assert_eq!(parsed.with_timezone(&Utc), at);
    let back: Record = serde_json::from_value(v).unwrap();
    assert_eq!(back, row);
}

#[test]
fn every_state_serialises_snake_case() {
    let cases = [
        (State::Ok, "ok"),
        (State::AuthFailed, "auth_failed"),
        (State::ModelMissing, "model_missing"),
        (State::QuotaExhausted, "quota_exhausted"),
        (State::RateLimited, "rate_limited"),
        (State::Degraded, "degraded"),
        (State::Unknown, "unknown"),
    ];
    for (state, want) in cases {
        assert_eq!(serde_json::to_value(state).unwrap(), want);
        let back: State = serde_json::from_value(serde_json::json!(want)).unwrap();
        assert_eq!(back, state);
    }
}

#[test]
fn control_an_unknown_state_name_is_refused() {
    // negative control: the enum is closed; a reader never maps a stranger to a state
    assert!(serde_json::from_value::<State>(serde_json::json!("all_good")).is_err());
    assert!(serde_json::from_value::<State>(serde_json::json!("OK")).is_err());
}

#[test]
fn key_is_kind_provider_account_model_in_slug_form() {
    // known answers
    assert_eq!(
        record_key(Kind::Api, "zhipu", "nusy-product-team", "glm-5.3"),
        "api.zhipu.nusy-product-team.glm-5-3"
    );
    assert_eq!(
        record_key(Kind::Local, "qwen", "dgx1", "qwen3"),
        "local.qwen.dgx1.qwen3"
    );
    assert_eq!(
        record_key(Kind::Subscription, "anthropic", "m5", "max"),
        "subscription.anthropic.m5.max"
    );
}

#[test]
fn key_never_carries_an_email_or_extra_separators() {
    // KV keys exclude `@`; a `.` or `/` inside a part would add a token
    let k = record_key(
        Kind::Api,
        "together",
        "hank@example.com",
        "meta-llama/Llama-3.3-70B-Instruct-Turbo",
    );
    assert!(!k.contains('@'), "{k}");
    assert_eq!(k.split('.').count(), 4, "{k}");
    assert!(
        k.chars()
            .all(|c| c.is_ascii_alphanumeric() || "._-".contains(c)),
        "{k}"
    );
    assert!(k.starts_with("api.together."), "{k}");
}

#[test]
fn slug_keeps_safe_chars_and_replaces_the_rest() {
    assert_eq!(slug("nusy-product-team"), "nusy-product-team");
    assert_eq!(slug("glm_4"), "glm_4");
    assert_eq!(slug("a.b/c@d"), "a-b-c-d");
}
