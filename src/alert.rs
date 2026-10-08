//! `quotabus alert` (DESIGN §3 alert, §10 Q8; EXP-004): one item per crossing, never auto-closed.
//!
//! **Crossing-dedup.** For each configured (service, slot) key the alert keeps `alert.<key>` in the same store,
//! holding the last alerted state ([`AlertEntry`]). A fresh measured state that is not `ok` and differs from the last
//! alerted state is a crossing: it is filed to every configured sink (and printed on stdout), then recorded. Only a
//! fresh measured `ok` clears the entry (it is rewritten as `ok`, which re-arms the key); CANNOT-ASSESS (a row whose
//! state is `unknown`, an unreadable row) and UNKNOWN (absent, expired — the freshness rule, §2) never clear it and
//! never file. Nothing is ever closed, moved or updated on a board: the only call is `create`.
//!
//! [INFERENCE] decisions the design does not make, documented in `docs/DESIGN.md` §3 alert:
//! - A move from one bad state to a different bad state (`model_missing` → `auth_failed`) is a new crossing and files
//!   again: the entry holds "the last alerted STATE", and the operator's fix differs per state. The same bad state
//!   across any number of cycles, with CANNOT-ASSESS or UNKNOWN in between, files once.
//! - `degraded` and `rate_limited` are not `ok`, so they are crossings like any other bad state.
//! - Partial sink failure: the crossing is recorded only when every sink succeeded. The sinks that did succeed are
//!   remembered in the entry's `pending`, so the next run retries only the ones that failed — a failed webhook does
//!   not file a second kanban item.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::freshness::Verdict;
use crate::record::{Record, State};

/// The `contract` of an `alert.<key>` entry.
pub const ALERT_CONTRACT: &str = "quotabus-alert/1";

/// The store key of `key`'s dedup state.
pub fn alert_key(key: &str) -> String {
    format!("{}{key}", crate::backend::ENTRY_PREFIX)
}

/// The value at `alert.<key>`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlertEntry {
    pub contract: String,
    pub key: String,
    /// The last state fully alerted (every sink succeeded); `ok` once a measured `ok` re-armed the key.
    pub state: State,
    /// When this entry was written.
    pub at: DateTime<Utc>,
    /// A crossing some sinks have filed and others have not yet: retried next run, on the missing sinks only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending: Option<Pending>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pending {
    pub state: State,
    /// The sinks (`nusy-kanban`, `yurtle-kanban`, `webhook`) that already filed it.
    pub filed: Vec<String>,
}

impl AlertEntry {
    pub fn new(key: &str, state: State, pending: Option<Pending>, at: DateTime<Utc>) -> Self {
        AlertEntry {
            contract: ALERT_CONTRACT.to_string(),
            key: key.to_string(),
            state,
            at,
            pending,
        }
    }
}

/// What to do for one key.
#[derive(Debug, Clone, PartialEq)]
pub enum Decision {
    /// No crossing, nothing to clear.
    Nothing,
    /// A measured `ok` after an alert: rewrite the entry as `ok` (re-armed). Nothing is filed or closed.
    Clear,
    /// A crossing into `state`: file it to every sink not in `skip` (those already filed it, see [`Pending`]).
    File { state: State, skip: Vec<String> },
}

/// The crossing rule for one key: its verdict under the freshness rule and its current entry.
pub fn decide(verdict: &Verdict, entry: Option<&AlertEntry>) -> Decision {
    let Verdict::State { state, .. } = verdict else {
        // CANNOT-ASSESS and UNKNOWN never clear and never file
        return Decision::Nothing;
    };
    if *state == State::Ok {
        return match entry {
            Some(e) if e.state != State::Ok || e.pending.is_some() => Decision::Clear,
            _ => Decision::Nothing,
        };
    }
    let last = entry.map(|e| e.state).unwrap_or(State::Ok);
    if *state == last {
        return Decision::Nothing;
    }
    let skip = entry
        .and_then(|e| e.pending.as_ref())
        .filter(|p| p.state == *state)
        .map(|p| p.filed.clone())
        .unwrap_or_default();
    Decision::File {
        state: *state,
        skip,
    }
}

/// One crossing, as every sink describes it. Built from the row; every text field is redacted before it is used.
#[derive(Debug, Clone, Serialize)]
pub struct Crossing {
    pub contract: &'static str,
    pub key: String,
    pub service: String,
    pub model: String,
    pub provider: String,
    pub account: String,
    pub family: String,
    pub state: State,
    pub reason: Option<String>,
    pub error: Option<String>,
    pub checked_at: DateTime<Utc>,
    pub observed_by: String,
    pub title: String,
}

impl Crossing {
    /// `redact` is applied to every free-text field the row carries (its error and reason).
    pub fn new(
        service: &str,
        model: &str,
        row: &Record,
        redact: impl Fn(&str) -> String,
    ) -> Crossing {
        let title = format!(
            "provider-status: {service} {model} is {}",
            row.state.as_str()
        );
        Crossing {
            contract: ALERT_CONTRACT,
            key: row.key.clone(),
            service: service.to_string(),
            model: model.to_string(),
            provider: row.provider.clone(),
            account: row.account.clone(),
            family: row.family.clone(),
            state: row.state,
            reason: row.reason.as_deref().map(&redact),
            error: row.error.as_deref().map(&redact),
            checked_at: row.checked_at,
            observed_by: row.observed_by.clone(),
            title: redact(&title),
        }
    }

