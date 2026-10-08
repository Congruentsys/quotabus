//! The selector (DESIGN §3 "selector"): a pure function over status rows plus the static model table in the config
//! (family, cost class, roles, context) that names a healthy model for a role, excluding the families asked. It
//! applies the freshness rule (§2) and never acts (§10 Q6). EXP-003.

use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::config::Config;
use crate::record::Record;

/// `--prefer cheapest|fastest|largest-context`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Prefer {
    /// By the service's `cost_class`.
    Cheapest,
    /// By the row's `latency_ms`, lowest first.
    Fastest,
    /// By the model's context in `[service.context]`, largest first.
    LargestContext,
}

impl FromStr for Prefer {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "cheapest" => Ok(Prefer::Cheapest),
            "fastest" => Ok(Prefer::Fastest),
            "largest-context" => Ok(Prefer::LargestContext),
            other => Err(format!(
                "--prefer {other:?} is not cheapest, fastest or largest-context"
            )),
        }
    }
}

/// What is asked: `--role`, `--exclude-family` (any number), `--prefer`, and the time the freshness rule is applied at.
#[derive(Debug, Clone, PartialEq)]
pub struct Query {
    pub role: String,
    pub exclude_families: Vec<String>,
    pub prefer: Option<Prefer>,
    pub now: DateTime<Utc>,
}

/// One qualifying (service, model), with why it qualified and ranked where it is.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Candidate {
    pub service: String,
    pub provider: String,
    pub model: String,
    pub family: String,
    pub key: String,
    pub reason: String,
}

/// Every qualifying candidate, best first. Empty when nothing qualifies.
pub fn select(rows: &[Record], query: &Query, config: &Config) -> Vec<Candidate> {
    let _ = (rows, query, config);
    todo!("EXP-003: select")
}
