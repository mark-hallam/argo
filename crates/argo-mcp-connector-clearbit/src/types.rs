//! Clearbit DTOs.
//!
//! Public types (`ClearbitPerson`, `ClearbitCompany`, `ClearbitCombinedLookup`)
//! are the caller-facing shape. The `*Wire` structs mirror Clearbit's nested
//! JSON envelope (`name.fullName`, `employment.domain`, `metrics.employees`,
//! …) and are converted to the public form at the serde boundary via `From`
//! impls so callers never see wire oddities.

use serde::Deserialize;

/// Clearbit person enrichment result. Fields trace 1:1 to Clearbit's
/// `name.fullName`, `linkedin.handle`, `employment.{name,domain,title,
/// seniority,role}`, and `geo.{city,state,country}` (concatenated into
/// [`Self::location`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClearbitPerson {
    /// Clearbit-issued opaque id.
    pub id: String,
    /// `name.fullName`.
    pub full_name: String,
    /// Echoed lookup-key email.
    pub email: String,
    /// LinkedIn path segment (not URL).
    pub linkedin_handle: Option<String>,
    /// Twitter username (no `@`).
    pub twitter_handle: Option<String>,
    /// `employment.name`.
    pub company_name: Option<String>,
    /// `employment.domain`.
    pub company_domain: Option<String>,
    /// `employment.title`.
    pub title: Option<String>,
    /// `executive` / `manager` / `individual_contributor`.
    pub seniority: Option<String>,
    /// `leadership` / `engineering` / `sales` / …
    pub role: Option<String>,
    /// `"City, State, Country"` (nil parts elided).
    pub location: Option<String>,
}

/// Clearbit company enrichment result. Fields trace to `category.industry`,
/// `category.industryGroup`, `metrics.{employees,annualRevenue}`,
/// `fundingTotal`, and `tech`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClearbitCompany {
    /// Clearbit-issued opaque id.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Primary domain (lookup key for `enrich_company_by_domain`).
    pub domain: String,
    /// Top-level industry (e.g. `Software`).
    pub industry: Option<String>,
    /// Finer-grained industry group.
    pub category_industry: Option<String>,
    /// Headcount estimate.
    pub employees: Option<u32>,
    /// Annual revenue estimate, USD.
    pub annual_revenue_usd: Option<u64>,
    /// Lifetime funding raised, USD.
    pub funding_total_usd: Option<u64>,
    /// Tech-stack tags detected on the company's web properties.
    pub tech_stack: Vec<String>,
    /// LinkedIn company handle.
    pub linkedin_handle: Option<String>,
    /// Twitter company handle.
    pub twitter_handle: Option<String>,
}

/// Combined person + company lookup result.
///
/// Returned by [`crate::ClearbitConnector::combined_lookup`], which calls
/// Clearbit's `/v2/combined/find` and saves the caller a round trip.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ClearbitCombinedLookup {
    /// Person enrichment, if Clearbit knew the email.
    pub person: Option<ClearbitPerson>,
    /// Company enrichment for the person's employer, if known.
    pub company: Option<ClearbitCompany>,
}

// -- wire shapes -------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub(crate) struct PersonWire {
    pub id: String,
    #[serde(default)]
    pub name: NameWire,
    pub email: Option<String>,
    pub linkedin: Option<HandleWire>,
    pub twitter: Option<HandleWire>,
    pub employment: Option<EmploymentWire>,
    pub geo: Option<GeoWire>,
}

#[derive(Debug, Deserialize, Default)]
pub(crate) struct NameWire {
    #[serde(rename = "fullName")]
    pub full_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct HandleWire {
    pub handle: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct EmploymentWire {
    pub name: Option<String>,
    pub domain: Option<String>,
    pub title: Option<String>,
    pub seniority: Option<String>,
    pub role: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct GeoWire {
    pub city: Option<String>,
    pub state: Option<String>,
    pub country: Option<String>,
}

impl From<PersonWire> for ClearbitPerson {
    fn from(w: PersonWire) -> Self {
        let location = join_location(w.geo.as_ref());
        let (company_name, company_domain, title, seniority, role) = w
            .employment
            .map(|e| (e.name, e.domain, e.title, e.seniority, e.role))
            .unwrap_or((None, None, None, None, None));
        Self {
            id: w.id,
            full_name: w.name.full_name.unwrap_or_default(),
            email: w.email.unwrap_or_default(),
            linkedin_handle: w.linkedin.and_then(|h| h.handle),
            twitter_handle: w.twitter.and_then(|h| h.handle),
            company_name,
            company_domain,
            title,
            seniority,
            role,
            location,
        }
    }
}

fn join_location(geo: Option<&GeoWire>) -> Option<String> {
    let g = geo?;
    let parts: Vec<String> = [g.city.clone(), g.state.clone(), g.country.clone()]
        .into_iter()
        .flatten()
        .filter(|s| !s.is_empty())
        .collect();
    (!parts.is_empty()).then(|| parts.join(", "))
}

#[derive(Debug, Deserialize)]
pub(crate) struct CompanyWire {
    pub id: String,
    pub name: Option<String>,
    pub domain: Option<String>,
    pub category: Option<CategoryWire>,
    pub metrics: Option<MetricsWire>,
    #[serde(rename = "fundingTotal")]
    pub funding_total: Option<u64>,
    #[serde(default)]
    pub tech: Vec<String>,
    pub linkedin: Option<HandleWire>,
    pub twitter: Option<HandleWire>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CategoryWire {
    pub industry: Option<String>,
    #[serde(rename = "industryGroup")]
    pub industry_group: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MetricsWire {
    pub employees: Option<u32>,
    #[serde(rename = "annualRevenue")]
    pub annual_revenue: Option<u64>,
}

impl From<CompanyWire> for ClearbitCompany {
    fn from(w: CompanyWire) -> Self {
        let (industry, category_industry) = w
            .category
            .map(|c| (c.industry, c.industry_group))
            .unwrap_or((None, None));
        let (employees, annual_revenue) = w
            .metrics
            .map(|m| (m.employees, m.annual_revenue))
            .unwrap_or((None, None));
        Self {
            id: w.id,
            name: w.name.unwrap_or_default(),
            domain: w.domain.unwrap_or_default(),
            industry,
            category_industry,
            employees,
            annual_revenue_usd: annual_revenue,
            funding_total_usd: w.funding_total,
            tech_stack: w.tech,
            linkedin_handle: w.linkedin.and_then(|h| h.handle),
            twitter_handle: w.twitter.and_then(|h| h.handle),
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct CombinedWire {
    pub person: Option<PersonWire>,
    pub company: Option<CompanyWire>,
}

impl From<CombinedWire> for ClearbitCombinedLookup {
    fn from(w: CombinedWire) -> Self {
        Self {
            person: w.person.map(Into::into),
            company: w.company.map(Into::into),
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct DiscoveryWire {
    #[serde(default)]
    pub results: Vec<CompanyWire>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn join_location_skips_missing_and_handles_none() {
        let geo = GeoWire {
            city: Some("Berlin".into()),
            state: None,
            country: Some("Germany".into()),
        };
        assert_eq!(join_location(Some(&geo)).as_deref(), Some("Berlin, Germany"));
        let empty = GeoWire { city: None, state: Some(String::new()), country: None };
        assert!(join_location(Some(&empty)).is_none());
        assert!(join_location(None).is_none());
    }
}
