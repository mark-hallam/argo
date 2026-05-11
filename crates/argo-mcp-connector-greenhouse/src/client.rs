//! Greenhouse Harvest REST client.
//!
//! Targets `https://harvest.greenhouse.io/v1/...`. Authenticates with HTTP
//! Basic where the API key is the username and the password is empty —
//! Greenhouse's documented contract. Reqwest's `basic_auth(key, Some(""))`
//! emits exactly `Authorization: Basic <base64("key:")>`.
//!
//! Writes (`advance_application`, `reject_application`, `add_note`) all
//! require an `On-Behalf-Of` header naming the Greenhouse user the action
//! is attributed to. The connector validates that id pre-flight; absent
//! or zero values fail before any network call so audit gaps cannot
//! happen silently.

use crate::types::{
    format_since, AdvanceBody, ApplicationWire, CandidateWire, GhApplication, GhCandidate, GhJob,
    GhJobStatus, GhNoteVisibility, JobWire, NoteBody, RejectBody,
};
use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, Utc};
use reqwest::header::RETRY_AFTER;
use reqwest::StatusCode;

const DEFAULT_BASE_URL: &str = "https://harvest.greenhouse.io";
const ON_BEHALF_OF: &str = "On-Behalf-Of";
const MAX_RETRY_AFTER_SECS: u64 = 60;

/// Greenhouse Harvest connector.
pub struct GreenhouseConnector {
    client: reqwest::Client,
    api_key: String,
    base_url: String,
}

