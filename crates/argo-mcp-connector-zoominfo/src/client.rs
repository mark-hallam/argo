//! ZoomInfo REST client (OAuth2 client-credentials).
//!
//! Base host `https://api.zoominfo.com`. Endpoints: `/authenticate`,
//! `/lookup/contact`, `/lookup/company`, `/search/contact`, `/search/company`.
//!
//! ## Status semantics
//!
//! - `200` → typed body
//! - `400` → [`ZoomInfoError::InvalidFilter`]
//! - `401` → force-refresh + retry; second hit → [`ZoomInfoError::AuthFailed`]
//! - `403` with body matching `/credit|quota/i` → [`ZoomInfoError::CreditsExhausted`]
//! - `404` → typed `*NotFound` per endpoint kind
//! - `429` → sleep `Retry-After` seconds + jitter `[0, 1s)`, retry
//! - Other 4xx / 5xx → [`ZoomInfoError::Transport`]
//!
//! Token is cached + refreshed proactively 5 min before its 1-hour TTL.

use crate::types::{
    AuthWire, ZoomInfoCompany, ZoomInfoCompanySearchResult, ZoomInfoError, ZoomInfoPerson,
    ZoomInfoPersonSearchResult,
};
use reqwest::header::RETRY_AFTER;
use reqwest::StatusCode;
use serde::de::DeserializeOwned;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

const PROD_BASE: &str = "https://api.zoominfo.com";
const AUTH_PATH: &str = "/authenticate";
const PERSON_LOOKUP_PATH: &str = "/lookup/contact";
const COMPANY_LOOKUP_PATH: &str = "/lookup/company";
const PERSON_SEARCH_PATH: &str = "/search/contact";
const COMPANY_SEARCH_PATH: &str = "/search/company";
const TOKEN_LIFETIME: Duration = Duration::from_secs(60 * 60);
const TOKEN_REFRESH_LEEWAY: Duration = Duration::from_secs(5 * 60);
const MAX_RETRY_AFTER_SECS: u64 = 60;
const DEFAULT_JITTER_MS: u64 = 1_000;
// Per-account credit-balance header. Advisory; logged when present, no error if absent.
const CREDITS_HEADER: &str = "X-RateLimit-Remaining-Credits";

/// ZoomInfo credentials for client-credentials OAuth2. Equivalent to the
/// `ARGO_ZOOMINFO_USERNAME` / `_PASSWORD` / `_CLIENT_ID` env trio.
#[derive(Debug, Clone)]
pub struct ZoomInfoCredentials {
    /// ZoomInfo account username.
    pub username: String,
    /// ZoomInfo account password.
    pub password: String,
    /// OAuth client id issued by ZoomInfo to the operator's account.
    pub client_id: String,
}

impl ZoomInfoCredentials {
    /// Construct credentials from explicit values. All three must be non-empty.
    pub fn new(
        username: impl Into<String>,
        password: impl Into<String>,
        client_id: impl Into<String>,
    ) -> Self {
        Self {
            username: username.into(),
            password: password.into(),
            client_id: client_id.into(),
        }
    }

    /// Pull credentials from `ARGO_ZOOMINFO_USERNAME` / `_PASSWORD` / `_CLIENT_ID`.
    pub fn from_env() -> Result<Self, ZoomInfoError> {
        Ok(Self::new(
            read_env("ARGO_ZOOMINFO_USERNAME")?,
            read_env("ARGO_ZOOMINFO_PASSWORD")?,
            read_env("ARGO_ZOOMINFO_CLIENT_ID")?,
        ))
    }

    fn validate(&self) -> Result<(), ZoomInfoError> {
        for (n, v) in [
            ("username", &self.username),
            ("password", &self.password),
            ("client_id", &self.client_id),
        ] {
            if v.trim().is_empty() {
                return Err(ZoomInfoError::Config(format!("{n} must not be empty")));
            }
        }
        Ok(())
    }
}

fn read_env(key: &str) -> Result<String, ZoomInfoError> {
    match std::env::var(key) {
        Ok(v) if !v.trim().is_empty() => Ok(v),
        _ => Err(ZoomInfoError::Config(format!(
            "{key} unset or empty (required for ZoomInfo OAuth2)"
        ))),
    }
}

struct CachedToken {
    jwt: String,
    expires_at: Instant,
}

#[derive(Clone, Copy)]
enum EntityKind {
    Person,
    Company,
}

/// ZoomInfo Sales Insights connector.
pub struct ZoomInfoClient {
    http: reqwest::Client,
    creds: ZoomInfoCredentials,
    base_url: String,
    token: Mutex<Option<CachedToken>>,
    jitter_ms: u64,
}

