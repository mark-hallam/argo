//! Replicate REST client.
//!
//! All requests hit `https://api.replicate.com/v1/...` (overridable via
//! [`ReplicateConnector::with_base_url`] for wiremock). Auth header is
//! `Authorization: Token <api_token>` — Replicate explicitly uses the
//! `Token` scheme, not `Bearer`.
//!
//! ## Status semantics
//!
//! - `200`/`201` → typed deserialize.
//! - `401` → [`ArgoError::Provider`] prefixed with `"replicate auth"`.
//!   (`ProviderAuth` doesn't exist in [`argo_types::ArgoError`]; the
//!   prefix is the agreed marker for triage.)
//! - `404` on get / cancel → [`ArgoError::Provider`] with `"not found"`.
//! - Other 4xx/5xx → [`ArgoError::Provider`].

use crate::poll::{next_delay, POLL_BACKOFF_MS};
use crate::types::{
    AccountInfo, AccountInfoWire, ModelInfo, ModelInfoWire, Prediction, PredictionId,
    PredictionWire,
};
use argo_types::{ArgoError, ArgoResult};
use reqwest::StatusCode;
use serde::de::DeserializeOwned;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const REPLICATE_BASE: &str = "https://api.replicate.com";
const ACCOUNT_CACHE_TTL: Duration = Duration::from_secs(60);

/// Replicate API connector — wraps prediction lifecycle, model discovery,
/// and account info.
///
/// `ReplicateConnector` is `Send + Sync` and cheap to clone — internally
/// it holds a `reqwest::Client` (already pooled) and a single mutex
/// guarding the 60s account cache.
pub struct ReplicateConnector {
    client: reqwest::Client,
    api_token: String,
    base_url: String,
    poll_schedule_ms: Vec<u64>,
    poll_tick_override: Option<Duration>,
    account_cache: Mutex<Option<(Instant, AccountInfo)>>,
    account_cache_ttl: Duration,
}

