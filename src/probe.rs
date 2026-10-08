//! The probe runner (DESIGN §4): one row per (service, model), keys from the environment only.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use futures::future::join_all;
use reqwest::header::HeaderValue;
use serde_json::json;

use crate::balance::{apply_floor, exhaust, extract_amount};
use crate::classify::{HttpOutcome, Outcome, Thresholds, classify};
use crate::config::{BalanceEndpoint, BalanceSpec, Config, Protocol, ServiceConfig};
use crate::freshness::{Verdict, freshness};
use crate::record::{Balance, CONTRACT, Kind, Probe, ProbeSource, Record, State, record_key};
use crate::redact::Redactor;
use crate::schedule::{BALANCE, QueryKind, is_due};
use crate::secret::Secret;

type EnvLookup = Box<dyn Fn(&str) -> Option<String> + Send + Sync>;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const TOTAL_TIMEOUT: Duration = Duration::from_secs(30);
/// The probe prompt is a constant (DESIGN §6: a record never reveals a user's prompt).
const PROMPT: &str = "Reply with exactly: OK";
const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Runs one probe cycle over every configured service, or only what is due. A service's `base_url` (and balance
/// `url`) is where it is probed, so a test points it at a local stub.
pub struct Runner {
    config: Config,
    env: EnvLookup,
}

/// How a service authenticates this cycle.
enum Auth {
    Key(Secret),
    /// A `local` service with no `secret` configured: probed without a key.
    None,
    /// The secret NAME is configured but resolves unset or empty (or an `api` service names none).
    Unset(Option<String>),
}

/// What one service's probe needs, shared by its model probes.
struct Ctx<'a> {
    client: &'a reqwest::Client,
    redactor: &'a Redactor,
    config: &'a Config,
    /// The cycle's clock: every row it writes is `checked_at` this.
    now: DateTime<Utc>,
}

/// What is due for one service this cycle.
struct Due<'a> {
    /// The models whose API probe is due.
    models: Vec<&'a str>,
    /// Whether the balance read is due (false when the service has no balance endpoint).
    balance: bool,
    /// The service's last balance row in the store, if any: used for the floor when only the API probe is due.
    stored_balance: Option<&'a Record>,
}

impl Runner {
    /// `env` resolves a secret NAME to its value (`None` or empty ⇒ the service is not probed).
    pub fn new<F>(config: Config, env: F) -> Runner
    where
        F: Fn(&str) -> Option<String> + Send + Sync + 'static,
    {
        Runner {
            config,
            env: Box::new(env),
        }
    }

    /// Resolve secrets from the process environment.
    pub fn from_process_env(config: Config) -> Runner {
        Runner::new(config, |name| std::env::var(name).ok())
    }

    fn auth(&self, svc: &ServiceConfig) -> Auth {
        match &svc.secret {
            Some(name) => match (self.env)(name).filter(|v| !v.is_empty()) {
                Some(v) => Auth::Key(Secret::new(v)),
                None => Auth::Unset(Some(name.clone())),
            },
            None if svc.kind == Kind::Local => Auth::None,
            None => Auth::Unset(None),
        }
    }

    /// A redactor holding every secret value the configured names resolve to, and the configured account labels.
    pub fn redactor(&self) -> Redactor {
        let mut r = Redactor::default();
        for svc in &self.config.services {
            r.allow_label(svc.account.clone());
            if let Auth::Key(s) = self.auth(svc) {
                r.add(s);
            }
        }
        r
    }

