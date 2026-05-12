//! Lever Hire REST client.
//!
//! Targets `https://api.lever.co/v1/`. Authenticates with HTTP Basic where
//! the API key is the username and the password is empty — Lever's
//! documented contract. Reqwest's `basic_auth(key, Some(""))` emits exactly
//! `Authorization: Basic <base64("key:")>`.
//!
//! ## Pagination
//!
//! List endpoints follow Lever's `{ data, hasNext, next }` envelope. When
//! `hasNext == true`, the opaque `next` cursor is replayed via the `offset`
//! query parameter on the subsequent request. All three list methods
//! accumulate every page before returning.
//!
//! ## Status semantics
//!
//! - `200` → typed payload.
//! - `404` (single-resource only) → `Ok(None)`.
//! - `429` → honour `Retry-After` (seconds, capped at 60) and retry once;
//!   a second 429 surfaces as `ArgoError::Provider`.
//! - Other 4xx/5xx → `ArgoError::Provider`.

use crate::types::{
    format_since, LeverEnvelope, LeverPage, NoteBody, OpportunityWire, PostingWire, StageMoveBody,
    StageWire,
};
use crate::{LeverCandidate, LeverPosting, LeverStage};
use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, Utc};
use reqwest::header::RETRY_AFTER;
use reqwest::StatusCode;
use serde::de::DeserializeOwned;

const DEFAULT_BASE_URL: &str = "https://api.lever.co";
const MAX_RETRY_AFTER_SECS: u64 = 60;

/// Lever Hire connector.
pub struct LeverConnector {
    client: reqwest::Client,
    api_key: String,
    base_url: String,
}

