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
    Backend, BackendError, BusUrl, Config, FileBackend, Kind, Listing, NatsKv, Record, Redactor,
    Runner, Verdict, freshness,
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
    /// Run what is due (each query kind on its own [intervals] entry) and write its rows to the backend.
    Probe {
        /// Run every query kind now, due or not.
        #[arg(long)]
        force: bool,
    },
    /// Read rows and apply the freshness rule.
    Status {
        /// Print a JSON array: one {service, key, verdict, reason, age_s, row} per configured (service, model); a
        /// subscription service has one entry, its account.
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
    /// A healthy model for a role, excluding families: `provider model` on line 1 (one line per `--n`), the ranked
    /// list with reasons under `--json`; exit 0 chosen · 2 CANNOT-ASSESS (store unreadable) · 3 nothing qualifies.
    Select {
        #[arg(long)]
        role: String,
        /// A family never chosen (the author's); repeatable.
        #[arg(long = "exclude-family")]
        exclude_family: Vec<String>,
        /// cheapest, fastest or largest-context.
        #[arg(long)]
        prefer: Option<String>,
        /// How many to print.
        #[arg(long, default_value_t = 1)]
        n: usize,
        #[arg(long)]
        json: bool,
    },
    /// File one item per crossing (a service going bad) to every configured `[alert.*]` sink and stdout; the last
    /// alerted state is kept at `alert.<key>` in the store, and only a measured `ok` clears it (DESIGN §3; EXP-004).
    Alert,
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
    let is_probe = matches!(cli.command, Command::Probe { .. });
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
    let mut redactor = runner.redactor();
    // HAZ-001: credentials in the bus URL are secrets too, registered BEFORE the logger (and so before any
    // connection): async-nats's own debug/trace lines may carry the server address with its password
    if let Some((url, _)) = nats_url(&config) {
        for s in BusUrl::parse(&url).secrets() {
            redactor.add(s);
        }
    }
    let redactor = Arc::new(redactor);
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
        Command::Probe { force } => rt.block_on(probe(&config, &runner, &redactor, force)),
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
        Command::Select {
            role,
            exclude_family,
            prefer,
            n,
            json,
        } => rt.block_on(select(
            &config,
            &redactor,
            &SelectArgs {
                role,
                exclude_family,
                prefer,
                n,
                json,
            },
        )),
        Command::Alert => rt.block_on(alert(&config, &redactor)),
    };
    ExitCode::from(code)
}

/// Exit code of `alert` when a sink (or the dedup write) failed: the crossing is retried on the next run.
const ALERT_SINK_FAILED: u8 = 1;
/// How long one kanban create may take (`yurtle-kanban --push` fetches, commits and pushes).
const KANBAN_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);
/// How long the webhook POST may take.
const WEBHOOK_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);

/// `quotabus alert` (DESIGN §3 alert; EXP-004): read the configured keys with the freshness rule, file each crossing
/// to every `[alert.*]` sink and stdout, keep the dedup state at `alert.<key>`. Exit 0 when every sink it called
/// succeeded (or nothing crossed) · 1 a sink or a dedup write failed (retried next run) · 2 CANNOT-ASSESS (the store
/// cannot be read; nothing is filed).
async fn alert(config: &Config, redactor: &Redactor) -> u8 {
    let result = match nats_url(config) {
        Some((url, bucket)) => match NatsKv::connect_reader(&url, &bucket).await {
            Ok(kv) => Ok(alert_with(&kv, config, redactor).await),
            Err(e) => Err(e),
        },
        None => match file_dir(config) {
            Ok(dir) => Ok(alert_with(&FileBackend::new(dir), config, redactor).await),
            Err(e) => Err(BackendError::Other(e)),
        },
    };
    match result {
        Ok(Ok(rc)) => rc,
        Ok(Err(e)) | Err(e) => {
            eprintln!(
                "quotabus: CANNOT-ASSESS: {}; nothing filed",
                redactor.redact(&e.to_string())
            );
            CANNOT_ASSESS
        }
    }
}

