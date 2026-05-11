//! Greenhouse Harvest MCP connector.
//!
//! Typed REST client for the Greenhouse Harvest API
//! (`https://harvest.greenhouse.io/v1/`). Covers the recruiter-screener
//! vertical's load-bearing surface: read the live candidate pipeline and
//! write advance/reject/note decisions back into the ATS.
//!
//! ```no_run
//! use argo_mcp_connector_greenhouse::GreenhouseConnector;
//!
//! # async fn doc() -> argo_types::ArgoResult<()> {
//! let gh = GreenhouseConnector::new("harvest-api-key".into())?;
//! let _candidates = gh.list_candidates(None).await?;
//! # Ok(()) }
//! ```
//!
//! ## Auth
//!
//! Greenhouse Harvest uses **HTTP Basic** with the API key as the username
//! and an **empty password** (`Authorization: Basic <base64("key:")>`).
//! Reqwest's `basic_auth(key, Some(""))` produces exactly that header.
//!
//! ## On-Behalf-Of
//!
//! Greenhouse requires an `On-Behalf-Of: <user_id>` header on any write
//! that has compliance audit implications: `advance_application`,
//! `reject_application`, and `add_note`. The connector takes this user id
//! as an explicit parameter and validates it pre-flight; absent or zero
//! ids fail before any network call.
//!
//! ## Rate limits
//!
//! On a `429 Too Many Requests` the connector reads the `Retry-After`
//! header (seconds), sleeps for that interval, and retries the request
//! exactly once. A second 429 propagates as an [`argo_types::ArgoError::Provider`].
//!
//! ## Pagination
//!
//! v1 surfaces `since: Option<DateTime<Utc>>` filtering via Greenhouse's
//! `updated_after` query parameter. Cursor pagination (`Link: rel="next"`)
//! is a documented follow-up and is **not** implemented.
//!
//! ## Scorecards
//!
//! Scorecard / interview-kit endpoints are out for v1. Recruiter notes go
//! through [`GreenhouseConnector::add_note`].

#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod client;
mod types;

pub use client::GreenhouseConnector;
pub use types::{
    GhApplication, GhApplicationStatus, GhCandidate, GhJob, GhJobStatus, GhNoteVisibility, GhStage,
};
