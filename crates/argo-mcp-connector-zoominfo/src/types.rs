//! ZoomInfo DTOs. Public types derive `Deserialize` directly (with serde
//! renames) — the upstream JSON is flat enough that Clearbit's wire/public
//! split is unnecessary. `seniority_level` and `revenue_band` stay
//! `String` — open-ended upstream enums that gain new tiers periodically.

use serde::{Deserialize, Deserializer};
use thiserror::Error;

/// ZoomInfo person enrichment result.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ZoomInfoPerson {
    /// ZoomInfo-issued opaque id (upstream may emit as string OR number).
    #[serde(deserialize_with = "deserialize_id")]
    pub id: String,
    /// First name.
    #[serde(default)]
    pub first_name: String,
    /// Last name.
    #[serde(default)]
    pub last_name: String,
    /// Echoed lookup-key email.
    #[serde(default)]
    pub email: String,
    /// Direct-dial phone (often null even on populated profiles).
    #[serde(default, alias = "directPhoneDoNotCall")]
    pub direct_phone: Option<String>,
    /// Mobile phone (premium tiers only).
    #[serde(default, alias = "mobilePhoneDoNotCall")]
    pub mobile_phone: Option<String>,
    /// Job title.
    #[serde(default)]
    pub job_title: Option<String>,
    /// Open-ended seniority bucket (`"C_Suite"`, `"VP"`, …); do not parse.
    #[serde(default, rename = "managementLevel", alias = "seniority")]
    pub seniority_level: Option<String>,
    /// Department (`"Engineering"`, `"Sales"`, …).
    #[serde(default)]
    pub department: Option<String>,
    /// Opaque company id, cross-references [`ZoomInfoCompany`].
    #[serde(default, deserialize_with = "deserialize_opt_id")]
    pub company_id: Option<String>,
    /// Full LinkedIn URL.
    #[serde(default, rename = "linkedInUrl", alias = "linkedinUrl")]
    pub linkedin_url: Option<String>,
}

/// ZoomInfo company enrichment result.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ZoomInfoCompany {
    /// ZoomInfo-issued opaque id.
    #[serde(deserialize_with = "deserialize_id")]
    pub id: String,
    /// Display name.
    #[serde(default)]
    pub name: String,
    /// Primary web domain.
    #[serde(default, alias = "website")]
    pub domain: String,
    /// Top-level industry classification.
    #[serde(default)]
    pub industry: Option<String>,
    /// Employee headcount estimate.
    #[serde(default)]
    pub employee_count: Option<u32>,
    /// Bucketed revenue range string; do not parse.
    #[serde(default, rename = "revenue", alias = "revenueBand")]
    pub revenue_band: Option<String>,
    /// Headquarters city.
    #[serde(default, alias = "city")]
    pub hq_city: Option<String>,
    /// Headquarters country.
    #[serde(default, alias = "country")]
    pub hq_country: Option<String>,
    /// Year founded.
    #[serde(default)]
    pub founded_year: Option<u16>,
    /// Dun & Bradstreet DUNS number.
    #[serde(default)]
    pub duns: Option<String>,
}

/// Paginated person-search result.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ZoomInfoPersonSearchResult {
    /// Matched persons on this page (≤25).
    #[serde(default, rename = "data")]
    pub entries: Vec<ZoomInfoPerson>,
    /// 1-indexed page number this response covers.
    #[serde(default = "default_page", rename = "currentPage")]
    pub page: u32,
    /// Total pages across the full result set.
    #[serde(default, rename = "maxResults")]
    pub total_pages: u32,
    /// Total matching entries across all pages.
    #[serde(default, rename = "totalResults")]
    pub total_entries: u32,
}

/// Paginated company-search result.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ZoomInfoCompanySearchResult {
    /// Matched companies on this page (≤25).
    #[serde(default, rename = "data")]
    pub entries: Vec<ZoomInfoCompany>,
    /// 1-indexed page number this response covers.
    #[serde(default = "default_page", rename = "currentPage")]
    pub page: u32,
    /// Total pages across the full result set.
    #[serde(default, rename = "maxResults")]
    pub total_pages: u32,
    /// Total matching entries across all pages.
    #[serde(default, rename = "totalResults")]
    pub total_entries: u32,
}

/// Errors surfaced by [`crate::ZoomInfoClient`]. Distinguishes ZoomInfo's
/// three `4xx` flavors: `401` → `AuthFailed` (after one forced-refresh
/// retry), `403` w/ credits-body → `CreditsExhausted` (per-account quota),
/// `429` → `RateLimited` (per-second).
#[derive(Debug, Error)]
pub enum ZoomInfoError {
    /// `401` after one forced token-refresh retry.
    #[error("zoominfo: authentication failed (check ARGO_ZOOMINFO_USERNAME / _PASSWORD / _CLIENT_ID)")]
    AuthFailed,
    /// Cached token reported expired before retry path resolved it.
    #[error("zoominfo: cached token expired")]
    TokenExpired,
    /// `429`. `retry_after_secs` parsed from `Retry-After`.
    #[error("zoominfo: rate limited (retry after {retry_after_secs}s)")]
    RateLimited {
        /// Seconds the client was told to wait (capped at 60).
        retry_after_secs: u64,
    },
    /// `403` w/ body indicating per-account credit pool exhausted.
    #[error("zoominfo: credit quota exhausted; replenishes monthly")]
    CreditsExhausted,
    /// Person lookup returned not-found.
    #[error("zoominfo: person not found")]
    PersonNotFound,
    /// Company lookup returned not-found.
    #[error("zoominfo: company not found")]
    CompanyNotFound,
    /// `400` with a filter-validation body.
    #[error("zoominfo: invalid filter: {0}")]
    InvalidFilter(String),
    /// Configuration error before any request was sent.
    #[error("zoominfo: configuration: {0}")]
    Config(String),
    /// reqwest-level transport error or non-classified HTTP response.
    #[error("zoominfo: transport: {0}")]
    Transport(String),
    /// Response body did not match the expected JSON shape.
    #[error("zoominfo: decode: {0}")]
    Decode(String),
}

#[derive(Debug, Deserialize)]
pub(crate) struct AuthWire {
    pub jwt: String,
}

fn default_page() -> u32 {
    1
}

fn stringify_value(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn deserialize_id<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    Ok(stringify_value(&serde_json::Value::deserialize(d)?))
}

fn deserialize_opt_id<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    Ok(Option::<serde_json::Value>::deserialize(d)?.as_ref().map(stringify_value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_id_as_string_or_number() {
        let p: ZoomInfoPerson = serde_json::from_value(serde_json::json!({
            "id": 42_i64, "firstName": "Ada"
        }))
        .unwrap();
        assert_eq!(p.id, "42");
        let p: ZoomInfoPerson = serde_json::from_value(serde_json::json!({
            "id": "p_x", "firstName": "Ada"
        }))
        .unwrap();
        assert_eq!(p.id, "p_x");
    }
}
