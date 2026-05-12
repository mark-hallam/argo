//! Apollo.io MCP connector.
//!
//! Typed REST client for [Apollo.io](https://apollo.io)'s prospect-data
//! APIs. Covers person + company enrichment and the paginated `mixed_*`
//! search endpoints. Read-only enrichment in v1; outbound campaigns and
//! sequence management are operator-side and intentionally not implemented.
//!
//! ```no_run
//! use argo_mcp_connector_apollo::{ApolloClient, PersonLookup};
//!
//! # async fn doc() -> Result<(), argo_mcp_connector_apollo::ApolloError> {
//! let apollo = ApolloClient::new("apollo_xxx".into())?;
//! let person = apollo.enrich_person(&PersonLookup::by_email("ada@example.test")).await?;
//! println!("{} ({:?}) at {:?}", person.name, person.email_status, person.title);
//! # Ok(()) }
//! ```
//!
//! ## Auth
//!
//! `X-Api-Key: <api_key>`. Operator supplies via `ARGO_APOLLO_API_KEY`;
//! this crate does not read the env directly.
//!
//! ## Rate limits and credits
//!
//! - `429` → internal retry honoring `Retry-After + jitter` up to two
//!   times; persistent `429` surfaces as [`ApolloError::RateLimited`].
//! - Monthly credit cap → [`ApolloError::CreditsExhausted`] (distinct
//!   from rate-limit because remediation differs).
//! - `X-Apollo-Credits-Used` (when present) is logged via
//!   `tracing::debug!`; absence is non-fatal.
//!
//! ## Operator compliance note
//!
//! Apollo's Terms of Service prohibit bulk scraping and long-term storage
//! of enrichment results beyond the operator's own use case. This crate
//! is a transport; the calling vertical must establish lawful basis.

#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod client;
mod types;

pub use client::{ApolloClient, CompanyLookup, PeopleSearchFilters, PersonLookup};
pub use types::{
    ApolloCompany, ApolloCompanySearchResult, ApolloError, ApolloPerson, ApolloPersonEmailStatus,
    ApolloPeopleSearchResult,
};
