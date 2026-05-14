//! [`DatadogLogsSink`] — Datadog Logs v2 dispatch.
//!
//! Endpoint: `<host>/api/v2/logs` (operators in EU regions point at
//! the EU host via `[siem].endpoint`). Auth: `DD-API-KEY` header.
//! Batch limit: 1000 events / request (matches the v2 intake's
//! 5 MiB / 1000-event cap).
use crate::sink::{classify_response, AuditEvent, SecretString, SiemSink, SinkError};
use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use std::time::Duration;

const MAX_BATCH: usize = 1000;
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(20);

/// Datadog Logs v2 dispatch sink.
#[derive(Clone)]
pub struct DatadogLogsSink {
    endpoint: String,
    /// `DD-API-KEY` value — never logged.
    api_key: SecretString,
    /// `ddsource` field stamped onto each event. Default `argo`.
    ddsource: String,
    /// `service` field stamped onto each event. Default `argo-gateway`.
    service: String,
    client: reqwest::Client,
}

impl std::fmt::Debug for DatadogLogsSink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DatadogLogsSink")
            .field("endpoint", &self.endpoint)
            .field("ddsource", &self.ddsource)
            .field("service", &self.service)
            .field("api_key", &"<redacted>")
            .finish()
    }
}

impl DatadogLogsSink {
    /// Build a new Datadog Logs sink. `endpoint` is the full v2
    /// intake URL — operators in EU regions point at the EU host.
    /// `api_key` is the per-org API key.
    ///
    /// # Errors
    /// Returns [`SinkError::Config`] if the reqwest client cannot be
    /// built.
    pub fn new(
        endpoint: String,
        api_key: String,
        ddsource: Option<String>,
        service: Option<String>,
    ) -> Result<Self, SinkError> {
        let client = reqwest::Client::builder()
            .timeout(DEFAULT_TIMEOUT)
            .build()
            .map_err(|e| SinkError::Config(format!("reqwest client build: {e}")))?;
        Ok(Self {
            endpoint,
            api_key: SecretString::new(api_key),
            ddsource: ddsource.unwrap_or_else(|| "argo".to_string()),
            service: service.unwrap_or_else(|| "argo-gateway".to_string()),
            client,
        })
    }

    fn headers(&self) -> Result<HeaderMap, SinkError> {
        let mut h = HeaderMap::new();
        h.insert(
            HeaderName::from_static("content-type"),
            HeaderValue::from_static("application/json"),
        );
        h.insert(
            HeaderName::from_static("dd-api-key"),
            HeaderValue::from_str(self.api_key.reveal())
                .map_err(|e| SinkError::Config(format!("invalid api key header: {e}")))?,
        );
        Ok(h)
    }

    /// Serialize a batch to a JSON array body. Public for unit-test
    /// inspection.
    #[must_use]
    pub fn encode_batch(&self, events: &[AuditEvent]) -> String {
        let arr: Vec<serde_json::Value> = events
            .iter()
            .map(|ev| {
                serde_json::json!({
                    "ddsource": self.ddsource,
                    "service": self.service,
                    "hostname": "argo",
                    "message": format!("argo audit kind={} seq={}", ev.kind, ev.seq),
                    "seq": ev.seq,
                    "id": ev.id,
                    "session_id": ev.session_id,
                    "kind": ev.kind,
                    "payload": ev.payload,
                    "timestamp_ms": ev.created_at.saturating_mul(1000),
                })
            })
            .collect();
        serde_json::Value::Array(arr).to_string()
    }
}

#[async_trait]
impl SiemSink for DatadogLogsSink {
    fn backend(&self) -> &'static str {
        "datadog"
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
            .post(&self.endpoint)
            .headers(headers)
            .body(body)
            .send()
            .await;
        classify_response(resp)
    }
}

#[cfg(test)]
#[path = "datadog_tests.rs"]
mod tests;
