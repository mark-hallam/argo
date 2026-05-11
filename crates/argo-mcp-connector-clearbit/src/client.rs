//! Clearbit REST client.
//!
//! Routes per-call across `person.clearbit.com`, `company.clearbit.com`,
//! and `discovery.clearbit.com`. Authenticates with HTTP Basic where the
//! API key is the username and the password is empty (reqwest's
//! `basic_auth(key, Some(""))` produces exactly `Basic <base64("key:")>`).
//!
//! ## Status semantics
//!
//! - `200` → `Ok(Some(_))` (or populated combined / list payload)
//! - `404` → `Ok(None)` — entity not in Clearbit, not an error
//! - `202` → poll with backoff `[1s, 2s, 4s, 8s, 15s]` (30s ceiling). After
//!   that, surface as [`ArgoError::Provider`] with a retry hint.
//! - `429` → honor `Retry-After`, re-issue the same request (one retry).
//! - Other 4xx/5xx → [`ArgoError::Provider`].

use crate::types::{
    ClearbitCombinedLookup, ClearbitCompany, ClearbitPerson, CombinedWire, CompanyWire,
    DiscoveryWire, PersonWire,
};
use argo_types::{ArgoError, ArgoResult};
use reqwest::header::RETRY_AFTER;
use reqwest::StatusCode;
use serde::de::DeserializeOwned;
use std::time::Duration;

const PERSON_BASE: &str = "https://person.clearbit.com";
const COMPANY_BASE: &str = "https://company.clearbit.com";
const DISCOVERY_BASE: &str = "https://discovery.clearbit.com";
const PRODUCTION_POLL_BACKOFF_MS: &[u64] = &[1_000, 2_000, 4_000, 8_000, 15_000];
const MAX_RETRY_AFTER_SECS: u64 = 60;

/// Clearbit enrichment connector.
pub struct ClearbitConnector {
    client: reqwest::Client,
    api_key: String,
    person_base: String,
    company_base: String,
    discovery_base: String,
    poll_backoff_ms: Vec<u64>,
}

