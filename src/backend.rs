//! Where rows live: a NATS KV bucket with per-key TTL, or a directory of `<key>.json` files (DESIGN §3, §8).

use std::future::Future;
use std::path::PathBuf;
use std::time::Duration;

use async_nats::jetstream::context::PublishErrorKind;
use async_nats::jetstream::{self, kv};
use futures::StreamExt;

use crate::record::{Record, State};
use crate::secret::Secret;

/// Remove every credential `bus` carried from `text` (an error from the client), whatever the redactor knows.
fn scrub(bus: &BusUrl, text: &str) -> String {
    let mut values: Vec<String> = bus
        .secrets()
        .iter()
        .map(|s| s.expose().to_string())
        .collect();
    values.sort_by_key(|v| std::cmp::Reverse(v.len()));
    let mut out = text.to_string();
    for v in values {
        out = out.replace(&v, "***");
    }
    out
}

#[derive(Debug)]
pub enum BackendError {
    /// The bus could not be reached.
    Unreachable(String),
    /// The bucket (or the file backend's row directory) does not exist; a reader never creates it. The payload
    /// names it, e.g. `bucket ai_status` or `row directory /path`.
    BucketMissing(String),
    Other(String),
}

impl std::fmt::Display for BackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackendError::Unreachable(m) => write!(f, "bus unreachable: {m}"),
            BackendError::BucketMissing(what) => write!(f, "{what} does not exist"),
            BackendError::Other(m) => f.write_str(m),
        }
    }
}

impl std::error::Error for BackendError {}

impl BackendError {
    /// The `cannot_assess:<why>` a reader reports when this error stops it.
    pub fn reason(&self) -> &'static str {
        match self {
            BackendError::Unreachable(_) => "cannot_assess:bus_unreachable",
            BackendError::BucketMissing(_) => "cannot_assess:bucket_missing",
            BackendError::Other(_) => "cannot_assess:backend_error",
        }
    }
}

/// What a reader found: the rows it could parse, and the keys present whose value is not a row.
#[derive(Debug, Default)]
pub struct Listing {
    pub rows: Vec<Record>,
    /// (key, why) for each key that exists but does not parse: CANNOT-ASSESS, never absent (DESIGN §2).
    pub unreadable: Vec<(String, String)>,
}

pub trait Backend {
    /// Write a row under `row.key`; a bus backend gives it a per-key TTL of `row.ttl_s`.
    fn put(&self, row: &Record) -> impl Future<Output = Result<(), BackendError>> + Send;
    /// The row for `key`, or `None` when it is absent (or expired out of the bucket).
    fn get(&self, key: &str) -> impl Future<Output = Result<Option<Record>, BackendError>> + Send;
    /// Every key present. A failed read part-way through is an error for the whole listing.
    fn scan(&self) -> impl Future<Output = Result<Listing, BackendError>> + Send;
    /// Every row present (unparseable keys are left out; [`Backend::scan`] reports them).
    fn list(&self) -> impl Future<Output = Result<Vec<Record>, BackendError>> + Send;
}

fn parse_row(key: &str, bytes: &[u8]) -> Result<Record, String> {
    serde_json::from_slice(bytes).map_err(|e| format!("{key} is not a row: {e}"))
}

/// `<dir>/<key>.json`.
#[derive(Debug, Clone)]
pub struct FileBackend {
    pub dir: PathBuf,
}

impl FileBackend {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        FileBackend { dir: dir.into() }
    }

    fn path(&self, key: &str) -> Result<PathBuf, BackendError> {
        // a key is slugs joined by `.`; refuse anything that could leave the directory
        if key.is_empty() || key.contains('/') || key.contains('\\') || key.starts_with('.') {
            return Err(BackendError::Other(format!("invalid key {key:?}")));
        }
        Ok(self.dir.join(format!("{key}.json")))
    }
}

fn other(context: &str, e: impl std::fmt::Display) -> BackendError {
    BackendError::Other(format!("{context}: {e}"))
}

impl Backend for FileBackend {
    async fn put(&self, row: &Record) -> Result<(), BackendError> {
        let path = self.path(&row.key)?;
        tokio::fs::create_dir_all(&self.dir)
            .await
            .map_err(|e| other(&format!("cannot create {}", self.dir.display()), e))?;
        let bytes = serde_json::to_vec(row).map_err(|e| other("serialise row", e))?;
        // write then rename, so a reader never sees half a row
        let tmp = self.dir.join(format!(".{}.json.tmp", row.key));
        tokio::fs::write(&tmp, bytes)
            .await
            .map_err(|e| other(&format!("cannot write {}", tmp.display()), e))?;
        tokio::fs::rename(&tmp, &path)
            .await
            .map_err(|e| other(&format!("cannot write {}", path.display()), e))
    }

