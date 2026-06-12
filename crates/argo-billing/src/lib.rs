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
/// Operator-driven replay of dead-lettered Stripe events (Stream
/// `codex-billing-webhook-resilience`). Reads the canonical
/// `raw_payload` and re-runs the same classify-then-apply path.
pub mod replay;
/// DEC-053 metered-overage reporting — compute period-to-date overage
/// units from the live usage meter and push them to a Stripe metered
/// price, idempotently (`action=set` + `Idempotency-Key`).
pub mod overage;
/// DEC-053 per-teammate seat-quantity sync — re-assert the Pro
/// subscription's licensed-item quantity to the tenant's active
/// (non-archived, non-frozen) teammate count.
pub mod quantity_sync;
/// DEC-053 overage-report **job**: the batch caller that turns a
/// snapshot of every Pro tenant's live meter into Stripe usage records
/// (`pricing-wire-overage-report-and-cap-enforcement`).
pub mod overage_job;

pub use quantity_sync::{
    count_active_teammates, desired_quantity, plan_reconcile, sync_seat_quantity, QuantityChange,
    QuantityChangeReason, ReconcileDecision, SeatItem,
};
pub use overage::{
    est_overage_cost_micros, idempotency_key, overage_units, plan_overage, report_overage_usage,
    OverageReport, STRIPE_PRICE_PRO_OVERAGE_ENV,
};
pub use overage_job::{
    plan_overage_reports, run_overage_reports, OverageRunSummary, PlannedOverageReport,
    TenantOverageMeter,
};
pub use replay::{replay_event, replay_since, ReplayOutcome};
pub use stripe_client::{StripeClient, StripeConfig};
pub use webhook::{
    record_apply_failure, FailureOutcome, SubscriptionEvent, WebhookError, WebhookOutcome,
    DEFAULT_MAX_RETRIES,
};