    /// One scheduled cycle (CHORE-007): make a query kind's calls only when that kind is DUE, i.e. its own
    /// `[intervals]` entry has elapsed since that kind's last row in `store_rows` (no row = due); `force` runs every
    /// kind. Due-ness is per row: each (service, model) for the API probe, each service's balance row for the
    /// balance read. `now` is the clock (injectable for tests): every returned row has `checked_at == now` and
    /// `ttl_s` equal to its own kind's TTL. Rows are returned, not published.
    pub async fn run_due(
        &self,
        store_rows: &[Record],
        now: DateTime<Utc>,
        force: bool,
    ) -> Vec<Record> {
        let client = reqwest::Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(TOTAL_TIMEOUT)
            .build()
            .unwrap_or_default();
        let redactor = self.redactor();
        let ctx = Ctx {
            client: &client,
            redactor: &redactor,
            config: &self.config,
            now,
        };
        let stored: HashMap<&str, &Record> =
            store_rows.iter().map(|r| (r.key.as_str(), r)).collect();
        let due = |kind: QueryKind, key: &str| {
            force
                || is_due(
                    stored.get(key).copied(),
                    self.config.interval_for(kind),
                    now,
                )
        };
        let services = self
            .config
            .services
            .iter()
            // subscriptions are read per host by `quotabus agent` (a later expedition), never by the central probe
            .filter(|s| s.kind != Kind::Subscription)
            .map(|svc| {
                let balance_key = balance_key(svc);
                let due = Due {
                    models: svc
                        .models
                        .iter()
                        .filter(|m| {
                            due(
                                QueryKind::Api,
                                &record_key(svc.kind, &svc.provider, &svc.account, m),
                            )
                        })
                        .map(String::as_str)
                        .collect(),
                    balance: matches!(svc.balance, BalanceSpec::Endpoint(_))
                        && due(QueryKind::Balance, &balance_key),
                    stored_balance: stored.get(balance_key.as_str()).copied(),
                };
                probe_service(&ctx, svc, self.auth(svc), due)
            });
        join_all(services).await.into_iter().flatten().collect()
    }

    /// One unscheduled cycle, every kind now: one redacted row per (service, model), each carrying its balance.
    /// The balance's own row (which only the schedule needs) is left out. Rows are returned, not published.
    pub async fn run_once(&self) -> Vec<Record> {
        let mut rows = self.run_due(&[], Utc::now(), true).await;
        rows.retain(|r| r.probe.name != BALANCE);
        rows
    }
}

/// `<kind>.<provider>.<account>.balance`: the row a service's balance read writes.
fn balance_key(svc: &ServiceConfig) -> String {
    record_key(svc.kind, &svc.provider, &svc.account, BALANCE)
}

async fn probe_service(
    ctx: &Ctx<'_>,
    svc: &ServiceConfig,
    auth: Auth,
    due: Due<'_>,
) -> Vec<Record> {
    let probe_name = match svc.protocol {
        Some(Protocol::Anthropic) => "messages",
        Some(Protocol::Openai) | None => "chat_completions",
    };
    let (base, protocol) = match (&svc.base_url, svc.protocol) {
        (Some(b), Some(p)) => (b.trim_end_matches('/'), p),
        _ => {
            return unmeasured(
                ctx,
                svc,
                &due.models,
                probe_name,
                "no_endpoint",
                "no base_url/protocol configured",
            );
        }
    };
    let key = match auth {
        Auth::Key(s) => Some(s),
        Auth::None => None,
        Auth::Unset(name) => {
            let why = match name {
                Some(n) => format!("environment variable {n} is unset or empty; not probed"),
                None => "no secret configured; not probed".to_string(),
            };
            return unmeasured(ctx, svc, &due.models, probe_name, "secret_unset", &why);
        }
    };

    let balance = async {
        match (&svc.balance, &key, due.balance) {
            (BalanceSpec::Endpoint(b), Some(k), true) => {
                Some(balance_row(ctx, svc, fetch_balance(ctx, svc, b, k).await))
            }
            _ => None,
        }
    };
    let models = join_all(
        due.models
            .iter()
            .map(|model| probe_model(ctx, svc, protocol, base, model, key.as_ref(), probe_name)),
    );
    let (balance, mut rows) = futures::join!(balance, models);

    // the floor and the published balance come from this cycle's read, else from the last read while it is fresh
    let last_read = balance.as_ref().or(due
        .stored_balance
        .filter(|r| matches!(freshness(Some(r), ctx.now), Verdict::State { .. })));
    if let Some(b) = last_read.filter(|b| b.state != State::Unknown) {
        for r in &mut rows {
            // the floor only ever turns a working model into quota_exhausted; the balance is on every row unless
            // the service says `publish_balance = false` (then the balance row carries none either)
            if b.state == State::QuotaExhausted {
                r.state = exhaust(r.state);
            }
            r.balance = b.balance.clone();
        }
    }
    rows.extend(balance);
    rows
}

