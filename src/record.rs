//! The status record (DESIGN §2).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// The record contract version.
pub const CONTRACT: &str = "ai-status/1";

/// What a probe measured. Serialised snake_case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    Ok,
    AuthFailed,
    ModelMissing,
    QuotaExhausted,
    RateLimited,
    Degraded,
    Unknown,
}

/// `api` | `subscription` | `local`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Api,
    Subscription,
    Local,
}

/// `official` | `undocumented` | `file` | `header`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProbeSource {
    Official,
    Undocumented,
    File,
    Header,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Probe {
    pub name: String,
    pub source: ProbeSource,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Balance {
    pub amount: f64,
    pub currency: String,
    /// The endpoint the balance came from (never a query string).
    pub source: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Headroom {
    pub requests_remaining: Option<u64>,
    pub tokens_remaining: Option<u64>,
    pub reset_at: Option<String>,
    pub window_pct: Option<f64>,
    pub window: Option<String>,
}

/// One record per (kind, provider, account, model).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Record {
    pub contract: String,
    pub key: String,
    pub kind: Kind,
    pub provider: String,
    pub account: String,
    pub model: String,
    pub family: String,
    pub state: State,
    /// Required when `state == unknown`: `cannot_assess:<why>`.
    pub reason: Option<String>,
    pub balance: Option<Balance>,
    pub headroom: Option<Headroom>,
    pub latency_ms: Option<u64>,
    pub probe: Probe,
    /// Redacted, at most `ERROR_CAP` chars.
    pub error: Option<String>,
    pub checked_at: DateTime<Utc>,
    pub ttl_s: u64,
    pub observed_by: String,
}

/// A KV-key-safe slug: every char outside `[A-Za-z0-9_-]` becomes `-`.
pub fn slug(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect()
}

/// `<kind>.<provider>.<account>.<model>`, each part a slug.
pub fn record_key(kind: Kind, provider: &str, account: &str, model: &str) -> String {
    format!(
        "{}.{}.{}.{}",
        kind.as_str(),
        slug(provider),
        slug(account),
        slug(model)
    )
}

impl State {
    /// The snake_case name, as serialised.
    pub fn as_str(self) -> &'static str {
        match self {
            State::Ok => "ok",
            State::AuthFailed => "auth_failed",
            State::ModelMissing => "model_missing",
            State::QuotaExhausted => "quota_exhausted",
            State::RateLimited => "rate_limited",
            State::Degraded => "degraded",
            State::Unknown => "unknown",
        }
    }
}

impl Kind {
    /// The snake_case name, as serialised and as the first token of a key.
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Api => "api",
            Kind::Subscription => "subscription",
            Kind::Local => "local",
        }
    }
}

impl ProbeSource {
    pub fn as_str(self) -> &'static str {
        match self {
            ProbeSource::Official => "official",
            ProbeSource::Undocumented => "undocumented",
            ProbeSource::File => "file",
            ProbeSource::Header => "header",
        }
    }
}
