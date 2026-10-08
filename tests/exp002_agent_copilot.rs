//! EXP-002 Plan 3 / DESIGN §4 Copilot row / SIG-003 Q4: `quotabus agent` reads Copilot through the host's own `gh`
//! login, `gh api copilot_internal/user` (undocumented): `quota_snapshots.premium_interactions.{remaining,
//! percent_remaining, reset_date}` (else the top-level `quota_reset_date`). `remaining == 0` or "exceeded your monthly
//! quota" ⇒ `quota_exhausted`. Every row is labelled `probe.source = undocumented`, `probe.name = copilot_internal`.
//! The source is OFF by default: a copilot service runs it only when its `sources` lists `copilot_internal`.
//! `gh` here is a fake script on a temp PATH that logs its argv; no real login, no token.

mod exp002;

use exp002::*;

const H1: &str = "qbhost1";
/// A fake GitHub token in `gh`'s error text: the redactor's `ghp_` pattern must scrub it.
const FAKE_GHP: &str = "ghp_FAKEnotARealToken0123456789abcd";

fn user_json(remaining: i64, percent_remaining: f64) -> String {
    serde_json::json!({
        "login": "qb-fake-user",
        "copilot_plan": "individual",
        "quota_reset_date": "2026-11-01",
        "quota_snapshots": {
            "chat": {"entitlement": 0, "remaining": 0, "percent_remaining": 100.0, "unlimited": true},
            "premium_interactions": {
                "entitlement": 300,
                "remaining": remaining,
                "percent_remaining": percent_remaining,
                "unlimited": false,
                "overage_permitted": false,
                "reset_date": "2026-11-01"
            }
        }
    })
    .to_string()
}

fn copilot_only(h: &Host) -> std::path::PathBuf {
    h.config(COPILOT_SVC)
}

#[test]
fn known_answer_remaining_and_monthly_window() {
    let h = Host::new(H1);
    h.fake_gh(&user_json(120, 40.0), "", 0);
    h.agent(&copilot_only(&h), &[]);
    let r = must_row(&h.store(), &copilot_key(H1));
    assert_eq!(r["state"], "ok", "{r}");
    assert_eq!(r["kind"], "subscription");
    assert_eq!(r["provider"], "github");
    assert_eq!(r["account"], H1);
    assert!(
        r["observed_by"]
            .as_str()
            .unwrap()
            .starts_with(&format!("{H1}/quotabus@"))
    );
    assert_eq!(r["headroom"]["requests_remaining"], 120, "{r}");
    let w = window(&r, "monthly");
    assert!(
        approx(f(&w["used_pct"]), 60.0, 1e-9),
        "100 − percent_remaining: {w}"
    );
    assert!(
        w["reset_at"]
            .as_str()
            .is_some_and(|s| s.starts_with("2026-11-01")),
        "reset_date → reset_at: {w}"
    );
}

#[test]
fn every_copilot_row_is_labelled_undocumented() {
    let h = Host::new(H1);
    h.fake_gh(&user_json(120, 40.0), "", 0);
    h.agent(&copilot_only(&h), &[]);
    let r = must_row(&h.store(), &copilot_key(H1));
    assert_eq!(r["probe"]["source"], "undocumented", "{r}");
    assert_eq!(r["probe"]["name"], "copilot_internal");
}

#[test]
fn gh_is_asked_for_copilot_internal_user_with_no_token_on_argv() {
    let h = Host::new(H1);
    h.fake_gh(&user_json(120, 40.0), "", 0);
    h.agent(&copilot_only(&h), &[]);
    let calls = h.gh_calls();
    assert_eq!(calls.len(), 1, "one gh call: {calls:?}");
    let argv = &calls[0];
    assert!(argv.starts_with("api "), "{argv}");
    assert!(argv.contains("copilot_internal/user"), "{argv}");
    for bad in ["Authorization", "token", "ghp_", "gho_"] {
        assert!(!argv.contains(bad), "argv carries {bad:?}: {argv}");
    }
}

#[test]
fn zero_remaining_reads_quota_exhausted() {
    let h = Host::new(H1);
    h.fake_gh(&user_json(0, 0.0), "", 0);
    h.agent(&copilot_only(&h), &[]);
    assert_eq!(
        must_row(&h.store(), &copilot_key(H1))["state"],
        "quota_exhausted"
    );
}

