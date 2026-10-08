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
        todo!("Verdict::exit_code")
    }

    /// The state's snake_case name, `CANNOT-ASSESS` or `UNKNOWN`.
    pub fn label(&self) -> String {
        todo!("Verdict::label")
    }
}

/// Apply the rule to the row for a key (or none) at `now`.
pub fn freshness(row: Option<&Record>, now: DateTime<Utc>) -> Verdict {
    let _ = (row, now);
    todo!("freshness")
}