impl GreenhouseConnector {
    /// Build a connector with an explicit Harvest API key.
    pub fn new(api_key: String) -> ArgoResult<Self> {
        if api_key.trim().is_empty() {
            return Err(ArgoError::Schema(
                "greenhouse: api_key must not be empty".into(),
            ));
        }
        let client = reqwest::Client::builder()
            .user_agent(concat!(
                "argo-mcp-connector-greenhouse/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .map_err(|e| ArgoError::Provider(format!("greenhouse http client: {e}")))?;
        Ok(Self {
            client,
            api_key,
            base_url: DEFAULT_BASE_URL.to_string(),
        })
    }

    /// Override the Harvest base URL — tests point this at wiremock.
    #[must_use]
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into().trim_end_matches('/').to_string();
        self
    }

    /// GET `/v1/candidates` (optionally filtered by `updated_after`).
    pub async fn list_candidates(
        &self,
        since: Option<DateTime<Utc>>,
    ) -> ArgoResult<Vec<GhCandidate>> {
        let url = format!("{}/v1/candidates", self.base_url);
        let query: Vec<(&str, String)> = since
            .map(|ts| vec![("updated_after", format_since(ts))])
            .unwrap_or_default();
        let resp = self
            .send_with_retry(|| self.authed(self.client.get(&url)).query(&query))
            .await?;
        let wire: Vec<CandidateWire> = decode_json("candidates", resp).await?;
        Ok(wire.into_iter().map(Into::into).collect())
    }

    /// GET `/v1/candidates/{id}`.
    pub async fn get_candidate(&self, id: i64) -> ArgoResult<GhCandidate> {
        let url = format!("{}/v1/candidates/{}", self.base_url, id);
        let resp = self.send_with_retry(|| self.authed(self.client.get(&url))).await?;
        let wire: CandidateWire = decode_json("candidate", resp).await?;
        Ok(wire.into())
    }

    /// GET `/v1/applications` (optionally filtered by candidate).
    pub async fn list_applications(
        &self,
        candidate_id: Option<i64>,
    ) -> ArgoResult<Vec<GhApplication>> {
        let url = format!("{}/v1/applications", self.base_url);
        let query: Vec<(&str, String)> = candidate_id
            .map(|id| vec![("candidate_id", id.to_string())])
            .unwrap_or_default();
        let resp = self
            .send_with_retry(|| self.authed(self.client.get(&url)).query(&query))
            .await?;
        let wire: Vec<ApplicationWire> = decode_json("applications", resp).await?;
        Ok(wire.into_iter().map(Into::into).collect())
    }

    /// GET `/v1/jobs` (optionally filtered by status).
    pub async fn list_jobs(&self, status: Option<GhJobStatus>) -> ArgoResult<Vec<GhJob>> {
        let url = format!("{}/v1/jobs", self.base_url);
        let query: Vec<(&str, &str)> = status
            .map(|s| vec![("status", s.as_query_str())])
            .unwrap_or_default();
        let resp = self
            .send_with_retry(|| self.authed(self.client.get(&url)).query(&query))
            .await?;
        let wire: Vec<JobWire> = decode_json("jobs", resp).await?;
        Ok(wire.into_iter().map(Into::into).collect())
    }

    /// POST `/v1/applications/{id}/move` to advance to `to_stage_id`.
    ///
    /// Greenhouse's `/move` endpoint requires `from_stage_id`. The connector
    /// fetches the application first to learn its current stage, then issues
    /// the move. Two requests per call by construction.
    pub async fn advance_application(
        &self,
        app_id: i64,
        to_stage_id: i64,
        on_behalf_of_user_id: i64,
    ) -> ArgoResult<GhApplication> {
        require_on_behalf_of(on_behalf_of_user_id)?;
        let get_url = format!("{}/v1/applications/{}", self.base_url, app_id);
        let app_resp = self
            .send_with_retry(|| self.authed(self.client.get(&get_url)))
            .await?;
        let current: ApplicationWire = decode_json("application", app_resp).await?;
        let from_stage_id = current.current_stage.as_ref().map(|s| s.id);

        let move_url = format!("{}/v1/applications/{}/move", self.base_url, app_id);
        let body = AdvanceBody {
            from_stage_id,
            to_stage_id,
        };
        let resp = self
            .send_with_retry(|| {
                self.authed(self.client.post(&move_url))
                    .header(ON_BEHALF_OF, on_behalf_of_user_id.to_string())
                    .json(&body)
            })
            .await?;
        let wire: ApplicationWire = decode_json("application.move", resp).await?;
        Ok(wire.into())
    }

    /// POST `/v1/applications/{id}/reject`.
    pub async fn reject_application(
        &self,
        app_id: i64,
        rejection_reason_id: i64,
        on_behalf_of_user_id: i64,
        notes: Option<&str>,
    ) -> ArgoResult<GhApplication> {
        require_on_behalf_of(on_behalf_of_user_id)?;
        let url = format!("{}/v1/applications/{}/reject", self.base_url, app_id);
        let body = RejectBody {
            rejection_reason_id,
            notes,
        };
        let resp = self
            .send_with_retry(|| {
                self.authed(self.client.post(&url))
                    .header(ON_BEHALF_OF, on_behalf_of_user_id.to_string())
                    .json(&body)
            })
            .await?;
        let wire: ApplicationWire = decode_json("application.reject", resp).await?;
        Ok(wire.into())
    }

    /// POST `/v1/candidates/{id}/activity_feed/notes`.
    pub async fn add_note(
        &self,
        candidate_id: i64,
        on_behalf_of_user_id: i64,
        body: &str,
        visibility: GhNoteVisibility,
    ) -> ArgoResult<()> {
        require_on_behalf_of(on_behalf_of_user_id)?;
        let url = format!(
            "{}/v1/candidates/{}/activity_feed/notes",
            self.base_url, candidate_id
        );
        let note = NoteBody {
            user_id: on_behalf_of_user_id,
            body,
            visibility: visibility.as_wire_str(),
        };
        let resp = self
            .send_with_retry(|| {
                self.authed(self.client.post(&url))
                    .header(ON_BEHALF_OF, on_behalf_of_user_id.to_string())
                    .json(&note)
            })
            .await?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp
                .text()
                .await
                .map_err(|e| ArgoError::Provider(format!("greenhouse note read body: {e}")))?;
            return Err(ArgoError::Provider(format!(
                "greenhouse note: HTTP {status}: {text}"
            )));
        }
        Ok(())
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
            .map_err(|e| ArgoError::Provider(format!("greenhouse request: {e}")))?;
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
            .map_err(|e| ArgoError::Provider(format!("greenhouse retry: {e}")))
    }
}

fn require_on_behalf_of(user_id: i64) -> ArgoResult<()> {
    if user_id <= 0 {
        return Err(ArgoError::Schema(
            "greenhouse: On-Behalf-Of user id is required for write operations".into(),
        ));
    }
    Ok(())
}

async fn decode_json<T: serde::de::DeserializeOwned>(
    label: &str,
    resp: reqwest::Response,
) -> ArgoResult<T> {
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| ArgoError::Provider(format!("greenhouse {label} read body: {e}")))?;
    if !status.is_success() {
        return Err(ArgoError::Provider(format!(
            "greenhouse {label}: HTTP {status}: {text}"
        )));
    }
    serde_json::from_str(&text)
        .map_err(|e| ArgoError::Provider(format!("greenhouse {label} decode: {e}: {text}")))
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod client_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_rejects_empty_api_key() {
        let result = GreenhouseConnector::new("   ".into());
        assert!(result.is_err(), "blank api_key must be rejected");
        assert!(matches!(result, Err(ArgoError::Schema(_))));
    }

    #[test]
    fn on_behalf_of_required_for_writes() {
        assert!(require_on_behalf_of(0).is_err());
        assert!(require_on_behalf_of(-5).is_err());
        assert!(require_on_behalf_of(123).is_ok());
    }
}