async fn alert_with(
    backend: &impl Backend,
    config: &Config,
    redactor: &Redactor,
) -> Result<u8, BackendError> {
    use quotabus::alert::{AlertEntry, Crossing, Decision, Pending, alert_key, decide_listed};
    let listing = backend.scan().await?;
    let by_key: HashMap<&str, &Record> = listing.rows.iter().map(|r| (r.key.as_str(), r)).collect();
    let unreadable: HashSet<&str> = listing.unreadable.iter().map(|(k, _)| k.as_str()).collect();
    let now = Utc::now();
    let client = reqwest::Client::builder()
        .timeout(WEBHOOK_TIMEOUT)
        .build()
        .map_err(|e| BackendError::Other(format!("cannot build the HTTP client: {e}")))?;
    let mut rc = 0;
    // CHORE-024: `checked` counts the api and subscription slots alert examines; `skipped` counts the slots of local
    // services (CHORE-021: never alerted), so `checked + skipped` is every slot. A local service that discovers its
    // models counts its slots as `status` does (`slots_in`): the ids of its newest cycle in the store, or one slot
    // (its id) when the store holds none of its rows.
    let (mut filed, mut cleared, mut checked, mut skipped) = (0usize, 0usize, 0usize, 0usize);
    let mut out = std::io::stdout();
    for svc in &config.services {
        if !quotabus::alert::alerts_for(svc.kind) {
            // CHORE-021: a local service files no alert; its `alert.<key>` is neither read nor written
            skipped += svc.slots_in(&listing.rows).len();
            continue;
        }
        for model in &svc.slots() {
            checked += 1;
            let key = quotabus::record_key(svc.kind, &svc.provider, &svc.account, model);
            let row = by_key.get(key.as_str()).copied();
            let verdict = if unreadable.contains(key.as_str()) {
                Verdict::CannotAssess {
                    reason: "cannot_assess:unreadable_row".to_string(),
                }
            } else {
                freshness(row, now)
            };
            let akey = alert_key(&key);
            let entry = match backend.get_entry(&akey).await {
                Ok(None) => None,
                Ok(Some(bytes)) => match serde_json::from_slice::<AlertEntry>(&bytes) {
                    Ok(e) => Some(e),
                    Err(e) => {
                        // unknown whether it was filed: neither file (a duplicate) nor clear; the operator decides
                        eprintln!(
                            "quotabus: {akey} is not an alert entry ({e}); {key} skipped until it is fixed or removed"
                        );
                        rc = ALERT_SINK_FAILED;
                        continue;
                    }
                },
                Err(e) => {
                    eprintln!(
                        "quotabus: cannot read {akey}: {}; {key} skipped",
                        redactor.redact(&e.to_string())
                    );
                    rc = ALERT_SINK_FAILED;
                    continue;
                }
            };
            let write = |e: AlertEntry| async move {
                let bytes =
                    serde_json::to_vec(&e).map_err(|x| BackendError::Other(x.to_string()))?;
                backend.put_entry(&alert_key(&e.key), bytes).await
            };
            let row_reason = row.and_then(|r| r.reason.as_deref());
            match decide_listed(&verdict, row_reason, entry.as_ref(), &config.alert.states) {
                Decision::Nothing => {}
                Decision::Clear => {
                    match write(AlertEntry::new(&key, quotabus::State::Ok, None, now)).await {
                        Ok(()) => {
                            cleared += 1;
                            let line = format!(
                                "CLEARED {key} ok ({} {model}): a measured ok re-armed it; nothing was closed\n",
                                svc.id
                            );
                            let _ = out.write_all(redactor.redact(&line).as_bytes());
                        }
                        Err(e) => {
                            eprintln!(
                                "quotabus: cannot clear {key}: {}",
                                redactor.redact(&e.to_string())
                            );
                            rc = ALERT_SINK_FAILED;
                        }
                    }
                }
                Decision::File { state, skip } => {
                    let Some(row) = row else { continue };
                    let crossing = Crossing::new(&svc.id, model, row, |t| redactor.redact_error(t));
                    let (done, failed) =
                        file_crossing(config, redactor, &client, &crossing, &skip).await;
                    let mut line = format!(
                        "ALERT {key} {} ({} {model}): {}",
                        state.as_str(),
                        svc.id,
                        crossing.title
                    );
                    let mut all_done: Vec<String> = skip.clone();
                    all_done.extend(done.iter().map(|(n, _)| n.clone()));
                    let named: Vec<String> = done
                        .iter()
                        .map(|(n, id)| match id {
                            Some(id) => format!("{n} ({id})"),
                            None => n.clone(),
                        })
                        .collect();
                    if !named.is_empty() {
                        line.push_str(&format!("; filed to {}", named.join(", ")));
                    }
                    if !skip.is_empty() {
                        line.push_str(&format!("; already filed to {}", skip.join(", ")));
                    }
                    let previous = entry
                        .as_ref()
                        .map(|e| e.state)
                        .unwrap_or(quotabus::State::Ok);
                    let next = if failed.is_empty() {
                        AlertEntry::new(&key, state, None, now)
                    } else {
                        rc = ALERT_SINK_FAILED;
                        let why: Vec<String> =
                            failed.iter().map(|(n, e)| format!("{n}: {e}")).collect();
                        line.push_str(&format!("; FAILED {} (retried next run)", why.join("; ")));
                        AlertEntry::new(
                            &key,
                            previous,
                            Some(Pending {
                                state,
                                filed: all_done,
                            }),
                            now,
                        )
                    };
                    line.push('\n');
                    let _ = out.write_all(redactor.redact(&line).as_bytes());
                    let wrote_pending = next.pending.is_some();
                    if let Err(e) = write(next).await {
                        eprintln!(
                            "quotabus: cannot record {akey}: {}; the crossing may be filed again next run",
                            redactor.redact(&e.to_string())
                        );
                        rc = ALERT_SINK_FAILED;
                    } else if !wrote_pending {
                        filed += 1;
                    }
                }
            }
        }
    }
    let _ = out.write_all(
        format!(
            "alert: {checked} keys checked, {skipped} local skipped, {filed} crossings filed, {cleared} re-armed\n"
        )
            .as_bytes(),
    );
    Ok(rc)
}

