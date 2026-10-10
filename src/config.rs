//! `quotabus.toml` (DESIGN §3). A `secret` is a NAME; its value comes from the environment.

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::record::Kind;
use crate::schedule::{BALANCE, QueryKind};

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

/// `[probe]`. How often each kind of query runs is `[intervals]`, not here (CHORE-007).
#[derive(Debug, Clone, PartialEq)]
pub struct ProbeConfig {
    /// Default 20.
    pub max_tokens: u32,
    /// A 200 slower than this reads `degraded`. Default 10000.
    pub degraded_latency_ms: u64,
    /// A row that would read `ok` reads `degraded` when any window is at or above this % (DESIGN §2 "a window ≥
    /// warn %"; CHORE-010, steer bucket 2). Default 90; 0 < x ≤ 100.
    pub warn_pct: f64,
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
    /// Put the balance on the row (DESIGN §6). Default true; false still reads it and applies the floor.
    pub publish_balance: bool,
    /// `[service.context]`: each model's context window in tokens (the selector's `--prefer largest-context`).
    pub context: std::collections::BTreeMap<String, u64>,
    /// `probe = false`: listed by `status`, `select` and `alert` (rows read from the store) but never probed by this
    /// process; another process owns it (CHORE-023). Default true.
    pub probe: bool,
}

