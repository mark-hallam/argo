//! Lever ATS MCP connector.
//!
//! Typed REST client for the Lever Hire API (`https://api.lever.co/v1/`).
//! Pairs with [`argo-mcp-connector-greenhouse`] to give the recruiter +
//! recruiter-screener verticals coverage of the two major tech-recruiting
//! ATSes.
//!
//! ```no_run
//! use argo_mcp_connector_lever::LeverConnector;
//!
//! # async fn doc() -> argo_types::ArgoResult<()> {
//! let lv = LeverConnector::new("lever-api-key".into())?;
//! let _opportunities = lv.list_candidates(None).await?;
//! # Ok(()) }
//! ```
//!
//! ## Auth
//!
//! Lever uses **HTTP Basic** with the API key as the username and an
//! **empty password** — `Authorization: Basic <base64("key:")>`. Reqwest's
//! `basic_auth(key, Some(""))` produces exactly that header. This is the
//! same pattern as Clearbit and Greenhouse.
//!
//! ## Pagination
//!
//! Lever responses wrap the payload in `{ "data": [...], "hasNext": bool,
//! "next": "<cursor>" }`. The connector follows the documented cursor
//! style: when `hasNext == true`, the opaque `next` cursor is passed back
//! to Lever via the `offset` query parameter on the subsequent request.
//! `list_candidates`, `list_postings`, and `list_stages` accumulate all
//! pages before returning.
//!
//! ## `None` vs error
//!
//! `get_candidate` returns `Ok(None)` for `404` — Lever has no record of
//! the opportunity id — and a `Provider` error for any other non-2xx.
//!
//! ## Rate limits
//!
//! `429 Too Many Requests` is honoured once: the connector reads
//! `Retry-After` (seconds, capped at 60), sleeps, and retries the same
//! request. A second 429 propagates as a `Provider` error.
//!
//! ## Stage validation
//!
//! Stage ids are per-tenant and the brief is explicit that we must not
//! hardcode them. [`LeverConnector::advance_stage`] calls
//! [`LeverConnector::list_stages`] first and rejects the request with
//! `ArgoError::Schema` if the target id is not present — this surfaces
//! mis-paired stage/posting calls before they hit Lever's API.
//!
//! ## Out of scope (v1)
//!
//! Lever Nurture (the marketing CRM half of the product), offer-letter
//! generation, bulk import, and admin/permissions endpoints are
//! deliberately not covered. This connector targets Lever Hire only.

#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod client;
mod types;

pub use client::LeverConnector;
pub use types::{LeverCandidate, LeverPosting, LeverStage};