/// File one crossing to every configured sink not in `skip`: the sinks that succeeded (with the item id a kanban
/// sink printed, when it printed one) and those that failed (with why, redacted).
async fn file_crossing(
    config: &Config,
    redactor: &Redactor,
    client: &reqwest::Client,
    crossing: &quotabus::alert::Crossing,
    skip: &[String],
) -> (Vec<(String, Option<String>)>, Vec<(String, String)>) {
    use quotabus::alert::{Board, WEBHOOK, kanban_args};
    let mut done = Vec::new();
    let mut failed = Vec::new();
    let body = redactor.redact(&crossing.body());
    for (board, sink) in [
        (Board::NusyKanban, &config.alert.nusy_kanban),
        (Board::YurtleKanban, &config.alert.yurtle_kanban),
    ] {
        let Some(sink) = sink else { continue };
        if skip.iter().any(|s| s == board.name()) {
            continue;
        }
        let args = kanban_args(board, &sink.item_type, &sink.tags, &crossing.title);
        match run_kanban(&sink.command, &args, &body).await {
            Ok(id) => done.push((board.name().to_string(), id.map(|i| redactor.redact(&i)))),
            Err(e) => failed.push((board.name().to_string(), redactor.redact_error(&e))),
        }
    }
    if let Some(hook) = &config.alert.webhook
        && !skip.iter().any(|s| s == WEBHOOK)
    {
        // every text field was redacted when the crossing was built; the JSON is redacted again as a whole
        let json = serde_json::to_string(crossing).unwrap_or_default();
        let json = redactor.redact(&json);
        let sent = client
            .post(&hook.url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(json)
            .send()
            .await;
        match sent {
            Ok(r) if r.status().is_success() => done.push((WEBHOOK.to_string(), None)),
            // the URL is never printed: a webhook URL is often itself a credential
            Ok(r) => failed.push((WEBHOOK.to_string(), format!("HTTP {}", r.status().as_u16()))),
            Err(e) => failed.push((
                WEBHOOK.to_string(),
                redactor.redact_error(&e.without_url().to_string()),
            )),
        }
    }
    (done, failed)
}

/// Run one kanban create with the body on stdin. The child's environment is `CHILD_ENV` only (PATH, HOME, TMPDIR,
/// USER, LANG, TERM, each if set); a sink that needs more (a kanban server, an SSH agent) is a wrapper script that
/// sets it. Ok with the first `SIG-…`-shaped id it printed, if any.
async fn run_kanban(command: &str, args: &[String], body: &str) -> Result<Option<String>, String> {
    use tokio::io::AsyncWriteExt;
    let mut cmd = tokio::process::Command::new(command);
    cmd.args(args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);
    // r1 F3: an allowlist only, as the `claude` fallback has: under `doppler run` the environment holds every secret
    // of the project, configured or not, and none of it reaches a sink
    cmd.env_clear();
    for n in quotabus::subscription::CHILD_ENV {
        if let Some(v) = std::env::var_os(n) {
            cmd.env(n, v);
        }
    }
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("cannot run {command}: {e}"))?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(body.as_bytes())
            .await
            .map_err(|e| format!("cannot write the body to {command}: {e}"))?;
    }
    let out = tokio::time::timeout(KANBAN_TIMEOUT, child.wait_with_output())
        .await
        .map_err(|_| {
            format!(
                "{command} did not finish within {}s",
                KANBAN_TIMEOUT.as_secs()
            )
        })?
        .map_err(|e| format!("{command}: {e}"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        let last = err
            .lines()
            .rev()
            .find(|l| !l.trim().is_empty())
            .unwrap_or("");
        return Err(format!(
            "{command} exited {}: {}",
            out.status
                .code()
                .map_or("by a signal".to_string(), |c| c.to_string()),
            last.trim()
        ));
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    let id = stdout
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
        .find(|w| {
            w.split_once('-').is_some_and(|(p, n)| {
                !p.is_empty()
                    && p.chars().all(|c| c.is_ascii_uppercase())
                    && !n.is_empty()
                    && n.chars().all(|c| c.is_ascii_digit())
            })
        });
    Ok(id.map(str::to_string))
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

/// The rows already in the store: what is due is read from them. A store never written to is empty.
async fn stored(backend: &impl Backend) -> Result<Vec<Record>, String> {
    match backend.list().await {
        Ok(rows) => Ok(rows),
        Err(BackendError::BucketMissing(_)) => Ok(Vec::new()),
        Err(e) => Err(e.to_string()),
    }
}

/// Why a `probe` run wrote nothing (or not everything).
enum ProbeError {
    /// The store could not be read, so what is due is unknown; nothing was probed.
    StoreUnread(String),
    /// The rows (or some of them) could not be written, or the backend could not be opened.
    Publish(String),
}

impl ProbeError {
    /// The stderr line, redacted.
    fn message(&self, redactor: &Redactor) -> String {
        match self {
            ProbeError::StoreUnread(e) => format!(
                "quotabus: cannot read the store to tell what is due: {}; `quotabus probe --force` probes anyway",
                redactor.redact_error(e)
            ),
            ProbeError::Publish(e) => {
                format!("quotabus: cannot publish: {}", redactor.redact_error(e))
            }
        }
    }
}

/// Read the store, run what is due (everything with `force`), write the rows back: the rows, and how many were
/// written or why they were not.
async fn cycle(
    backend: &impl Backend,
    runner: &Runner,
    force: bool,
) -> (Vec<Record>, Result<usize, ProbeError>) {
    let existing = match stored(backend).await {
        Ok(rows) => rows,
        // probing blind would make every kind due on every tick: refuse rather than spend the calls
        Err(e) if !force => return (Vec::new(), Err(ProbeError::StoreUnread(e))),
        Err(_) => Vec::new(),
    };
    let rows = runner.run_due(&existing, Utc::now(), force).await;
    let written = publish(backend, &rows).await.map_err(ProbeError::Publish);
    (rows, written)
}

async fn probe(config: &Config, runner: &Runner, redactor: &Redactor, force: bool) -> u8 {
    let (rows, result) = match nats_url(config) {
        Some((url, bucket)) => match NatsKv::connect_publisher(&url, &bucket).await {
            Ok(kv) => {
                let (rows, n) = cycle(&kv, runner, force).await;
                (rows, n.map(|n| format!("{n} rows to bucket {bucket}")))
            }
            Err(e) => (Vec::new(), Err(ProbeError::Publish(e.to_string()))),
        },
        None => match file_dir(config) {
            Ok(dir) => {
                let (rows, n) = cycle(&FileBackend::new(&dir), runner, force).await;
                (rows, n.map(|n| format!("{n} rows to {}", dir.display())))
            }
            Err(e) => (Vec::new(), Err(ProbeError::Publish(e))),
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
    if rows.is_empty() && result.is_ok() {
        out.push_str("nothing due\n");
    }
    print!("{}", redactor.redact(&out));
    match result {
        Ok(what) => {
            println!("published {what}");
            0
        }
        Err(e) => {
            eprintln!("{}", e.message(redactor));
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
        for model in &svc.slots_in(&listing.rows) {
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
        // our own message (a bucket or directory name, a client error), redacted but not capped: the path survives
        eprintln!(
            "quotabus: CANNOT-ASSESS: {}",
            redactor.redact(&e.to_string())
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

struct SelectArgs {
    role: String,
    exclude_family: Vec<String>,
    prefer: Option<String>,
    n: usize,
    json: bool,
}

/// `quotabus select`: read the store (never write it, DESIGN §10 Q6) and print the first `--n` candidates. Exit 0
/// chosen · 2 CANNOT-ASSESS (the store cannot be read, or `--prefer` is not a known word) · 3 nothing qualifies.
/// Whenever the rc is not 0, stdout names no model, so `$(quotabus select …)` never yields a usable pick.
async fn select(config: &Config, redactor: &Redactor, args: &SelectArgs) -> u8 {
    let prefer = match args.prefer.as_deref().map(quotabus::Prefer::from_str) {
        None => None,
        Some(Ok(p)) => Some(p),
        Some(Err(e)) => {
            eprintln!("quotabus: CANNOT-ASSESS: {}", redactor.redact(&e));
            return CANNOT_ASSESS;
        }
    };
    let listing = match read_rows(config).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!(
                "quotabus: CANNOT-ASSESS: {}; nothing selected",
                redactor.redact(&e.to_string())
            );
            return CANNOT_ASSESS;
        }
    };
    let query = quotabus::Query {
        role: args.role.clone(),
        exclude_families: args.exclude_family.clone(),
        prefer,
        now: Utc::now(),
    };
    let mut chosen = quotabus::select(&listing.rows, &query, config);
    chosen.truncate(args.n.max(1));
    let out = if args.json {
        let mut s = serde_json::to_string_pretty(&chosen).unwrap_or_else(|_| "[]".to_string());
        s.push('\n');
        s
    } else {
        chosen
            .iter()
            .map(|c| format!("{} {}\n", c.provider, c.model))
            .collect()
    };
    print!("{}", redactor.redact(&out));
    if chosen.is_empty() {
        let excluding = if args.exclude_family.is_empty() {
            String::new()
        } else {
            format!(" excluding {}", args.exclude_family.join(", "))
        };
        eprintln!(
            "quotabus: nothing qualifies for role {:?}{excluding}: no configured model with the role has a fresh ok row",
            args.role
        );
        return UNKNOWN;
    }
    0
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
        // a measured row's own reason (`window_near_limit`, CHORE-018 r1 F2) when the verdict carries none
        let mut detail = (e.verdict.reason())
            .or_else(|| row.and_then(|r| r.reason.clone()))
            .unwrap_or_default();
        if let Some(b) = row.and_then(|r| r.balance.as_ref()) {
            if !detail.is_empty() {
                detail.push(' ');
            }
            detail.push_str(&format!("balance {} {}", b.amount, b.currency));
        }
        for w in row
            .and_then(|r| r.headroom.as_ref())
            .map(|h| h.windows.as_slice())
            .unwrap_or_default()
        {
            if !detail.is_empty() {
                detail.push(' ');
            }
            detail.push_str(&format!("{} {:.0}%", w.window, w.used_pct));
            if let Some(s) = w.reset_in_s {
                detail.push_str(&format!(" (resets in {})", human_age(s)));
            }
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