impl ZoomInfoClient {
    /// Build a new client. Validates credentials before constructing the HTTP client.
    pub fn new(creds: ZoomInfoCredentials) -> Result<Self, ZoomInfoError> {
        creds.validate()?;
        let http = reqwest::Client::builder()
            .user_agent(concat!(
                "argo-mcp-connector-zoominfo/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .map_err(|e| ZoomInfoError::Transport(format!("build http: {e}")))?;
        Ok(Self {
            http,
            creds,
            base_url: PROD_BASE.to_string(),
            token: Mutex::new(None),
            jitter_ms: DEFAULT_JITTER_MS,
        })
    }

    /// Point the client at a different base URL — wiremock uses this.
    #[must_use]
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into().trim_end_matches('/').to_string();
        self
    }

    /// Override the `429`-back-off jitter ceiling (ms). Tests set 0 for determinism.
    #[must_use]
    pub fn with_jitter_ms(mut self, ms: u64) -> Self {
        self.jitter_ms = ms;
        self
    }

    /// `POST /lookup/contact` — person enrichment. Pass any combination of
    /// `email`, `firstName`+`lastName`+`companyName`, or `linkedInUrl` in `body`.
    pub async fn enrich_person(
        &self,
        body: serde_json::Value,
    ) -> Result<ZoomInfoPerson, ZoomInfoError> {
        self.post_json_with_auth(PERSON_LOOKUP_PATH, body, EntityKind::Person)
            .await
    }

    /// Convenience wrapper: `enrich_person({"emailAddress": email})`.
    pub async fn enrich_person_by_email(
        &self,
        email: &str,
    ) -> Result<ZoomInfoPerson, ZoomInfoError> {
        self.enrich_person(serde_json::json!({ "emailAddress": email }))
            .await
    }

    /// `POST /lookup/company` — company enrichment. Pass any of
    /// `companyWebsite` (domain), `companyName`, or `duns` in `body`.
    pub async fn enrich_company(
        &self,
        body: serde_json::Value,
    ) -> Result<ZoomInfoCompany, ZoomInfoError> {
        self.post_json_with_auth(COMPANY_LOOKUP_PATH, body, EntityKind::Company)
            .await
    }

    /// Convenience wrapper: `enrich_company({"companyWebsite": domain})`.
    pub async fn enrich_company_by_domain(
        &self,
        domain: &str,
    ) -> Result<ZoomInfoCompany, ZoomInfoError> {
        self.enrich_company(serde_json::json!({ "companyWebsite": domain }))
            .await
    }

    /// `POST /search/contact` — paginated person search. `page` 1-indexed;
    /// `page_size` capped at 25 (ZoomInfo's max).
    pub async fn search_people(
        &self,
        filters: serde_json::Value,
        page: u32,
        page_size: u32,
    ) -> Result<ZoomInfoPersonSearchResult, ZoomInfoError> {
        let body = merge_pagination(filters, page, page_size);
        self.post_json_with_auth(PERSON_SEARCH_PATH, body, EntityKind::Person)
            .await
    }

    /// `POST /search/company` — paginated company search.
    pub async fn search_companies(
        &self,
        filters: serde_json::Value,
        page: u32,
        page_size: u32,
    ) -> Result<ZoomInfoCompanySearchResult, ZoomInfoError> {
        let body = merge_pagination(filters, page, page_size);
        self.post_json_with_auth(COMPANY_SEARCH_PATH, body, EntityKind::Company)
            .await
    }

    async fn token(&self) -> Result<String, ZoomInfoError> {
        let mut guard = self.token.lock().await;
        if let Some(t) = guard.as_ref() {
            if t.expires_at > Instant::now() + TOKEN_REFRESH_LEEWAY {
                return Ok(t.jwt.clone());
            }
        }
        let jwt = self.fetch_token().await?;
        *guard = Some(CachedToken {
            jwt: jwt.clone(),
            expires_at: Instant::now() + TOKEN_LIFETIME,
        });
        Ok(jwt)
    }

    async fn force_refresh_token(&self) -> Result<String, ZoomInfoError> {
        let mut guard = self.token.lock().await;
        *guard = None;
        let jwt = self.fetch_token().await?;
        *guard = Some(CachedToken {
            jwt: jwt.clone(),
            expires_at: Instant::now() + TOKEN_LIFETIME,
        });
        Ok(jwt)
    }

    async fn fetch_token(&self) -> Result<String, ZoomInfoError> {
        let body = serde_json::json!({
            "username": self.creds.username,
            "password": self.creds.password,
            "clientId": self.creds.client_id,
        });
        let resp = self
            .http
            .post(format!("{}{}", self.base_url, AUTH_PATH))
            .json(&body)
            .send()
            .await
            .map_err(|e| ZoomInfoError::Transport(format!("authenticate send: {e}")))?;
        let status = resp.status();
        if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
            return Err(ZoomInfoError::AuthFailed);
        }
        if !status.is_success() {
            let txt = resp.text().await.unwrap_or_default();
            return Err(ZoomInfoError::Transport(format!(
                "authenticate HTTP {status}: {txt}"
            )));
        }
        let wire: AuthWire = resp
            .json()
            .await
            .map_err(|e| ZoomInfoError::Decode(format!("auth body: {e}")))?;
        if wire.jwt.trim().is_empty() {
            return Err(ZoomInfoError::Decode("auth: empty jwt".into()));
        }
        Ok(wire.jwt)
    }