impl ServiceConfig {
    /// The model slots this service writes rows under: its models, or for a subscription its id (one row per
    /// account, DESIGN §2). A balance row is not a slot.
    pub fn slots(&self) -> Vec<String> {
        match self.kind {
            Kind::Subscription => vec![self.id.clone()],
            _ => self.models.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    pub bus: Option<BusConfig>,
    pub file: Option<FileConfig>,
    pub probe: ProbeConfig,
    /// `[intervals]`: how often each query kind runs.
    pub intervals: PerKind,
    /// `[ttl]`: each kind's row TTL; unset ⇒ 3 × that kind's own interval.
    pub ttl: PerKindTtl,
    /// The `[[service]]` array.
    pub services: Vec<ServiceConfig>,
    /// `[alert.*]`: where `quotabus alert` files a crossing (DESIGN §3 alert; EXP-004).
    pub alert: AlertConfig,
}

/// `[alert.*]`: the sinks `quotabus alert` files one item per crossing to (DESIGN §3 alert, §10 Q8; EXP-004). A sink
/// left out is not used; stdout is always written.
#[derive(Debug, Clone, PartialEq)]
pub struct AlertConfig {
    /// `[alert.nusy-kanban]`.
    pub nusy_kanban: Option<KanbanSink>,
    /// `[alert.yurtle-kanban]`.
    pub yurtle_kanban: Option<KanbanSink>,
    /// `[alert.webhook]`.
    pub webhook: Option<WebhookSink>,
    /// `[alert] states`: the names a crossing files for (CHORE-018; Captain 2026-10-09 on SIG-010). A configured
    /// list replaces [`DEFAULT_ALERT_STATES`]; every name is one of [`ALERT_STATE_NAMES`].
    pub states: Vec<String>,
}

impl Default for AlertConfig {
    fn default() -> Self {
        AlertConfig {
            nusy_kanban: None,
            yurtle_kanban: None,
            webhook: None,
            states: DEFAULT_ALERT_STATES.iter().map(|s| s.to_string()).collect(),
        }
    }
}

/// Every name `[alert] states` accepts: the bad row states but `unknown`, plus `unreachable` (a CANNOT-ASSESS row
/// whose reason is `cannot_assess:unreachable`) and `window_near_limit` (a `degraded` row the warn rule wrote).
pub const ALERT_STATE_NAMES: [&str; 7] = [
    "quota_exhausted",
    "auth_failed",
    "model_missing",
    "rate_limited",
    "degraded",
    "unreachable",
    "window_near_limit",
];

/// `[alert] states` when it is absent (SIG-010, the recommended default): the hard failures plus a near-limit window.
pub const DEFAULT_ALERT_STATES: [&str; 5] = [
    "quota_exhausted",
    "auth_failed",
    "model_missing",
    "unreachable",
    "window_near_limit",
];

/// `[alert.nusy-kanban]` / `[alert.yurtle-kanban]`: the command run, the item type it creates and the tags it carries.
#[derive(Debug, Clone, PartialEq)]
pub struct KanbanSink {
    /// The executable: a bare name looked up on PATH, or a path.
    pub command: String,
    /// The item type created (`signal`, SIG-006 / §10 Q8).
    pub item_type: String,
    pub tags: Vec<String>,
}

/// `[alert.webhook]`: a JSON POST per crossing.
#[derive(Debug, Clone, PartialEq)]
pub struct WebhookSink {
    pub url: String,
}

/// One duration per query kind (`[intervals]`).
#[derive(Debug, Clone, PartialEq)]
pub struct PerKind {
    pub api: Duration,
    pub balance: Duration,
    pub subscription: Duration,
}

/// `[ttl]`: a kind left out takes 3 × its own interval.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PerKindTtl {
    pub api: Option<Duration>,
    pub balance: Option<Duration>,
    pub subscription: Option<Duration>,
}

/// The default interval of the API and balance kinds: twice a day (the Captain on SIG-004, 2026-10-08).
pub const DEFAULT_INTERVAL: Duration = Duration::from_secs(12 * 3600);

/// The default interval of the subscription kind: hourly (the Captain on EXP-002's rescope, 2026-10-08).
pub const DEFAULT_SUBSCRIPTION_INTERVAL: Duration = Duration::from_secs(3600);

/// A kind's default TTL is this many of its own intervals: 3 missed runs = UNKNOWN.
pub const TTL_INTERVALS: u32 = 3;

impl Default for PerKind {
    fn default() -> Self {
        PerKind {
            api: DEFAULT_INTERVAL,
            balance: DEFAULT_INTERVAL,
            subscription: DEFAULT_SUBSCRIPTION_INTERVAL,
        }
    }
}

impl PerKind {
    pub fn get(&self, kind: QueryKind) -> Duration {
        match kind {
            QueryKind::Api => self.api,
            QueryKind::Balance => self.balance,
            QueryKind::Subscription => self.subscription,
        }
    }
}

impl PerKindTtl {
    pub fn get(&self, kind: QueryKind) -> Option<Duration> {
        match kind {
            QueryKind::Api => self.api,
            QueryKind::Balance => self.balance,
            QueryKind::Subscription => self.subscription,
        }
    }
}

impl Default for ProbeConfig {
    fn default() -> Self {
        ProbeConfig {
            max_tokens: 20,
            degraded_latency_ms: 10_000,
            warn_pct: DEFAULT_WARN_PCT,
        }
    }
}

/// `[probe] warn_pct`'s default (CHORE-010, steer bucket 2, 2026-10-08).
pub const DEFAULT_WARN_PCT: f64 = 90.0;

/// The default bucket name.
pub const DEFAULT_BUCKET: &str = "ai_status";

/// `[alert.*]` as written to [`AlertConfig`]. A kanban sink's `command` defaults to its own name and its
/// `item_type` to `signal` (SIG-006, §10 Q8); `tags` default to none.
fn alert_config(raw: Option<raw::Alert>) -> Result<AlertConfig, ConfigError> {
    let Some(a) = raw else {
        return Ok(AlertConfig::default());
    };
    let kanban = |name: &str, k: Option<raw::Kanban>| -> Result<Option<KanbanSink>, ConfigError> {
        let Some(k) = k else { return Ok(None) };
        let command = k.command.unwrap_or_else(|| name.to_string());
        let item_type = k.item_type.unwrap_or_else(|| "signal".to_string());
        if command.trim().is_empty() || item_type.trim().is_empty() {
            return Err(ConfigError(format!(
                "[alert.{name}] command and item_type must not be empty"
            )));
        }
        if k.tags
            .iter()
            .any(|t| t.trim().is_empty() || t.contains(','))
        {
            return Err(ConfigError(format!(
                "[alert.{name}] tags: each tag is one non-empty word without a comma"
            )));
        }
        Ok(Some(KanbanSink {
            command,
            item_type,
            tags: k.tags,
        }))
    };
    let webhook = match a.webhook {
        Some(w) if !(w.url.starts_with("http://") || w.url.starts_with("https://")) => {
            return Err(ConfigError(
                "[alert.webhook] url must be http:// or https://".into(),
            ));
        }
        w => w.map(|w| WebhookSink { url: w.url }),
    };
    Ok(AlertConfig {
        nusy_kanban: kanban("nusy-kanban", a.nusy_kanban)?,
        yurtle_kanban: kanban("yurtle-kanban", a.yurtle_kanban)?,
        webhook,
        states: alert_states(a.states)?,
    })
}

/// `[alert] states`: absent → the default; each name must be one of [`ALERT_STATE_NAMES`].
fn alert_states(raw: Option<Vec<String>>) -> Result<Vec<String>, ConfigError> {
    let Some(names) = raw else {
        return Ok(AlertConfig::default().states);
    };
    if let Some(bad) = names
        .iter()
        .find(|n| !ALERT_STATE_NAMES.contains(&n.as_str()))
    {
        return Err(ConfigError(format!(
            "[alert] states: unknown name {bad:?}; valid names are {}",
            ALERT_STATE_NAMES.join(", ")
        )));
    }
    let mut out: Vec<String> = Vec::new();
    for n in names {
        if !out.contains(&n) {
            out.push(n);
        }
    }
    Ok(out)
}

/// `[probe] warn_pct`: a number, 0 < x ≤ 100 (CHORE-010).
fn warn_pct(v: &toml::Value) -> Result<f64, ConfigError> {
    let n = match v {
        toml::Value::Float(f) => *f,
        toml::Value::Integer(i) => *i as f64,
        _ => {
            return Err(ConfigError(format!(
                "[probe] warn_pct must be a number (0 < warn_pct <= 100), not {v}"
            )));
        }
    };
    if n.is_finite() && n > 0.0 && n <= 100.0 {
        Ok(n)
    } else {
        Err(ConfigError(format!(
            "[probe] warn_pct = {n} is out of range: 0 < warn_pct <= 100"
        )))
    }
}

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
        pub intervals: Option<PerKind>,
        pub ttl: Option<PerKind>,
        #[serde(default)]
        pub service: Vec<Service>,
        pub alert: Option<Alert>,
    }

    /// `[alert.*]`.
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Alert {
        #[serde(rename = "nusy-kanban")]
        pub nusy_kanban: Option<Kanban>,
        #[serde(rename = "yurtle-kanban")]
        pub yurtle_kanban: Option<Kanban>,
        pub webhook: Option<Webhook>,
        pub states: Option<Vec<String>>,
    }

    /// `[alert.nusy-kanban]` / `[alert.yurtle-kanban]`.
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Kanban {
        pub command: Option<String>,
        pub item_type: Option<String>,
        #[serde(default)]
        pub tags: Vec<String>,
    }

    /// `[alert.webhook]`.
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct Webhook {
        pub url: String,
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
        /// Retired by CHORE-007: read only to refuse it with a pointer to `[intervals]`.
        pub interval: Option<toml::Value>,
        /// Retired by CHORE-007: read only to refuse it with a pointer to `[ttl]`.
        pub ttl: Option<toml::Value>,
        pub max_tokens: Option<u32>,
        pub degraded_latency_ms: Option<u64>,
        /// Read as a value so a non-number is refused with an error that names the key.
        pub warn_pct: Option<toml::Value>,
    }

    /// `[intervals]` / `[ttl]`: one duration per query kind.
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct PerKind {
        pub api: Option<String>,
        pub balance: Option<String>,
        pub subscription: Option<String>,
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
    #[serde(deny_unknown_fields)]
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
        pub publish_balance: Option<bool>,
        #[serde(default)]
        pub context: std::collections::BTreeMap<String, u64>,
    }
}