    async fn get(&self, key: &str) -> Result<Option<Record>, BackendError> {
        match self.read_raw(key).await? {
            Some(bytes) => parse_row(key, &bytes)
                .map(Some)
                .map_err(BackendError::Other),
            None => Ok(None),
        }
    }

    async fn scan(&self) -> Result<Listing, BackendError> {
        let mut rd = match tokio::fs::read_dir(&self.dir).await {
            Ok(rd) => rd,
            // a missing directory is not "no rows": it may be a mistyped `[file] dir` (the probe creates it)
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(BackendError::BucketMissing(format!(
                    "row directory {}",
                    self.dir.display()
                )));
            }
            Err(e) => return Err(other(&format!("cannot read {}", self.dir.display()), e)),
        };
        let mut keys = Vec::new();
        while let Some(entry) = rd
            .next_entry()
            .await
            .map_err(|e| other(&format!("cannot read {}", self.dir.display()), e))?
        {
            let name = entry.file_name().to_string_lossy().into_owned();
            if let Some(key) = name.strip_suffix(".json")
                && !key.starts_with('.')
            {
                keys.push(key.to_string());
            }
        }
        keys.sort();
        let mut listing = Listing::default();
        for key in keys {
            if let Some(bytes) = self.read_raw(&key).await? {
                push_parsed(&mut listing, key, &bytes);
            }
        }
        Ok(listing)
    }

    async fn list(&self) -> Result<Vec<Record>, BackendError> {
        Ok(self.scan().await?.rows)
    }
}

impl FileBackend {
    async fn read_raw(&self, key: &str) -> Result<Option<Vec<u8>>, BackendError> {
        let path = self.path(key)?;
        match tokio::fs::read(&path).await {
            Ok(bytes) => Ok(Some(bytes)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(other(&format!("cannot read {}", path.display()), e)),
        }
    }
}

fn push_parsed(listing: &mut Listing, key: String, bytes: &[u8]) {
    match parse_row(&key, bytes) {
        Ok(r) => listing.rows.push(r),
        Err(why) => {
            tracing::warn!("unreadable row: {why}");
            listing.unreadable.push((key, why));
        }
    }
}

/// How long the server keeps the delete marker it writes when a key's TTL expires (so a watcher sees the delete).
const MARKER_TTL: Duration = Duration::from_secs(300);
/// Connect and request deadline for the bus: a bus that is down fails promptly.
const BUS_TIMEOUT: Duration = Duration::from_secs(5);

/// A NATS KV bucket.
pub struct NatsKv {
    pub bucket: String,
    js: jetstream::Context,
    store: kv::Store,
}

/// A bus URL split into the address async-nats connects to (no userinfo) and the credentials it carried
/// (`nats://user:pass@host` or a token-only `nats://token@host`; HAZ-001). A comma-separated server list is split per
/// server and handed to async-nats as a list; the connection has one set of credentials, so the FIRST server with
/// userinfo supplies them (every server's userinfo is still registered as a secret).
#[derive(Clone)]
pub struct BusUrl {
    /// The server address(es) with any userinfo removed, comma-joined: the only form we may print.
    pub address: String,
    /// The same, one per server: what we connect to.
    pub servers: Vec<String>,
    pub user: Option<String>,
    pub password: Option<Secret>,
    pub token: Option<Secret>,
    /// Every userinfo value as written (percent-encoded or not) and decoded, for the redactor.
    raw_secrets: Vec<Secret>,
}

impl std::fmt::Debug for BusUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BusUrl({})", self.display())
    }
}

impl BusUrl {
    pub fn parse(url: &str) -> BusUrl {
        let mut out = BusUrl {
            address: String::new(),
            servers: Vec::new(),
            user: None,
            password: None,
            token: None,
            raw_secrets: Vec::new(),
        };
        let mut servers = Vec::new();
        for server in url.split(',') {
            let server = server.trim();
            let (scheme, rest) = match server.find("://") {
                Some(i) => (&server[..i + 3], &server[i + 3..]),
                None => ("", server),
            };
            let auth_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
            let (authority, tail) = rest.split_at(auth_end);
            let Some(at) = authority.rfind('@') else {
                servers.push(server.to_string());
                continue;
            };
            let (userinfo, host) = (&authority[..at], &authority[at + 1..]);
            servers.push(format!("{scheme}{host}{tail}"));
            match userinfo.split_once(':') {
                Some((u, p)) => {
                    out.raw_secrets.push(Secret::new(p));
                    if out.user.is_none() && out.token.is_none() {
                        out.user = Some(percent_decode(u));
                        out.password = Some(Secret::new(percent_decode(p)));
                    }
                }
                None => {
                    out.raw_secrets.push(Secret::new(userinfo));
                    if out.user.is_none() && out.token.is_none() {
                        out.token = Some(Secret::new(percent_decode(userinfo)));
                    }
                }
            }
        }
        out.address = servers.join(",");
        out.servers = servers;
        out
    }