    /// The kanban item's body (markdown, on the sink's stdin).
    pub fn body(&self) -> String {
        let mut b = format!(
            "quotabus measured **{} {}** as `{}`.\n\n\
             - key: `{}`\n- state: `{}`\n- service: {} (provider {}, account {}, family {})\n",
            self.service,
            self.model,
            self.state.as_str(),
            self.key,
            self.state.as_str(),
            self.service,
            self.provider,
            self.account,
            self.family,
        );
        if let Some(r) = &self.reason {
            b.push_str(&format!("- reason: {r}\n"));
        }
        if let Some(e) = &self.error {
            b.push_str(&format!("- error: {e}\n"));
        }
        b.push_str(&format!(
            "- checked_at: {}\n- observed_by: {}\n\n\
             One item per crossing (quotabus DESIGN §3 alert): nothing more is filed for this key until a measured \
             `ok` re-arms it, and quotabus never closes this item. Close it by hand once the service is fixed; \
             `quotabus status --check {}` reads it now.\n",
            self.checked_at.to_rfc3339(),
            self.observed_by,
            self.service,
        ));
        b
    }
}

/// The argv (after the command) of a kanban sink's create. nusy-kanban takes its flags before the positionals
/// (`nusy-kanban create --help`); yurtle-kanban takes `create <type> <title>` then `--push` (an atomic, shared write:
/// never `--no-push`, never `--assign`).
pub fn kanban_args(board: Board, item_type: &str, tags: &[String], title: &str) -> Vec<String> {
    let mut flags = Vec::new();
    if !tags.is_empty() {
        flags.push("--tags".to_string());
        flags.push(tags.join(","));
    }
    flags.push("--body-file".to_string());
    flags.push("-".to_string());
    let create = "create".to_string();
    match board {
        Board::NusyKanban => {
            let mut a = vec![create];
            a.extend(flags);
            a.push(item_type.to_string());
            a.push(title.to_string());
            a
        }
        Board::YurtleKanban => {
            let mut a = vec![
                create,
                item_type.to_string(),
                title.to_string(),
                "--push".to_string(),
            ];
            a.extend(flags);
            a
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Board {
    NusyKanban,
    YurtleKanban,
}

impl Board {
    /// The sink's name, as `[alert.<name>]` and [`Pending::filed`] write it.
    pub fn name(self) -> &'static str {
        match self {
            Board::NusyKanban => "nusy-kanban",
            Board::YurtleKanban => "yurtle-kanban",
        }
    }
}

/// The webhook sink's name.
pub const WEBHOOK: &str = "webhook";

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh(state: State) -> Verdict {
        Verdict::State {
            state,
            age: chrono::Duration::zero(),
        }
    }

    fn entry(state: State, pending: Option<Pending>) -> AlertEntry {
        AlertEntry::new("k", state, pending, Utc::now())
    }

    #[test]
    fn the_crossing_rule() {
        use State::*;
        let file = |s| Decision::File {
            state: s,
            skip: vec![],
        };
        assert_eq!(decide(&fresh(ModelMissing), None), file(ModelMissing));
        assert_eq!(
            decide(&fresh(ModelMissing), Some(&entry(Ok, None))),
            file(ModelMissing)
        );
        assert_eq!(
            decide(&fresh(ModelMissing), Some(&entry(ModelMissing, None))),
            Decision::Nothing
        );
        // [INFERENCE] bad -> a different bad files again
        assert_eq!(
            decide(&fresh(AuthFailed), Some(&entry(ModelMissing, None))),
            file(AuthFailed)
        );
        assert_eq!(
            decide(&fresh(Ok), Some(&entry(ModelMissing, None))),
            Decision::Clear
        );
        assert_eq!(decide(&fresh(Ok), None), Decision::Nothing);
        assert_eq!(
            decide(&fresh(Ok), Some(&entry(Ok, None))),
            Decision::Nothing
        );
        let cannot = Verdict::CannotAssess {
            reason: "cannot_assess:x".into(),
        };
        let unknown = Verdict::Unknown {
            reason: crate::freshness::UnknownReason::Expired,
        };
        for v in [cannot, unknown] {
            assert_eq!(
                decide(&v, Some(&entry(ModelMissing, None))),
                Decision::Nothing
            );
            assert_eq!(decide(&v, None), Decision::Nothing);
        }
    }

    #[test]
    fn a_partial_failure_retries_only_the_missing_sinks() {
        let p = Pending {
            state: State::ModelMissing,
            filed: vec!["nusy-kanban".into()],
        };
        let e = entry(State::Ok, Some(p));
        assert_eq!(
            decide(&fresh(State::ModelMissing), Some(&e)),
            Decision::File {
                state: State::ModelMissing,
                skip: vec!["nusy-kanban".into()]
            }
        );
        // a different bad state is a different crossing: every sink files it
        assert_eq!(
            decide(&fresh(State::AuthFailed), Some(&e)),
            Decision::File {
                state: State::AuthFailed,
                skip: vec![]
            }
        );
        // a measured ok drops the pending crossing
        assert_eq!(decide(&fresh(State::Ok), Some(&e)), Decision::Clear);
    }

    #[test]
    fn kanban_argv_shapes() {
        let tags = vec!["provider-status".to_string(), "infra".to_string()];
        assert_eq!(
            kanban_args(Board::NusyKanban, "signal", &tags, "t"),
            [
                "create",
                "--tags",
                "provider-status,infra",
                "--body-file",
                "-",
                "signal",
                "t"
            ]
        );
        assert_eq!(
            kanban_args(Board::YurtleKanban, "signal", &tags[..1], "t"),
            [
                "create",
                "signal",
                "t",
                "--push",
                "--tags",
                "provider-status",
                "--body-file",
                "-"
            ]
        );
    }
}
