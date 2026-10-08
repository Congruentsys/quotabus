//! The freshness rule, applied by every reader (DESIGN §2).

use chrono::{DateTime, Utc};

use crate::record::{Record, State};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnknownReason {
    Absent,
    Expired,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Verdict {
    /// A fresh measured row: its state and age.
    State { state: State, age: chrono::Duration },
    /// A measured failure to measure (the row's reason), or the bus/bucket could not be read.
    CannotAssess { reason: String },
    /// No row, or one older than its `ttl_s`.
    Unknown { reason: UnknownReason },
}

impl Verdict {
    /// `--check` exit code: 0 ok · 1 not ok · 2 CANNOT-ASSESS · 3 UNKNOWN.
    pub fn exit_code(&self) -> i32 {
        match self {
            Verdict::State {
                state: State::Ok, ..
            } => 0,
            Verdict::State { .. } => 1,
            Verdict::CannotAssess { .. } => 2,
            Verdict::Unknown { .. } => 3,
        }
    }

    /// The state's snake_case name, `CANNOT-ASSESS` or `UNKNOWN`.
    pub fn label(&self) -> String {
        match self {
            Verdict::State { state, .. } => state.as_str().to_string(),
            Verdict::CannotAssess { .. } => "CANNOT-ASSESS".to_string(),
            Verdict::Unknown { .. } => "UNKNOWN".to_string(),
        }
    }

    /// `absent` / `expired` / the row's `cannot_assess:…`; `None` for a measured state.
    pub fn reason(&self) -> Option<String> {
        match self {
            Verdict::State { .. } => None,
            Verdict::CannotAssess { reason } => Some(reason.clone()),
            Verdict::Unknown { reason } => Some(reason.as_str().to_string()),
        }
    }
}

impl UnknownReason {
    pub fn as_str(self) -> &'static str {
        match self {
            UnknownReason::Absent => "absent",
            UnknownReason::Expired => "expired",
        }
    }
}

/// Apply the rule to the row for a key (or none) at `now`.
pub fn freshness(row: Option<&Record>, now: DateTime<Utc>) -> Verdict {
    let Some(row) = row else {
        return Verdict::Unknown {
            reason: UnknownReason::Absent,
        };
    };
    // `checked_at + ttl_s < now`, strict; a ttl too large to represent never expires
    let expires_at = i64::try_from(row.ttl_s)
        .ok()
        .and_then(chrono::Duration::try_seconds)
        .and_then(|ttl| row.checked_at.checked_add_signed(ttl));
    if expires_at.is_some_and(|at| at < now) {
        return Verdict::Unknown {
            reason: UnknownReason::Expired,
        };
    }
    if row.state == State::Unknown {
        return Verdict::CannotAssess {
            reason: row
                .reason
                .clone()
                .unwrap_or_else(|| "cannot_assess:unspecified".to_string()),
        };
    }
    Verdict::State {
        state: row.state,
        age: now - row.checked_at,
    }
}
