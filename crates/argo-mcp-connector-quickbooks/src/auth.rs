//! QuickBooks Online OAuth2 refresh flow.
//!
//! QBO issues short-lived (1h) access tokens and long-lived (100d) refresh
//! tokens. `QuickBooksAuth` holds the client credentials + refresh token, and
//! caches the access token in memory; on a 401 the client calls `refresh()`
//! once before failing the original request.

use serde::Deserialize;
use std::sync::Mutex;

/// QBO connected-app client id env var.
pub const QB_CLIENT_ID_ENV: &str = "ARGO_QB_CLIENT_ID";
/// QBO connected-app client secret env var.
pub const QB_CLIENT_SECRET_ENV: &str = "ARGO_QB_CLIENT_SECRET";
/// QBO long-lived refresh token env var.
pub const QB_REFRESH_TOKEN_ENV: &str = "ARGO_QB_REFRESH_TOKEN";
/// QBO realm (company) id env var.
pub const QB_REALM_ID_ENV: &str = "ARGO_QB_REALM_ID";
/// QBO host override env var (`sandbox` or `production`).
pub const QB_HOST_ENV: &str = "ARGO_QB_HOST";
/// Default QBO OAuth token endpoint.
pub const QB_TOKEN_URL: &str = "https://oauth.platform.intuit.com/oauth2/v1/tokens/bearer";

/// OAuth2 holder for QuickBooks Online.
///
/// Access tokens are kept in-memory only — a process restart re-uses the
/// refresh token to obtain a fresh access token. The refresh endpoint can be
/// overridden via `with_token_url` for tests.
#[derive(Debug)]
pub struct QuickBooksAuth {
    client_id: String,
    client_secret: String,
    refresh_token: String,
    token_url: String,
    access_token: Mutex<Option<String>>,
}

impl QuickBooksAuth {
    /// Build from `ARGO_QB_CLIENT_ID`, `ARGO_QB_CLIENT_SECRET`, and
    /// `ARGO_QB_REFRESH_TOKEN`.
    pub fn from_env() -> Result<Self, String> {
        let client_id = required_env(QB_CLIENT_ID_ENV)?;
        let client_secret = required_env(QB_CLIENT_SECRET_ENV)?;
        let refresh_token = required_env(QB_REFRESH_TOKEN_ENV)?;
        Ok(Self::new(client_id, client_secret, refresh_token))
    }

    /// Build with explicit credentials. The token URL defaults to the QBO
    /// production OAuth endpoint.
    #[must_use]
    pub fn new(client_id: String, client_secret: String, refresh_token: String) -> Self {
        Self {
            client_id,
            client_secret,
            refresh_token,
            token_url: QB_TOKEN_URL.to_string(),
            access_token: Mutex::new(None),
        }
    }

    /// Override the OAuth token endpoint. Used by `client_tests` to point at
    /// a wiremock server.
    #[must_use]
    pub fn with_token_url(mut self, url: impl Into<String>) -> Self {
        self.token_url = url.into();
        self
    }

    /// Seed an access token directly. Useful in tests and for callers that
    /// already hold a fresh token from another path.
    pub fn set_access_token(&self, token: impl Into<String>) {
        if let Ok(mut slot) = self.access_token.lock() {
            *slot = Some(token.into());
        }
    }

    /// Return the cached access token, or `None` if not yet refreshed.
    #[must_use]
    pub fn access_token(&self) -> Option<String> {
        self.access_token.lock().ok().and_then(|t| t.clone())
    }

    /// Force a refresh against the token endpoint and update the cache.
    pub async fn refresh(&self, http: &reqwest::Client) -> Result<String, String> {
        let resp = http
            .post(&self.token_url)
            .basic_auth(&self.client_id, Some(&self.client_secret))
            .header(reqwest::header::ACCEPT, "application/json")
            .form(&[
                ("grant_type", "refresh_token"),
                ("refresh_token", self.refresh_token.as_str()),
            ])
            .send()
            .await
            .map_err(|e| format!("quickbooks oauth refresh: {e}"))?;
        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| format!("quickbooks oauth refresh: read body: {e}"))?;
        if !status.is_success() {
            return Err(format!("quickbooks oauth refresh: HTTP {status}: {text}"));
        }
        let parsed: RefreshResponse = serde_json::from_str(&text)
            .map_err(|e| format!("quickbooks oauth refresh: decode {e}: {text}"))?;
        if parsed.access_token.trim().is_empty() {
            return Err("quickbooks oauth refresh: empty access_token".into());
        }
        if let Ok(mut slot) = self.access_token.lock() {
            *slot = Some(parsed.access_token.clone());
        }
        Ok(parsed.access_token)
    }
}

#[derive(Debug, Deserialize)]
struct RefreshResponse {
    access_token: String,
    #[serde(default)]
    #[allow(dead_code)]
    expires_in: u64,
    #[serde(default)]
    #[allow(dead_code)]
    refresh_token: Option<String>,
}

fn required_env(name: &str) -> Result<String, String> {
    let value = argo_config::cred::get(name).map_err(|_| format!("missing {name}"))?;
    if value.trim().is_empty() {
        Err(format!("missing {name}"))
    } else {
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_env_returns_clear_error() {
        std::env::remove_var(QB_CLIENT_ID_ENV);
        let err = QuickBooksAuth::from_env().unwrap_err();
        assert!(err.contains(QB_CLIENT_ID_ENV));
    }

    #[test]
    fn set_access_token_round_trips() {
        let auth = QuickBooksAuth::new("cid".into(), "secret".into(), "rtok".into());
        assert_eq!(auth.access_token(), None);
        auth.set_access_token("seed");
        assert_eq!(auth.access_token().as_deref(), Some("seed"));
    }
}
