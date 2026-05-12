//! Apollo.io REST client. See crate docs for status semantics.

use crate::types::{
    ApolloCompany, ApolloCompanySearchResult, ApolloError, ApolloPeopleSearchResult, ApolloPerson,
    CompaniesSearchEnvelope, CompanyEnrichEnvelope, PeopleSearchEnvelope, PersonMatchEnvelope,
};
use reqwest::header::{HeaderValue, RETRY_AFTER};
use reqwest::StatusCode;
use serde::Serialize;
use std::time::Duration;

const PRODUCTION_BASE: &str = "https://api.apollo.io/api/v1";
const CREDITS_HEADER: &str = "X-Apollo-Credits-Used";
const MAX_RETRY_AFTER_SECS: u64 = 60;
const MAX_429_RETRIES: u8 = 2;
/// Apollo's `mixed_*/search` hard cap on `per_page`.
const MAX_PER_PAGE: u32 = 25;

/// Apollo.io enrichment + search connector.
pub struct ApolloClient {
    http: reqwest::Client,
    api_key: String,
    base: String,
    retry_jitter_ms: u64,
}

/// Person-enrichment lookup keys. Apollo's `/people/match` accepts any
/// subset; supply whichever you have.
#[derive(Debug, Clone, Default, Serialize)]
pub struct PersonLookup {
    /// Email address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// Full LinkedIn profile URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linkedin_url: Option<String>,
    /// First name (paired with `last_name` for name+company lookup).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_name: Option<String>,
    /// Last name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_name: Option<String>,
    /// Employer display name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_name: Option<String>,
    /// Employer domain.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
}

impl PersonLookup {
    /// Lookup by email.
    pub fn by_email(email: impl Into<String>) -> Self {
        Self { email: Some(email.into()), ..Self::default() }
    }
    /// Lookup by LinkedIn URL.
    pub fn by_linkedin_url(url: impl Into<String>) -> Self {
        Self { linkedin_url: Some(url.into()), ..Self::default() }
    }
    /// Lookup by name + company (domain and/or organization_name).
    pub fn by_name_company(
        first_name: impl Into<String>,
        last_name: impl Into<String>,
        organization_name: Option<String>,
        domain: Option<String>,
    ) -> Self {
        Self {
            first_name: Some(first_name.into()),
            last_name: Some(last_name.into()),
            organization_name,
            domain,
            ..Self::default()
        }
    }

    fn is_empty(&self) -> bool {
        self.email.is_none()
            && self.linkedin_url.is_none()
            && self.first_name.is_none()
            && self.last_name.is_none()
            && self.organization_name.is_none()
            && self.domain.is_none()
    }
}

/// Company-enrichment lookup keys.
#[derive(Debug, Clone, Default, Serialize)]
pub struct CompanyLookup {
    /// Primary domain.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    /// Display name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl CompanyLookup {
    /// Lookup by primary domain.
    pub fn by_domain(domain: impl Into<String>) -> Self {
        Self { domain: Some(domain.into()), name: None }
    }
    /// Lookup by display name.
    pub fn by_name(name: impl Into<String>) -> Self {
        Self { domain: None, name: Some(name.into()) }
    }
}

/// Filters for `mixed_people/search` and `mixed_companies/search`.
#[derive(Debug, Clone, Default, Serialize)]
pub struct PeopleSearchFilters {
    /// 1-based page index.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    /// Results per page; clamped to [1, `MAX_PER_PAGE`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub per_page: Option<u32>,
    /// Free-text query.
    #[serde(rename = "q_keywords", skip_serializing_if = "Option::is_none")]
    pub keywords: Option<String>,
    /// Title contains (any).
    #[serde(rename = "person_titles", skip_serializing_if = "Vec::is_empty", default)]
    pub person_titles: Vec<String>,
    /// Organization domains.
    #[serde(rename = "organization_domains", skip_serializing_if = "Vec::is_empty", default)]
    pub organization_domains: Vec<String>,
}

