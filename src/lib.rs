//! quotabus: does each LLM key and coding subscription work right now, and how much is left?
//!
//! The record and the freshness rule are `docs/DESIGN.md` §2; the architecture, config and outputs §3; the probe
//! catalogue §4; security §6.

pub mod alert;
pub mod backend;
pub mod balance;
pub mod classify;
pub mod config;
pub mod freshness;
pub mod probe;
pub mod record;
pub mod redact;
pub mod schedule;
pub mod secret;
pub mod select;
pub mod subscription;
pub mod yurtle;

pub use backend::{Backend, BackendError, BusUrl, FileBackend, Listing, NatsKv};
pub use classify::{Classification, HttpOutcome, Outcome, Thresholds, classify};
pub use config::{Config, ConfigError, ServiceConfig};
pub use freshness::{UnknownReason, Verdict, freshness};
pub use probe::Runner;
pub use record::{
    Balance, CONTRACT, Headroom, Kind, Probe, ProbeSource, Record, State, Window, record_key, slug,
};
pub use redact::{ERROR_CAP, Redactor};
pub use schedule::QueryKind;
pub use secret::Secret;
pub use select::{Candidate, Prefer, Query, select};

/// The crate version, as written into `observed_by`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// This host's short name (the first label of the hostname, as `hostname -s` prints it).
pub fn host_label() -> String {
    let mut buf = [0u8; 256];
    // SAFETY: `buf` is a valid writable buffer of the length passed; gethostname NUL-terminates on success.
    let rc = unsafe { libc::gethostname(buf.as_mut_ptr().cast(), buf.len()) };
    if rc != 0 {
        return "unknown-host".to_string();
    }
    let end = buf.iter().position(|b| *b == 0).unwrap_or(buf.len());
    let full = String::from_utf8_lossy(&buf[..end]);
    match full.split('.').next() {
        Some(short) if !short.is_empty() => short.to_string(),
        _ => "unknown-host".to_string(),
    }
}

/// `<host>/quotabus@<version>` (DESIGN §2 `observed_by`).
pub fn observed_by() -> String {
    format!("{}/quotabus@{VERSION}", host_label())
}
