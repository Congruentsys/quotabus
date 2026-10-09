//! Pure classification of one probe's outcome to a state and headroom (DESIGN §2 `state`, §4).

use crate::record::{Headroom, State};

/// What the HTTP call returned.
#[derive(Debug, Clone, PartialEq)]
pub struct HttpOutcome {
    pub status: u16,
    /// Header (name, value) pairs; names compare case-insensitively.
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub latency_ms: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    Http(HttpOutcome),
    /// Connection refused / DNS / TLS: the probe could not measure.
    Unreachable,
    /// The call did not finish within the client's timeout.
    Timeout,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Thresholds {
    /// A 200 slower than this (strictly greater) reads `degraded`.
    pub degraded_latency_ms: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Classification {
    pub state: State,
    /// `Some("cannot_assess:<why>")` when `state == Unknown`.
    pub reason: Option<String>,
    pub headroom: Option<Headroom>,
    pub latency_ms: Option<u64>,
}

/// Body phrases that mean the account is out of money or quota, whatever the status code says.
const QUOTA_PHRASES: &[&str] = &[
    "exceeded your monthly quota",
    "exceeded your current quota",
    "enforced_spend_limit_reached",
    "insufficient_quota",
    "insufficient balance",
    "credit balance is too low",
];

fn header<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.trim())
}

fn cannot_assess(why: &str, latency_ms: Option<u64>) -> Classification {
    Classification {
        state: State::Unknown,
        reason: Some(format!("cannot_assess:{why}")),
        headroom: None,
        latency_ms,
    }
}

/// Map an outcome for `model` to a state, reason and headroom.
pub fn classify(outcome: &Outcome, model: &str, thresholds: &Thresholds) -> Classification {
    let h = match outcome {
        Outcome::Unreachable => return cannot_assess("unreachable", None),
        Outcome::Timeout => return cannot_assess("timeout", None),
        Outcome::Http(h) => h,
    };
    let latency = Some(h.latency_ms);
    let body = h.body.to_ascii_lowercase();
    let says_quota = QUOTA_PHRASES.iter().any(|p| body.contains(p));
    let retry_window = header(&h.headers, "retry-after").is_some()
        || h.headers.iter().any(|(k, _)| {
            let k = k.to_ascii_lowercase();
            k.starts_with("x-ratelimit-reset")
                || k.starts_with("anthropic-ratelimit-") && k.ends_with("-reset")
        });
    let state = match h.status {
        200..=299 if h.latency_ms > thresholds.degraded_latency_ms => State::Degraded,
        200..=299 => State::Ok,
        400..=499 if says_quota => State::QuotaExhausted,
        401 | 403 => State::AuthFailed,
        402 => State::QuotaExhausted,
        404 if names_the_model(&body, model) => State::ModelMissing,
        404 => return cannot_assess("not_found", latency),
        429 if retry_window => State::RateLimited,
        429 => State::QuotaExhausted,
        400 | 422 => return cannot_assess("bad_request", latency),
        500..=599 => return cannot_assess("server_error", latency),
        other => return cannot_assess(&format!("http_{other}"), latency),
    };
    Classification {
        state,
        reason: None,
        headroom: headroom_from_headers(&h.headers),
        latency_ms: latency,
    }
}

/// A 404 is about the model when its body names the model id, or says "model" at all (a wrong base URL 404s with
/// a body that does neither).
fn names_the_model(body_lower: &str, model: &str) -> bool {
    body_lower.contains(&model.to_ascii_lowercase()) || body_lower.contains("model")
}

fn parse_u64(v: Option<&str>) -> Option<u64> {
    v.and_then(|s| s.parse().ok())
}

/// Headroom from `anthropic-ratelimit-*` and `x-ratelimit-*` headers; `None` when there are none.
pub fn headroom_from_headers(headers: &[(String, String)]) -> Option<Headroom> {
    let get = |name: &str| header(headers, name);
    let h = Headroom {
        requests_remaining: parse_u64(get("anthropic-ratelimit-requests-remaining"))
            .or_else(|| parse_u64(get("x-ratelimit-remaining-requests"))),
        tokens_remaining: parse_u64(get("anthropic-ratelimit-tokens-remaining"))
            .or_else(|| parse_u64(get("x-ratelimit-remaining-tokens"))),
        // an RFC 3339 instant; the `x-ratelimit-reset-*` headers carry a duration (`600ms`), not an instant
        reset_at: get("anthropic-ratelimit-requests-reset")
            .or_else(|| get("anthropic-ratelimit-tokens-reset"))
            .map(str::to_string),
        window_pct: None,
        window: None,
        windows: Vec::new(),
    };
    (h != Headroom::default()).then_some(h)
}

/// CHORE-010 (DESIGN §2 "a window ≥ warn %"): `state` as the warn rule leaves it. An `ok` reads `degraded` when ANY
/// window in `headroom` (`windows[].used_pct`, or `window_pct`) is ≥ `warn_pct`; every other state is returned as it
/// is (a worse state is never softened). STUB (test partner): the implementer writes it and applies it to every row.
pub fn warn_state(state: State, headroom: Option<&Headroom>, warn_pct: f64) -> State {
    let _ = (state, headroom, warn_pct);
    todo!("CHORE-010: warn_state")
}