#[test]
fn control_one_remaining_is_not_exhausted() {
    let h = Host::new(H1);
    h.fake_gh(&user_json(1, 0.3), "", 0);
    h.agent(&copilot_only(&h), &[]);
    let r = must_row(&h.store(), &copilot_key(H1));
    assert_ne!(r["state"], "quota_exhausted", "1 left is not 0 left: {r}");
    assert_ne!(r["state"], "unknown", "{r}");
}

#[test]
fn exceeded_your_monthly_quota_reads_quota_exhausted() {
    let h = Host::new(H1);
    h.fake_gh(
        r#"{"message":"You have exceeded your monthly quota for premium requests."}"#,
        "gh: You have exceeded your monthly quota for premium requests. (HTTP 429)",
        1,
    );
    h.agent(&copilot_only(&h), &[]);
    assert_eq!(
        must_row(&h.store(), &copilot_key(H1))["state"],
        "quota_exhausted"
    );
}

#[test]
fn a_gh_that_is_not_logged_in_reads_cannot_assess_and_leaks_nothing() {
    let h = Host::new(H1);
    h.fake_gh(
        "",
        &format!(
            "To get started with GitHub CLI, please run:  gh auth login\nstale token {FAKE_GHP}"
        ),
        4,
    );
    let out = h.agent(&copilot_only(&h), &[]);
    let r = must_row(&h.store(), &copilot_key(H1));
    assert_eq!(r["state"], "unknown", "{r}");
    assert!(
        r["reason"]
            .as_str()
            .unwrap_or("")
            .starts_with("cannot_assess:"),
        "{r}"
    );
    assert_eq!(
        r["probe"]["source"], "undocumented",
        "a failed read is labelled too"
    );
    let all = format!("{r}{}", text(&out));
    assert!(
        !all.contains(FAKE_GHP),
        "the token in gh's stderr leaked:\n{all}"
    );
}

#[test]
fn no_gh_on_path_reads_cannot_assess() {
    let h = Host::new(H1);
    assert!(
        !std::path::Path::new("/usr/bin/gh").exists() && !std::path::Path::new("/bin/gh").exists(),
        "precondition: no gh on the test PATH's system dirs"
    );
    h.agent(&copilot_only(&h), &[]);
    let r = must_row(&h.store(), &copilot_key(H1));
    assert_eq!(r["state"], "unknown", "{r}");
    assert!(
        r["reason"]
            .as_str()
            .unwrap_or("")
            .starts_with("cannot_assess:"),
        "{r}"
    );
}

#[test]
fn an_unparseable_answer_reads_cannot_assess() {
    let h = Host::new(H1);
    h.fake_gh("<html>not json</html>", "", 0);
    h.agent(&copilot_only(&h), &[]);
    let r = must_row(&h.store(), &copilot_key(H1));
    assert_eq!(r["state"], "unknown", "{r}");
    assert!(
        r["reason"]
            .as_str()
            .unwrap_or("")
            .starts_with("cannot_assess:"),
        "{r}"
    );
}

#[test]
fn the_undocumented_source_is_off_by_default() {
    // a copilot service with no `sources`: SIG-003, off in the FOSS default — gh is never run, nothing reads ok
    let h = Host::new(H1);
    h.fake_gh(&user_json(120, 40.0), "", 0);
    let svc = "[[service]]\nid = \"copilot\"\nkind = \"subscription\"\nprovider = \"github\"\n\
               family = \"openai\"\naccount = \"{host}\"\n";
    h.agent(&h.config(svc), &[]);
    assert!(
        h.gh_calls().is_empty(),
        "gh ran with the source off: {:?}",
        h.gh_calls()
    );
    if let Some(r) = row(&h.store(), &copilot_key(H1)) {
        assert_ne!(
            r["state"], "ok",
            "a source that is off measures nothing: {r}"
        );
    }
}

#[test]
fn control_the_same_service_with_the_source_on_runs_gh() {
    let h = Host::new(H1);
    h.fake_gh(&user_json(120, 40.0), "", 0);
    h.agent(&copilot_only(&h), &[]);
    assert_eq!(
        h.gh_calls().len(),
        1,
        "the fake gh is reachable, so 'never run' above is a measurement"
    );
}
