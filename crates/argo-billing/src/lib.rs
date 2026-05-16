//! `argo-billing` — Stripe subscription lifecycle for the v1 commercial
//! launch (`claude-stripe-subscription-integration`).
//!
//! Three surface areas:
//!
//! - [`stripe_client::StripeClient`] — thin HTTP wrapper around the
//!   `api.stripe.com` REST endpoints used by Checkout, Customer Portal,
//!   and the Subscription resource.
//! - [`checkout`] / [`portal`] — high-level helpers that build the
//!   form-encoded request bodies the SPA + gateway funnel through.
//! - [`webhook`] — Stripe-signed inbound event handler: HMAC-SHA256
//!   `Stripe-Signature` verification (with the 5-minute tolerance window
//!   Stripe documents), idempotent dedup via the `stripe_event` table,
//!   and a `customer.subscription.*` → `Tier` mapper that the gateway
//!   persists onto `workspace.tier` via [`tier_store`].
//!
//! Tier persistence lives in [`tier_store`]: the brief held
//! `argo-store` to *additive* migrations + `argo-identity::tier` to
//! consume-only, so the writeback path runs raw `sqlx` from this crate
//! against the pool exposed by `SessionStore::pool()`.

#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

/// HTTP wrapper around the Stripe API (Checkout, Portal, Subscriptions).
pub mod stripe_client;
/// Checkout-session creation helpers.
pub mod checkout;
/// Customer-portal session creation helpers.
pub mod portal;
/// Inbound webhook handling: signature verify + idempotency + tier dispatch.
pub mod webhook;
/// Persistence helpers — `stripe_customer`, `stripe_event`, `workspace.tier`.
pub mod tier_store;

pub use stripe_client::{StripeClient, StripeConfig};
pub use webhook::{SubscriptionEvent, WebhookError, WebhookOutcome};