impl ApolloClient {
    /// Build a connector with an Apollo API key.
    pub fn new(api_key: String) -> Result<Self, ApolloError> {
        if api_key.trim().is_empty() {
            return Err(ApolloError::InvalidConfig("api_key must not be empty".into()));
        }
        let http = reqwest::Client::builder()
            .user_agent(concat!("argo-mcp-connector-apollo/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| ApolloError::Provider(format!("http client: {e}")))?;
        Ok(Self {
            http,
            api_key,
            base: PRODUCTION_BASE.to_string(),
            retry_jitter_ms: 250,
        })
    }

    /// Override the base URL. Tests point this at wiremock.
    #[must_use]
    pub fn with_base_url(mut self, base: impl Into<String>) -> Self {
        self.base = base.into().trim_end_matches('/').to_string();
        self
    }

    /// Maximum random jitter (millis) added to `Retry-After` sleeps.
    /// Tests pass `0` for determinism.
    #[must_use]
    pub fn with_retry_jitter_ms(mut self, jitter_ms: u64) -> Self {
        self.retry_jitter_ms = jitter_ms;
        self
    }

    /// POST `/people/match` — enrich a single person.
    pub async fn enrich_person(
        &self,
        lookup: &PersonLookup,
    ) -> Result<ApolloPerson, ApolloError> {
        if lookup.is_empty() {
            return Err(ApolloError::InvalidFilter(
                "PersonLookup requires at least one key".into(),
            ));
        }
        let url = format!("{}/people/match", self.base);
        let env: PersonMatchEnvelope = self.post_json("people/match", &url, lookup).await?;
        env.person.map(Into::into).ok_or(ApolloError::PersonNotFound)
    }

    /// POST `/organizations/enrich` — enrich a single company.
    pub async fn enrich_company(
        &self,
        lookup: &CompanyLookup,
    ) -> Result<ApolloCompany, ApolloError> {
        if lookup.domain.is_none() && lookup.name.is_none() {
            return Err(ApolloError::InvalidFilter(
                "CompanyLookup requires domain or name".into(),
            ));
        }
        let url = format!("{}/organizations/enrich", self.base);
        let env: CompanyEnrichEnvelope =
            self.post_json("organizations/enrich", &url, lookup).await?;
        env.organization
            .map(Into::into)
            .ok_or(ApolloError::CompanyNotFound)
    }

    /// POST `/mixed_people/search` — paginated people search (≤25/page).
    pub async fn search_people(
        &self,
        filters: &PeopleSearchFilters,
    ) -> Result<ApolloPeopleSearchResult, ApolloError> {
        let url = format!("{}/mixed_people/search", self.base);
        let clamped = clamp_per_page(filters);
        let env: PeopleSearchEnvelope = self
            .post_json("mixed_people/search", &url, &clamped)
            .await?;
        let p = env.pagination.unwrap_or_default();
        Ok(ApolloPeopleSearchResult {
            entries: env.people.into_iter().map(Into::into).collect(),
            page: p.page,
            total_pages: p.total_pages,
            total_entries: p.total_entries,
        })
    }

    /// POST `/mixed_companies/search` — paginated companies search.
    pub async fn search_companies(
        &self,
        filters: &PeopleSearchFilters,
    ) -> Result<ApolloCompanySearchResult, ApolloError> {
        let url = format!("{}/mixed_companies/search", self.base);
        let clamped = clamp_per_page(filters);
        let env: CompaniesSearchEnvelope = self
            .post_json("mixed_companies/search", &url, &clamped)
            .await?;
        let p = env.pagination.unwrap_or_default();
        Ok(ApolloCompanySearchResult {
            entries: env.organizations.into_iter().map(Into::into).collect(),
            page: p.page,
            total_pages: p.total_pages,
            total_entries: p.total_entries,
        })
    }

    async fn post_json<B: Serialize, R: serde::de::DeserializeOwned>(
        &self,
        label: &str,
        url: &str,
        body: &B,
    ) -> Result<R, ApolloError> {
        let mut attempts: u8 = 0;
        loop {
            let resp = self
                .http
                .post(url)
                .header("X-Api-Key", &self.api_key)
                .header(reqwest::header::ACCEPT, "application/json")
                .json(body)
                .send()
                .await
                .map_err(|e| ApolloError::Provider(format!("apollo {label} request: {e}")))?;
            log_credits(label, resp.headers().get(CREDITS_HEADER));
            let status = resp.status();
            if status.is_success() {
                let text = resp.text().await.map_err(|e| {
                    ApolloError::Provider(format!("apollo {label} body: {e}"))
                })?;
                return serde_json::from_str::<R>(&text).map_err(|e| {
                    ApolloError::Provider(format!("apollo {label} decode: {e}: {text}"))
                });
            }
            if status == StatusCode::TOO_MANY_REQUESTS && attempts < MAX_429_RETRIES {
                let wait = parse_retry_after(resp.headers().get(RETRY_AFTER));
                let jitter = jitter_ms(self.retry_jitter_ms);
                tokio::time::sleep(Duration::from_secs(wait) + Duration::from_millis(jitter))
                    .await;
                attempts += 1;
                continue;
            }
            return Err(map_error(label, status, resp).await);
        }
    }
}

fn clamp_per_page(filters: &PeopleSearchFilters) -> PeopleSearchFilters {
    let mut out = filters.clone();
    if let Some(pp) = out.per_page {
        out.per_page = Some(pp.clamp(1, MAX_PER_PAGE));
    }
    out
}

fn parse_retry_after(header: Option<&HeaderValue>) -> u64 {
    header
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(1)
        .min(MAX_RETRY_AFTER_SECS)
}

fn jitter_ms(max_jitter_ms: u64) -> u64 {
    if max_jitter_ms == 0 {
        return 0;
    }
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64)
        .unwrap_or(0);
    nanos % max_jitter_ms.max(1)
}

fn log_credits(label: &str, header: Option<&HeaderValue>) {
    if let Some(v) = header.and_then(|h| h.to_str().ok()) {
        tracing::debug!(target: "argo_mcp_connector_apollo", label, credits_used = v, "apollo credits");
    }
}

async fn map_error(label: &str, status: StatusCode, resp: reqwest::Response) -> ApolloError {
    if status == StatusCode::UNAUTHORIZED {
        return ApolloError::AuthFailed;
    }
    if status == StatusCode::TOO_MANY_REQUESTS {
        let wait = parse_retry_after(resp.headers().get(RETRY_AFTER));
        let body = resp.text().await.unwrap_or_default();
        if is_credit_exhaustion(&body) {
            return ApolloError::CreditsExhausted;
        }
        return ApolloError::RateLimited { retry_after_secs: wait };
    }
    if status == StatusCode::NOT_FOUND {
        let route = label.split('/').next().unwrap_or(label);
        return if route.starts_with("people") || route.starts_with("mixed_people") {
            ApolloError::PersonNotFound
        } else {
            ApolloError::CompanyNotFound
        };
    }
    let body = resp.text().await.unwrap_or_default();
    if status == StatusCode::UNPROCESSABLE_ENTITY || status == StatusCode::PAYMENT_REQUIRED {
        if is_credit_exhaustion(&body) {
            return ApolloError::CreditsExhausted;
        }
        return ApolloError::InvalidFilter(truncate(&body, 240));
    }
    ApolloError::Provider(format!("apollo {label}: HTTP {status}: {}", truncate(&body, 240)))
}

fn is_credit_exhaustion(body: &str) -> bool {
    let lower = body.to_lowercase();
    lower.contains("outoffreeblocks")
        || lower.contains("credits_exhausted")
        || lower.contains("credit limit")
        || lower.contains("out of credits")
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        s.chars().take(max).collect::<String>() + "…"
    }
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod client_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_rejects_blank_api_key() {
        assert!(matches!(
            ApolloClient::new("   ".into()),
            Err(ApolloError::InvalidConfig(_))
        ));
    }

    #[test]
    fn parse_retry_after_caps_at_max_and_defaults_to_one() {
        let h = HeaderValue::from_static("9999");
        assert_eq!(parse_retry_after(Some(&h)), MAX_RETRY_AFTER_SECS);
        assert_eq!(parse_retry_after(None), 1);
    }

    #[test]
    fn clamp_per_page_clamps_into_inclusive_one_to_max() {
        let f = PeopleSearchFilters { per_page: Some(500), ..Default::default() };
        assert_eq!(clamp_per_page(&f).per_page, Some(MAX_PER_PAGE));
        let f = PeopleSearchFilters { per_page: Some(0), ..Default::default() };
        assert_eq!(clamp_per_page(&f).per_page, Some(1));
    }
}