/// The balance row: `quota_exhausted` at or below the floor, else `ok`; `unknown` when the read failed.
fn balance_row(ctx: &Ctx<'_>, svc: &ServiceConfig, read: Option<(Balance, f64)>) -> Record {
    let mut r = row(ctx, svc, BALANCE, BALANCE, QueryKind::Balance);
    match read {
        Some((bal, floor)) => {
            r.state = apply_floor(State::Ok, bal.amount, floor);
            if svc.publish_balance {
                r.balance = Some(bal);
            }
        }
        None => r.reason = Some("cannot_assess:balance_unreadable".to_string()),
    }
    tracing::info!(key = %r.key, state = r.state.as_str(), "balance read");
    r
}

/// Rows for a service that could not be measured at all (no call is made).
fn unmeasured(
    ctx: &Ctx<'_>,
    svc: &ServiceConfig,
    models: &[&str],
    probe_name: &str,
    why: &str,
    error: &str,
) -> Vec<Record> {
    models
        .iter()
        .map(|model| {
            let mut r = row(ctx, svc, model, probe_name, QueryKind::Api);
            r.state = State::Unknown;
            r.reason = Some(format!("cannot_assess:{why}"));
            r.error = Some(ctx.redactor.redact_error(error));
            tracing::info!(key = %r.key, reason = why, "not probed");
            r
        })
        .collect()
}

fn row(
    ctx: &Ctx<'_>,
    svc: &ServiceConfig,
    model: &str,
    probe_name: &str,
    kind: QueryKind,
) -> Record {
    Record {
        contract: CONTRACT.to_string(),
        key: record_key(svc.kind, &svc.provider, &svc.account, model),
        kind: svc.kind,
        provider: svc.provider.clone(),
        account: svc.account.clone(),
        model: model.to_string(),
        family: svc.family.clone(),
        state: State::Unknown,
        reason: None,
        balance: None,
        headroom: None,
        latency_ms: None,
        probe: Probe {
            name: probe_name.to_string(),
            source: ProbeSource::Official,
        },
        error: None,
        checked_at: ctx.now,
        ttl_s: ctx.config.ttl_for(kind).as_secs(),
        observed_by: crate::observed_by(),
    }
}

async fn probe_model(
    ctx: &Ctx<'_>,
    svc: &ServiceConfig,
    protocol: Protocol,
    base: &str,
    model: &str,
    key: Option<&Secret>,
    probe_name: &str,
) -> Record {
    let max_tokens = ctx.config.probe.max_tokens;
    let request = match protocol {
        Protocol::Anthropic => {
            let mut req = ctx
                .client
                .post(format!("{base}/v1/messages"))
                .header("anthropic-version", ANTHROPIC_VERSION)
                .json(&json!({
                    "model": model,
                    "max_tokens": max_tokens,
                    "thinking": {"type": "disabled"},
                    "messages": [{"role": "user", "content": PROMPT}],
                }));
            if let Some(k) = key {
                req = req.header("x-api-key", sensitive(k));
            }
            req
        }
        Protocol::Openai => {
            // OpenAI's own models refuse `max_tokens` on chat completions; other OpenAI-protocol providers want it
            let ceiling = if svc.provider == "openai" {
                "max_completion_tokens"
            } else {
                "max_tokens"
            };
            let mut body = json!({
                "model": model,
                "messages": [{"role": "user", "content": PROMPT}],
            });
            body[ceiling] = json!(max_tokens);
            let mut req = ctx
                .client
                .post(format!("{base}/chat/completions"))
                .json(&body);
            if let Some(k) = key {
                req = req.header(
                    reqwest::header::AUTHORIZATION,
                    sensitive_str(&format!("Bearer {}", k.expose())),
                );
            }
            req
        }
    };

    let started = Instant::now();
    // `.json()` already sets the one `Content-Type: application/json`; adding another doubles it (xAI answers 415)
    let (outcome, transport_error) = match request.send().await {
        Ok(resp) => {
            let status = resp.status().as_u16();
            let headers = resp
                .headers()
                .iter()
                .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or("").to_string()))
                .collect();
            match resp.text().await {
                Ok(body) => (
                    Outcome::Http(HttpOutcome {
                        status,
                        headers,
                        body,
                        latency_ms: started.elapsed().as_millis() as u64,
                    }),
                    None,
                ),
                Err(e) => transport(e),
            }
        }
        Err(e) => transport(e),
    };

    let c = classify(
        &outcome,
        model,
        &Thresholds {
            degraded_latency_ms: ctx.config.probe.degraded_latency_ms,
        },
    );
    let mut r = row(ctx, svc, model, probe_name, QueryKind::Api);
    r.state = c.state;
    r.reason = c.reason.map(|s| ctx.redactor.redact(&s));
    r.headroom = c.headroom.map(|mut h| {
        h.reset_at = h.reset_at.map(|s| ctx.redactor.redact_error(&s));
        h
    });
    r.latency_ms = c.latency_ms;
    r.error = match (&outcome, transport_error) {
        (_, Some(e)) => Some(ctx.redactor.redact_error(&e)),
        (Outcome::Http(h), None) if !(200..300).contains(&h.status) => {
            Some(ctx.redactor.redact_error(h.body.trim()))
        }
        _ => None,
    };
    tracing::info!(key = %r.key, state = r.state.as_str(), latency_ms = ?r.latency_ms, "probed");
    if let Some(e) = &r.error {
        tracing::debug!(key = %r.key, error = %e, "probe error");
    }
    r
}