impl Config {
    pub fn from_toml_str(text: &str) -> Result<Config, ConfigError> {
        let raw: raw::File =
            toml::from_str(text).map_err(|e| ConfigError(format!("invalid config: {e}")))?;
        Config::from_raw(raw)
    }

    /// The parsed file (from either front-end) checked and converted.
    fn from_raw(raw: raw::File) -> Result<Config, ConfigError> {
        let mut probe = ProbeConfig::default();
        if let Some(p) = raw.probe {
            // one interval for every call was EXP-001's; each kind now has its own (SIG-004, CHORE-007)
            if p.interval.is_some() {
                return Err(ConfigError(
                    "[probe] interval is retired: set each kind's own in [intervals] (api = \"12h\", \
                     balance = \"12h\", subscription = \"1h\")"
                        .into(),
                ));
            }
            if p.ttl.is_some() {
                return Err(ConfigError(
                    "[probe] ttl is retired: set each kind's own in [ttl] (api, balance, subscription; \
                     default 3 × its interval)"
                        .into(),
                ));
            }
            if let Some(m) = p.max_tokens {
                probe.max_tokens = m;
            }
            if let Some(d) = p.degraded_latency_ms {
                probe.degraded_latency_ms = d;
            }
            if let Some(w) = p.warn_pct {
                probe.warn_pct = warn_pct(&w)?;
            }
        }
        let mut intervals = PerKind::default();
        if let Some(i) = raw.intervals {
            if let Some(a) = i.api {
                intervals.api = parse_duration(&a)?;
            }
            if let Some(b) = i.balance {
                intervals.balance = parse_duration(&b)?;
            }
            if let Some(s) = i.subscription {
                intervals.subscription = parse_duration(&s)?;
            }
        }
        let mut ttl = PerKindTtl::default();
        if let Some(t) = raw.ttl {
            ttl.api = t.api.as_deref().map(parse_duration).transpose()?;
            ttl.balance = t.balance.as_deref().map(parse_duration).transpose()?;
            ttl.subscription = t.subscription.as_deref().map(parse_duration).transpose()?;
        }
        for (name, kind) in [
            ("api", QueryKind::Api),
            ("balance", QueryKind::Balance),
            ("subscription", QueryKind::Subscription),
        ] {
            let t = ttl
                .get(kind)
                .unwrap_or_else(|| intervals.get(kind).saturating_mul(TTL_INTERVALS));
            if t.as_secs() == 0 {
                return Err(ConfigError(format!("[ttl] {name} must be at least 1s")));
            }
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
            // the balance read writes its own row under the model slot `balance` (schedule::BALANCE)
            if matches!(balance, BalanceSpec::Endpoint(_))
                && s.models.iter().any(|m| crate::record::slug(m) == BALANCE)
            {
                return Err(ConfigError(format!(
                    "service {:?}: a model named {BALANCE:?} would share the balance row's key",
                    s.id
                )));
            }
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
                publish_balance: s.publish_balance.unwrap_or(true),
                context: s.context,
                probe: true, // CHORE-023 stub: `probe = false` is not parsed yet
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
            intervals,
            ttl,
            services,
            alert: alert_config(raw.alert)?,
        })
    }

    /// The Yurtle front-end (DESIGN §3 "Config — one model, two front-ends"): the same rows as the TOML, read from the
    /// `yurtle-table` block(s) of a Yurtle v2.1 markdown file; the rest of the file is ignored. EXP-004. The reader
    /// ([`crate::yurtle`]) turns the blocks into the same tables the TOML front-end parses, so both are checked by
    /// one path.
    pub fn from_yurtle_str(text: &str) -> Result<Config, ConfigError> {
        let table = crate::yurtle::config_table(text)?;
        let raw: raw::File = toml::Value::Table(table)
            .try_into()
            .map_err(|e| ConfigError(format!("invalid Yurtle config: {e}")))?;
        Config::from_raw(raw)
    }

    /// How often `kind` is queried: `[intervals] <kind>` (default 12h; subscription 1h).
    pub fn interval_for(&self, kind: QueryKind) -> Duration {
        self.intervals.get(kind)
    }

    /// How long a `kind` row is trustworthy (its `ttl_s`): `[ttl] <kind>`, default 3 × that kind's own interval.
    pub fn ttl_for(&self, kind: QueryKind) -> Duration {
        self.ttl
            .get(kind)
            .unwrap_or_else(|| self.interval_for(kind).saturating_mul(TTL_INTERVALS))
    }

    pub fn load(path: &Path) -> Result<Config, ConfigError> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| ConfigError(format!("cannot read {}: {e}", path.display())))?;
        // a `.md` path is the Yurtle twin (`quotabus.yurtle.md`); anything else is TOML
        let parsed = if path.extension().is_some_and(|e| e == "md") {
            Config::from_yurtle_str(&text)
        } else {
            Config::from_toml_str(&text)
        };
        parsed.map_err(|e| ConfigError(format!("{}: {}", path.display(), e.0)))
    }
}