impl ReplicateConnector {
    /// Build a connector with an explicit Replicate API token.
    pub fn new(api_token: String) -> ArgoResult<Self> {
        if api_token.trim().is_empty() {
            return Err(ArgoError::Schema(
                "replicate: api_token must not be empty".into(),
            ));
        }
        let client = reqwest::Client::builder()
            .user_agent(concat!(
                "argo-mcp-connector-replicate/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .map_err(|e| ArgoError::Provider(format!("replicate http client: {e}")))?;
        Ok(Self {
            client,
            api_token,
            base_url: REPLICATE_BASE.to_string(),
            poll_schedule_ms: POLL_BACKOFF_MS.to_vec(),
            poll_tick_override: None,
            account_cache: Mutex::new(None),
            account_cache_ttl: ACCOUNT_CACHE_TTL,
        })
    }

    /// Override the API base URL — wiremock points here. Production code
    /// should never call this.
    #[must_use]
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into().trim_end_matches('/').to_string();
        self
    }

    /// Replace the polling backoff schedule (millis between polls).
    /// Production uses `[1_000, 2_000, 4_000, 8_000, 15_000]`; tests inject
    /// zeros so polling completes instantly.
    #[must_use]
    pub fn with_poll_backoff_ms(mut self, schedule: Vec<u64>) -> Self {
        self.poll_schedule_ms = schedule;
        self
    }

    /// Replace the wall-clock granularity used by the timeout check.
    /// Tests pass a very small value so the timeout fires without holding
    /// real time. Production omits this — the schedule itself paces.
    #[must_use]
    pub fn with_poll_tick(mut self, tick: Duration) -> Self {
        self.poll_tick_override = Some(tick);
        self
    }

    /// Replace the account-cache TTL. Production uses 60s; tests use a
    /// short value to exercise expiry.
    #[must_use]
    pub fn with_account_cache_ttl(mut self, ttl: Duration) -> Self {
        self.account_cache_ttl = ttl;
        self
    }

    /// POST `/v1/predictions` then poll `GET /v1/predictions/<id>` until
    /// the prediction reaches a terminal state or `timeout` elapses.
    ///
    /// Returns the terminal [`Prediction`] (whose `status` may be
    /// `Succeeded`, `Failed`, or `Canceled`). Polling itself is not a
    /// failure mode — only the timeout and HTTP layer surface errors.
    pub async fn run_prediction(
        &self,
        model: &str,
        input: serde_json::Value,
        timeout: Duration,
    ) -> ArgoResult<Prediction> {
        let started = Instant::now();
        let mut prediction = self.start_prediction_full(model, input).await?;
        let mut polls: usize = 0;
        while !prediction.status.is_terminal() {
            if started.elapsed() >= timeout {
                return Err(ArgoError::Provider(format!(
                    "replicate run_prediction timeout: id={} model={} after {:?}",
                    prediction.id, prediction.model, timeout
                )));
            }
            let delay = next_delay(&self.poll_schedule_ms, polls);
            let delay = self.poll_tick_override.unwrap_or(delay);
            tokio::time::sleep(delay).await;
            polls = polls.saturating_add(1);
            prediction = self.get_prediction(&prediction.id).await?;
        }
        Ok(prediction)
    }

    /// POST `/v1/predictions` and return the new prediction id immediately.
    /// Callers that need to pace polling themselves should pair this with
    /// [`Self::get_prediction`].
    pub async fn start_prediction(
        &self,
        model: &str,
        input: serde_json::Value,
    ) -> ArgoResult<PredictionId> {
        Ok(self.start_prediction_full(model, input).await?.id)
    }

    async fn start_prediction_full(
        &self,
        model: &str,
        input: serde_json::Value,
    ) -> ArgoResult<Prediction> {
        let url = format!("{}/v1/predictions", self.base_url);
        let body = serde_json::json!({ "model": model, "input": input });
        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Token {}", self.api_token))
            .header(reqwest::header::ACCEPT, "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| ArgoError::Provider(format!("replicate POST predictions: {e}")))?;
        let wire: PredictionWire = decode("start_prediction", resp).await?;
        wire.into_prediction()
            .map_err(|e| ArgoError::Provider(format!("replicate start_prediction: {e}")))
    }

    /// GET `/v1/predictions/<id>`. Idempotent — callable many times.
    pub async fn get_prediction(&self, id: &PredictionId) -> ArgoResult<Prediction> {
        let url = format!("{}/v1/predictions/{}", self.base_url, id.as_str());
        let resp = self
            .client
            .get(&url)
            .header("Authorization", format!("Token {}", self.api_token))
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await
            .map_err(|e| ArgoError::Provider(format!("replicate GET prediction: {e}")))?;
        let wire: PredictionWire = decode("get_prediction", resp).await?;
        wire.into_prediction()
            .map_err(|e| ArgoError::Provider(format!("replicate get_prediction: {e}")))
    }

    /// POST `/v1/predictions/<id>/cancel`. Returning `Ok(())` means the
    /// cancellation was accepted, not that the prediction has reached
    /// `Canceled` — poll [`Self::get_prediction`] to confirm.
    pub async fn cancel_prediction(&self, id: &PredictionId) -> ArgoResult<()> {
        let url = format!("{}/v1/predictions/{}/cancel", self.base_url, id.as_str());
        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Token {}", self.api_token))
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await
            .map_err(|e| ArgoError::Provider(format!("replicate POST cancel: {e}")))?;
        let status = resp.status();
        if status.is_success() {
            return Ok(());
        }
        Err(http_error("cancel_prediction", status, resp).await)
    }

    /// GET `/v1/models/<owner>/<name>`. Returns capability + license info
    /// for vertical-side policy decisions.
    pub async fn get_model(&self, owner: &str, name: &str) -> ArgoResult<ModelInfo> {
        let url = format!("{}/v1/models/{}/{}", self.base_url, owner, name);
        let resp = self
            .client
            .get(&url)
            .header("Authorization", format!("Token {}", self.api_token))
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await
            .map_err(|e| ArgoError::Provider(format!("replicate GET model: {e}")))?;
        let wire: ModelInfoWire = decode("get_model", resp).await?;
        Ok(wire.into())
    }

    /// GET `/v1/account`. Successful responses are memoized for 60s
    /// (configurable via [`Self::with_account_cache_ttl`]).
    pub async fn account(&self) -> ArgoResult<AccountInfo> {
        if let Some(cached) = self.cached_account() {
            return Ok(cached);
        }
        let url = format!("{}/v1/account", self.base_url);
        let resp = self
            .client
            .get(&url)
            .header("Authorization", format!("Token {}", self.api_token))
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await
            .map_err(|e| ArgoError::Provider(format!("replicate GET account: {e}")))?;
        let wire: AccountInfoWire = decode("account", resp).await?;
        let info: AccountInfo = wire.into();
        self.store_account(info.clone());
        Ok(info)
    }

    fn cached_account(&self) -> Option<AccountInfo> {
        let guard = self.account_cache.lock().ok()?;
        let (at, info) = guard.as_ref()?;
        if at.elapsed() < self.account_cache_ttl {
            Some(info.clone())
        } else {
            None
        }
    }

    fn store_account(&self, info: AccountInfo) {
        if let Ok(mut guard) = self.account_cache.lock() {
            *guard = Some((Instant::now(), info));
        }
    }
}

async fn decode<T: DeserializeOwned>(op: &str, resp: reqwest::Response) -> ArgoResult<T> {
    let status = resp.status();
    if status.is_success() {
        let text = resp
            .text()
            .await
            .map_err(|e| ArgoError::Provider(format!("replicate {op} read body: {e}")))?;
        return serde_json::from_str::<T>(&text).map_err(|e| {
            ArgoError::Provider(format!("replicate {op} decode: {e}: {text}"))
        });
    }
    Err(http_error(op, status, resp).await)
}

async fn http_error(op: &str, status: StatusCode, resp: reqwest::Response) -> ArgoError {
    let text = resp.text().await.unwrap_or_default();
    if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
        return ArgoError::Provider(format!(
            "replicate auth {op}: HTTP {status}: {text}"
        ));
    }
    if status == StatusCode::NOT_FOUND {
        return ArgoError::Provider(format!(
            "replicate {op}: not found: HTTP {status}: {text}"
        ));
    }
    ArgoError::Provider(format!("replicate {op}: HTTP {status}: {text}"))
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod client_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_rejects_empty_token() {
        let result = ReplicateConnector::new("   ".into());
        assert!(matches!(result, Err(ArgoError::Schema(_))));
    }

    #[test]
    fn with_base_url_trims_trailing_slash() {
        let rc = ReplicateConnector::new("tok".into())
            .unwrap()
            .with_base_url("http://localhost:1234/");
        assert_eq!(rc.base_url, "http://localhost:1234");
    }
}
