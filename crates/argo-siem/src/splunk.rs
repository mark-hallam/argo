//! [`SplunkHecSink`] — Splunk HEC dispatch.
//!
//! Endpoint: `<host>/services/collector`, NDJSON HEC envelopes, one
//! event per line. Auth: `Authorization: Splunk <token>` (wrapped in
//! [`SecretString`] so `Debug` / `tracing` never see it). Batch
//! limit: 500 events / request, well under HEC's default 1 MiB cap.
use crate::sink::{classify_response, AuditEvent, SecretString, SiemSink, SinkError};
use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use std::time::Duration;

const HEC_PATH: &str = "/services/collector";
const MAX_BATCH: usize = 500;
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(20);

/// Splunk HEC dispatch sink. Constructed once at gateway boot from
/// `[siem] endpoint` + `[siem] token`. Cheap to clone.
#[derive(Clone)]
pub struct SplunkHecSink {
    endpoint: String,
    /// Full POST URL = `endpoint + HEC_PATH`.
    post_url: String,
    /// `Splunk <token>` — wrapped in [`SecretString`] so the token
    /// never appears in `Debug` output or `tracing` field rendering.
    token: SecretString,
    /// `sourcetype` field stamped onto every HEC envelope. Defaults
    /// to `argo:audit`; operators override via `[siem].sourcetype`.
    sourcetype: String,
    client: reqwest::Client,
}

impl std::fmt::Debug for SplunkHecSink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SplunkHecSink")
            .field("endpoint", &self.endpoint)
            .field("sourcetype", &self.sourcetype)
            .field("token", &"<redacted>")
            .finish()
    }
}

impl SplunkHecSink {
    /// Build a new Splunk HEC sink. `endpoint` is the cluster host
    /// (e.g. `https://splunk.example.com:8088`); `HEC_PATH` is
    /// appended automatically. `token` is the HEC token issued by
    /// the operator's Splunk admin.
    ///
    /// # Errors
    /// Returns [`SinkError::Config`] if the reqwest client cannot be
    /// built (invalid TLS roots etc.). The error surface is
    /// `SinkError` rather than `reqwest::Error` so the dispatcher's
    /// startup-side handling has a single error type.
    pub fn new(
        endpoint: String,
        token: String,
        sourcetype: Option<String>,
    ) -> Result<Self, SinkError> {
        let trimmed = endpoint.trim_end_matches('/').to_string();
        let post_url = format!("{trimmed}{HEC_PATH}");
        let client = reqwest::Client::builder()
            .timeout(DEFAULT_TIMEOUT)
            .build()
            .map_err(|e| SinkError::Config(format!("reqwest client build: {e}")))?;
        Ok(Self {
            endpoint: trimmed,
            post_url,
            token: SecretString::new(token),
            sourcetype: sourcetype.unwrap_or_else(|| "argo:audit".to_string()),
            client,
        })
    }

    fn headers(&self) -> Result<HeaderMap, SinkError> {
        let mut h = HeaderMap::new();
        h.insert(
            HeaderName::from_static("content-type"),
            HeaderValue::from_static("application/json"),
        );
        let auth = format!("Splunk {}", self.token.reveal());
        h.insert(
            HeaderName::from_static("authorization"),
            HeaderValue::from_str(&auth)
                .map_err(|e| SinkError::Config(format!("invalid token header: {e}")))?,
        );
        Ok(h)
    }

    /// Serialize a batch to NDJSON-shaped HEC envelopes (one JSON
    /// object per line, no array wrapper). Public for unit-test
    /// inspection.
    #[must_use]
    pub fn encode_batch(&self, events: &[AuditEvent]) -> String {
        let mut buf = String::with_capacity(events.len() * 256);
        for ev in events {
            let envelope = serde_json::json!({
                "time": ev.created_at,
                "sourcetype": self.sourcetype,
                "source": "argo",
                "event": {
                    "seq": ev.seq,
                    "id": ev.id,
                    "session_id": ev.session_id,
                    "kind": ev.kind,
                    "payload": ev.payload,
                },
            });
            if let Ok(line) = serde_json::to_string(&envelope) {
                buf.push_str(&line);
                buf.push('\n');
            }
        }
        buf
    }
}

#[async_trait]
impl SiemSink for SplunkHecSink {
    fn backend(&self) -> &'static str {
        "splunk"
    }

    fn endpoint(&self) -> &str {
        &self.endpoint
    }

    fn max_batch(&self) -> usize {
        MAX_BATCH
    }

    async fn ship(&self, events: Vec<AuditEvent>) -> Result<(), SinkError> {
        if events.is_empty() {
            return Ok(());
        }
        let body = self.encode_batch(&events);
        let headers = self.headers()?;
        let resp = self
            .client
            .post(&self.post_url)
            .headers(headers)
            .body(body)
            .send()
            .await;
        classify_response(resp)
    }
}

#[cfg(test)]
#[path = "splunk_tests.rs"]
mod tests;
