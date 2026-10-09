//! CHORE-010 (DESIGN §2 `state`: `degraded` … "or a window ≥ warn %"; steer bucket 2, 2026-10-08): `[probe] warn_pct`
//! (a number, 0 < x ≤ 100, default 90). A row that would otherwise read `ok` reads `degraded` when ANY of its windows
//! (`headroom.windows[].used_pct`, or `headroom.window_pct`) is ≥ `warn_pct`; a worse state is never softened.
//!
//! Known answers: 89.9 → ok, 90 → degraded, 97 → degraded. The control: `warn_pct = 100` and 97 reads ok. The
//! mutations these tests catch: no rule at all (90 / 97 stay ok), `>` for `≥` (90 stays ok), only the top
//! `window_pct` or only `windows[]` read (the "any window" cases), the rule applied to every state (quota_exhausted
//! softened to degraded), a hard-coded 90 (the `warn_pct = 100` control), and an unchecked range (0 / 101 / NaN
//! accepted).
//!
//! Seams fixed here (for the implementer): `Config::probe.warn_pct: f64`; `quotabus::classify::warn_state(state,
//! headroom, warn_pct) -> State`, the pure rule, which an API row (that has only `window_pct`) is checked through.
//! Subscription rows are checked through the public path a real row takes: `Runner::run_due` against a wiremock stub
//! of the unified headers / stream-json / `copilot_internal/user`. Fakes only: fake tokens, no real key, no bus.

mod common;
mod exp002;

use chrono::{Duration, Utc};
use exp002::*;
use quotabus::classify::warn_state;
use quotabus::{Config, Headroom, Kind, Query, Record, State, Window, record_key, select};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ── DoD 1: config ───────────────────────────────────────────────────────────────────────────────────────────────

fn refused(text: &str) -> String {
    match Config::from_toml_str(text) {
        Ok(c) => panic!(
            "config must be refused, but parsed (warn_pct {:?}):\n{text}",
            c.probe.warn_pct
        ),
        Err(e) => e.to_string(),
    }
}

#[test]
fn warn_pct_defaults_to_90() {
    let c = Config::from_toml_str("").expect("an empty config parses");
    assert_eq!(c.probe.warn_pct, 90.0);
    let c = Config::from_toml_str("[probe]\nmax_tokens = 7\n").expect("parses");
    assert_eq!(c.probe.warn_pct, 90.0, "a [probe] table without warn_pct");
}

#[test]
fn warn_pct_is_read_from_probe() {
    for (text, want) in [
        ("85", 85.0),
        ("85.5", 85.5),
        ("100", 100.0),
        ("0.5", 0.5),
        ("90", 90.0),
    ] {
        let toml = format!("[probe]\nwarn_pct = {text}\n");
        let c = Config::from_toml_str(&toml)
            .unwrap_or_else(|e| panic!("warn_pct = {text} must be accepted: {e}"));
        assert_eq!(c.probe.warn_pct, want, "warn_pct = {text}");
    }
}

#[test]
fn an_out_of_range_warn_pct_is_refused_naming_the_key() {
    for bad in ["0", "0.0", "-5", "100.01", "101", "1000", "nan", "inf"] {
        let e = refused(&format!("[probe]\nwarn_pct = {bad}\n"));
        assert!(
            e.contains("warn_pct"),
            "warn_pct = {bad}: the error must name the key: {e}"
        );
        // a key the parser does not know is refused too, and names it: that is not the range check
        assert!(
            !e.contains("unknown field"),
            "warn_pct = {bad}: refused as an unknown key, not as out of range: {e}"
        );
    }
}

#[test]
fn a_non_number_warn_pct_is_refused() {
    let e = refused("[probe]\nwarn_pct = \"ninety\"\n");
    assert!(e.contains("warn_pct"), "{e}");
    assert!(
        !e.contains("unknown field"),
        "refused as an unknown key, not as a non-number: {e}"
    );
}

// ── DoD 2: the rule, pure (an API row carries only `window_pct`) ────────────────────────────────────────────────

fn top_only(pct: f64) -> Headroom {
    Headroom {
        window_pct: Some(pct),
        window: Some("five_hour".into()),
        ..Headroom::default()
    }
}

fn win(name: &str, used: f64) -> Window {
    Window {
        window: name.into(),
        used_pct: used,
        reset_at: None,
        reset_in_s: None,
        pace: None,
    }
}

fn windows_only(ws: &[(&str, f64)]) -> Headroom {
    Headroom {
        windows: ws.iter().map(|(n, u)| win(n, *u)).collect(),
        ..Headroom::default()
    }
}

