//! Xero accounting MCP connector.
//!
//! Typed REST client for the Xero Accounting API covering invoices,
//! contacts, payments, and chart of accounts. Pairs with the existing
//! QuickBooks connector to close the small-business accounting gap in
//! AU/NZ/UK markets where Xero dominates.
//!
//! ```no_run
//! use argo_mcp_connector_xero::{XeroConnector, XeroInvoiceStatus};
//!
//! # async fn doc() -> argo_types::ArgoResult<()> {
//! let xero = XeroConnector::new(
//!     "oauth2-access-token".into(),
//!     "tenant-id-of-the-organization".into(),
//! );
//! let _drafts = xero.list_invoices(Some(XeroInvoiceStatus::Draft)).await?;
//! # Ok(()) }
//! ```
//!
//! ## Auth
//!
//! Xero uses OAuth 2.0 bearer tokens with a separate `xero-tenant-id`
//! header on every request — the tenant id is per-organization, not
//! per-user, so a consultant connecting multiple client orgs picks the
//! tenant id per call. The connector does **not** refresh tokens; the
//! caller holds an already-fresh access token and rotates it
//! out-of-band. A 401 response surfaces as a typed `Provider` error.
//!
//! ## Money
//!
//! Xero emits amounts as JSON decimal numbers (e.g. `199.99`). The
//! public API works in integer cents (`i64`) — conversion happens at
//! the serde boundary in [`types`]. Callers never see floats.
//!
//! ## Dates
//!
//! Xero returns dates in Microsoft `.NET`-style serialization
//! (`/Date(1234567890000+0000)/`) in some endpoints; [`types`]
//! provides a tolerant deserializer that also accepts ISO-8601.

#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod client;
pub mod types;

pub use client::{XeroConnector, XERO_API_BASE_URL};
pub use types::{
    cents_to_dollars, dollars_to_cents, NewPayment, NewXeroInvoice, XeroAccount, XeroContact,
    XeroInvoice, XeroInvoiceStatus, XeroLineItem, XeroPayment,
};

/// Env var: Xero OAuth2 access token for the integration smoke test.
pub const XERO_TEST_TOKEN_ENV: &str = "ARGO_XERO_TEST_TOKEN";
/// Env var: Xero tenant id (organization) for the integration smoke test.
pub const XERO_TEST_TENANT_ENV: &str = "ARGO_XERO_TEST_TENANT_ID";
