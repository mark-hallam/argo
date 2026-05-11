//! Zoom Server-to-Server OAuth2.
//!
//! Server-to-Server apps use the `account_credentials` grant: no
//! user-context, no refresh-token rotation, no consent screen. The
//! Server-to-Server app must be created in the Zoom marketplace under
//! the operator's account; the resulting Account ID + Client ID +
//! Client Secret are the only credentials the connector needs.
//!
//! `ZoomS2sAuth` caches the access token in memory and reuses it until
//! `expires_in - REFRESH_SKEW_SECS` has elapsed; the client also
//! refreshes once on a 401 before failing the original request.

use serde::Deserialize;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Account-id env var.
pub const ZOOM_ACCOUNT_ID_ENV: &str = "ARGO_ZOOM_ACCOUNT_ID";
/// Client-id env var.
pub const ZOOM_CLIENT_ID_ENV: &str = "ARGO_ZOOM_CLIENT_ID";
/// Client-secret env var.
pub const ZOOM_CLIENT_SECRET_ENV: &str = "ARGO_ZOOM_CLIENT_SECRET";
/// Default token endpoint.
pub const ZOOM_TOKEN_URL: &str = "https://zoom.us/oauth/token";
/// Skew margin: refresh this many seconds before stated expiry.
pub const REFRESH_SKEW_SECS: u64 = 60;

#[derive(Debug, Clone)]
struct CachedToken {
    token: String,
    expires_at: Instant,
}

/// OAuth2 holder for a Zoom Server-to-Server app.
#[derive(Debug)]
pub struct ZoomS2sAuth {
    account_id: String,
    client_id: String,
    client_secret: String,
    token_url: String,
    cached: Mutex<Option<CachedToken>>,
}

impl ZoomS2sAuth {
    /// Build from the three operator credentials.
    #[must_use]
    pub fn new(account_id: String, client_id: String, client_secret: String) -> Self {
        Self {
            account_id,
            client_id,
            client_secret,
            token_url: ZOOM_TOKEN_URL.to_string(),
            cached: Mutex::new(None),
        }
    }

    /// Build from `ARGO_ZOOM_ACCOUNT_ID` / `ARGO_ZOOM_CLIENT_ID` /
    /// `ARGO_ZOOM_CLIENT_SECRET`.
    pub fn from_env() -> Result<Self, String> {
        Ok(Self::new(
            required_env(ZOOM_ACCOUNT_ID_ENV)?,
            required_env(ZOOM_CLIENT_ID_ENV)?,
            required_env(ZOOM_CLIENT_SECRET_ENV)?,
        ))
    }

    /// Override the OAuth token endpoint (tests).
    #[must_use]
    pub fn with_token_url(mut self, url: impl Into<String>) -> Self {
        self.token_url = url.into();
        self
    }

    /// Seed an access token directly with an explicit lifetime (tests).
    pub fn set_access_token(&self, token: impl Into<String>, expires_in_secs: u64) {
        let deadline = Instant::now()
            + Duration::from_secs(expires_in_secs.saturating_sub(REFRESH_SKEW_SECS));
        if let Ok(mut slot) = self.cached.lock() {
            *slot = Some(CachedToken {
                token: token.into(),
                expires_at: deadline,
            });
        }
    }

    /// Cached access token, if still within its lifetime.
    #[must_use]
    pub fn cached_token(&self) -> Option<String> {
        let now = Instant::now();
        self.cached.lock().ok().and_then(|slot| {
            slot.as_ref()
                .filter(|t| t.expires_at > now)
                .map(|t| t.token.clone())
        })
    }

    /// Force a refresh and update the cache.
    pub async fn refresh(&self, http: &reqwest::Client) -> Result<String, String> {
        let resp = http
            .post(&self.token_url)
            .basic_auth(&self.client_id, Some(&self.client_secret))
            .header(reqwest::header::ACCEPT, "application/json")
            .query(&[
                ("grant_type", "account_credentials"),
                ("account_id", self.account_id.as_str()),
            ])
            .send()
            .await
            .map_err(|e| format!("zoom oauth refresh: {e}"))?;
        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| format!("zoom oauth refresh: read body: {e}"))?;
        if !status.is_success() {
            return Err(format!("zoom oauth refresh: HTTP {status}: {text}"));
        }
        let parsed: RefreshResponse = serde_json::from_str(&text)
            .map_err(|e| format!("zoom oauth refresh: decode {e}: {text}"))?;
        if parsed.access_token.trim().is_empty() {
            return Err("zoom oauth refresh: empty access_token".into());
        }
        let lifetime = parsed.expires_in.saturating_sub(REFRESH_SKEW_SECS).max(1);
        let deadline = Instant::now() + Duration::from_secs(lifetime);
        if let Ok(mut slot) = self.cached.lock() {
            *slot = Some(CachedToken {
                token: parsed.access_token.clone(),
                expires_at: deadline,
            });
        }
        Ok(parsed.access_token)
    }

    /// Drop the cached token without an HTTP call.
    pub fn invalidate(&self) {
        if let Ok(mut slot) = self.cached.lock() {
            *slot = None;
        }
    }
}

#[derive(Debug, Deserialize)]
struct RefreshResponse {
    access_token: String,
    #[serde(default = "default_expires_in")]
    expires_in: u64,
}

fn default_expires_in() -> u64 {
    3600
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
        std::env::remove_var(ZOOM_ACCOUNT_ID_ENV);
        let err = ZoomS2sAuth::from_env().unwrap_err();
        assert!(err.contains(ZOOM_ACCOUNT_ID_ENV));
    }

    #[test]
    fn set_access_token_within_lifetime_is_returned() {
        let auth = ZoomS2sAuth::new("acct".into(), "cid".into(), "secret".into());
        assert_eq!(auth.cached_token(), None);
        auth.set_access_token("seed", 3600);
        assert_eq!(auth.cached_token().as_deref(), Some("seed"));
    }

    #[test]
    fn expired_token_is_not_returned() {
        let auth = ZoomS2sAuth::new("acct".into(), "cid".into(), "secret".into());
        auth.set_access_token("stale", 0);
        assert_eq!(auth.cached_token(), None);
    }

    #[test]
    fn invalidate_clears_cache() {
        let auth = ZoomS2sAuth::new("acct".into(), "cid".into(), "secret".into());
        auth.set_access_token("seed", 3600);
        auth.invalidate();
        assert_eq!(auth.cached_token(), None);
    }
}
