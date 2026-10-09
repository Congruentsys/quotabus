//! The selector (DESIGN §3 "selector"): a pure function over status rows plus the static model table in the config
//! (family, cost class, roles, context) that names a healthy model for a role, excluding the families asked. It
//! applies the freshness rule (§2) and never acts (§10 Q6). EXP-003.

use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::config::Config;
use crate::freshness::{Verdict, freshness};
use crate::record::{Kind, Record, State, record_key};

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
///
/// A configured (service, model slot) qualifies when the service is not a subscription seat (SIG-009), lists `query.role`, neither the service's family nor
/// its row's family is excluded, and the freshness rule (§2) applied at `query.now` gives a measured `ok`: an absent,
/// expired, CANNOT-ASSESS or any other state (`degraded` included) is never chosen. Ordering: `--prefer`'s key first,
/// then the others as tie-breakers, then the row key, so the answer is total and repeatable.
pub fn select(rows: &[Record], query: &Query, config: &Config) -> Vec<Candidate> {
    let prefer = query.prefer.unwrap_or(Prefer::Cheapest);
    let excluded = |family: &str| {
        query
            .exclude_families
            .iter()
            .any(|x| x.trim().eq_ignore_ascii_case(family.trim()))
    };
    let mut ranked: Vec<(Rank, Candidate)> = Vec::new();
    for svc in &config.services {
        // a subscription seat is never a candidate, as decided on SIG-009 (steer bucket 2, 2026-10-08: an agent
        // session's decision, not a Captain ruling, open to the Captain's veto; DESIGN §3): its row is per ACCOUNT,
        // with the service id in the model slot, so there is no model to print as `provider model` (review r1 F1)
        if svc.kind == Kind::Subscription
            || !svc.roles.iter().any(|r| r == &query.role)
            || excluded(&svc.family)
        {
            continue;
        }
        for slot in svc.slots() {
            let key = record_key(svc.kind, &svc.provider, &svc.account, &slot);
            // the newest row for the key, should the input hold more than one
            let Some(row) = rows
                .iter()
                .filter(|r| r.key == key)
                .max_by_key(|r| r.checked_at)
            else {
                continue;
            };
            if row.kind == Kind::Subscription || excluded(&row.family) {
                continue;
            }
            let Verdict::State {
                state: State::Ok,
                age,
            } = freshness(Some(row), query.now)
            else {
                continue;
            };
            let context = svc.context.get(&slot).copied();
            let rank = Rank {
                cost: cost_rank(svc.cost_class.as_deref()),
                latency: row.latency_ms,
                context,
            };
            let reason = format!(
                "ok {}s ago (ttl {}s); role {}; family {}; cost {}; latency {}; context {}; ranked by {}",
                age.num_seconds().max(0),
                row.ttl_s,
                query.role,
                svc.family,
                svc.cost_class.as_deref().unwrap_or("unset"),
                row.latency_ms
                    .map(|l| format!("{l} ms"))
                    .unwrap_or_else(|| "unmeasured".to_string()),
                context
                    .map(|c| format!("{c} tokens"))
                    .unwrap_or_else(|| "unset".to_string()),
                prefer.as_str(),
            );
            ranked.push((
                rank,
                Candidate {
                    service: svc.id.clone(),
                    provider: svc.provider.clone(),
                    model: slot.clone(),
                    family: svc.family.clone(),
                    key,
                    reason,
                },
            ));
        }
    }
    ranked.sort_by(|(a, ca), (b, cb)| {
        let cost = a.cost.cmp(&b.cost);
        // None (unmeasured / unset) sorts last in both
        let latency = a
            .latency
            .unwrap_or(u64::MAX)
            .cmp(&b.latency.unwrap_or(u64::MAX));
        let context = b.context.unwrap_or(0).cmp(&a.context.unwrap_or(0));
        let order = match prefer {
            Prefer::Cheapest => cost.then(latency).then(context),
            Prefer::Fastest => latency.then(cost).then(context),
            Prefer::LargestContext => context.then(latency).then(cost),
        };
        order.then_with(|| ca.key.cmp(&cb.key))
    });
    ranked.into_iter().map(|(_, c)| c).collect()
}

/// What a candidate is ordered by.
struct Rank {
    cost: u8,
    latency: Option<u64>,
    context: Option<u64>,
}

/// `cost_class`, cheapest first: `local` (own hardware), `subscription`/`free` (already paid for; a seat itself is
/// never a candidate, SIG-009, but an API service may declare the class), `metered`, then
/// anything else or unset. [INFERENCE: the design names the classes it uses, `local` and `metered`, but no order.]
fn cost_rank(class: Option<&str>) -> u8 {
    match class.map(str::trim) {
        Some("local") => 0,
        Some("subscription") | Some("free") => 1,
        Some("metered") => 2,
        _ => 3,
    }
}

impl Prefer {
    /// The CLI word.
    pub fn as_str(self) -> &'static str {
        match self {
            Prefer::Cheapest => "cheapest",
            Prefer::Fastest => "fastest",
            Prefer::LargestContext => "largest-context",
        }
    }
}
