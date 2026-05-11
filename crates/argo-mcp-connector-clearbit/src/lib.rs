//! Clearbit MCP connector.
//!
//! Typed REST client for Clearbit's person + company enrichment APIs. Covers
//! the lead-research vertical's optional enrichment surface: look up a
//! person by email, a company by domain, both in one combined call, and
//! prospect for companies via discovery search.
//!
//! ```no_run
//! use argo_mcp_connector_clearbit::ClearbitConnector;
//!
//! # async fn doc() -> argo_types::ArgoResult<()> {
//! let cb = ClearbitConnector::new("sk_xxx".into())?;
//! if let Some(person) = cb.enrich_person_by_email("ada@example.test").await? {
//!     println!("{} works at {:?}", person.full_name, person.company_domain);
//! }
//! # Ok(()) }
//! ```
//!
//! ## Auth
//!
//! Clearbit uses **HTTP Basic** with the API key as the username and an
//! **empty password** (`Authorization: Basic <base64("key:")>`). Reqwest's
//! `basic_auth(key, Some(""))` emits exactly that header.
//!
//! ## Subdomain routing
//!
//! Clearbit splits its surface across three hosts and the connector routes
//! per operation:
//!
//! | Method                       | Host                          |
//! |------------------------------|-------------------------------|
//! | `enrich_person_by_email`     | `person.clearbit.com`         |
//! | `enrich_company_by_domain`   | `company.clearbit.com`        |
//! | `combined_lookup`            | `person.clearbit.com`         |
//! | `discovery_search`           | `discovery.clearbit.com`      |
//!
//! ## 202-Accepted async polling
//!
//! When Clearbit has not cached the requested entity it queues a backfill
//! and returns `202 Accepted`. The connector polls with backoff
//! `[1s, 2s, 4s, 8s, 15s]` (30s ceiling). The poll resolves to:
//!
//! - `Some(...)` if a `200` arrives mid-schedule;
//! - `None` if the upstream eventually returns `404` (entity unknown);
//! - [`argo_types::ArgoError::Provider`] if the schedule exhausts at `202`
//!   (hint: enrichment still queued, retry later).
//!
//! Do NOT conflate `202` with `429`: both can carry `Retry-After`, but the
//! semantics differ — `202` is "we're fetching this", `429` is "you're going
//! too fast". The connector handles both, distinctly.
//!
//! ## `None` vs error
//!
//! `404` returns `Ok(None)` — that means "not in Clearbit", which is a
//! routine outcome for cold-prospect emails. `401`/`403` propagate as
//! `Provider` errors so misconfigured credentials surface loudly.
//!
//! ## Privacy
//!
//! Clearbit lookups are GDPR-sensitive. The connector itself is dumb (no
//! consent management) — calling verticals' prompts and persona must
//! establish operator-side lawful basis. This is intentional: the
//! enrichment layer cannot know what consent the caller has obtained.

#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod client;
mod types;

pub use client::ClearbitConnector;
pub use types::{ClearbitCombinedLookup, ClearbitCompany, ClearbitPerson};
