//! Shared helpers for the EXP-002 tests (rescoped by the Captain 2026-10-08: every subscription account is read
//! CENTRALLY by `quotabus probe`; nothing runs on other hosts). Fakes only: a wiremock stub for the Anthropic and
//! GitHub endpoints, fake tokens, and a fake `claude` shell script. No real key, no real bus, no real `claude`.
//!
//! The seams these tests fix (for the implementer):
//! - A `kind = "subscription"` service is read by `Runner::run_due` / `quotabus probe` (no longer skipped). Its row
//!   key is `subscription.<provider>.<account>.<service id>` (the service id is the model slot); `account` is the
//!   configured label; `observed_by` is the probe host (`quotabus::observed_by()`); `ttl_s` the subscription TTL;
//!   `checked_at` the cycle's `now`.
//! - A source runs only when the service's `sources` lists it (SIG-003: undocumented sources are opt-in):
//!   `unified_headers` (Claude direct read, labelled `probe.source = undocumented`, `probe.name = unified_headers`),
//!   `stream_json` (Claude fallback, `official`, `probe.name = stream_json`), `copilot_internal` (`undocumented`,
//!   `probe.name = copilot_internal`).
//! - Claude direct read: `POST <base_url>/v1/messages`, `Authorization: Bearer <token>`, `anthropic-beta` including
//!   `oauth-2025-04-20`, body `max_tokens: 1` and `model = models[0]`, no `x-api-key`. Headers
//!   `anthropic-ratelimit-unified-{5h,7d}-{utilization (0..1), reset (epoch s), status}` and
//!   `anthropic-ratelimit-unified-status`. `allowed` → ok, `rejected` → quota_exhausted, 401/403 → auth_failed.
//! - Fallback (the 200 carried no unified headers, and `stream_json` is listed): run the `claude` binary once —
//!   resolved through the Runner's env lookup as `QUOTABUS_CLAUDE_BIN`, else `claude` on PATH — with
//!   `-p … --output-format stream-json`, the token ONLY in the child's env as `CLAUDE_CODE_OAUTH_TOKEN`; read
//!   `rate_limit_event.rate_limit_info.unifiedWindows.{five_hour,seven_day}.{utilization,resetsAt}`. No binary, a
//!   failed run or no event → `unknown`, `cannot_assess:<why>`.
//! - Copilot: `GET <base_url>/copilot_internal/user` with the token in `Authorization`; `monthly` window
//!   (`used_pct = 100 − percent_remaining`, `reset_at` from `quota_reset_date`), `headroom.requests_remaining`.
//! - `headroom.windows[]`: `{window: five_hour|seven_day|monthly, used_pct, reset_at (RFC 3339), reset_in_s,
//!   pace}`, `reset_in_s = reset − checked_at`, `pace = (used_pct/100) / (elapsed/len)`, `elapsed = len −
//!   reset_in_s`, `len` 18 000 s (5 h) / 604 800 s (7 d). A window the source did not carry is left out (never 0).
#![allow(dead_code)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use chrono::{DateTime, TimeZone, Utc};
use quotabus::{Config, Record, Runner};
use serde_json::Value;
use wiremock::{MockServer, Request, ResponseTemplate};

/// A fake setup-token, shaped like one (`sk-ant-oat01-…`) so the pattern scrub sees it too.
pub const FAKE_OAT: &str = "sk-ant-oat01-FAKEnotARealSetupToken-0123456789abcdef";
/// A fake GitHub token that matches NO redaction pattern: only the value scrub can catch it.
pub const FAKE_GH: &str = "fakeQBghTOKEN9c1d2e3fNOTREAL";
pub const CLAUDE_SECRET: &str = "QB_CLAUDE_TOKEN_HANKH95";
pub const GH_SECRET: &str = "QB_GITHUB_TOKEN";
pub const MODEL: &str = "claude-haiku-4-5";

/// The measured 2026-10-08 resets (CHORE-002 / the rescoped item): 5 h 1791495000, 7 d 1792051200.
pub const RESET_5H: i64 = 1_791_495_000;
pub const RESET_7D: i64 = 1_792_051_200;

