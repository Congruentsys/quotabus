//! Where rows live: a NATS KV bucket with per-key TTL, or a directory of `<key>.json` files (DESIGN §3, §8).

use std::future::Future;
use std::path::PathBuf;

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
        write!(f, "{self:?}")
    }
}

impl std::error::Error for BackendError {}

pub trait Backend {
    /// Write a row under `row.key`; a bus backend gives it a per-key TTL of `row.ttl_s`.
    fn put(&self, row: &Record) -> impl Future<Output = Result<(), BackendError>> + Send;
    /// The row for `key`, or `None` when it is absent (or expired out of the bucket).
    fn get(&self, key: &str) -> impl Future<Output = Result<Option<Record>, BackendError>> + Send;
    /// Every row present.
    fn list(&self) -> impl Future<Output = Result<Vec<Record>, BackendError>> + Send;
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
}

impl Backend for FileBackend {
    async fn put(&self, _row: &Record) -> Result<(), BackendError> {
        todo!("FileBackend::put")
    }
    async fn get(&self, _key: &str) -> Result<Option<Record>, BackendError> {
        todo!("FileBackend::get")
    }
    async fn list(&self) -> Result<Vec<Record>, BackendError> {
        todo!("FileBackend::list")
    }
}

/// A NATS KV bucket.
pub struct NatsKv {
    pub bucket: String,
}

impl NatsKv {
    /// Connect and create the bucket if missing, WITH per-key TTL (`limit_markers`, so the stream has
    /// `allow_msg_ttl`). For the probe.
    pub async fn connect_publisher(url: &str, bucket: &str) -> Result<NatsKv, BackendError> {
        let _ = (url, bucket);
        todo!("NatsKv::connect_publisher")
    }

    /// Connect to an existing bucket; never creates it. For readers.
    pub async fn connect_reader(url: &str, bucket: &str) -> Result<NatsKv, BackendError> {
        let _ = (url, bucket);
        todo!("NatsKv::connect_reader")
    }
}

impl Backend for NatsKv {
    async fn put(&self, _row: &Record) -> Result<(), BackendError> {
        todo!("NatsKv::put")
    }
    async fn get(&self, _key: &str) -> Result<Option<Record>, BackendError> {
        todo!("NatsKv::get")
    }
    async fn list(&self) -> Result<Vec<Record>, BackendError> {
        todo!("NatsKv::list")
    }
}