impl ClearbitConnector {
    /// Build a connector with an explicit Clearbit API key.
    pub fn new(api_key: String) -> ArgoResult<Self> {
        if api_key.trim().is_empty() {
            return Err(ArgoError::Schema(
                "clearbit: api_key must not be empty".into(),
            ));
        }
        let client = reqwest::Client::builder()
            .user_agent(concat!(
                "argo-mcp-connector-clearbit/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .map_err(|e| ArgoError::Provider(format!("clearbit http client: {e}")))?;
        Ok(Self {
            client,
            api_key,
            person_base: PERSON_BASE.to_string(),
            company_base: COMPANY_BASE.to_string(),
            discovery_base: DISCOVERY_BASE.to_string(),
            poll_backoff_ms: PRODUCTION_POLL_BACKOFF_MS.to_vec(),
        })
    }

    /// Override all three Clearbit subdomain bases — wiremock points here.
    /// Production code should never call this.
    #[must_use]
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        let base = base_url.into().trim_end_matches('/').to_string();
        self.person_base = base.clone();
        self.company_base = base.clone();
        self.discovery_base = base;
        self
    }

    /// Replace the 202-Accepted polling backoff (millis between polls).
    /// Production uses `[1_000, 2_000, 4_000, 8_000, 15_000]`; tests use
    /// zeros so the polling path runs instantly.
    #[must_use]
    pub fn with_poll_backoff_ms(mut self, schedule: Vec<u64>) -> Self {
        self.poll_backoff_ms = schedule;
        self
    }

    /// GET `https://person.clearbit.com/v2/people/find?email=<email>`.
    pub async fn enrich_person_by_email(
        &self,
        email: &str,
    ) -> ArgoResult<Option<ClearbitPerson>> {
        let url = format!("{}/v2/people/find", self.person_base);
        let body: Option<PersonWire> = self
            .get_with_polling("person", &url, &[("email", email)])
            .await?;
        Ok(body.map(Into::into))
    }

    /// GET `https://company.clearbit.com/v2/companies/find?domain=<domain>`.
    pub async fn enrich_company_by_domain(
        &self,
        domain: &str,
    ) -> ArgoResult<Option<ClearbitCompany>> {
        let url = format!("{}/v2/companies/find", self.company_base);
        let body: Option<CompanyWire> = self
            .get_with_polling("company", &url, &[("domain", domain)])
            .await?;
        Ok(body.map(Into::into))
    }

    /// GET `https://person.clearbit.com/v2/combined/find?email=<email>`.
    /// Returns an empty `ClearbitCombinedLookup` (both `None`) for 404.
    pub async fn combined_lookup(&self, email: &str) -> ArgoResult<ClearbitCombinedLookup> {
        let url = format!("{}/v2/combined/find", self.person_base);
        let body: Option<CombinedWire> = self
            .get_with_polling("combined", &url, &[("email", email)])
            .await?;
        Ok(body.map(Into::into).unwrap_or_default())
    }

    /// GET `https://discovery.clearbit.com/v1/companies/search?query=<q>&limit=<n>`.
    pub async fn discovery_search(
        &self,
        query: &str,
        limit: u8,
    ) -> ArgoResult<Vec<ClearbitCompany>> {
        let url = format!("{}/v1/companies/search", self.discovery_base);
        let limit_str = limit.max(1).to_string();
        let body: Option<DiscoveryWire> = self
            .get_with_polling(
                "discovery",
                &url,
                &[("query", query), ("limit", limit_str.as_str())],
            )
            .await?;
        Ok(body
            .map(|w| w.results.into_iter().map(Into::into).collect())
            .unwrap_or_default())
    }

    async fn get_with_polling<T: DeserializeOwned>(
        &self,
        label: &str,
        url: &str,
        query: &[(&str, &str)],
    ) -> ArgoResult<Option<T>> {
        let mut polls: usize = 0;
        loop {
            let resp = self.send_once(label, url, query).await?;
            let status = resp.status();
            if status == StatusCode::OK {
                let text = read_body(label, resp).await?;
                if text.trim().is_empty() {
                    return Ok(None);
                }
                return serde_json::from_str::<T>(&text)
                    .map(Some)
                    .map_err(|e| {
                        ArgoError::Provider(format!("clearbit {label} decode: {e}: {text}"))
                    });
            }
            if status == StatusCode::NOT_FOUND {
                return Ok(None);
            }
            if status == StatusCode::ACCEPTED {
                let Some(delay_ms) = self.poll_backoff_ms.get(polls).copied() else {
                    return Err(ArgoError::Provider(format!(
                        "clearbit {label}: 202 polling exhausted after 30s ceiling \
                         (hint: enrichment still queued, retry later)"
                    )));
                };
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                polls += 1;
                continue;
            }
            if status == StatusCode::TOO_MANY_REQUESTS {
                let wait = parse_retry_after(resp.headers().get(RETRY_AFTER));
                tokio::time::sleep(Duration::from_secs(wait)).await;
                continue;
            }
            let text = resp.text().await.unwrap_or_default();
            return Err(ArgoError::Provider(format!(
                "clearbit {label}: HTTP {status}: {text}"
            )));
        }
    }

    async fn send_once(
        &self,
        label: &str,
        url: &str,
        query: &[(&str, &str)],
    ) -> ArgoResult<reqwest::Response> {
        self.client
            .get(url)
            .basic_auth(&self.api_key, Some(""))
            .header(reqwest::header::ACCEPT, "application/json")
            .query(query)
            .send()
            .await
            .map_err(|e| ArgoError::Provider(format!("clearbit {label} request: {e}")))
    }
}

fn parse_retry_after(header: Option<&reqwest::header::HeaderValue>) -> u64 {
    header
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(1)
        .min(MAX_RETRY_AFTER_SECS)
}

async fn read_body(label: &str, resp: reqwest::Response) -> ArgoResult<String> {
    resp.text()
        .await
        .map_err(|e| ArgoError::Provider(format!("clearbit {label} read body: {e}")))
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod client_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_rejects_empty_api_key() {
        let result = ClearbitConnector::new("   ".into());
        assert!(result.is_err());
        assert!(matches!(result, Err(ArgoError::Schema(_))));
    }

    #[test]
    fn parse_retry_after_caps_at_max() {
        let header = reqwest::header::HeaderValue::from_static("9999");
        assert_eq!(parse_retry_after(Some(&header)), MAX_RETRY_AFTER_SECS);
        assert_eq!(parse_retry_after(None), 1);
    }

    #[test]
    fn with_base_url_routes_all_three_subdomains() {
        let cb = ClearbitConnector::new("k".into())
            .unwrap()
            .with_base_url("http://localhost:1234");
        assert_eq!(cb.person_base, "http://localhost:1234");
        assert_eq!(cb.company_base, "http://localhost:1234");
        assert_eq!(cb.discovery_base, "http://localhost:1234");
    }
}