/// The fake clock: 9 000 s (half the 5 h window) before the measured 5 h reset.
pub fn now() -> DateTime<Utc> {
    Utc.timestamp_opt(RESET_5H - 9000, 0).unwrap()
}

pub fn claude_svc(id: &str, account: &str, base: &str, sources: &[&str]) -> String {
    let src: Vec<String> = sources.iter().map(|s| format!("{s:?}")).collect();
    format!(
        "[[service]]\nid = {id:?}\nkind = \"subscription\"\nprovider = \"anthropic\"\nfamily = \"anthropic\"\n\
         account = {account:?}\nbase_url = {base:?}\nmodels = [{MODEL:?}]\nsecret = {CLAUDE_SECRET:?}\n\
         sources = [{}]\n",
        src.join(", ")
    )
}

pub fn claude_both(base: &str) -> String {
    claude_svc(
        "claude-hankh95",
        "hankh95",
        base,
        &["unified_headers", "stream_json"],
    )
}

pub fn copilot_svc(base: &str, sources: &[&str]) -> String {
    let src: Vec<String> = sources.iter().map(|s| format!("{s:?}")).collect();
    format!(
        "[[service]]\nid = \"copilot\"\nkind = \"subscription\"\nprovider = \"github\"\nfamily = \"openai\"\n\
         account = \"hankh95\"\nbase_url = {base:?}\nsecret = {GH_SECRET:?}\nsources = [{}]\n",
        src.join(", ")
    )
}

pub const CLAUDE_KEY: &str = "subscription.anthropic.hankh95.claude-hankh95";
pub const COPILOT_KEY: &str = "subscription.github.hankh95.copilot";

pub fn config(text: &str) -> Config {
    Config::from_toml_str(text).unwrap_or_else(|e| panic!("test config must parse: {e}\n{text}"))
}

/// A Runner whose env lookup knows the two fake tokens and, if given, `QUOTABUS_CLAUDE_BIN`.
pub fn runner(cfg: Config, claude_bin: Option<&Path>) -> Runner {
    let mut m: HashMap<String, String> = HashMap::new();
    m.insert(CLAUDE_SECRET.into(), FAKE_OAT.into());
    m.insert(GH_SECRET.into(), FAKE_GH.into());
    if let Some(b) = claude_bin {
        m.insert("QUOTABUS_CLAUDE_BIN".into(), b.display().to_string());
    }
    Runner::new(cfg, move |n: &str| m.get(n).cloned())
}

/// The measured header set, as a 200 `/v1/messages` reply (Some = that window's (utilization, reset, status)).
pub fn unified(
    code: u16,
    five: Option<(f64, i64, &str)>,
    seven: Option<(f64, i64, &str)>,
    status: Option<&str>,
) -> ResponseTemplate {
    let mut t = ResponseTemplate::new(code).set_body_json(serde_json::json!({
        "id": "msg_fake", "type": "message", "role": "assistant", "model": MODEL,
        "content": [{"type": "text", "text": "o"}], "stop_reason": "max_tokens",
        "usage": {"input_tokens": 35, "output_tokens": 1}
    }));
    for (w, v) in [("5h", five), ("7d", seven)] {
        if let Some((u, r, s)) = v {
            t = t
                .insert_header(
                    format!("anthropic-ratelimit-unified-{w}-utilization").as_str(),
                    u.to_string().as_str(),
                )
                .insert_header(
                    format!("anthropic-ratelimit-unified-{w}-reset").as_str(),
                    r.to_string().as_str(),
                )
                .insert_header(
                    format!("anthropic-ratelimit-unified-{w}-status").as_str(),
                    s,
                );
        }
    }
    if let Some(s) = status {
        t = t
            .insert_header("anthropic-ratelimit-unified-status", s)
            .insert_header("anthropic-ratelimit-unified-overage-status", "rejected");
    }
    t
}

