//! `[billing]` and nested `[billing.webhook]` config sections
//! (Stream `codex-billing-webhook-resilience`). Drives the retry
//! ceiling the Stripe webhook honours before it gives up retrying
//! and dead-letters the event for operator replay.

use serde::{Deserialize, Serialize};

/// Default ceiling on webhook apply re-attempts before the gateway
/// returns 200 and writes a dead-letter row. Matches Stripe's own
/// documented 5-step exponential redelivery schedule.
pub const DEFAULT_BILLING_WEBHOOK_MAX_RETRIES: u32 = 5;

/// `[billing]` — top-level billing knobs.
///
/// # Examples
///
/// ```rust
/// use argo_config::BillingConfig;
///
/// let cfg = BillingConfig::default();
/// assert_eq!(cfg.webhook.max_retries, 5);
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct BillingConfig {
    /// `[billing.webhook]` — Stripe-inbound webhook tuning.
    pub webhook: BillingWebhookConfig,
}

/// `[billing.webhook]` — knobs that govern how aggressively the
/// gateway retries an apply failure before dead-lettering. The
/// retry ceiling is also what the dead-letter writer compares
/// against when deciding 500-vs-200 on each redelivery.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BillingWebhookConfig {
    /// Apply-attempt cap. After this many failed re-deliveries the
    /// gateway returns 200 (so Stripe stops retrying), writes the
    /// event to `billing_event_dead_letter`, and the operator must
    /// run `argo billing replay --event-id <id>` once the upstream
    /// transient cause clears.
    pub max_retries: u32,
}

impl Default for BillingWebhookConfig {
    fn default() -> Self {
        Self {
            max_retries: DEFAULT_BILLING_WEBHOOK_MAX_RETRIES,
        }
    }
}
