//! Subscription reads by the central probe (DESIGN §4, EXP-002): every account is read from the probe host with the
//! account's own token, taken from the environment by the NAME config gives. Nothing runs on any other host.
//!
//! - Claude (`provider = "anthropic"`): a 1-token `POST <base>/v1/messages` with the setup-token as a Bearer and the
//!   `oauth-2025-04-20` beta; the `anthropic-ratelimit-unified-*` headers give the 5 h and 7 d windows
//!   (`unified_headers`, undocumented). If the reply carries none, `claude -p --output-format stream-json` runs once
//!   with the token in its environment and its `rate_limit_event` gives them (`stream_json`, official).
//! - Copilot (`provider = "github"`): `GET <base>/copilot_internal/user` with the token (`copilot_internal`,
//!   undocumented) gives a `monthly` window and the premium requests remaining.
//!
//! A source runs only when the service's `sources` lists it (SIG-003). A window the source did not carry is left out,
//! never written as 0 %.

use std::process::Stdio;
use std::time::{Duration, Instant};

use chrono::{DateTime, Months, NaiveDate, TimeZone, Utc};
use serde_json::{Value, json};

use crate::classify::{HttpOutcome, Outcome, Thresholds, classify};
use crate::config::ServiceConfig;
use crate::probe::{Auth, Ctx, row, sensitive_str, transport};
use crate::record::{Headroom, ProbeSource, Record, State, Window};
use crate::schedule::QueryKind;
use crate::secret::Secret;

/// The Claude direct read (undocumented headers).
pub const UNIFIED_HEADERS: &str = "unified_headers";
/// The Claude fallback (Claude Code's own output).
pub const STREAM_JSON: &str = "stream_json";
/// The Copilot read (undocumented endpoint).
pub const COPILOT_INTERNAL: &str = "copilot_internal";
/// Every source a subscription service may list.
pub const SOURCES: [&str; 3] = [UNIFIED_HEADERS, STREAM_JSON, COPILOT_INTERNAL];

/// The model of the direct read and the fallback when the service lists none (measured 2026-10-08).
pub const DEFAULT_CLAUDE_MODEL: &str = "claude-haiku-4-5";
const ANTHROPIC_BASE: &str = "https://api.anthropic.com";
const GITHUB_BASE: &str = "https://api.github.com";
const OAUTH_BETA: &str = "oauth-2025-04-20";
const ANTHROPIC_VERSION: &str = "2023-06-01";
/// A constant prompt (DESIGN §6): a row never reveals a user's prompt.
const PROMPT: &str = "Reply with exactly: OK";
const FIVE_HOUR_S: i64 = 18_000;
const SEVEN_DAY_S: i64 = 604_800;
/// The only variables a child process inherits from quotabus's environment, each only if set: the `claude` fallback
/// (EXP-002 r1 F1) and `quotabus alert`'s kanban sinks (EXP-004 r1 F3). Under `doppler run` the environment holds
/// every secret of the project, configured or not; none of it reaches a child.
pub const CHILD_ENV: [&str; 6] = ["PATH", "HOME", "TMPDIR", "USER", "LANG", "TERM"];
/// The fallback `claude -p` run is killed after this long.
const CLAUDE_TIMEOUT: Duration = Duration::from_secs(120);

/// Read one subscription account: always exactly one row, keyed `subscription.<provider>.<account>.<service id>`.
pub(crate) async fn read(ctx: &Ctx<'_>, svc: &ServiceConfig, auth: Auth) -> Record {
    for s in &svc.sources {
        if !SOURCES.contains(&s.as_str()) {
            tracing::warn!(service = %svc.id, source = %s, "unknown subscription source; ignored");
        }
    }
    let listed = |name: &str| svc.sources.iter().any(|s| s == name);
    let r = match svc.provider.as_str() {
        "anthropic" => {
            let (direct, fallback) = (listed(UNIFIED_HEADERS), listed(STREAM_JSON));
            let name = if direct || !fallback {
                UNIFIED_HEADERS
            } else {
                STREAM_JSON
            };
            match (direct || fallback, token(auth)) {
                (false, _) => unmeasured(ctx, svc, name, "no_source", "no Claude source listed"),
                (true, Err(why)) => unmeasured(ctx, svc, name, "secret_unset", &why),
                (true, Ok(t)) => claude(ctx, svc, &t, direct, fallback).await,
            }
        }
        "github" => match (listed(COPILOT_INTERNAL), token(auth)) {
            (false, _) => unmeasured(
                ctx,
                svc,
                COPILOT_INTERNAL,
                "no_source",
                "copilot_internal is not listed in sources",
            ),
            (true, Err(why)) => unmeasured(ctx, svc, COPILOT_INTERNAL, "secret_unset", &why),
            (true, Ok(t)) => copilot(ctx, svc, &t).await,
        },
        other => unmeasured(
            ctx,
            svc,
            "subscription",
            "unsupported_provider",
            &format!("no subscription reader for provider {other:?}"),
        ),
    };
    tracing::info!(key = %r.key, state = r.state.as_str(), probe = %r.probe.name, "subscription read");
    if let Some(e) = &r.error {
        tracing::debug!(key = %r.key, error = %e, "subscription error");
    }
    r
}