#[test]
fn window_pct_known_answers() {
    assert_eq!(
        warn_state(State::Ok, Some(&top_only(89.9)), 90.0),
        State::Ok
    );
    assert_eq!(
        warn_state(State::Ok, Some(&top_only(90.0)), 90.0),
        State::Degraded
    );
    assert_eq!(
        warn_state(State::Ok, Some(&top_only(97.0)), 90.0),
        State::Degraded
    );
    assert_eq!(
        warn_state(State::Ok, Some(&top_only(100.0)), 90.0),
        State::Degraded
    );
}

#[test]
fn control_warn_pct_100_leaves_97_ok() {
    assert_eq!(
        warn_state(State::Ok, Some(&top_only(97.0)), 100.0),
        State::Ok
    );
    assert_eq!(
        warn_state(State::Ok, Some(&top_only(99.99)), 100.0),
        State::Ok
    );
    // and 100 is still reached at 100: the threshold is ≥, not "never"
    assert_eq!(
        warn_state(State::Ok, Some(&top_only(100.0)), 100.0),
        State::Degraded
    );
    // the threshold is the argument, not a constant 90
    assert_eq!(
        warn_state(State::Ok, Some(&top_only(85.0)), 80.0),
        State::Degraded
    );
    assert_eq!(
        warn_state(State::Ok, Some(&top_only(85.0)), 90.0),
        State::Ok
    );
}

#[test]
fn any_window_in_windows_counts() {
    // the near-limit window is not the first one, and window_pct is absent
    let h = windows_only(&[("five_hour", 10.0), ("seven_day", 95.0)]);
    assert_eq!(warn_state(State::Ok, Some(&h), 90.0), State::Degraded);
    let h = windows_only(&[("five_hour", 97.0), ("seven_day", 12.0)]);
    assert_eq!(warn_state(State::Ok, Some(&h), 90.0), State::Degraded);
    // negative control: every window under the threshold
    let h = windows_only(&[("five_hour", 89.9), ("seven_day", 50.0)]);
    assert_eq!(warn_state(State::Ok, Some(&h), 90.0), State::Ok);
    // window_pct under, a window over: any of them counts
    let mut h = windows_only(&[("five_hour", 20.0), ("monthly", 91.0)]);
    h.window_pct = Some(20.0);
    assert_eq!(warn_state(State::Ok, Some(&h), 90.0), State::Degraded);
}

#[test]
fn no_headroom_or_no_window_leaves_ok() {
    assert_eq!(warn_state(State::Ok, None, 90.0), State::Ok);
    let h = Headroom {
        requests_remaining: Some(5),
        tokens_remaining: Some(10),
        ..Headroom::default()
    };
    assert_eq!(warn_state(State::Ok, Some(&h), 90.0), State::Ok);
}

#[test]
fn a_worse_state_is_never_softened() {
    for s in [
        State::QuotaExhausted,
        State::RateLimited,
        State::AuthFailed,
        State::ModelMissing,
        State::Unknown,
        State::Degraded,
    ] {
        assert_eq!(
            warn_state(s, Some(&top_only(97.0)), 90.0),
            s,
            "{s:?} at 97 %"
        );
        assert_eq!(
            warn_state(s, Some(&top_only(10.0)), 90.0),
            s,
            "{s:?} at 10 %"
        );
    }
}

// ── DoD 2: through the public path a subscription row takes ─────────────────────────────────────────────────────

async fn unified_row(cfg_prefix: &str, five: f64, seven: f64, overall: &str) -> Record {
    let s = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(unified(
            200,
            Some((five, RESET_5H, overall)),
            Some((seven, RESET_7D, overall)),
            Some(overall),
        ))
        .mount(&s)
        .await;
    let text = format!("{cfg_prefix}{}", claude_both(&s.uri()));
    let rows = runner(config(&text), None).run_due(&[], now(), true).await;
    let r = find(&rows, CLAUDE_KEY).clone();
    assert_eq!(r.probe.name, "unified_headers", "{}", json(&r));
    r
}

fn used(r: &Record, name: &str) -> f64 {
    f(&must_window(r, name)["used_pct"])
}

#[tokio::test]
async fn unified_5h_0_899_reads_ok() {
    let r = unified_row("", 0.899, 0.10, "allowed").await;
    assert_eq!(used(&r, "five_hour"), 89.9, "the window the row carries");
    assert_eq!(r.state, State::Ok, "{}", json(&r));
}

#[tokio::test]
async fn unified_5h_0_90_reads_degraded() {
    let r = unified_row("", 0.90, 0.10, "allowed").await;
    assert_eq!(used(&r, "five_hour"), 90.0);
    assert_eq!(r.state, State::Degraded, "{}", json(&r));
}

#[tokio::test]
async fn unified_5h_0_97_reads_degraded() {
    let r = unified_row("", 0.97, 0.10, "allowed").await;
    assert_eq!(used(&r, "five_hour"), 97.0);
    assert_eq!(r.state, State::Degraded, "{}", json(&r));
}

