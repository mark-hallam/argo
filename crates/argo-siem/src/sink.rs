//! [`SiemSink`] trait + canonical [`AuditEvent`] shape. Each sink
//! accepts the same JSON projection; operators map fields to their
//! SIEM-side schema (sourcetype / ECS / facets) externally. New
//! sinks only have to implement [`SiemSink::ship`].
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// One `audit_record` row projected to the canonical shape sinks
/// accept. `seq` is the monotonic key; `id` (hex-encoded UUID-v7)
/// is what SIEMs dedupe on when receiving a re-shipped batch after
/// a crash.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// `audit_record.seq`.
    pub seq: i64,
    /// `audit_record.id` hex-encoded.
    pub id: String,
    /// `audit_record.session_id` hex-encoded, if non-null.
    pub session_id: Option<String>,
    /// `audit_record.kind`.
    pub kind: String,
    /// Decoded `audit_record.payload_json`.
    pub payload: serde_json::Value,
    /// `audit_record.created_at` (unix seconds).
    pub created_at: i64,
}

/// Errors a sink can surface to the dispatcher. The dispatcher
/// classifies these to decide retry vs. advance-and-log.
#[derive(Debug, thiserror::Error)]
pub enum SinkError {
    /// 5xx, network, or timeout — dispatcher retries with backoff.
    #[error("transient: {0}")]
    Transient(String),
    /// 4xx other than auth — dispatcher advances past the batch and
    /// writes an audit row; retrying won't help (bad shape, bad token).
    #[error("permanent: {0}")]
    Permanent(String),
    /// Serialization / configuration error — startup-side; not
    /// reachable from a healthy `ship` call but kept on the enum so
    /// the dispatcher's error-classification match stays exhaustive.
    #[error("config: {0}")]
    Config(String),
}

/// Async sink trait. Implementors are cheap to clone (typically an
/// `Arc<reqwest::Client>` + config).
#[async_trait]
pub trait SiemSink: Send + Sync {
    /// Backend discriminator stamped on the cursor row (`splunk`,
    /// `elastic`, `datadog`).
    fn backend(&self) -> &'static str;
    /// Ingest URL stamped on the cursor row so a config change to a
    /// different endpoint starts a fresh cursor.
    fn endpoint(&self) -> &str;
    /// Per-sink batch cap.
    fn max_batch(&self) -> usize;
    /// Ship one batch. Implementors MUST return
    /// [`SinkError::Transient`] for any network failure / timeout /
    /// 5xx so the dispatcher retries with backoff.
    async fn ship(&self, events: Vec<AuditEvent>) -> Result<(), SinkError>;
}

/// Credential wrapper whose `Debug` impl prints `<redacted>`. Use
/// [`SecretString::reveal`] only to hand the value to a header
/// builder; never log, `format!`, or persist the returned `&str`.
#[derive(Clone)]
pub struct SecretString(String);

impl SecretString {
    /// Wrap a credential.
    #[must_use]
    pub fn new(s: String) -> Self {
        Self(s)
    }
    /// Expose the underlying credential.
    #[must_use]
    pub fn reveal(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for SecretString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<redacted>")
    }
}

/// Map `reqwest` send result → [`SinkError`] (transient on 5xx /
/// network, permanent on 4xx). Shared so the three sinks classify
/// identically.
pub(crate) fn classify_response(
    resp: Result<reqwest::Response, reqwest::Error>,
) -> Result<(), SinkError> {
    match resp {
        Ok(r) if r.status().is_success() => Ok(()),
        Ok(r) if r.status().is_server_error() => Err(SinkError::Transient(format!(
            "siem sink 5xx: {}",
            r.status()
        ))),
        Ok(r) => Err(SinkError::Permanent(format!(
            "siem sink 4xx/3xx: {}",
            r.status()
        ))),
        Err(e) if e.is_timeout() || e.is_connect() => {
            Err(SinkError::Transient(format!("siem sink network: {e}")))
        }
        Err(e) => Err(SinkError::Transient(format!("siem sink request: {e}"))),
    }
}