fn token(auth: Auth) -> Result<Secret, String> {
    match auth {
        Auth::Key(s) => Ok(s),
        Auth::Unset(Some(n)) => Err(format!(
            "environment variable {n} is unset or empty; not read"
        )),
        Auth::Unset(None) | Auth::None => Err("no secret configured; not read".to_string()),
    }
}

fn source_of(name: &str) -> ProbeSource {
    if name == STREAM_JSON {
        ProbeSource::Official
    } else {
        ProbeSource::Undocumented
    }
}

/// The row for this service, labelled with the source `name`.
fn base_row(ctx: &Ctx<'_>, svc: &ServiceConfig, name: &str) -> Record {
    let mut r = row(ctx, svc, &svc.id, name, QueryKind::Subscription);
    r.probe.source = source_of(name);
    r
}

/// A row written without a usage reading.
fn unmeasured(ctx: &Ctx<'_>, svc: &ServiceConfig, name: &str, why: &str, error: &str) -> Record {
    let mut r = base_row(ctx, svc, name);
    r.state = State::Unknown;
    r.reason = Some(format!("cannot_assess:{why}"));
    r.error = Some(ctx.redactor.redact_error(error));
    r
}

/// The headroom of a set of windows: every window, plus the most-used one as `window` / `window_pct` / `reset_at`.
fn headroom(windows: Vec<Window>, requests_remaining: Option<u64>) -> Option<Headroom> {
    let top = windows
        .iter()
        .max_by(|a, b| a.used_pct.total_cmp(&b.used_pct))
        .cloned();
    let h = Headroom {
        requests_remaining,
        tokens_remaining: None,
        reset_at: top.as_ref().and_then(|w| w.reset_at.clone()),
        window_pct: top.as_ref().map(|w| w.used_pct),
        window: top.map(|w| w.window),
        windows,
    };
    (h != Headroom::default()).then_some(h)
}

/// A unified status word → state: `rejected` exhausts, `allowed_warning` is degraded, anything else is ok.
fn status_state(statuses: &[&str]) -> State {
    if statuses.iter().any(|s| s.eq_ignore_ascii_case("rejected")) {
        State::QuotaExhausted
    } else if statuses
        .iter()
        .any(|s| s.eq_ignore_ascii_case("allowed_warning"))
    {
        State::Degraded
    } else {
        State::Ok
    }
}

fn epoch(secs: i64) -> Option<DateTime<Utc>> {
    Utc.timestamp_opt(secs, 0).single()
}

/// The system prompt Claude Code sends; the direct read needs it (HAZ-003: without it, 429 and no unified headers).
const CLAUDE_CODE_SYSTEM: &str = "You are Claude Code, Anthropic's official CLI for Claude.";

// ── Claude ──────────────────────────────────────────────────────────────────────────────────────────────────────

async fn claude(
    ctx: &Ctx<'_>,
    svc: &ServiceConfig,
    token: &Secret,
    direct: bool,
    fallback: bool,
) -> Record {
    let model = svc
        .models
        .first()
        .map(String::as_str)
        .unwrap_or(DEFAULT_CLAUDE_MODEL);
    let missed = if direct {
        match claude_direct(ctx, svc, token, model).await {
            Ok(r) => return r,
            Err(m) => Some(m),
        }
    } else {
        None
    };
    match (fallback, missed) {
        (true, _) => claude_stream_json(ctx, svc, token, model).await,
        (false, Some((why, error))) => unmeasured(ctx, svc, UNIFIED_HEADERS, &why, &error),
        (false, None) => unmeasured(ctx, svc, UNIFIED_HEADERS, "no_source", "no Claude source"),
    }
}

