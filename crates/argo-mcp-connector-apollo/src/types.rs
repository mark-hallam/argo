//! Apollo.io DTOs. Public types are caller-facing; `*Wire` structs mirror
//! Apollo's JSON envelope and convert via `From` at the serde boundary.

use serde::Deserialize;

/// Apollo's email-verification grade. Surface directly; do NOT collapse
/// to a boolean — the outreach decision branches on the exact value.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum ApolloPersonEmailStatus {
    Verified,
    LikelyToEngage,
    Unverified,
    Unavailable,
    Risky,
    /// Any other / future Apollo grade value, preserved verbatim.
    Other(String),
}

impl ApolloPersonEmailStatus {
    pub(crate) fn from_str(s: &str) -> Self {
        match s {
            "verified" => Self::Verified,
            "likely_to_engage" => Self::LikelyToEngage,
            "unverified" => Self::Unverified,
            "unavailable" => Self::Unavailable,
            "risky" => Self::Risky,
            other => Self::Other(other.to_string()),
        }
    }
}

/// Apollo person enrichment result.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(missing_docs)]
pub struct ApolloPerson {
    pub id: String,
    pub name: String,
    pub title: Option<String>,
    pub email: Option<String>,
    pub email_status: Option<ApolloPersonEmailStatus>,
    pub linkedin_url: Option<String>,
    pub phone: Option<String>,
    pub company_id: Option<String>,
    pub seniority: Option<String>,
    pub departments: Vec<String>,
}

/// Apollo company enrichment result. `revenue_band` is Apollo's string
/// range (e.g. `"$1M-$10M"`); keep as string, do not parse to numeric.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(missing_docs)]
pub struct ApolloCompany {
    pub id: String,
    pub name: String,
    pub domain: Option<String>,
    pub industry: Option<String>,
    pub employee_count: Option<u32>,
    pub revenue_band: Option<String>,
    pub headquarters_city: Option<String>,
    pub founded_year: Option<u32>,
}

/// Paginated `mixed_people/search` result.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[allow(missing_docs)]
pub struct ApolloPeopleSearchResult {
    pub entries: Vec<ApolloPerson>,
    pub page: u32,
    pub total_pages: u32,
    pub total_entries: u32,
}

/// Paginated `mixed_companies/search` result.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[allow(missing_docs)]
pub struct ApolloCompanySearchResult {
    pub entries: Vec<ApolloCompany>,
    pub page: u32,
    pub total_pages: u32,
    pub total_entries: u32,
}

/// Typed error surface. Variants distinguish remediation paths the
/// caller is expected to branch on.
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum ApolloError {
    #[error("apollo: invalid configuration: {0}")]
    InvalidConfig(String),
    #[error("apollo: authentication failed (check ARGO_APOLLO_API_KEY)")]
    AuthFailed,
    #[error("apollo: rate limited; retry after {retry_after_secs}s")]
    RateLimited { retry_after_secs: u64 },
    /// Monthly credit cap — distinct from rate-limit (plan vs back-off).
    #[error("apollo: monthly credit quota exhausted")]
    CreditsExhausted,
    #[error("apollo: person not found")]
    PersonNotFound,
    #[error("apollo: company not found")]
    CompanyNotFound,
    #[error("apollo: invalid filter: {0}")]
    InvalidFilter(String),
    #[error("apollo: provider error: {0}")]
    Provider(String),
}

// -- wire shapes -------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub(crate) struct PersonMatchEnvelope {
    pub person: Option<PersonWire>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CompanyEnrichEnvelope {
    pub organization: Option<OrganizationWire>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PeopleSearchEnvelope {
    #[serde(default)]
    pub people: Vec<PersonWire>,
    #[serde(default)]
    pub pagination: Option<PaginationWire>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CompaniesSearchEnvelope {
    #[serde(default)]
    pub organizations: Vec<OrganizationWire>,
    #[serde(default)]
    pub pagination: Option<PaginationWire>,
}

#[derive(Debug, Deserialize, Default)]
pub(crate) struct PaginationWire {
    #[serde(default)]
    pub page: u32,
    #[serde(default)]
    pub total_pages: u32,
    #[serde(default)]
    pub total_entries: u32,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PersonWire {
    pub id: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub name: Option<String>,
    pub title: Option<String>,
    pub email: Option<String>,
    pub email_status: Option<String>,
    pub linkedin_url: Option<String>,
    #[serde(default)]
    pub phone_numbers: Vec<PhoneWire>,
    pub organization_id: Option<String>,
    pub seniority: Option<String>,
    #[serde(default)]
    pub departments: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PhoneWire {
    pub sanitized_number: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct OrganizationWire {
    pub id: String,
    pub name: Option<String>,
    pub primary_domain: Option<String>,
    pub website_url: Option<String>,
    pub industry: Option<String>,
    pub estimated_num_employees: Option<u32>,
    pub organization_revenue_printed: Option<String>,
    pub city: Option<String>,
    pub founded_year: Option<u32>,
}

impl From<PersonWire> for ApolloPerson {
    fn from(w: PersonWire) -> Self {
        let name = w.name.unwrap_or_else(|| match (w.first_name, w.last_name) {
            (Some(f), Some(l)) => format!("{f} {l}"),
            (Some(f), None) => f,
            (None, Some(l)) => l,
            (None, None) => String::new(),
        });
        let phone = w
            .phone_numbers
            .into_iter()
            .find_map(|p| p.sanitized_number)
            .filter(|s| !s.is_empty());
        Self {
            id: w.id,
            name,
            title: w.title,
            email: w.email,
            email_status: w.email_status.as_deref().map(ApolloPersonEmailStatus::from_str),
            linkedin_url: w.linkedin_url,
            phone,
            company_id: w.organization_id,
            seniority: w.seniority,
            departments: w.departments,
        }
    }
}

impl From<OrganizationWire> for ApolloCompany {
    fn from(w: OrganizationWire) -> Self {
        let domain = w.primary_domain.or_else(|| {
            w.website_url
                .as_deref()
                .and_then(|u| u.split("://").nth(1).unwrap_or(u).split('/').next())
                .map(str::to_string)
                .filter(|s| !s.is_empty())
        });
        Self {
            id: w.id,
            name: w.name.unwrap_or_default(),
            domain,
            industry: w.industry,
            employee_count: w.estimated_num_employees,
            revenue_band: w.organization_revenue_printed,
            headquarters_city: w.city,
            founded_year: w.founded_year,
        }
    }
}
