//! `quotabus.toml` (DESIGN §3). A `secret` is a NAME; its value comes from the environment.

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::record::Kind;

#[derive(Debug)]
pub struct ConfigError(pub String);

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ConfigError {}

/// `[bus]`. Omitted ⇒ the file backend (DESIGN §8).
#[derive(Debug, Clone, PartialEq)]
pub struct BusConfig {
    pub url: String,
    /// Default `ai_status`.
    pub bucket: String,
}

/// `[file]`: where the file backend keeps `<key>.json`. Default `~/.local/state/quotabus`.
#[derive(Debug, Clone, PartialEq)]
pub struct FileConfig {
    pub dir: PathBuf,
}

/// `[probe]`.
#[derive(Debug, Clone, PartialEq)]
pub struct ProbeConfig {
    /// Default 15m.
    pub interval: Duration,
    /// Default 45m; written to each row as `ttl_s`.
    pub ttl: Duration,
    /// Default 20.
    pub max_tokens: u32,
    /// A 200 slower than this reads `degraded`. Default 10000.
    pub degraded_latency_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    /// `POST <base>/v1/messages`.
    Anthropic,
    /// `POST <base>/chat/completions`.
    Openai,
}

/// `[service.balance]`.
#[derive(Debug, Clone, PartialEq)]
pub struct BalanceEndpoint {
    pub url: String,
    /// e.g. `balance_infos[0].total_balance`.
    pub path: String,
    pub currency: String,
    /// Default 0.
    pub floor: f64,
}

/// `balance = "none"` (or omitted), or a `[service.balance]` table.
#[derive(Debug, Clone, PartialEq)]
pub enum BalanceSpec {
    None,
    Endpoint(BalanceEndpoint),
}

/// One `[[service]]`.
#[derive(Debug, Clone, PartialEq)]
pub struct ServiceConfig {
    pub id: String,
    pub kind: Kind,
    pub provider: String,
    pub family: String,
    pub account: String,
    pub base_url: Option<String>,
    pub protocol: Option<Protocol>,
    pub models: Vec<String>,
    pub roles: Vec<String>,
    pub cost_class: Option<String>,
    /// The NAME of the environment variable holding the key.
    pub secret: Option<String>,
    pub balance: BalanceSpec,
    pub sources: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    pub bus: Option<BusConfig>,
    pub file: Option<FileConfig>,
    pub probe: ProbeConfig,
    /// The `[[service]]` array.
    pub services: Vec<ServiceConfig>,
}

impl Config {
    pub fn from_toml_str(text: &str) -> Result<Config, ConfigError> {
        let _ = text;
        todo!("Config::from_toml_str")
    }

    pub fn load(path: &Path) -> Result<Config, ConfigError> {
        let _ = path;
        todo!("Config::load")
    }
}
