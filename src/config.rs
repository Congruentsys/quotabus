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

impl Default for ProbeConfig {
    fn default() -> Self {
        ProbeConfig {
            interval: Duration::from_secs(15 * 60),
            ttl: Duration::from_secs(45 * 60),
            max_tokens: 20,
            degraded_latency_ms: 10_000,
        }
    }
}

/// The default bucket name.
pub const DEFAULT_BUCKET: &str = "ai_status";

/// `<n><unit>` with unit `ms`, `s`, `m`, `h` or `d` (e.g. `90s`, `15m`, `2h`).
pub fn parse_duration(text: &str) -> Result<Duration, ConfigError> {
    let t = text.trim();
    let split = t
        .find(|c: char| !c.is_ascii_digit())
        .ok_or_else(|| ConfigError(format!("duration {text:?} needs a unit (ms, s, m, h, d)")))?;
    let (num, unit) = t.split_at(split);
    let n: u64 = num
        .parse()
        .map_err(|_| ConfigError(format!("duration {text:?} is not <number><unit>")))?;
    let secs = |mult: u64| {
        n.checked_mul(mult)
            .map(Duration::from_secs)
            .ok_or_else(|| ConfigError(format!("duration {text:?} is too large")))
    };
    match unit {
        "ms" => Ok(Duration::from_millis(n)),
        "s" => secs(1),
        "m" => secs(60),
        "h" => secs(3600),
        "d" => secs(86_400),
        _ => Err(ConfigError(format!(
            "duration {text:?}: unknown unit {unit:?} (ms, s, m, h, d)"
        ))),
    }
}

/// The TOML as written; converted to [`Config`] after parsing.
mod raw {
    use serde::Deserialize;

    use crate::record::Kind;

    #[derive(Deserialize)]
    pub struct File {
        pub bus: Option<Bus>,
        pub file: Option<FileTable>,
        pub probe: Option<Probe>,
        #[serde(default)]
        pub service: Vec<Service>,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Bus {
        pub url: String,
        pub bucket: Option<String>,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct FileTable {
        pub dir: String,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Probe {
        pub interval: Option<String>,
        pub ttl: Option<String>,
        pub max_tokens: Option<u32>,
        pub degraded_latency_ms: Option<u64>,
    }

    #[derive(Deserialize, Clone, Copy)]
    #[serde(rename_all = "lowercase")]
    pub enum Protocol {
        Anthropic,
        Openai,
    }

    #[derive(Deserialize)]
    #[serde(untagged)]
    pub enum Balance {
        Word(String),
        Endpoint {
            url: String,
            path: String,
            currency: String,
            floor: Option<f64>,
        },
    }

    #[derive(Deserialize)]
    pub struct Service {
        pub id: String,
        pub kind: Kind,
        pub provider: String,
        pub family: String,
        pub account: String,
        pub base_url: Option<String>,
        pub protocol: Option<Protocol>,
        #[serde(default)]
        pub models: Vec<String>,
        #[serde(default)]
        pub roles: Vec<String>,
        pub cost_class: Option<String>,
        pub secret: Option<String>,
        pub balance: Option<Balance>,
        #[serde(default)]
        pub sources: Vec<String>,
    }
}

impl Config {
    pub fn from_toml_str(text: &str) -> Result<Config, ConfigError> {
        let raw: raw::File =
            toml::from_str(text).map_err(|e| ConfigError(format!("invalid config: {e}")))?;

        let mut probe = ProbeConfig::default();
        if let Some(p) = raw.probe {
            if let Some(i) = p.interval {
                probe.interval = parse_duration(&i)?;
            }
            if let Some(t) = p.ttl {
                probe.ttl = parse_duration(&t)?;
            }
            if let Some(m) = p.max_tokens {
                probe.max_tokens = m;
            }
            if let Some(d) = p.degraded_latency_ms {
                probe.degraded_latency_ms = d;
            }
        }
        if probe.ttl.as_secs() == 0 {
            return Err(ConfigError("[probe] ttl must be at least 1s".into()));
        }

        let mut services = Vec::with_capacity(raw.service.len());
        for s in raw.service {
            if s.id.trim().is_empty() {
                return Err(ConfigError("a [[service]] has an empty id".into()));
            }
            if services.iter().any(|x: &ServiceConfig| x.id == s.id) {
                return Err(ConfigError(format!("service id {:?} appears twice", s.id)));
            }
            let balance = match s.balance {
                None => BalanceSpec::None,
                Some(raw::Balance::Word(w)) if w == "none" => BalanceSpec::None,
                Some(raw::Balance::Word(w)) => {
                    return Err(ConfigError(format!(
                        "service {:?}: balance = {w:?}; use \"none\" or a [service.balance] table",
                        s.id
                    )));
                }
                Some(raw::Balance::Endpoint {
                    url,
                    path,
                    currency,
                    floor,
                }) => BalanceSpec::Endpoint(BalanceEndpoint {
                    url,
                    path,
                    currency,
                    floor: floor.unwrap_or(0.0),
                }),
            };
            services.push(ServiceConfig {
                id: s.id,
                kind: s.kind,
                provider: s.provider,
                family: s.family,
                account: s.account,
                base_url: s.base_url,
                protocol: s.protocol.map(|p| match p {
                    raw::Protocol::Anthropic => Protocol::Anthropic,
                    raw::Protocol::Openai => Protocol::Openai,
                }),
                models: s.models,
                roles: s.roles,
                cost_class: s.cost_class,
                secret: s.secret.filter(|n| !n.is_empty()),
                balance,
                sources: s.sources,
            });
        }

        Ok(Config {
            bus: raw.bus.map(|b| BusConfig {
                url: b.url,
                bucket: b.bucket.unwrap_or_else(|| DEFAULT_BUCKET.to_string()),
            }),
            file: raw.file.map(|f| FileConfig {
                dir: PathBuf::from(f.dir),
            }),
            probe,
            services,
        })
    }

    pub fn load(path: &Path) -> Result<Config, ConfigError> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| ConfigError(format!("cannot read {}: {e}", path.display())))?;
        Config::from_toml_str(&text)
            .map_err(|e| ConfigError(format!("{}: {}", path.display(), e.0)))
    }
}
