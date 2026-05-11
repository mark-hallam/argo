//! QuickBooks Online MCP connector.
//!
//! Typed REST client for the QBO Customer / Invoice / Purchase (expense) /
//! Account surface. Pairs with the bookkeeping-assistant vertical to graduate
//! it from paper-only ledger work to writing journal entries into a real
//! general ledger.
//!
//! ```no_run
//! use argo_mcp_connector_quickbooks::{
//!     QuickBooksAuth, QuickBooksConnector, QuickBooksHost,
//! };
//!
//! # async fn doc() -> argo_types::ArgoResult<()> {
//! let auth = QuickBooksAuth::new(
//!     "client-id".into(),
//!     "client-secret".into(),
//!     "refresh-token".into(),
//! );
//! let qb = QuickBooksConnector::new(
//!     auth,
//!     "9341452094938041".into(),
//!     QuickBooksHost::Sandbox,
//! )?;
//! let _customers = qb.list_customers(None).await?;
//! # Ok(()) }
//! ```
//!
//! ## Auth
//!
//! QBO OAuth2 issues short-lived (~1h) access tokens and long-lived (~100d)
//! refresh tokens. The connector holds the refresh token, caches the access
//! token in-memory, and transparently refreshes on a 401 before propagating
//! the error.
//!
//! ## Money
//!
//! QBO emits amounts as JSON decimal dollars (`12.34`). The public API works
//! in integer cents (`i64`) — conversion happens at the serde boundary in
//! [`types`]. Callers never see floats.

#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

pub mod auth;
mod client;
pub mod types;

pub use auth::QuickBooksAuth;
pub use client::{QuickBooksConnector, QuickBooksHost};
pub use types::{
    cents_to_dollars, dollars_to_cents, QbAccount, QbCustomer, QbDocStatus, QbExpense,
    QbExpenseDraft, QbInvoice, QbInvoiceDraft, QbLine,
};