/// The direct read: `Ok` is the row (a reading, or an auth failure, which is an answer); `Err` is (why, error) when
/// the reply carried no unified headers, so the fallback may run.
async fn claude_direct(
    ctx: &Ctx<'_>,
    svc: &ServiceConfig,
    token: &Secret,
    model: &str,
) -> Result<Record, (String, String)> {
    let base = svc
        .base_url
        .as_deref()
        .unwrap_or(ANTHROPIC_BASE)
        .trim_end_matches('/');
    let req = ctx
        .client
        .post(format!("{base}/v1/messages"))
        .header("anthropic-version", ANTHROPIC_VERSION)
        .header("anthropic-beta", OAUTH_BETA)
        .header(
            reqwest::header::AUTHORIZATION,
            sensitive_str(&format!("Bearer {}", token.expose())),
        )
        .json(&json!({
            "model": model,
            "max_tokens": 1,
            // HAZ-003: without Claude Code's system prompt a setup-token gets 429 with no unified headers
            "system": CLAUDE_CODE_SYSTEM,
            "messages": [{"role": "user", "content": PROMPT}],
        }));
    let started = Instant::now();
    let h = match fetch(req, started).await {
        Ok(h) => h,
        Err((outcome, e)) => {
            let why = classify(&outcome, model, &thresholds(ctx))
                .reason
                .unwrap_or_default();
            let why = why.trim_start_matches("cannot_assess:").to_string();
            return Err((why, ctx.redactor.redact_error(&e)));
        }
    };
    let get = |name: &str| {
        h.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.trim().to_string())
    };
    let mut windows = Vec::new();
    let mut statuses = Vec::new();
    for (tag, name, len) in [
        ("5h", "five_hour", FIVE_HOUR_S),
        ("7d", "seven_day", SEVEN_DAY_S),
    ] {
        let used = get(&format!("anthropic-ratelimit-unified-{tag}-utilization"))
            .and_then(|v| v.parse::<f64>().ok())
            .filter(|u| u.is_finite());
        if let Some(u) = used {
            let reset = get(&format!("anthropic-ratelimit-unified-{tag}-reset"))
                .and_then(|v| v.parse::<i64>().ok())
                .and_then(epoch);
            windows.push(Window::measured(name, u * 100.0, reset, Some(len), ctx.now));
        }
        if let Some(s) = get(&format!("anthropic-ratelimit-unified-{tag}-status")) {
            statuses.push(s);
        }
    }
    let overall = get("anthropic-ratelimit-unified-status");
    if let Some(s) = &overall {
        statuses.push(s.clone());
    }
    let error_body = |h: &HttpOutcome| {
        (!(200..300).contains(&h.status)).then(|| ctx.redactor.redact_error(h.body.trim()))
    };
    if windows.is_empty() && overall.is_none() {
        // no reading; a 401/403 is still the answer about this token, and is never retried through `claude`
        let c = classify(&Outcome::Http(h.clone()), model, &thresholds(ctx));
        if c.state == State::AuthFailed {
            let mut r = base_row(ctx, svc, UNIFIED_HEADERS);
            r.state = State::AuthFailed;
            r.latency_ms = Some(h.latency_ms);
            r.error = error_body(&h);
            return Ok(r);
        }
        let error = format!(
            "HTTP {} carried no anthropic-ratelimit-unified headers{}",
            h.status,
            error_body(&h).map(|b| format!(": {b}")).unwrap_or_default()
        );
        // HAZ-003: a bare 429 is the server refusing the call, not a quota reading; name it so
        let why = if h.status == 429 {
            "http_429_no_unified_headers"
        } else {
            "no_unified_headers"
        };
        return Err((why.to_string(), ctx.redactor.redact_error(&error)));
    }
    let statuses: Vec<&str> = statuses.iter().map(String::as_str).collect();
    let mut r = base_row(ctx, svc, UNIFIED_HEADERS);
    r.state = status_state(&statuses);
    r.latency_ms = Some(h.latency_ms);
    r.headroom = headroom(windows, None);
    r.error = error_body(&h);
    Ok(r)
}

