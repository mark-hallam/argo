//! HTTP client for the subset of Stripe REST APIs the billing flow uses.
//!
//! Authenticates with `STRIPE_SECRET_KEY` (env), retries are intentionally
//! omitted: Checkout / Portal creation are user-driven and the SPA
//! surfaces errors directly. Webhook delivery is Stripe's responsibility
//! to retry — receiver-side idempotency lives in [`crate::tier_store`].
//!
//! Form-encoded bodies match the Stripe convention; nested map fields
//! (e.g. `metadata[tenant_id]`) are serialised as the bracket syntax
//! Stripe's API expects rather than JSON.

use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

/// Environment variable carrying the Stripe secret key (sk_test_… or sk_live_…).
pub const STRIPE_SECRET_KEY_ENV: &str = "STRIPE_SECRET_KEY";
/// Environment variable carrying the Stripe webhook signing secret (whsec_…).
pub const STRIPE_WEBHOOK_SECRET_ENV: &str = "STRIPE_WEBHOOK_SECRET";
/// Environment variable optionally overriding the Stripe API base URL (testing).
pub const STRIPE_BASE_URL_ENV: &str = "STRIPE_BASE_URL";
/// Default Stripe API base URL.
pub const DEFAULT_BASE_URL: &str = "https://api.stripe.com";

/// Resolved billing config — secret key + webhook secret + API base URL.
#[derive(Debug, Clone)]
pub struct StripeConfig {
    /// Stripe secret API key. `sk_test_…` for the v1 weekend.
    pub secret_key: String,
    /// Stripe webhook signing secret (`whsec_…`).
    pub webhook_secret: String,
    /// API base URL — almost always `DEFAULT_BASE_URL`.
    pub base_url: String,
}

impl StripeConfig {
    /// Load the secret key, webhook secret, and base URL from environment.
    pub fn from_env() -> Result<Self, StripeError> {
        let secret_key = read_required(STRIPE_SECRET_KEY_ENV)?;
        let webhook_secret = read_required(STRIPE_WEBHOOK_SECRET_ENV)?;
        let base_url = std::env::var(STRIPE_BASE_URL_ENV)
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
        Ok(Self {
            secret_key,
            webhook_secret,
            base_url: base_url.trim_end_matches('/').to_string(),
        })
    }
}

fn read_required(name: &str) -> Result<String, StripeError> {
    let value = std::env::var(name).map_err(|_| StripeError::MissingEnv(name.to_string()))?;
    if value.trim().is_empty() {
        return Err(StripeError::MissingEnv(name.to_string()));
    }
    Ok(value)
}

/// Authenticated Stripe API client.
#[derive(Clone)]
pub struct StripeClient {
    http: reqwest::Client,
    config: StripeConfig,
}

impl StripeClient {
    /// Build from an explicit config.
    pub fn new(config: StripeConfig) -> Result<Self, StripeError> {
        let http = reqwest::Client::builder()
            .user_agent(concat!("argo-billing/", env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(15))
            .build()
            .map_err(|e| StripeError::Http(format!("client build: {e}")))?;
        Ok(Self { http, config })
    }

    /// Build from environment variables.
    pub fn from_env() -> Result<Self, StripeError> {
        Self::new(StripeConfig::from_env()?)
    }

    /// Stripe webhook signing secret (for [`crate::webhook::verify_signature`]).
    #[must_use]
    pub fn webhook_secret(&self) -> &str {
        &self.config.webhook_secret
    }

    /// POST a form-encoded body to `path` and decode the JSON response.
    pub async fn post_form(
        &self,
        path: &str,
        form: &[(String, String)],
    ) -> Result<serde_json::Value, StripeError> {
        let url = format!("{}{}", self.config.base_url, path);
        let resp = self
            .http
            .post(&url)
            .bearer_auth(&self.config.secret_key)
            .form(form)
            .send()
            .await
            .map_err(|e| StripeError::Http(format!("POST {path}: {e}")))?;
        decode(path, "POST", resp).await
    }

    /// GET a Stripe resource and decode the JSON response.
    pub async fn get(&self, path: &str) -> Result<serde_json::Value, StripeError> {
        let url = format!("{}{}", self.config.base_url, path);
        let resp = self
            .http
            .get(&url)
            .bearer_auth(&self.config.secret_key)
            .send()
            .await
            .map_err(|e| StripeError::Http(format!("GET {path}: {e}")))?;
        decode(path, "GET", resp).await
    }
}

async fn decode(
    path: &str,
    method: &str,
    resp: reqwest::Response,
) -> Result<serde_json::Value, StripeError> {
    let status = resp.status();
    let body = resp
        .text()
        .await
        .map_err(|e| StripeError::Http(format!("read {method} {path}: {e}")))?;
    let value: serde_json::Value = if body.trim().is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str(&body)
            .map_err(|e| StripeError::Decode(format!("{method} {path}: {e}: {body}")))?
    };
    if status.is_success() {
        return Ok(value);
    }
    let message = value
        .get("error")
        .and_then(|e| e.get("message"))
        .and_then(|m| m.as_str())
        .unwrap_or_default()
        .to_string();
    Err(StripeError::Api {
        status: status.as_u16(),
        message,
    })
}

/// A minimal slice of a Stripe Subscription object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionSlice {
    /// Stripe subscription id (`sub_…`).
    pub id: String,
    /// Stripe customer id (`cus_…`).
    pub customer: String,
    /// Subscription status: `active`, `trialing`, `past_due`, `canceled`, …
    pub status: String,
}

/// All the failure modes the Stripe layer can surface.
#[derive(Debug, Error)]
pub enum StripeError {
    /// Required environment variable is missing or empty.
    #[error("missing env var {0}")]
    MissingEnv(String),
    /// Transport-level failure (DNS, TCP, timeout).
    #[error("stripe http: {0}")]
    Http(String),
    /// JSON decode failure against a 2xx response body.
    #[error("stripe decode: {0}")]
    Decode(String),
    /// Stripe returned a non-2xx response.
    #[error("stripe api {status}: {message}")]
    Api {
        /// HTTP status code Stripe returned.
        status: u16,
        /// Free-form `error.message` lifted from the body.
        message: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_env_is_clear() {
        std::env::remove_var(STRIPE_SECRET_KEY_ENV);
        let err = read_required(STRIPE_SECRET_KEY_ENV).unwrap_err();
        assert!(matches!(err, StripeError::MissingEnv(_)), "{err:?}");
    }

    #[test]
    fn webhook_secret_round_trip() {
        let cfg = StripeConfig {
            secret_key: "sk_test_x".to_string(),
            webhook_secret: "whsec_x".to_string(),
            base_url: DEFAULT_BASE_URL.to_string(),
        };
        let client = StripeClient::new(cfg).expect("client");
        assert_eq!(client.webhook_secret(), "whsec_x");
    }
}
