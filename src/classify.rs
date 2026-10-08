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
    /// Connection refused / DNS / timeout: the probe could not measure.
    Unreachable,
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

/// Map an outcome for `model` to a state, reason and headroom.
pub fn classify(outcome: &Outcome, model: &str, thresholds: &Thresholds) -> Classification {
    let _ = (outcome, model, thresholds);
    todo!("classify")
}

/// Headroom from `anthropic-ratelimit-*` and `x-ratelimit-*` headers; `None` when there are none.
pub fn headroom_from_headers(headers: &[(String, String)]) -> Option<Headroom> {
    let _ = headers;
    todo!("headroom_from_headers")
}
