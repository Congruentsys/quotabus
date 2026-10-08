//! Where rows live: a NATS KV bucket with per-key TTL, or a directory of `<key>.json` files (DESIGN §3, §8).

use std::future::Future;
use std::path::PathBuf;
use std::time::Duration;

use async_nats::jetstream::{self, kv};
use futures::StreamExt;

use crate::record::Record;

#[derive(Debug)]
pub enum BackendError {
    /// The bus could not be reached.
    Unreachable(String),
    /// The bucket does not exist (a reader never creates it).
    BucketMissing(String),
    Other(String),
}

impl std::fmt::Display for BackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackendError::Unreachable(m) => write!(f, "bus unreachable: {m}"),
            BackendError::BucketMissing(b) => write!(f, "bucket {b} does not exist"),
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
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Listing::default()),
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

async fn connect(url: &str) -> Result<jetstream::Context, BackendError> {
    let client = async_nats::ConnectOptions::new()
        .connection_timeout(BUS_TIMEOUT)
        .request_timeout(Some(BUS_TIMEOUT))
        .connect(url)
        .await
        .map_err(|e| BackendError::Unreachable(e.to_string()))?;
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
                Err(BackendError::BucketMissing(bucket.to_string()))
            }
            GetStreamErrorKind::Request => Err(BackendError::Unreachable(e.to_string())),
            _ => Err(other(&format!("bucket {bucket}"), e)),
        },
    }
}

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
        // async-nats 0.50's Store has no put-with-TTL: publish to the key's subject with a `Nats-TTL` header
        let mut headers = async_nats::HeaderMap::new();
        headers.insert(
            async_nats::header::NATS_MESSAGE_TTL,
            format!("{}s", row.ttl_s.max(1)).as_str(),
        );
        self.js
            .publish_with_headers(self.put_subject(&row.key), headers, bytes.into())
            .await
            .map_err(|e| other(&format!("put {}", row.key), e))?
            .await
            .map_err(|e| other(&format!("put {} (no ack)", row.key), e))?;
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

impl NatsKv {
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