#[tokio::test]
async fn unified_7d_0_97_reads_degraded_with_5h_low() {
    let r = unified_row("", 0.10, 0.97, "allowed").await;
    assert_eq!(r.state, State::Degraded, "any window: {}", json(&r));
}

#[tokio::test]
async fn control_unified_warn_pct_100_leaves_0_97_ok() {
    let r = unified_row("[probe]\nwarn_pct = 100\n\n", 0.97, 0.10, "allowed").await;
    assert_eq!(used(&r, "five_hour"), 97.0);
    assert_eq!(r.state, State::Ok, "warn_pct = 100: {}", json(&r));
}

#[tokio::test]
async fn unified_warn_pct_is_read_from_config() {
    // 85 % is under the default 90 but over a configured 80
    let r = unified_row("", 0.85, 0.10, "allowed").await;
    assert_eq!(r.state, State::Ok, "default 90: {}", json(&r));
    let r = unified_row("[probe]\nwarn_pct = 80\n\n", 0.85, 0.10, "allowed").await;
    assert_eq!(r.state, State::Degraded, "warn_pct = 80: {}", json(&r));
}

#[tokio::test]
async fn unified_rejected_at_97_stays_quota_exhausted() {
    let r = unified_row("", 0.97, 0.10, "rejected").await;
    assert_eq!(
        r.state,
        State::QuotaExhausted,
        "never softened: {}",
        json(&r)
    );
}

#[tokio::test]
async fn stream_json_0_97_reads_degraded() {
    let s = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(unified(200, None, None, None))
        .mount(&s)
        .await;
    let event = format!(
        r#"{{"type":"rate_limit_event","rate_limit_info":{{"status":"allowed","resetsAt":{RESET_5H},"rateLimitType":"five_hour","unifiedWindows":{{"five_hour":{{"utilization":0.97,"resetsAt":{RESET_5H}}},"seven_day":{{"utilization":0.1,"resetsAt":{RESET_7D}}}}}}}}}"#
    );
    let dir = tempfile::tempdir().unwrap();
    let bin = fake_claude(dir.path(), &stream(Some(&event)), 0);
    let rows = runner(config(&claude_both(&s.uri())), Some(&bin))
        .run_due(&[], now(), true)
        .await;
    let r = find(&rows, CLAUDE_KEY).clone();
    assert_eq!(r.probe.name, "stream_json", "{}", json(&r));
    assert_eq!(used(&r, "five_hour"), 97.0);
    assert_eq!(r.state, State::Degraded, "{}", json(&r));
}

async fn copilot_row(cfg_prefix: &str, premium: serde_json::Value) -> Record {
    let s = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/copilot_internal/user"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "login": "qb-fake-user",
            "copilot_plan": "individual",
            "quota_reset_date": "2026-11-01",
            "quota_snapshots": {"premium_interactions": premium}
        })))
        .mount(&s)
        .await;
    let text = format!(
        "{cfg_prefix}{}",
        copilot_svc(&s.uri(), &["copilot_internal"])
    );
    let rows = runner(config(&text), None).run_due(&[], now(), true).await;
    find(&rows, COPILOT_KEY).clone()
}

#[tokio::test]
async fn copilot_monthly_97_reads_degraded_and_89_9_ok() {
    let r = copilot_row(
        "",
        serde_json::json!({"entitlement": 300, "remaining": 9, "percent_remaining": 3.0, "unlimited": false}),
    )
    .await;
    assert_eq!(used(&r, "monthly"), 97.0);
    assert_eq!(r.state, State::Degraded, "{}", json(&r));

    let r = copilot_row(
        "",
        serde_json::json!({"entitlement": 300, "remaining": 30, "percent_remaining": 10.1, "unlimited": false}),
    )
    .await;
    assert_eq!(used(&r, "monthly"), 89.9);
    assert_eq!(r.state, State::Ok, "{}", json(&r));

    let r = copilot_row(
        "[probe]\nwarn_pct = 100\n\n",
        serde_json::json!({"entitlement": 300, "remaining": 9, "percent_remaining": 3.0, "unlimited": false}),
    )
    .await;
    assert_eq!(r.state, State::Ok, "warn_pct = 100: {}", json(&r));
}

#[tokio::test]
async fn copilot_exhausted_stays_quota_exhausted() {
    let r = copilot_row(
        "",
        serde_json::json!({"entitlement": 300, "remaining": 0, "percent_remaining": 0.0, "unlimited": false}),
    )
    .await;
    assert_eq!(used(&r, "monthly"), 100.0);
    assert_eq!(
        r.state,
        State::QuotaExhausted,
        "never softened: {}",
        json(&r)
    );
}

// ── DoD 3: select refuses such a row; alert files it as any bad state ───────────────────────────────────────────