    /// Every secret value the URL carried (raw and decoded), to register with the [`crate::Redactor`] before any
    /// connection is attempted: async-nats's own debug/trace logging may carry them.
    pub fn secrets(&self) -> Vec<Secret> {
        let mut v = self.raw_secrets.clone();
        v.extend(self.password.iter().cloned());
        v.extend(self.token.iter().cloned());
        v.retain(|s| !s.expose().is_empty());
        v
    }

    /// The URL safe to print: `nats://user:***@host` (or `nats://***@host` for a token).
    pub fn display(&self) -> String {
        let (first, rest) = match self.address.split_once(',') {
            Some((a, b)) => (a, Some(b)),
            None => (self.address.as_str(), None),
        };
        let who = match (&self.user, &self.token) {
            (Some(u), _) => Some(format!("{u}:***@")),
            (None, Some(_)) => Some("***@".to_string()),
            _ => None,
        };
        let first = match (who, first.find("://")) {
            (Some(w), Some(i)) => format!("{}{w}{}", &first[..i + 3], &first[i + 3..]),
            (Some(w), None) => format!("{w}{first}"),
            (None, _) => first.to_string(),
        };
        match rest {
            Some(r) => format!("{first},{r}"),
            None => first,
        }
    }
}

/// Decode `%XX` escapes (RFC 3986 userinfo); an invalid escape is kept as written.
fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%'
            && i + 2 < b.len()
            && let Some(v) = std::str::from_utf8(&b[i + 1..i + 3])
                .ok()
                .and_then(|h| u8::from_str_radix(h, 16).ok())
        {
            out.push(v);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

async fn connect(url: &str) -> Result<jetstream::Context, BackendError> {
    let bus = BusUrl::parse(url);
    let options = match (&bus.user, &bus.password, &bus.token) {
        (Some(u), Some(p), _) => {
            async_nats::ConnectOptions::with_user_and_password(u.clone(), p.expose().to_string())
        }
        (_, _, Some(t)) => async_nats::ConnectOptions::with_token(t.expose().to_string()),
        _ => async_nats::ConnectOptions::new(),
    };
    let client = options
        .connection_timeout(BUS_TIMEOUT)
        .request_timeout(Some(BUS_TIMEOUT))
        .connect(bus.servers.as_slice())
        .await
        .map_err(|e| BackendError::Unreachable(scrub(&bus, &e.to_string())))?;
    Ok(jetstream::new(client))
}

/// The bucket's stream, or `BucketMissing` when there is none.
async fn bucket_stream(
    js: &jetstream::Context,
    bucket: &str,
) -> Result<jetstream::stream::Stream, BackendError> {
    use async_nats::jetstream::context::GetStreamErrorKind;
    match js.get_stream(format!("KV_{bucket}")).await {
        Ok(s) => Ok(s),
        Err(e) => match e.kind() {
            GetStreamErrorKind::JetStream(je)
                if je.error_code() == jetstream::ErrorCode::STREAM_NOT_FOUND =>
            {
                Err(BackendError::BucketMissing(format!("bucket {bucket}")))
            }
            GetStreamErrorKind::Request => Err(BackendError::Unreachable(e.to_string())),
            _ => Err(other(&format!("bucket {bucket}"), e)),
        },
    }
}

/// The subject a state change of `key` is announced on: `ai.status.changed.<key>` (DESIGN §3 Outputs).
pub fn change_subject(key: &str) -> String {
    format!("{CHANGE_SUBJECT_PREFIX}{key}")
}

/// The prefix of every change subject; subscribe to `ai.status.changed.>` for all of them.
pub const CHANGE_SUBJECT_PREFIX: &str = "ai.status.changed.";

impl NatsKv {
    /// Connect and create the bucket if missing, WITH per-key TTL (`limit_markers`, so the stream has
    /// `allow_msg_ttl`). For the probe.
    pub async fn connect_publisher(url: &str, bucket: &str) -> Result<NatsKv, BackendError> {
        let js = connect(url).await?;
        match bucket_stream(&js, bucket).await {
            Ok(stream) => {
                if !stream.cached_info().config.allow_message_ttl {
                    return Err(BackendError::Other(format!(
                        "bucket {bucket} exists without per-key TTL (allow_msg_ttl); quotabus needs a bucket it \
                         created, or one made with `nats kv add {bucket} --marker-ttl=…`"
                    )));
                }
            }
            Err(BackendError::BucketMissing(_)) => {
                js.create_key_value(kv::Config {
                    bucket: bucket.to_string(),
                    description: "quotabus: AI provider & account status (ai-status/1)".to_string(),
                    history: 1,
                    limit_markers: Some(MARKER_TTL),
                    ..Default::default()
                })
                .await
                .map_err(|e| other(&format!("cannot create bucket {bucket}"), e))?;
            }
            Err(e) => return Err(e),
        }
        Self::open(js, bucket).await
    }

    /// Connect to an existing bucket; never creates it. For readers.
    pub async fn connect_reader(url: &str, bucket: &str) -> Result<NatsKv, BackendError> {
        let js = connect(url).await?;
        bucket_stream(&js, bucket).await?;
        Self::open(js, bucket).await
    }

    async fn open(js: jetstream::Context, bucket: &str) -> Result<NatsKv, BackendError> {
        let store = js
            .get_key_value(bucket)
            .await
            .map_err(|e| other(&format!("bucket {bucket}"), e))?;
        Ok(NatsKv {
            bucket: bucket.to_string(),
            js,
            store,
        })
    }

    /// The subject a put for `key` goes to, `$KV.<bucket>.<key>`, as `Store::put` builds it (quotabus uses the
    /// default JetStream API prefix, so there is no domain prefix to add).
    fn put_subject(&self, key: &str) -> String {
        let prefix = self.store.put_prefix.as_ref().unwrap_or(&self.store.prefix);
        format!("{prefix}{key}")
    }
}

impl Backend for NatsKv {
    async fn put(&self, row: &Record) -> Result<(), BackendError> {
        let bytes = serde_json::to_vec(row).map_err(|e| other("serialise row", e))?;
        // The write is conditional on the revision it read (`Nats-Expected-Last-Subject-Sequence`, as async-nats's own
        // `Store::update` does), so the announce decision is made against the row actually replaced: a concurrent
        // writer that got in between makes the server refuse this write, and it re-reads and retries (review r1 F2).
        // The previous row's state decides the change subject (DESIGN §3 Outputs). Absent (never written, or expired
        // out) and unreadable both count as a change [INFERENCE]: the reader then learns of a state it could not have
        // known. After CAS_ATTEMPTS refusals, or when the last revision cannot be read, the row is written
        // unconditionally and announced: a duplicate announce is harmless, a missing one is not.
        let subject = self.put_subject(&row.key);
        let mut previous = None;
        let mut written = false;
        for _ in 0..CAS_ATTEMPTS {
            let Some((revision, prev)) = self.last_revision(&subject).await else {
                break;
            };
            let mut headers = ttl_headers(row);
            headers.insert(
                async_nats::header::NATS_EXPECTED_LAST_SUBJECT_SEQUENCE,
                revision.to_string().as_str(),
            );
            let ack = self
                .js
                .publish_with_headers(subject.clone(), headers, bytes.clone().into())
                .await
                .map_err(|e| other(&format!("put {}", row.key), e))?
                .await;
            match ack {
                Ok(_) => {
                    previous = prev;
                    written = true;
                    break;
                }
                Err(e) if e.kind() == PublishErrorKind::WrongLastSequence => continue,
                Err(e) => return Err(other(&format!("put {} (no ack)", row.key), e)),
            }
        }
        if !written {
            // async-nats 0.50's Store has no put-with-TTL: publish to the key's subject with a `Nats-TTL` header
            self.js
                .publish_with_headers(subject, ttl_headers(row), bytes.clone().into())
                .await
                .map_err(|e| other(&format!("put {}", row.key), e))?
                .await
                .map_err(|e| other(&format!("put {} (no ack)", row.key), e))?;
        }
        if previous != Some(row.state) {
            // core NATS, not JetStream: no stream captures `ai.status.changed.>`, so there is no ack to wait for. The
            // row is already stored, so a failed announcement does not fail the put.
            let client = self.js.client();
            let announced = client
                .publish(change_subject(&row.key), bytes.into())
                .await
                .is_ok();
            if announced {
                let _ = client.flush().await;
            }
        }
        Ok(())
    }

    async fn get(&self, key: &str) -> Result<Option<Record>, BackendError> {
        match self.read_raw(key).await? {
            Some(bytes) => parse_row(key, &bytes)
                .map(Some)
                .map_err(BackendError::Other),
            None => Ok(None),
        }
    }

    async fn scan(&self) -> Result<Listing, BackendError> {
        let mut keys = self.store.keys().await.map_err(|e| {
            BackendError::Unreachable(format!("cannot list bucket {}: {e}", self.bucket))
        })?;
        let mut names = Vec::new();
        while let Some(k) = keys.next().await {
            names.push(k.map_err(|e| {
                BackendError::Unreachable(format!("cannot list bucket {}: {e}", self.bucket))
            })?);
        }
        names.sort();
        let mut listing = Listing::default();
        for key in names {
            // a read that fails part-way is a failure to measure for the whole listing, never `absent`
            if let Some(bytes) = self.read_raw(&key).await? {
                push_parsed(&mut listing, key, &bytes);
            }
        }
        Ok(listing)
    }

    async fn list(&self) -> Result<Vec<Record>, BackendError> {
        Ok(self.scan().await?.rows)
    }
}

/// How many times a conditional write is retried after a concurrent writer moved the key's revision.
const CAS_ATTEMPTS: usize = 5;

/// The per-key TTL header of a row's write.
fn ttl_headers(row: &Record) -> async_nats::HeaderMap {
    let mut headers = async_nats::HeaderMap::new();
    headers.insert(
        async_nats::header::NATS_MESSAGE_TTL,
        format!("{}s", row.ttl_s.max(1)).as_str(),
    );
    headers
}

impl NatsKv {
    /// The last message on a key's subject: its stream sequence (0 when there is none) and, when it is a row, that
    /// row's state. A delete or TTL marker has no row, so its state is `None`. `None` when it cannot be read.
    async fn last_revision(&self, subject: &str) -> Option<(u64, Option<State>)> {
        use async_nats::jetstream::stream::LastRawMessageErrorKind;
        match self
            .store
            .stream
            .get_last_raw_message_by_subject(subject)
            .await
        {
            Ok(m) => Some((
                m.sequence,
                serde_json::from_slice::<Record>(&m.payload)
                    .ok()
                    .map(|r| r.state),
            )),
            Err(e) => match e.kind() {
                LastRawMessageErrorKind::NoMessageFound => Some((0, None)),
                LastRawMessageErrorKind::JetStream(je)
                    if je.error_code() == jetstream::ErrorCode::NO_MESSAGE_FOUND =>
                {
                    Some((0, None))
                }
                _ => None,
            },
        }
    }

    async fn read_raw(&self, key: &str) -> Result<Option<Vec<u8>>, BackendError> {
        match self.store.get(key).await {
            Ok(v) => Ok(v.map(|b| b.to_vec())),
            Err(e) if e.kind() == kv::EntryErrorKind::TimedOut => {
                Err(BackendError::Unreachable(format!("cannot read {key}: {e}")))
            }
            Err(e) => Err(other(&format!("cannot read {key}"), e)),
        }
    }
}

#[cfg(test)]
mod bus_url_tests {
    use super::BusUrl;

    #[test]
    fn user_and_password_are_split_out_and_never_displayed() {
        let b = BusUrl::parse("nats://qb:p%40ss@127.0.0.1:4222");
        assert_eq!(b.address, "nats://127.0.0.1:4222");
        assert_eq!(b.user.as_deref(), Some("qb"));
        assert_eq!(b.password.as_ref().map(|p| p.expose()), Some("p@ss"));
        assert_eq!(b.display(), "nats://qb:***@127.0.0.1:4222");
        let s: Vec<String> = b.secrets().iter().map(|s| s.expose().to_string()).collect();
        assert!(s.contains(&"p%40ss".to_string()) && s.contains(&"p@ss".to_string()));
    }

    #[test]
    fn a_token_url_and_a_plain_url() {
        let b = BusUrl::parse("nats://tok123@h:4222,nats://h2:4222");
        assert_eq!(b.address, "nats://h:4222,nats://h2:4222");
        assert_eq!(b.servers, ["nats://h:4222", "nats://h2:4222"]);
        assert_eq!(b.token.as_ref().map(|t| t.expose()), Some("tok123"));
        assert_eq!(b.display(), "nats://***@h:4222,nats://h2:4222");
        let p = BusUrl::parse("nats://127.0.0.1:4222");
        assert_eq!(p.address, "nats://127.0.0.1:4222");
        assert!(p.user.is_none() && p.token.is_none() && p.secrets().is_empty());
        assert_eq!(p.display(), "nats://127.0.0.1:4222");
    }
}
