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
    /// The most-used subscription window's `used_pct` (its name in `window`).
    pub window_pct: Option<f64>,
    pub window: Option<String>,
    /// A subscription's windows (EXP-002): only those the source carried; an absent window is left out, never 0 %.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub windows: Vec<Window>,
}

/// One subscription usage window (DESIGN §2): `five_hour`, `seven_day` or `monthly`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Window {
    pub window: String,
    /// 0–100, as measured.
    pub used_pct: f64,
    /// RFC 3339.
    pub reset_at: Option<String>,
    /// Seconds from `checked_at` to `reset_at`.
    pub reset_in_s: Option<i64>,
    /// `(used_pct / 100) / (elapsed / len)`: 1.0 uses the window up exactly at its reset; above 1, sooner.
    pub pace: Option<f64>,
}

impl Window {
    /// A window measured at `now`: `len_s` is the window's length (None when unknown, so no pace).
    pub fn measured(
        name: &str,
        used_pct: f64,
        reset: Option<DateTime<Utc>>,
        len_s: Option<i64>,
        now: DateTime<Utc>,
    ) -> Window {
        let reset_in_s = reset.map(|r| (r - now).num_seconds());
        let pace = match (reset_in_s, len_s) {
            (Some(left), Some(len)) if len > 0 && len - left > 0 => {
                Some((used_pct / 100.0) / ((len - left) as f64 / len as f64))
            }
            _ => None,
        };
        Window {
            window: name.to_string(),
            used_pct,
            reset_at: reset.map(|r| r.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
            reset_in_s,
            pace,
        }
    }
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
