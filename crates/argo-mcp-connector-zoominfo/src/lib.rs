//! ZoomInfo MCP connector.
//!
//! Typed REST client for ZoomInfo's Sales Insights API: person + company
//! enrichment and paginated search across both populations.
//!
//! ```no_run
//! use argo_mcp_connector_zoominfo::{ZoomInfoClient, ZoomInfoCredentials};
//!
//! # async fn doc() -> Result<(), argo_mcp_connector_zoominfo::ZoomInfoError> {
//! let zi = ZoomInfoClient::new(ZoomInfoCredentials::new("u", "p", "cid"))?;
//! let p = zi.enrich_person_by_email("ada@example.test").await?;
//! println!("{} ({:?})", p.first_name, p.job_title);
//! # Ok(()) }
//! ```
//!
//! ## Auth
//!
//! ZoomInfo uses **OAuth2 client-credentials**, not a static API key. The
//! connector calls `POST /authenticate`, caches the 1-hour bearer, and
//! refreshes ~5 min before expiry. On unexpected `401` the client force-
//! refreshes once and retries; a second `401` → [`ZoomInfoError::AuthFailed`].
//! Env-indirect: `ARGO_ZOOMINFO_USERNAME` / `_PASSWORD` / `_CLIENT_ID`.
//!
//! ## Credit awareness
//!
//! ZoomInfo charges per-lookup credits. The connector logs
//! `X-RateLimit-Remaining-Credits` via `tracing::debug!`; the header's
//! absence is not an error. `403` with credits-exhausted body surfaces as
//! [`ZoomInfoError::CreditsExhausted`] — distinct from `401` (auth) and
//! `429` (per-second rate limit).
//!
//! ## TOS / compliance
//!
//! Per ZoomInfo's terms, enrichment results must not be bulk-scraped or
//! retained long-term beyond the operator's own use case. The connector
//! does not retain results; operator-side retention policy is caller-side.

#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod client;
mod types;

pub use client::{ZoomInfoClient, ZoomInfoCredentials};
pub use types::{
    ZoomInfoCompany, ZoomInfoCompanySearchResult, ZoomInfoError, ZoomInfoPerson,
    ZoomInfoPersonSearchResult,
};

/// JSON-RPC method names this connector responds to when wrapped in an
/// `argo-mcp` server.
pub const METHODS: &[&str] = &[
    "zoominfo.enrich_person",
    "zoominfo.enrich_company",
    "zoominfo.search_people",
    "zoominfo.search_companies",
];
