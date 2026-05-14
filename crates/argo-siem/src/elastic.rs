//! [`ElasticSink`] — Elasticsearch `_bulk` dispatch.
//!
//! Endpoint: `<host>/_bulk`, NDJSON body alternating
//! `{"index":{"_index":"…"}}` action lines and document lines. Auth
//! is optional — `[siem].token` becomes the verbatim `Authorization`
//! header value (`Basic …` / `ApiKey …`); empty → no auth header.
//! Batch limit: 1000 events / request (well under Elastic's ~5 MiB
//! `_bulk` recommendation for audit-shaped rows).
use crate::sink::{classify_response, AuditEvent, SecretString, SiemSink, SinkError};
use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use std::time::Duration;

const BULK_PATH: &str = "/_bulk";
const MAX_BATCH: usize = 1000;
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(20);

/// Elasticsearch `_bulk` dispatch sink.
#[derive(Clone)]
pub struct ElasticSink {
    endpoint: String,
    post_url: String,
    /// Pre-formatted `Authorization` header value (e.g. `Basic …`,
    /// `ApiKey …`) — wrapped so it never appears in `Debug`.
    /// Empty → no auth header sent.
    auth: Option<SecretString>,
    /// Templatable index name. Default `argo-audit`; the brief flags
    /// per-tenant templating as out-of-scope for v1.
    index: String,
    client: reqwest::Client,
}

impl std::fmt::Debug for ElasticSink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ElasticSink")
            .field("endpoint", &self.endpoint)
            .field("index", &self.index)
            .field("auth", &self.auth.as_ref().map(|_| "<redacted>"))
            .finish()
    }
}

impl ElasticSink {
    /// Build a new Elastic sink. `endpoint` is the cluster base URL;
    /// `BULK_PATH` is appended. `token` is the verbatim
    /// `Authorization` header value (operator picks `Basic …` /
    /// `ApiKey …` shape); empty string → no auth header.
    ///
    /// # Errors
    /// Returns [`SinkError::Config`] if the reqwest client cannot be
    /// built.
    pub fn new(
        endpoint: String,
        token: String,
        index: Option<String>,
    ) -> Result<Self, SinkError> {
        let trimmed = endpoint.trim_end_matches('/').to_string();
        let post_url = format!("{trimmed}{BULK_PATH}");
        let client = reqwest::Client::builder()
            .timeout(DEFAULT_TIMEOUT)
            .build()
            .map_err(|e| SinkError::Config(format!("reqwest client build: {e}")))?;
        let auth = if token.is_empty() {
            None
        } else {
            Some(SecretString::new(token))
        };
        Ok(Self {
            endpoint: trimmed,
            post_url,
            auth,
            index: index.unwrap_or_else(|| "argo-audit".to_string()),
            client,
        })
    }

    fn headers(&self) -> Result<HeaderMap, SinkError> {
        let mut h = HeaderMap::new();
        // Elasticsearch's `_bulk` endpoint requires
        // `application/x-ndjson`; `application/json` is rejected.
        h.insert(
            HeaderName::from_static("content-type"),
            HeaderValue::from_static("application/x-ndjson"),
        );
        if let Some(a) = &self.auth {
            h.insert(
                HeaderName::from_static("authorization"),
                HeaderValue::from_str(a.reveal())
                    .map_err(|e| SinkError::Config(format!("invalid auth header: {e}")))?,
            );
        }
        Ok(h)
    }

    /// Serialize a batch to the alternating index-action / document
    /// NDJSON shape. Public for unit-test inspection.
    #[must_use]
    pub fn encode_batch(&self, events: &[AuditEvent]) -> String {
        let action = serde_json::json!({ "index": { "_index": &self.index } }).to_string();
        let mut buf = String::with_capacity(events.len() * 256);
        for ev in events {
            buf.push_str(&action);
            buf.push('\n');
            let doc = serde_json::json!({
                "seq": ev.seq,
                "id": ev.id,
                "session_id": ev.session_id,
                "kind": ev.kind,
                "payload": ev.payload,
                "@timestamp": ev.created_at,
            });
            if let Ok(line) = serde_json::to_string(&doc) {
                buf.push_str(&line);
                buf.push('\n');
            }
        }
        buf
    }
}

#[async_trait]
impl SiemSink for ElasticSink {
    fn backend(&self) -> &'static str {
        "elastic"
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
#[path = "elastic_tests.rs"]
mod tests;