/// The fallback: `claude -p … --output-format stream-json` once, the token only in the child's environment.
async fn claude_stream_json(
    ctx: &Ctx<'_>,
    svc: &ServiceConfig,
    token: &Secret,
    model: &str,
) -> Record {
    let bin = ctx.claude_bin.as_deref().unwrap_or("claude");
    let mut cmd = tokio::process::Command::new(bin);
    cmd.args([
        "-p",
        PROMPT,
        "--output-format",
        "stream-json",
        "--verbose",
        "--model",
        model,
        "--max-turns",
        "1",
    ])
    // no project's settings, hooks or CLAUDE.md are picked up from wherever the probe was started
    .current_dir(std::env::temp_dir())
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::null())
    .kill_on_drop(true);
    // r1 F1: the child gets an allowlist only, never the probe's environment (under `doppler run` that holds every
    // Doppler secret, configured or not), plus this account's token. HOME still carries the user-level ~/.claude
    // settings (and its login), which `claude` reads; the token in CLAUDE_CODE_OAUTH_TOKEN takes precedence.
    cmd.env_clear();
    for n in CHILD_ENV {
        if let Some(v) = std::env::var_os(n) {
            cmd.env(n, v);
        }
    }
    cmd.env("CLAUDE_CODE_OAUTH_TOKEN", token.expose());
    let started = Instant::now();
    let out = match cmd.spawn() {
        Ok(child) => tokio::time::timeout(CLAUDE_TIMEOUT, child.wait_with_output()).await,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return unmeasured(
                ctx,
                svc,
                STREAM_JSON,
                "claude_not_found",
                "no claude binary (QUOTABUS_CLAUDE_BIN or claude on PATH)",
            );
        }
        Err(e) => {
            return unmeasured(
                ctx,
                svc,
                STREAM_JSON,
                "claude_failed",
                &format!("cannot run claude: {e}"),
            );
        }
    };
    let out = match out {
        Ok(Ok(o)) => o,
        Ok(Err(e)) => {
            return unmeasured(
                ctx,
                svc,
                STREAM_JSON,
                "claude_failed",
                &format!("claude: {e}"),
            );
        }
        Err(_) => {
            return unmeasured(ctx, svc, STREAM_JSON, "timeout", "claude -p did not finish");
        }
    };
    let latency = started.elapsed().as_millis() as u64;
    let event = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l.trim()).ok())
        .rfind(|v| v["type"] == "rate_limit_event");
    let Some(event) = event else {
        let (why, error) = if out.status.success() {
            (
                "no_rate_limit_event",
                "claude -p printed no rate_limit_event",
            )
        } else {
            ("claude_failed", "claude -p exited non-zero")
        };
        return unmeasured(ctx, svc, STREAM_JSON, why, error);
    };
    let info = &event["rate_limit_info"];
    let mut windows = Vec::new();
    for (name, len) in [("five_hour", FIVE_HOUR_S), ("seven_day", SEVEN_DAY_S)] {
        let w = &info["unifiedWindows"][name];
        if let Some(u) = w["utilization"].as_f64().filter(|u| u.is_finite()) {
            let reset = w["resetsAt"].as_i64().and_then(epoch);
            windows.push(Window::measured(name, u * 100.0, reset, Some(len), ctx.now));
        }
    }
    let mut r = base_row(ctx, svc, STREAM_JSON);
    r.state = status_state(&[info["status"].as_str().unwrap_or("")]);
    r.latency_ms = Some(latency);
    r.headroom = headroom(windows, None);
    r
}

// ── Copilot ─────────────────────────────────────────────────────────────────────────────────────────────────────