impl LeverConnector {
    /// Build a connector with an explicit Lever API key.
    ///
    /// Returns `ArgoError::Schema` if `api_key` is blank — matches the
    /// validation pattern used by the Greenhouse and Clearbit siblings.
    /// (The brief shows `-> Self` but a blank key would produce a real but
    /// invalid `Authorization` header that only fails at request time —
    /// surfacing the misconfiguration up-front is more useful and matches
    /// every other Basic-auth connector in the workspace.)
    pub fn new(api_key: String) -> ArgoResult<Self> {
        if api_key.trim().is_empty() {
            return Err(ArgoError::Schema(
                "lever: api_key must not be empty".into(),
            ));
        }
        let client = reqwest::Client::builder()
            .user_agent(concat!(
                "argo-mcp-connector-lever/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .map_err(|e| ArgoError::Provider(format!("lever http client: {e}")))?;
        Ok(Self {
            client,
            api_key,
            base_url: DEFAULT_BASE_URL.to_string(),
        })
    }

    /// Override the Lever base URL — tests point this at wiremock.
    #[must_use]
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into().trim_end_matches('/').to_string();
        self
    }

    /// GET `/v1/opportunities` (optionally filtered by `updated_at_start`).
    ///
    /// Accumulates every page of results before returning.
    pub async fn list_candidates(
        &self,
        since: Option<DateTime<Utc>>,
    ) -> ArgoResult<Vec<LeverCandidate>> {
        let url = format!("{}/v1/opportunities", self.base_url);
        let base_query: Vec<(&'static str, String)> = since
            .map(|ts| vec![("updated_at_start", format_since(ts))])
            .unwrap_or_default();
        let wires: Vec<OpportunityWire> = self
            .get_paginated("opportunities", &url, &base_query)
            .await?;
        Ok(wires.into_iter().map(Into::into).collect())
    }

    /// GET `/v1/opportunities/{id}`. `404` returns `Ok(None)`.
    pub async fn get_candidate(&self, candidate_id: &str) -> ArgoResult<Option<LeverCandidate>> {
        let url = format!("{}/v1/opportunities/{}", self.base_url, candidate_id);
        let resp = self
            .send_with_retry(|| self.authed(self.client.get(&url)))
            .await?;
        let status = resp.status();
        if status == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let env: LeverEnvelope<OpportunityWire> = decode_json("opportunity", resp).await?;
        Ok(Some(env.data.into()))
    }

    /// POST `/v1/opportunities/{id}/notes` with `{"value": "<note>"}`.
    pub async fn add_note(&self, candidate_id: &str, note: &str) -> ArgoResult<()> {
        let url = format!(
            "{}/v1/opportunities/{}/notes",
            self.base_url, candidate_id
        );
        let body = NoteBody { value: note };
        let resp = self
            .send_with_retry(|| self.authed(self.client.post(&url)).json(&body))
            .await?;
        let status = resp.status();
        if status.is_success() {
            return Ok(());
        }
        let text = resp
            .text()
            .await
            .map_err(|e| ArgoError::Provider(format!("lever note read body: {e}")))?;
        Err(ArgoError::Provider(format!(
            "lever note: HTTP {status}: {text}"
        )))
    }

    /// POST `/v1/opportunities/{id}/stage` with `{"stage": "<stage_id>"}`.
    ///
    /// Pre-validates `stage_id` against [`Self::list_stages`] — a stage id
    /// that is not present in the tenant's stage set returns
    /// `ArgoError::Schema` before any write hits Lever's API. Lever stage
    /// ids are per-tenant, so the brief is explicit that callers should
    /// resolve them rather than hardcode.
    pub async fn advance_stage(&self, candidate_id: &str, stage_id: &str) -> ArgoResult<()> {
        let stages = self.list_stages().await?;
        if !stages.iter().any(|s| s.id == stage_id) {
            return Err(ArgoError::Schema(format!(
                "lever: stage id {stage_id:?} not found in tenant stage set (\
                 call list_stages and pick by name)"
            )));
        }
        let url = format!(
            "{}/v1/opportunities/{}/stage",
            self.base_url, candidate_id
        );
        let body = StageMoveBody { stage: stage_id };
        let resp = self
            .send_with_retry(|| self.authed(self.client.post(&url)).json(&body))
            .await?;
        let status = resp.status();
        if status.is_success() {
            return Ok(());
        }
        let text = resp
            .text()
            .await
            .map_err(|e| ArgoError::Provider(format!("lever stage read body: {e}")))?;
        Err(ArgoError::Provider(format!(
            "lever stage: HTTP {status}: {text}"
        )))
    }

    /// GET `/v1/postings`. Accumulates every page.
    pub async fn list_postings(&self) -> ArgoResult<Vec<LeverPosting>> {
        let url = format!("{}/v1/postings", self.base_url);
        let wires: Vec<PostingWire> = self.get_paginated("postings", &url, &[]).await?;
        Ok(wires.into_iter().map(Into::into).collect())
    }

    /// GET `/v1/stages`. Accumulates every page.
    pub async fn list_stages(&self) -> ArgoResult<Vec<LeverStage>> {
        let url = format!("{}/v1/stages", self.base_url);
        let wires: Vec<StageWire> = self.get_paginated("stages", &url, &[]).await?;
        Ok(wires.into_iter().map(Into::into).collect())
    }

    async fn get_paginated<T: DeserializeOwned>(
        &self,
        label: &'static str,
        url: &str,
        base_query: &[(&'static str, String)],
    ) -> ArgoResult<Vec<T>> {
        let mut acc: Vec<T> = Vec::new();
        let mut cursor: Option<String> = None;
        loop {
            let resp = self
                .send_with_retry(|| {
                    let mut req = self.authed(self.client.get(url)).query(base_query);
                    if let Some(c) = cursor.as_deref() {
                        req = req.query(&[("offset", c)]);
                    }
                    req
                })
                .await?;
            let page: LeverPage<T> = decode_json(label, resp).await?;
            acc.extend(page.data);
            match (page.has_next, page.next) {
                (true, Some(next)) if !next.is_empty() => cursor = Some(next),
                _ => return Ok(acc),
            }
        }
    }

    fn authed(&self, builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        builder
            .basic_auth(&self.api_key, Some(""))
            .header(reqwest::header::ACCEPT, "application/json")
    }

    async fn send_with_retry<F>(&self, build: F) -> ArgoResult<reqwest::Response>
    where
        F: Fn() -> reqwest::RequestBuilder,
    {
        let first = build()
            .send()
            .await
            .map_err(|e| ArgoError::Provider(format!("lever request: {e}")))?;
        if first.status() != StatusCode::TOO_MANY_REQUESTS {
            return Ok(first);
        }
        let wait_secs = first
            .headers()
            .get(RETRY_AFTER)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(1)
            .min(MAX_RETRY_AFTER_SECS);
        tokio::time::sleep(std::time::Duration::from_secs(wait_secs)).await;
        build()
            .send()
            .await
            .map_err(|e| ArgoError::Provider(format!("lever retry: {e}")))
    }
}

async fn decode_json<T: DeserializeOwned>(
    label: &str,
    resp: reqwest::Response,
) -> ArgoResult<T> {
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| ArgoError::Provider(format!("lever {label} read body: {e}")))?;
    if !status.is_success() {
        return Err(ArgoError::Provider(format!(
            "lever {label}: HTTP {status}: {text}"
        )));
    }
    serde_json::from_str(&text)
        .map_err(|e| ArgoError::Provider(format!("lever {label} decode: {e}: {text}")))
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod client_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_rejects_empty_api_key() {
        let result = LeverConnector::new("   ".into());
        assert!(result.is_err(), "blank api_key must be rejected");
        assert!(matches!(result, Err(ArgoError::Schema(_))));
    }

    #[test]
    fn with_base_url_trims_trailing_slash() {
        let lv = LeverConnector::new("k".into())
            .unwrap()
            .with_base_url("http://localhost:1234/");
        assert_eq!(lv.base_url, "http://localhost:1234");
    }
}
