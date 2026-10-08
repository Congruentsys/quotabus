use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;
use std::str::FromStr;
use std::sync::Arc;

use chrono::Utc;
use clap::{Parser, Subcommand};
use quotabus::config::DEFAULT_BUCKET;
use quotabus::{
    Backend, BackendError, Config, FileBackend, Kind, Listing, NatsKv, Record, Redactor, Runner,
    Verdict, freshness,
};

#[derive(Parser)]
#[command(
    name = "quotabus",
    version,
    about = "AI provider & account status on a bus"
)]
struct Cli {
    /// Path to quotabus.toml (else $QUOTABUS_CONFIG, else ./quotabus.toml).
    #[arg(long, global = true)]
    config: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run one probe cycle and write one row per (service, model) to the backend.
    Probe,
    /// Read rows and apply the freshness rule.
    Status {
        /// Print a JSON array: one {service, key, verdict, reason, age_s, row} per configured (service, model).
        #[arg(long)]
        json: bool,
        /// Only services of this kind (api, local, subscription).
        #[arg(long)]
        kind: Option<String>,
        /// Only entries that are UNKNOWN (absent or expired).
        #[arg(long)]
        stale: bool,
        /// Exit 0 ok · 1 not ok · 2 CANNOT-ASSESS · 3 UNKNOWN for this service id (the worst of its models).
        #[arg(long)]
        check: Option<String>,
    },
}

/// Exit codes of `probe`: a bad config or a backend that cannot be written.
const PROBE_FAILED: u8 = 1;
const CANNOT_ASSESS: u8 = 2;
const UNKNOWN: u8 = 3;

fn main() -> ExitCode {
    let cli = Cli::parse();
    let path = cli
        .config
        .or_else(|| std::env::var_os("QUOTABUS_CONFIG").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("quotabus.toml"));
    let is_probe = matches!(cli.command, Command::Probe);
    let config = match Config::load(&path) {
        Ok(c) => c,
        Err(e) => {
            let msg = Redactor::default().redact(&e.to_string());
            return if is_probe {
                eprintln!("quotabus: {msg}");
                ExitCode::from(PROBE_FAILED)
            } else {
                println!("CANNOT-ASSESS: {msg}");
                ExitCode::from(CANNOT_ASSESS)
            };
        }
    };
    let runner = Runner::from_process_env(config.clone());
    let redactor = Arc::new(runner.redactor());
    init_logging(redactor.clone());

    let rt = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("quotabus: cannot start the runtime: {e}");
            return ExitCode::from(PROBE_FAILED);
        }
    };
    let code = match cli.command {
        Command::Probe => rt.block_on(probe(&config, &runner, &redactor)),
        Command::Status {
            json,
            kind,
            stale,
            check,
        } => rt.block_on(status(
            &config,
            &redactor,
            &StatusArgs {
                json,
                kind,
                stale,
                check,
            },
        )),
    };
    ExitCode::from(code)
}

/// tracing to stderr, every line through the redactor. `RUST_LOG` uses the `target=level,…` form.
fn init_logging(redactor: Arc<Redactor>) {
    let filter = std::env::var("RUST_LOG")
        .ok()
        .and_then(|s| tracing_subscriber::filter::Targets::from_str(&s).ok())
        .unwrap_or_else(|| {
            tracing_subscriber::filter::Targets::new()
                .with_default(tracing::Level::WARN)
                .with_target("quotabus", tracing::Level::INFO)
        });
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    let layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_writer(move || RedactingStderr {
            buf: Vec::new(),
            redactor: redactor.clone(),
        });
    let _ = tracing_subscriber::registry()
        .with(layer)
        .with(filter)
        .try_init();
}

/// Buffers one log event and writes it to stderr redacted when dropped.
struct RedactingStderr {
    buf: Vec<u8>,
    redactor: Arc<Redactor>,
}

