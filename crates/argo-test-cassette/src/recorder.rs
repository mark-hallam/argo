//! [`CassetteRecorder`]: a reqwest-backed adapter that records, replays,
//! or passes through HTTP exchanges based on `ARGO_TEST_CASSETTE_MODE`.
//!
//! Default mode is `replay` (no real network). Tests opt into `record`
//! when refreshing a cassette against a live API; `passthrough` is for
//! ad-hoc debug only.

use crate::cassette::{
    redact_body_json, redact_headers, Cassette, CassetteError, CassetteExchange,
    CassetteRequest, CassetteResponse, Result, DEFAULT_HEADER_ALLOWLIST,
};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CassetteMode {
    Record,
    Replay,
    Passthrough,
}

impl CassetteMode {
    /// Read `ARGO_TEST_CASSETTE_MODE`; default is [`CassetteMode::Replay`].
    pub fn from_env() -> Self {
        Self::parse(std::env::var("ARGO_TEST_CASSETTE_MODE").ok().as_deref())
    }

    pub fn parse(s: Option<&str>) -> Self {
        match s.map(str::to_ascii_lowercase).as_deref() {
            Some("record") => Self::Record,
            Some("passthrough") => Self::Passthrough,
            _ => Self::Replay,
        }
    }
}

pub struct CassetteRecorder {
    mode: CassetteMode,
    cassette: Cassette,
    client: reqwest::Client,
}

impl CassetteRecorder {
    /// Construct a recorder bound to a cassette; mode read from env.
    pub fn new(cassette: Cassette) -> Self {
        Self {
            mode: CassetteMode::from_env(),
            cassette,
            client: reqwest::Client::new(),
        }
    }

    pub fn with_mode(mut self, mode: CassetteMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn mode(&self) -> CassetteMode {
        self.mode
    }

    pub fn cassette(&self) -> &Cassette {
        &self.cassette
    }

    pub fn cassette_mut(&mut self) -> &mut Cassette {
        &mut self.cassette
    }

    /// Execute an HTTP exchange under the active mode.
    ///
    /// - `Replay`: pop+match the next recorded exchange; no network.
    /// - `Record`: hit the network, persist the redacted exchange.
    /// - `Passthrough`: hit the network, do not persist.
    pub async fn execute(
        &mut self,
        method: &str,
        url: &str,
        headers: BTreeMap<String, String>,
        body: Vec<u8>,
    ) -> Result<CassetteResponse> {
        let request_record = build_request_record(method, url, &headers, &body);
        match self.mode {
            CassetteMode::Replay => self.cassette.pop_next(&request_record),
            CassetteMode::Passthrough => {
                let (status, h, b) = self.do_request(method, url, &headers, body).await?;
                Ok(CassetteResponse {
                    status,
                    headers: redact_headers(&h, DEFAULT_HEADER_ALLOWLIST),
                    body_json: b,
                })
            }
            CassetteMode::Record => {
                let (status, h, b) = self.do_request(method, url, &headers, body).await?;
                let response = CassetteResponse {
                    status,
                    headers: redact_headers(&h, DEFAULT_HEADER_ALLOWLIST),
                    body_json: redact_body_json(b),
                };
                self.cassette.append(CassetteExchange {
                    request: request_record,
                    response: response.clone(),
                    recorded_at: chrono::Utc::now().to_rfc3339(),
                });
                self.cassette.save()?;
                Ok(response)
            }
        }
    }

    async fn do_request(
        &self,
        method: &str,
        url: &str,
        headers: &BTreeMap<String, String>,
        body: Vec<u8>,
    ) -> Result<(u16, BTreeMap<String, String>, serde_json::Value)> {
        let m = reqwest::Method::from_bytes(method.as_bytes())
            .map_err(|e| CassetteError::Io(format!("invalid method: {e}")))?;
        let mut builder = self.client.request(m, url);
        for (k, v) in headers {
            builder = builder.header(k.as_str(), v.as_str());
        }
        if !body.is_empty() {
            builder = builder.body(body);
        }
        let resp = builder
            .send()
            .await
            .map_err(|e| CassetteError::Io(format!("request: {e}")))?;
        let status = resp.status().as_u16();
        let mut response_headers = BTreeMap::new();
        for (name, value) in resp.headers() {
            if let Ok(s) = value.to_str() {
                response_headers.insert(name.as_str().to_string(), s.to_string());
            }
        }
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| CassetteError::Io(format!("body read: {e}")))?;
        let body_json =
            serde_json::from_slice::<serde_json::Value>(&bytes).unwrap_or(serde_json::Value::Null);
        Ok((status, response_headers, body_json))
    }
}

/// Build a redacted [`CassetteRequest`] from raw HTTP inputs (public so
/// tests can construct matchers without going through a recorder).
pub fn build_request_record(
    method: &str,
    url: &str,
    headers: &BTreeMap<String, String>,
    body: &[u8],
) -> CassetteRequest {
    let body_json = serde_json::from_slice::<serde_json::Value>(body).unwrap_or(serde_json::Value::Null);
    CassetteRequest {
        method: method.to_ascii_uppercase(),
        url: url.to_string(),
        headers: redact_headers(headers, DEFAULT_HEADER_ALLOWLIST),
        body_sha256: sha256_hex(body),
        body_json_redacted: redact_body_json(body_json),
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}