const TABLE: &str = r#"
[file]
dir = "/nonexistent/quotabus-chore010"

[[service]]
id = "glm"
kind = "api"
provider = "zhipu"
family = "zhipu"
account = "acct"
models = ["glm-5.3"]
roles = ["review"]
cost_class = "metered"
secret = "QB_UNUSED"

[[service]]
id = "kimi"
kind = "api"
provider = "moonshot"
family = "moonshot"
account = "acct"
models = ["kimi-k3"]
roles = ["review"]
cost_class = "metered"
secret = "QB_UNUSED"
"#;

fn api_row(provider: &str, model: &str, state: State, pct: f64) -> Record {
    let now = Utc::now();
    let key = record_key(Kind::Api, provider, "acct", model);
    let mut r = common::record(&key, state, now - Duration::seconds(30), 3600);
    r.model = model.into();
    r.headroom = Some(top_only(pct));
    r
}

fn picks(rows: &[Record]) -> Vec<String> {
    let cfg = Config::from_toml_str(TABLE).expect("table parses");
    let q = Query {
        role: "review".into(),
        exclude_families: vec![],
        prefer: None,
        now: Utc::now(),
    };
    select(rows, &q, &cfg)
        .into_iter()
        .map(|c| c.model)
        .collect()
}

#[test]
fn select_refuses_a_degraded_near_limit_row() {
    let rows = vec![
        api_row("zhipu", "glm-5.3", State::Degraded, 97.0),
        api_row("moonshot", "kimi-k3", State::Ok, 10.0),
    ];
    assert_eq!(picks(&rows), vec!["kimi-k3".to_string()]);
}

#[test]
fn control_select_takes_the_same_row_when_ok() {
    // the refusal above is the state, not the fixture: the same row reading ok is chosen
    let rows = vec![
        api_row("zhipu", "glm-5.3", State::Ok, 97.0),
        api_row("moonshot", "kimi-k3", State::Ok, 10.0),
    ];
    let got = picks(&rows);
    assert!(got.contains(&"glm-5.3".to_string()), "{got:?}");
}

#[test]
fn an_ok_api_row_at_97_through_the_rule_is_refused_by_select() {
    let mut near = api_row("zhipu", "glm-5.3", State::Ok, 97.0);
    near.state = warn_state(near.state, near.headroom.as_ref(), 90.0);
    assert_eq!(near.state, State::Degraded);
    let rows = vec![near, api_row("moonshot", "kimi-k3", State::Ok, 10.0)];
    assert_eq!(picks(&rows), vec!["kimi-k3".to_string()]);
}

// `alert_files_a_near_limit_degraded_as_any_bad_state` was retired by CHORE-018 (Captain on SIG-010): whether a
// degraded row files now depends on `[alert] states` and the row's reason; see tests/chore018_alert_states.rs.

// ── DoD 4: the docs ─────────────────────────────────────────────────────────────────────────────────────────────

fn read(rel: &str) -> String {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

/// The text of DESIGN's section `## <n>.` up to the next `## `.
fn section(design: &str, n: u32) -> String {
    let head = format!("\n## {n}. ");
    let start = design
        .find(&head)
        .unwrap_or_else(|| panic!("DESIGN has no §{n}"));
    let rest = &design[start + 1..];
    let end = rest[3..].find("\n## ").map(|i| i + 3).unwrap_or(rest.len());
    rest[..end].to_string()
}

#[test]
fn design_2_and_3_document_warn_pct_citing_chore_010() {
    let d = read("docs/DESIGN.md");
    for n in [2, 3] {
        let s = section(&d, n);
        assert!(s.contains("warn_pct"), "DESIGN §{n} does not name warn_pct");
        assert!(
            s.contains("CHORE-010"),
            "DESIGN §{n} does not cite CHORE-010"
        );
    }
    let s2 = section(&d, 2);
    assert!(
        s2.contains("steer"),
        "DESIGN §2 does not cite the steer decision"
    );
}

#[test]
fn control_the_section_reader_can_fail() {
    let d = read("docs/DESIGN.md");
    assert!(section(&d, 2).contains("freshness"));
    assert!(!section(&d, 2).contains("## 3."), "§2 must stop before §3");
    assert!(!section(&d, 3).contains("not-a-word-in-design-xyzzy"));
}

#[test]
fn the_example_config_documents_warn_pct() {
    let text = read("examples/quotabus.toml");
    assert!(
        text.contains("warn_pct"),
        "examples/quotabus.toml does not show warn_pct"
    );
    assert!(
        text.contains("CHORE-010"),
        "examples/quotabus.toml does not cite CHORE-010"
    );
    let c = Config::load(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/quotabus.toml"),
    )
    .expect("examples/quotabus.toml parses");
    assert_eq!(c.probe.warn_pct, 90.0, "the example shows the default");
}
