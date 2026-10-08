//! quotabus: does each LLM key and coding subscription work right now, and how much is left?
//!
//! The record and the freshness rule are `docs/DESIGN.md` §2; the architecture, config and outputs §3; the probe
//! catalogue §4; security §6.

pub mod backend;
pub mod balance;
pub mod classify;
pub mod config;
pub mod freshness;
pub mod probe;
pub mod record;
pub mod redact;
pub mod secret;

pub use backend::{Backend, BackendError, FileBackend, NatsKv};
pub use classify::{Classification, HttpOutcome, Outcome, Thresholds, classify};
pub use config::{Config, ConfigError, ServiceConfig};
pub use freshness::{UnknownReason, Verdict, freshness};
pub use probe::Runner;
pub use record::{
    Balance, CONTRACT, Headroom, Kind, Probe, ProbeSource, Record, State, record_key, slug,
};
pub use redact::{ERROR_CAP, Redactor};
pub use secret::Secret;

/// The crate version, as written into `observed_by`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// This host's short name (the first label of the hostname, as `hostname -s` prints it).
pub fn host_label() -> String {
    todo!("host_label")
}

/// `<host>/quotabus@<version>` (DESIGN §2 `observed_by`).
pub fn observed_by() -> String {
    todo!("observed_by")
}