impl Write for RedactingStderr {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        self.buf.extend_from_slice(data);
        Ok(data.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Drop for RedactingStderr {
    fn drop(&mut self) {
        if !self.buf.is_empty() {
            let text = self.redactor.redact(&String::from_utf8_lossy(&self.buf));
            let _ = std::io::stderr().write_all(text.as_bytes());
        }
    }
}

fn nats_url(config: &Config) -> Option<(String, String)> {
    let bucket = config
        .bus
        .as_ref()
        .map(|b| b.bucket.clone())
        .unwrap_or_else(|| DEFAULT_BUCKET.to_string());
    match std::env::var("QUOTABUS_NATS_URL") {
        Ok(url) if !url.is_empty() => Some((url, bucket)),
        _ => config.bus.as_ref().map(|b| (b.url.clone(), bucket)),
    }
}

fn file_dir(config: &Config) -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME").filter(|h| !h.is_empty());
    match &config.file {
        Some(f) => match (f.dir.strip_prefix("~"), &home) {
            (Ok(rest), Some(h)) => Ok(PathBuf::from(h).join(rest)),
            (Ok(_), None) => Err("[file] dir starts with ~ but HOME is unset".to_string()),
            (Err(_), _) => Ok(f.dir.clone()),
        },
        None => home
            .map(|h| PathBuf::from(h).join(".local/state/quotabus"))
            .ok_or_else(|| "no [bus] and no [file] dir, and HOME is unset".to_string()),
    }
}

async fn probe(config: &Config, runner: &Runner, redactor: &Redactor) -> u8 {
    let rows = runner.run_once().await;
    let result = match nats_url(config) {
        Some((url, bucket)) => match NatsKv::connect_publisher(&url, &bucket).await {
            Ok(kv) => publish(&kv, &rows)
                .await
                .map(|n| format!("{n} rows to bucket {bucket}")),
            Err(e) => Err(e.to_string()),
        },
        None => match file_dir(config) {
            Ok(dir) => {
                let fb = FileBackend::new(&dir);
                publish(&fb, &rows)
                    .await
                    .map(|n| format!("{n} rows to {}", dir.display()))
            }
            Err(e) => Err(e),
        },
    };
    let mut out = String::new();
    for r in &rows {
        out.push_str(&format!(
            "{} {}{}\n",
            r.key,
            r.state.as_str(),
            r.reason
                .as_deref()
                .map(|s| format!(" ({s})"))
                .unwrap_or_default()
        ));
    }
    print!("{}", redactor.redact(&out));
    match result {
        Ok(what) => {
            println!("published {what}");
            0
        }
        Err(e) => {
            eprintln!("quotabus: cannot publish: {}", redactor.redact_error(&e));
            PROBE_FAILED
        }
    }
}

async fn publish(backend: &impl Backend, rows: &[Record]) -> Result<usize, String> {
    let mut failed = Vec::new();
    for r in rows {
        if let Err(e) = backend.put(r).await {
            failed.push(e.to_string());
        }
    }
    match failed.first() {
        None => Ok(rows.len()),
        Some(first) => Err(format!(
            "{} of {} rows not written; first: {first}",
            failed.len(),
            rows.len()
        )),
    }
}

struct StatusArgs {
    json: bool,
    kind: Option<String>,
    stale: bool,
    check: Option<String>,
}

/// One configured (service, model) and what the freshness rule says about it.
struct Entry {
    service: String,
    model: String,
    key: String,
    row: Option<Record>,
    verdict: Verdict,
}

async fn read_rows(config: &Config) -> Result<Listing, BackendError> {
    match nats_url(config) {
        Some((url, bucket)) => NatsKv::connect_reader(&url, &bucket).await?.scan().await,
        None => {
            let dir = file_dir(config).map_err(BackendError::Other)?;
            FileBackend::new(dir).scan().await
        }
    }
}

async fn status(config: &Config, redactor: &Redactor, args: &StatusArgs) -> u8 {
    let kind = match args.kind.as_deref() {
        None => None,
        Some(k) => match serde_json::from_value::<Kind>(serde_json::json!(k)) {
            Ok(k) => Some(k),
            Err(_) => {
                println!("CANNOT-ASSESS: --kind {k:?} is not api, local or subscription");
                return CANNOT_ASSESS;
            }
        },
    };
    let services: Vec<_> = config
        .services
        .iter()
        .filter(|s| kind.is_none_or(|k| s.kind == k))
        .filter(|s| args.check.as_ref().is_none_or(|id| &s.id == id))
        .collect();
    if let Some(id) = &args.check
        && services.is_empty()
    {
        println!("UNKNOWN: no service {id:?} in the config");
        return UNKNOWN;
    }

    let (listing, failure) = match read_rows(config).await {
        Ok(l) => (l, None),
        Err(e) => (Listing::default(), Some(e)),
    };
    let by_key: HashMap<&str, &Record> = listing.rows.iter().map(|r| (r.key.as_str(), r)).collect();
    let unreadable: HashSet<&str> = listing.unreadable.iter().map(|(k, _)| k.as_str()).collect();
    let now = Utc::now();
    let mut entries = Vec::new();
    for svc in services {
        for model in &svc.models {
            let key = quotabus::record_key(svc.kind, &svc.provider, &svc.account, model);
            let row = by_key.get(key.as_str()).map(|r| (*r).clone());
            let verdict = match &failure {
                Some(e) => Verdict::CannotAssess {
                    reason: e.reason().to_string(),
                },
                // a row that exists but does not parse is a failure to measure, never `absent`
                None if unreadable.contains(key.as_str()) => Verdict::CannotAssess {
                    reason: "cannot_assess:unreadable_row".to_string(),
                },
                None => freshness(row.as_ref(), now),
            };
            entries.push(Entry {
                service: svc.id.clone(),
                model: model.clone(),
                key,
                row,
                verdict,
            });
        }
    }
    if args.stale {
        entries.retain(|e| matches!(e.verdict, Verdict::Unknown { .. }));
    }

    if let Some(e) = &failure {
        eprintln!(
            "quotabus: CANNOT-ASSESS: {}",
            redactor.redact_error(&e.to_string())
        );
    }
    let out = if args.json {
        json_out(&entries, now)
    } else {
        table_out(&entries, now)
    };
    print!("{}", redactor.redact(&out));

    if args.check.is_some() {
        let worst = entries.iter().map(|e| e.verdict.exit_code()).max();
        return u8::try_from(worst.unwrap_or(3)).unwrap_or(UNKNOWN);
    }
    if failure.is_some() { CANNOT_ASSESS } else { 0 }
}

fn age_s(row: Option<&Record>, now: chrono::DateTime<Utc>) -> Option<i64> {
    row.map(|r| (now - r.checked_at).num_seconds())
}

fn json_out(entries: &[Entry], now: chrono::DateTime<Utc>) -> String {
    let arr: Vec<serde_json::Value> = entries
        .iter()
        .map(|e| {
            serde_json::json!({
                "service": e.service,
                "key": e.key,
                "verdict": e.verdict.label(),
                "reason": e.verdict.reason(),
                "age_s": age_s(e.row.as_ref(), now),
                "row": e.row,
            })
        })
        .collect();
    let mut s = serde_json::to_string_pretty(&arr).unwrap_or_else(|_| "[]".to_string());
    s.push('\n');
    s
}

fn human_age(secs: i64) -> String {
    match secs {
        s if s < 0 => "0s".to_string(),
        s if s < 120 => format!("{s}s"),
        s if s < 7200 => format!("{}m", s / 60),
        s if s < 172_800 => format!("{}h", s / 3600),
        s => format!("{}d", s / 86_400),
    }
}

fn table_out(entries: &[Entry], now: chrono::DateTime<Utc>) -> String {
    let header = ["SERVICE", "MODEL", "STATE", "AGE", "SOURCE", "DETAIL"];
    let mut lines: Vec<[String; 6]> = vec![header.map(String::from)];
    for e in entries {
        let row = e.row.as_ref();
        let mut detail = e.verdict.reason().unwrap_or_default();
        if let Some(b) = row.and_then(|r| r.balance.as_ref()) {
            if !detail.is_empty() {
                detail.push(' ');
            }
            detail.push_str(&format!("balance {} {}", b.amount, b.currency));
        }
        lines.push([
            e.service.clone(),
            e.model.clone(),
            e.verdict.label(),
            age_s(row, now)
                .map(human_age)
                .unwrap_or_else(|| "-".to_string()),
            row.map(|r| r.probe.source.as_str().to_string())
                .unwrap_or_else(|| "-".to_string()),
            detail,
        ]);
    }
    let mut widths = [0usize; 6];
    for l in &lines {
        for (w, c) in widths.iter_mut().zip(l.iter()) {
            *w = (*w).max(c.chars().count());
        }
    }
    let mut out = String::new();
    for l in &lines {
        let mut line = String::new();
        for (i, (c, w)) in l.iter().zip(widths).enumerate() {
            if i + 1 == l.len() {
                line.push_str(c);
            } else {
                line.push_str(&format!("{c:<w$}  "));
            }
        }
        out.push_str(line.trim_end());
        out.push('\n');
    }
    out
}
