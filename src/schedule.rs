//! Query kinds and their schedules (CHORE-007): each kind of query has its own interval and TTL, and runs only when
//! it is due.

/// One kind of query a probe cycle makes, each on its own `[intervals]` / `[ttl]` key. EXP-002 adds `Subscription`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QueryKind {
    /// The messages / chat-completions probe (`[intervals] api`).
    Api,
    /// The balance endpoint GET (`[intervals] balance`).
    Balance,
}