/// A transport failure: the outcome and its (unredacted) description.
fn transport(e: reqwest::Error) -> (Outcome, Option<String>) {
    let outcome = if e.is_timeout() {
        Outcome::Timeout
    } else {
        Outcome::Unreachable
    };
    (outcome, Some(error_chain(e)))
}

/// An error and its sources on one line, without the URL.
fn error_chain(e: reqwest::Error) -> String {
    let e = e.without_url();
    let mut text = e.to_string();
    let mut src = std::error::Error::source(&e);
    while let Some(s) = src {
        text.push_str(": ");
        text.push_str(&s.to_string());
        src = s.source();
    }
    text
}

fn sensitive(key: &Secret) -> HeaderValue {
    sensitive_str(key.expose())
}

/// A header value marked sensitive, so the HTTP stack never prints it. An unencodable key becomes an empty value
/// (the provider then answers 401, which is the truth about such a key).
fn sensitive_str(value: &str) -> HeaderValue {
    let mut v = HeaderValue::from_str(value).unwrap_or_else(|_| HeaderValue::from_static(""));
    v.set_sensitive(true);
    v
}

/// GET the balance endpoint; the amount (with its floor) when it parses.
async fn fetch_balance(
    ctx: &Ctx<'_>,
    svc: &ServiceConfig,
    b: &BalanceEndpoint,
    key: &Secret,
) -> Option<(Balance, f64)> {
    let source = b.url.split(['?', '#']).next().unwrap_or("").to_string();
    let resp = ctx
        .client
        .get(&b.url)
        .header(
            reqwest::header::AUTHORIZATION,
            sensitive_str(&format!("Bearer {}", key.expose())),
        )
        .send()
        .await;
    let body = match resp {
        Ok(r) if r.status().is_success() => r.json::<serde_json::Value>().await.ok(),
        Ok(r) => {
            tracing::warn!(service = %svc.id, status = r.status().as_u16(), "balance endpoint refused");
            None
        }
        Err(e) => {
            let e = ctx.redactor.redact_error(&error_chain(e));
            tracing::warn!(service = %svc.id, error = %e, "balance endpoint unreachable");
            None
        }
    }?;
    let Some(amount) = extract_amount(&body, &b.path) else {
        tracing::warn!(service = %svc.id, path = %b.path, "balance not found at the configured path");
        return None;
    };
    Some((
        Balance {
            amount,
            currency: b.currency.clone(),
            source: ctx.redactor.redact(&source),
        },
        b.floor,
    ))
}