    async fn post_json_with_auth<T: DeserializeOwned>(
        &self,
        path: &str,
        body: serde_json::Value,
        kind: EntityKind,
    ) -> Result<T, ZoomInfoError> {
        let mut refreshed = false;
        loop {
            let token = self.token().await?;
            let resp = self
                .http
                .post(format!("{}{}", self.base_url, path))
                .bearer_auth(&token)
                .json(&body)
                .send()
                .await
                .map_err(|e| ZoomInfoError::Transport(format!("{path} send: {e}")))?;
            if let Some(remaining) = resp
                .headers()
                .get(CREDITS_HEADER)
                .and_then(|v| v.to_str().ok())
            {
                tracing::debug!(remaining_credits = %remaining, path, "zoominfo credit usage");
            }
            let status = resp.status();
            if status == StatusCode::OK {
                let text = resp
                    .text()
                    .await
                    .map_err(|e| ZoomInfoError::Transport(format!("{path} read body: {e}")))?;
                return serde_json::from_str::<T>(&text)
                    .map_err(|e| ZoomInfoError::Decode(format!("{path}: {e}: {text}")));
            }
            if status == StatusCode::UNAUTHORIZED {
                if refreshed {
                    return Err(ZoomInfoError::AuthFailed);
                }
                refreshed = true;
                self.force_refresh_token().await?;
                continue;
            }
            if status == StatusCode::FORBIDDEN {
                let txt = resp.text().await.unwrap_or_default();
                let lower = txt.to_ascii_lowercase();
                if lower.contains("credit") || lower.contains("quota") {
                    return Err(ZoomInfoError::CreditsExhausted);
                }
                return Err(ZoomInfoError::Transport(format!("{path}: 403: {txt}")));
            }
            if status == StatusCode::NOT_FOUND {
                return Err(match kind {
                    EntityKind::Person => ZoomInfoError::PersonNotFound,
                    EntityKind::Company => ZoomInfoError::CompanyNotFound,
                });
            }
            if status == StatusCode::TOO_MANY_REQUESTS {
                let wait_secs = parse_retry_after(resp.headers().get(RETRY_AFTER));
                let jitter_ms = sample_jitter_ms(self.jitter_ms);
                let total = wait_secs.saturating_mul(1_000).saturating_add(jitter_ms);
                tokio::time::sleep(Duration::from_millis(total)).await;
                continue;
            }
            if status == StatusCode::BAD_REQUEST {
                let txt = resp.text().await.unwrap_or_default();
                return Err(ZoomInfoError::InvalidFilter(txt));
            }
            let txt = resp.text().await.unwrap_or_default();
            return Err(ZoomInfoError::Transport(format!(
                "{path}: HTTP {status}: {txt}"
            )));
        }
    }
}

fn merge_pagination(
    mut filters: serde_json::Value,
    page: u32,
    page_size: u32,
) -> serde_json::Value {
    let capped = page_size.clamp(1, 25);
    if let Some(obj) = filters.as_object_mut() {
        obj.insert("page".into(), serde_json::json!(page.max(1)));
        obj.insert("recordsPerPage".into(), serde_json::json!(capped));
        filters
    } else {
        serde_json::json!({ "page": page.max(1), "recordsPerPage": capped })
    }
}

fn parse_retry_after(header: Option<&reqwest::header::HeaderValue>) -> u64 {
    header
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(1)
        .min(MAX_RETRY_AFTER_SECS)
}

fn sample_jitter_ms(ceiling: u64) -> u64 {
    if ceiling == 0 {
        return 0;
    }
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| u64::from(d.subsec_nanos()))
        .unwrap_or(0)
        % ceiling
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod client_tests;
