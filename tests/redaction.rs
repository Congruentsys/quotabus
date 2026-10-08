//! EXP-001 Plan 3 / DESIGN §6: the Secret newtype, the Redactor, the 200-char cap, and the leak checker's controls.

mod common;

use chrono::Utc;
use common::{FAKE_PLAIN, FAKE_SK, leaks};
use quotabus::{ERROR_CAP, Redactor, Secret, State};

#[test]
fn secret_display_and_debug_print_stars() {
    let s = Secret::new(FAKE_SK);
    assert_eq!(format!("{s}"), "***");
    let dbg = format!("{s:?}");
    assert!(dbg.contains("***"), "{dbg}");
    assert!(!dbg.contains(FAKE_SK), "{dbg}");
    assert_eq!(
        s.expose(),
        FAKE_SK,
        "the value is still there for the one header that needs it"
    );

    #[derive(Debug)]
    #[allow(dead_code)]
    struct Holder {
        key: Secret,
    }
    let held = format!(
        "{:?}",
        Holder {
            key: Secret::new(FAKE_PLAIN)
        }
    );
    assert!(!held.contains(FAKE_PLAIN), "{held}");
}

#[test]
fn redactor_scrubs_a_known_value_that_matches_no_pattern() {
    let r = Redactor::new(vec![Secret::new(FAKE_PLAIN)]);
    let out = r.redact(&format!("invalid api key: {FAKE_PLAIN} (check it)"));
    assert!(!out.contains(FAKE_PLAIN), "{out}");
    assert!(
        out.starts_with("invalid api key: "),
        "text around the value survives: {out}"
    );
}

#[test]
fn redactor_scrubs_patterns_it_was_never_told_about() {
    let r = Redactor::new(vec![]);
    let input = "a sk-live-abcdefghijklmnop1234 b ghp_abcdefghijklmnopqrstuvwxyz0123456789 c \
                 Authorization: Bearer eyJhbGciOiJIUzI1NiJ9.payload.sig d";
    let out = r.redact(input);
    assert!(
        leaks(&out, &[]).is_empty(),
        "{out} leaks {:?}",
        leaks(&out, &[])
    );
    for gone in [
        "abcdefghijklmnop1234",
        "abcdefghijklmnopqrstuvwxyz0123456789",
        "eyJhbGciOiJIUzI1NiJ9",
    ] {
        assert!(!out.contains(gone), "{out}");
    }
}

#[test]
fn control_redactor_leaves_clean_text_alone() {
    // known answer: the mutation "replace everything with ***" turns this red
    let r = Redactor::new(vec![Secret::new(FAKE_PLAIN)]);
    let clean = "model kimi-k2.5 not found (task-17, disk-full)";
    assert_eq!(r.redact(clean), clean);
}

#[test]
fn error_text_is_capped_at_200_chars() {
    assert_eq!(ERROR_CAP, 200);
    let r = Redactor::new(vec![]);
    let long = "x".repeat(1000);
    let out = r.redact_error(&long);
    assert!(out.chars().count() <= 200, "{} chars", out.chars().count());
    assert!(out.starts_with("xxxxxxxxxx"));
    // chars, not bytes: a multibyte body is cut on a char boundary and still capped
    let wide = "é".repeat(500);
    let out = r.redact_error(&wide);
    assert!(
        out.chars().count() <= 200 && out.chars().count() >= 100,
        "{} chars",
        out.chars().count()
    );
    // a short error is kept whole
    assert_eq!(
        r.redact_error("Insufficient Balance"),
        "Insufficient Balance"
    );
}

#[test]
fn the_cap_applies_after_redaction_so_a_cut_never_exposes_a_key_prefix() {
    let r = Redactor::new(vec![Secret::new(FAKE_PLAIN)]);
    let body = format!("{}{FAKE_PLAIN}{}", "y".repeat(190), "z".repeat(100));
    let out = r.redact_error(&body);
    assert!(out.chars().count() <= 200);
    assert!(
        !out.contains(&FAKE_PLAIN[..8]),
        "a truncated key is still a leak: {out}"
    );
}

// --- the leak checker's own controls: shown able to fail, and able to pass ---

#[test]
fn control_the_checker_catches_an_unredacted_row() {
    let mut row = common::record(
        "api.zhipu.nusy-product-team.glm-5-3",
        State::AuthFailed,
        Utc::now(),
        60,
    );
    row.error = Some(format!(
        "bad key {FAKE_PLAIN}; header Authorization: Bearer {FAKE_SK}"
    ));
    let json = serde_json::to_string(&row).unwrap();
    let found = leaks(&json, &[FAKE_PLAIN, FAKE_SK]);
    assert!(
        found.len() >= 3,
        "checker must catch the value, the Bearer and the sk- in {json}: {found:?}"
    );
    row.error = Some("bad key ghp_abcdefghij0123456789".into());
    assert!(!leaks(&serde_json::to_string(&row).unwrap(), &[]).is_empty());
}

#[test]
fn control_the_checker_passes_a_redacted_row() {
    let mut row = common::record(
        "api.zhipu.nusy-product-team.glm-5-3",
        State::AuthFailed,
        Utc::now(),
        60,
    );
    row.error = Some("bad key ***; header Authorization: Bearer ***; sk-***".into());
    let json = serde_json::to_string(&row).unwrap();
    assert_eq!(
        leaks(&json, &[FAKE_PLAIN, FAKE_SK]),
        Vec::<String>::new(),
        "{json}"
    );
}