/// The measured all-allowed reply: 5 h 0.3, 7 d 0.12.
pub fn measured() -> ResponseTemplate {
    unified(
        200,
        Some((0.3, RESET_5H, "allowed")),
        Some((0.12, RESET_7D, "allowed")),
        Some("allowed"),
    )
}

pub async fn requests(s: &MockServer, p: &str) -> Vec<Request> {
    s.received_requests()
        .await
        .unwrap_or_default()
        .into_iter()
        .filter(|r| r.url.path() == p)
        .collect()
}

pub fn find<'a>(rows: &'a [Record], key: &str) -> &'a Record {
    rows.iter().find(|r| r.key == key).unwrap_or_else(|| {
        let keys: Vec<&str> = rows.iter().map(|r| r.key.as_str()).collect();
        panic!("no row {key}; rows: {keys:?}")
    })
}

/// The row as JSON, so `headroom.windows` (a new field) needs no struct in the test to be read.
pub fn json(r: &Record) -> Value {
    serde_json::to_value(r).unwrap()
}

/// The `headroom.windows[]` entry named `name`, or None.
pub fn window(r: &Record, name: &str) -> Option<Value> {
    let v = json(r);
    v["headroom"]["windows"]
        .as_array()
        .and_then(|ws| ws.iter().find(|w| w["window"] == name).cloned())
}

pub fn must_window(r: &Record, name: &str) -> Value {
    window(r, name).unwrap_or_else(|| panic!("no {name} window in {}", json(r)["headroom"]))
}

pub fn approx(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

pub fn f(v: &Value) -> f64 {
    v.as_f64().unwrap_or_else(|| panic!("not a number: {v}"))
}

/// A stream-json transcript with the measured `rate_limit_event` (0.3 / 0.12), or none.
pub fn stream(event: Option<&str>) -> String {
    let mut lines = vec![
        r#"{"type":"system","subtype":"init","model":"claude-haiku-4-5","tools":[]}"#.to_string(),
        r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"ok"}]}}"#.to_string(),
    ];
    if let Some(e) = event {
        lines.push(e.to_string());
    }
    lines.push(
        r#"{"type":"result","subtype":"success","is_error":false,"result":"ok"}"#.to_string(),
    );
    lines.join("\n")
}

pub fn measured_event() -> String {
    format!(
        r#"{{"type":"rate_limit_event","rate_limit_info":{{"status":"allowed","resetsAt":{RESET_5H},"rateLimitType":"five_hour","unifiedWindows":{{"five_hour":{{"utilization":0.3,"resetsAt":{RESET_5H}}},"seven_day":{{"utilization":0.12,"resetsAt":{RESET_7D}}}}}}}}}"#
    )
}

/// A fake `claude` in `dir`: logs its argv and whether `CLAUDE_CODE_OAUTH_TOKEN` held the fake token (never the
/// token itself) to `dir/claude.log`, prints `stdout`, exits `code`.
pub fn fake_claude(dir: &Path, stdout: &str, code: i32) -> PathBuf {
    let p = dir.join("claude");
    let log = dir.join("claude.log");
    let script = format!(
        "#!/bin/sh\nprintf 'argv:%s\\n' \"$*\" >> '{log}'\n\
         if [ \"$CLAUDE_CODE_OAUTH_TOKEN\" = '{FAKE_OAT}' ]; then echo 'env:token-ok' >> '{log}'; \
         else echo 'env:token-missing' >> '{log}'; fi\n\
         cat <<'QB_OUT'\n{stdout}\nQB_OUT\nexit {code}\n",
        log = log.display()
    );
    std::fs::write(&p, script).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    p
}

pub fn claude_log(dir: &Path) -> Vec<String> {
    std::fs::read_to_string(dir.join("claude.log"))
        .map(|t| t.lines().map(str::to_string).collect())
        .unwrap_or_default()
}

pub fn claude_runs(dir: &Path) -> usize {
    claude_log(dir)
        .iter()
        .filter(|l| l.starts_with("argv:"))
        .count()
}
