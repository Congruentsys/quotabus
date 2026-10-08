//! Query kinds and their schedules (CHORE-007): each kind of query has its own interval and TTL, and runs only when
//! it is due.

use chrono::{DateTime, Utc};

use crate::record::Record;

/// One kind of query a probe cycle makes, each on its own `[intervals]` / `[ttl]` key. EXP-002 adds `Subscription`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QueryKind {
    /// The messages / chat-completions probe (`[intervals] api`).
    Api,
    /// The balance endpoint GET (`[intervals] balance`).
    Balance,
    /// EXP-002: a per-host subscription read by `quotabus agent` (`[intervals] subscription`, default 5 min; its TTL
    /// `[ttl] subscription`, default 3 × its own interval). STUB: not yet wired into config.
    Subscription,
}

/// The model slot (and `probe.name`) of a service's balance row: `<kind>.<provider>.<account>.balance`. The store
/// must hold the balance's last read for the balance kind to have a schedule of its own.
pub const BALANCE: &str = "balance";

/// The `reason`s of a row written without making a call (no key, no endpoint). Such a row is not a run: its kind
/// stays due, so a key that arrives is used at the next tick, not an interval later.
const NO_CALL: [&str; 2] = ["cannot_assess:secret_unset", "cannot_assess:no_endpoint"];

/// Due at `now` when there is no last row, the last row made no call, or `interval` has elapsed since it.
pub fn is_due(last: Option<&Record>, interval: std::time::Duration, now: DateTime<Utc>) -> bool {
    let Some(row) = last else {
        return true;
    };
    if row.reason.as_deref().is_some_and(|r| NO_CALL.contains(&r)) {
        return true;
    }
    // an interval too large to represent is never due again (only `--force` runs it)
    chrono::Duration::from_std(interval)
        .ok()
        .and_then(|i| row.checked_at.checked_add_signed(i))
        .is_some_and(|at| at <= now)
}
