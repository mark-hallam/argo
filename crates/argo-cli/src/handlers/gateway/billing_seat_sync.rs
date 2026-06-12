//! Best-effort per-teammate seat-quantity sync at the lifecycle seams
//! (`pricing-wire-quantity-sync-lifecycle-callsites`, DEC-053 — the
//! wire-or-defer pairing #2868 declared).
//!
//! One helper, five callers: hire, freeze, unfreeze, archive/restore
//! (`routes_worker_settings_danger`) and the invoice-webhook reconcile.
//! The load-bearing design decision lives here: lifecycle sites are
//! BEST-EFFORT — a Stripe outage (or an unconfigured Stripe in
//! dev/demo) must NEVER fail the customer mutation. The mutation wins;
//! the failure is audited; the webhook reconcile re-asserts the
//! absolute count and heals any drift by the next invoice boundary.
//!
//! The subscription id comes from the tenant's OWN stored billing
//! mapping (`tier_store::subscription_id_for_tenant`) — never from
//! request input — so a caller can never pair tenant A with
//! subscription B.

use argo_audit::{AuditKind, AuditSink};
use argo_billing::quantity_sync::{sync_seat_quantity, QuantityChangeReason};
use argo_billing::stripe_client::StripeClient;
use argo_billing::tier_store;
use argo_store::SessionStore;
use argo_types::TenantId;
use serde_json::json;
use std::sync::Arc;

/// Re-assert `tenant`'s billed seat quantity after a teammate
/// lifecycle change. Never returns an error — see module doc.
pub(super) async fn best_effort_seat_sync(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    reason: QuantityChangeReason,
) {
    // Unconfigured Stripe (dev / demo / self-host without billing) is
    // the COMMON case — quietly skip; nothing to sync against.
    let Ok(client) = StripeClient::from_env() else {
        return;
    };
    let subscription_id = match tier_store::subscription_id_for_tenant(store, tenant).await {
        // No subscription mapped (free tenant / checkout never run):
        // nothing to sync. Not an error.
        Ok(None) => return,
        Ok(Some(sid)) => sid,
        Err(e) => {
            tracing::warn!(
                target: "billing::seat_sync",
                error = %e,
                "seat sync skipped: subscription lookup failed",
            );
            return;
        }
    };
    let outcome =
        sync_seat_quantity(&client, store, tenant, &subscription_id, reason).await;
    let payload = match &outcome {
        Ok(Some(change)) => json!({
            "reason": reason.as_str(),
            "outcome": "applied",
            "subscription_item_id": change.subscription_item_id,
            "old_quantity": change.old_quantity,
            "new_quantity": change.new_quantity,
        }),
        Ok(None) => json!({ "reason": reason.as_str(), "outcome": "noop" }),
        Err(e) => {
            tracing::warn!(
                target: "billing::seat_sync",
                error = %e,
                reason = reason.as_str(),
                "seat-quantity sync failed; webhook reconcile will heal",
            );
            json!({ "reason": reason.as_str(), "outcome": "error", "error": e.to_string() })
        }
    };
    let audit = AuditSink::new_with_tenant(store.clone(), tenant.to_string());
    if let Err(e) = audit
        .write(None, AuditKind::SeatQuantitySynced, payload, None)
        .await
    {
        tracing::warn!(target: "billing::seat_sync", error = %e, "seat-sync audit write failed");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_billing::quantity_sync::QuantityChangeReason;

    async fn mem_store() -> Arc<SessionStore> {
        let store = SessionStore::open(":memory:").await.expect("open");
        store.migrate().await.expect("migrate");
        Arc::new(store)
    }

    /// The best-effort property: with Stripe UNCONFIGURED (the dev /
    /// demo / outage shape) the helper returns without error and the
    /// caller's mutation is unaffected. This is the failure-handling
    /// contract the brief scoped to its own review.
    #[tokio::test]
    async fn unconfigured_stripe_is_a_quiet_noop() {
        let _g = crate::handlers::gateway::shared_env_lock().await;
        std::env::remove_var("STRIPE_SECRET_KEY");
        std::env::remove_var("STRIPE_WEBHOOK_SECRET");
        let store = mem_store().await;
        let (tenant, _) = store
            .ensure_workspace("seat-sync-t", "/tmp/seat-sync-t")
            .await
            .expect("tenant");
        best_effort_seat_sync(&store, tenant, QuantityChangeReason::Hire).await;
        // No panic, no error — and no audit row was attempted (skip
        // happened before the sync), which we can't observe directly;
        // the contract under test is "the call returns".
    }

    /// A tenant with NO stored subscription mapping (free tenant /
    /// checkout never run) is also a quiet no-op even when Stripe env
    /// is set — the lookup path returns None before any network call.
    #[tokio::test]
    async fn unmapped_tenant_is_a_quiet_noop() {
        let _g = crate::handlers::gateway::shared_env_lock().await;
        std::env::set_var("STRIPE_SECRET_KEY", "sk_test_x");
        std::env::set_var("STRIPE_WEBHOOK_SECRET", "whsec_x");
        let store = mem_store().await;
        let (tenant, _) = store
            .ensure_workspace("seat-sync-u", "/tmp/seat-sync-u")
            .await
            .expect("tenant");
        best_effort_seat_sync(&store, tenant, QuantityChangeReason::Pause).await;
        std::env::remove_var("STRIPE_SECRET_KEY");
        std::env::remove_var("STRIPE_WEBHOOK_SECRET");
    }
}