async fn copilot(ctx: &Ctx<'_>, svc: &ServiceConfig, token: &Secret) -> Record {
    let base = svc
        .base_url
        .as_deref()
        .unwrap_or(GITHUB_BASE)
        .trim_end_matches('/');
    let req = ctx
        .client
        .get(format!("{base}/copilot_internal/user"))
        .header(
            reqwest::header::AUTHORIZATION,
            sensitive_str(&format!("Bearer {}", token.expose())),
        )
        .header(reqwest::header::ACCEPT, "application/json")
        .header(
            reqwest::header::USER_AGENT,
            format!("quotabus/{}", crate::VERSION),
        );
    let started = Instant::now();
    let mut r = base_row(ctx, svc, COPILOT_INTERNAL);
    let h = match fetch(req, started).await {
        Ok(h) => h,
        Err((outcome, e)) => {
            let c = classify(&outcome, &svc.id, &thresholds(ctx));
            r.reason = c.reason;
            r.error = Some(ctx.redactor.redact_error(&e));
            return r;
        }
    };
    r.latency_ms = Some(h.latency_ms);
    if !(200..300).contains(&h.status) {
        let c = classify(&Outcome::Http(h.clone()), &svc.id, &thresholds(ctx));
        r.state = c.state;
        r.reason = c.reason;
        if r.state == State::ModelMissing {
            r.state = State::Unknown;
            r.reason = Some("cannot_assess:not_found".to_string());
        }
        r.error = Some(ctx.redactor.redact_error(h.body.trim()));
        return r;
    }
    match copilot_reading(&h.body, ctx.now) {
        Some((state, windows, remaining)) => {
            r.state = state;
            r.headroom = headroom(windows, remaining);
        }
        None => {
            r.reason = Some("cannot_assess:unparseable".to_string());
            r.error = Some("copilot_internal/user: no quota_snapshots.premium_interactions".into());
        }
    }
    r
}

/// The state, `monthly` window and requests remaining from a `copilot_internal/user` body.
fn copilot_reading(body: &str, now: DateTime<Utc>) -> Option<(State, Vec<Window>, Option<u64>)> {
    let v: Value = serde_json::from_str(body).ok()?;
    let p = v.get("quota_snapshots")?.get("premium_interactions")?;
    if !p.is_object() {
        return None;
    }
    if p["unlimited"].as_bool() == Some(true) {
        return Some((State::Ok, Vec::new(), None));
    }
    let remaining = p["remaining"].as_f64();
    let pct_left = p["percent_remaining"].as_f64().or_else(|| {
        let ent = p["entitlement"].as_f64().filter(|e| *e > 0.0)?;
        Some(remaining? / ent * 100.0)
    });
    if remaining.is_none() && pct_left.is_none() {
        return None;
    }
    let reset = v["quota_reset_date_utc"]
        .as_str()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|d| d.with_timezone(&Utc))
        .or_else(|| {
            let d = NaiveDate::parse_from_str(v["quota_reset_date"].as_str()?, "%Y-%m-%d").ok()?;
            Some(d.and_hms_opt(0, 0, 0)?.and_utc())
        });
    let len = reset.and_then(|r| {
        let start = r.checked_sub_months(Months::new(1))?;
        Some((r - start).num_seconds())
    });
    let windows = pct_left
        .filter(|p| p.is_finite())
        .map(|left| {
            vec![Window::measured(
                "monthly",
                (100.0 - left).clamp(0.0, 100.0),
                reset,
                len,
                now,
            )]
        })
        .unwrap_or_default();
    let state = if remaining.is_some_and(|n| n <= 0.0) {
        State::QuotaExhausted
    } else {
        State::Ok
    };
    Some((state, windows, remaining.map(|n| n.max(0.0) as u64)))
}

// ── shared ──────────────────────────────────────────────────────────────────────────────────────────────────────

fn thresholds(ctx: &Ctx<'_>) -> Thresholds {
    Thresholds {
        degraded_latency_ms: ctx.config.probe.degraded_latency_ms,
    }
}

/// Send `req`: the HTTP outcome, or the transport failure and its (unredacted) description.
async fn fetch(
    req: reqwest::RequestBuilder,
    started: Instant,
) -> Result<HttpOutcome, (Outcome, String)> {
    let resp = req.send().await.map_err(|e| {
        let (o, d) = transport(e);
        (o, d.unwrap_or_default())
    })?;
    let status = resp.status().as_u16();
    let headers = resp
        .headers()
        .iter()
        .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();
    let body = resp.text().await.map_err(|e| {
        let (o, d) = transport(e);
        (o, d.unwrap_or_default())
    })?;
    Ok(HttpOutcome {
        status,
        headers,
        body,
        latency_ms: started.elapsed().as_millis() as u64,
    })
}
