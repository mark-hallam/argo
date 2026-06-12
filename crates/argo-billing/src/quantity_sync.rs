//! DEC-053 per-teammate seat-quantity sync — makes "$49 per NAMED
//! teammate, pause stops billing" true in code.
//!
//! The Pro subscription's **licensed** (per-seat) item quantity tracks
//! the tenant's count of ACTIVE teammates: non-archived AND non-frozen.
//! The metered overage item (see [`crate::overage`]) is left untouched —
//! this module only re-asserts the seat quantity.
//!
//! # Design: declarative convergence, not imperative `+1`/`-1` deltas
//!
//! Every lifecycle event (hire, pause, unpause, delete) and every
//! invoice webhook calls the SAME entrypoint, [`sync_seat_quantity`],
//! which recomputes the active count and re-asserts
//! `quantity = active_count` against Stripe. This makes the operation
//! idempotent and self-healing: a missed event is corrected by the next
//! sync, and the invoice-time reconciliation is just the same re-assert
//! landing on a billing boundary. An imperative `+1`/`-1` scheme would
//! instead accumulate drift on any missed or double-delivered event —
//! exactly the failure the reconciliation step exists to catch, so we
//! eliminate the class rather than patch it.
//!
//! # Proration
//!
//! Quantity changes use Stripe `proration_behavior=create_prorations`
//! (Stripe's default for quantity updates). A mid-cycle hire is charged
//! pro-rata for the remainder of the period; a mid-cycle pause issues a
//! pro-rata credit — so "pause stops billing" is honoured to the day,
//! not deferred to the next renewal.
//!
//! # The floor invariant
//!
//! Quantity is never below `1` while a subscription exists: Stripe
//! rejects a zero-quantity licensed item, and a paying tenant who has
//! paused every teammate still holds one seat until they cancel via the
//! Customer Portal. [`desired_quantity`] enforces the floor.

use crate::stripe_client::{StripeClient, StripeError};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use serde_json::Value;

/// Stripe `proration_behavior` applied to every seat-quantity change.
/// `create_prorations` charges/credits the remaining period pro-rata so
/// a mid-cycle pause stops billing immediately (DEC-053).
pub const PRORATION_BEHAVIOR: &str = "create_prorations";

/// The billed seat quantity for a given active-teammate count, floored
/// at `1` (the subscription always holds at least one seat while live).
///
/// ```
/// use argo_billing::quantity_sync::desired_quantity;
/// assert_eq!(desired_quantity(0), 1); // floor — every teammate paused
/// assert_eq!(desired_quantity(1), 1);
/// assert_eq!(desired_quantity(7), 7);
/// ```
#[must_use]
pub fn desired_quantity(active_teammates: u64) -> u64 {
    active_teammates.max(1)
}

/// Form body for a Stripe `POST /v1/subscription_items/{id}` quantity
/// update. Pure + testable: carries the new quantity and the
/// [`PRORATION_BEHAVIOR`] so the proration choice is auditable.
///
/// ```
/// use argo_billing::quantity_sync::quantity_update_form;
/// let form = quantity_update_form(3);
/// assert!(form.contains(&("quantity".to_string(), "3".to_string())));
/// ```
#[must_use]
pub fn quantity_update_form(quantity: u64) -> Vec<(String, String)> {
    vec![
        ("quantity".to_string(), quantity.to_string()),
        (
            "proration_behavior".to_string(),
            PRORATION_BEHAVIOR.to_string(),
        ),
    ]
}

/// The licensed (per-seat) subscription item resolved from a Stripe
/// subscription object — the item whose quantity this module syncs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeatItem {
    /// Stripe subscription-item id (`si_…`).
    pub item_id: String,
    /// Current billed quantity on that item.
    pub quantity: u64,
}

/// Pick the licensed seat item out of a Stripe subscription's
/// `items.data`, skipping any metered item (the overage price). Returns
/// `None` when the subscription has no licensed item (e.g. metered-only).
///
/// An item is treated as metered only when
/// `price.recurring.usage_type == "metered"`; everything else (including
/// a missing `usage_type`) defaults to licensed, since the per-seat Pro
/// price is the common case.
#[must_use]
pub fn parse_seat_item(subscription: &Value) -> Option<SeatItem> {
    let items = subscription.get("items")?.get("data")?.as_array()?;
    for item in items {
        let usage_type = item
            .get("price")
            .and_then(|p| p.get("recurring"))
            .and_then(|r| r.get("usage_type"))
            .and_then(|v| v.as_str())
            .unwrap_or("licensed");
        if usage_type == "metered" {
            continue;
        }
        let item_id = item.get("id").and_then(|v| v.as_str())?.to_string();
        let quantity = item.get("quantity").and_then(Value::as_u64).unwrap_or(1);
        return Some(SeatItem { item_id, quantity });
    }
    None
}

/// Pure reconcile decision: current billed quantity vs. the target
/// derived from the live active-teammate count. [`needs_update`] is the
/// drift signal both lifecycle events and invoice reconciliation act on.
///
/// [`needs_update`]: ReconcileDecision::needs_update
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReconcileDecision {
    /// Quantity Stripe currently bills.
    pub current: u64,
    /// Quantity it SHOULD bill (floored active count).
    pub target: u64,
}

impl ReconcileDecision {
    /// `true` when Stripe's quantity diverges from the active count and
    /// an update must be pushed.
    #[must_use]
    pub fn needs_update(&self) -> bool {
        self.current != self.target
    }
}

/// Compute the reconcile decision for a current quantity and active
/// count. Heals drift in either direction and honours the floor.
///
/// ```
/// use argo_billing::quantity_sync::plan_reconcile;
/// // hire: 2 active billed, 3rd hired → bump to 3.
/// assert!(plan_reconcile(2, 3).needs_update());
/// // already in sync → no-op.
/// assert!(!plan_reconcile(3, 3).needs_update());
/// // injected drift: Stripe stuck at 5, only 2 active → heal to 2.
/// assert_eq!(plan_reconcile(5, 2).target, 2);
/// ```
#[must_use]
pub fn plan_reconcile(current_quantity: u64, active_teammates: u64) -> ReconcileDecision {
    ReconcileDecision {
        current: current_quantity,
        target: desired_quantity(active_teammates),
    }
}

/// Why a seat-quantity change happened — written into the audit row so
/// "who/why" is recoverable (DEC-053 acceptance: every change audited).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuantityChangeReason {
    /// A teammate was hired.
    Hire,
    /// A teammate was paused (frozen).
    Pause,
    /// A paused teammate was reactivated.
    Unpause,
    /// A teammate was deleted (archived).
    Delete,
    /// Invoice-time drift reconciliation re-asserted the truth.
    Reconcile,
}

impl QuantityChangeReason {
    /// Stable lowercase slug for the audit row / logs.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            QuantityChangeReason::Hire => "hire",
            QuantityChangeReason::Pause => "pause",
            QuantityChangeReason::Unpause => "unpause",
            QuantityChangeReason::Delete => "delete",
            QuantityChangeReason::Reconcile => "reconcile",
        }
    }
}

/// A seat-quantity change that was applied to Stripe — returned by
/// [`sync_seat_quantity`] so the caller writes one audit row per change.
/// `None` from the sync means no change was needed (already in sync).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuantityChange {
    /// Tenant whose subscription changed.
    pub tenant_id: TenantId,
    /// Stripe subscription-item id that was updated.
    pub subscription_item_id: String,
    /// Quantity before the update.
    pub old_quantity: u64,
    /// Quantity after the update (the floored active count).
    pub new_quantity: u64,
    /// What triggered the change.
    pub reason: QuantityChangeReason,
}

/// Count a tenant's ACTIVE teammates: non-archived (`worker.archived_at
/// IS NULL`) and not currently frozen (no open `worker_freeze` row).
/// This is the source-of-truth count both checkout and the sync path
/// bill against.
///
/// # Errors
/// Returns [`ArgoError::Provider`] on a query failure.
pub async fn count_active_teammates(store: &SessionStore, tenant: TenantId) -> ArgoResult<u64> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM worker w \
         WHERE w.tenant_id = ? AND w.archived_at IS NULL \
           AND NOT EXISTS ( \
               SELECT 1 FROM worker_freeze f \
               WHERE f.tenant_id = w.tenant_id AND f.worker_id = w.id \
                 AND f.unfrozen_at IS NULL )",
    )
    .bind(tenant.to_string())
    .fetch_one(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("count_active_teammates: {e}")))?;
    Ok(count.max(0) as u64)
}

/// GET a Stripe subscription object (for reconciliation — reads the
/// current licensed-item quantity).
///
/// # Errors
/// Returns [`StripeError`] on transport or API failure.
pub async fn fetch_subscription(
    client: &StripeClient,
    subscription_id: &str,
) -> Result<Value, StripeError> {
    client
        .get(&format!("/v1/subscriptions/{subscription_id}"))
        .await
}

/// POST a seat-quantity update to Stripe for one subscription item.
///
/// # Errors
/// Returns [`StripeError`] on transport or API failure.
pub async fn set_seat_quantity(
    client: &StripeClient,
    subscription_item_id: &str,
    quantity: u64,
) -> Result<Value, StripeError> {
    let path = format!("/v1/subscription_items/{subscription_item_id}");
    client.post_form(&path, &quantity_update_form(quantity)).await
}

/// Re-assert `quantity = active_teammate_count` on the tenant's Pro
/// subscription. The single entrypoint for hire / pause / unpause /
/// delete and invoice reconciliation — all converge to the same truth.
///
/// Returns `Some(change)` when a Stripe update was applied (the caller
/// writes the audit row), `None` when already in sync.
///
/// # Errors
/// Returns [`ArgoError::Provider`] when the active count can't be read,
/// Stripe can't be reached, or the subscription has no licensed seat
/// item to update.
pub async fn sync_seat_quantity(
    client: &StripeClient,
    store: &SessionStore,
    tenant: TenantId,
    subscription_id: &str,
    reason: QuantityChangeReason,
) -> ArgoResult<Option<QuantityChange>> {
    let active = count_active_teammates(store, tenant).await?;
    let subscription = fetch_subscription(client, subscription_id)
        .await
        .map_err(|e| ArgoError::Provider(format!("fetch subscription {subscription_id}: {e}")))?;
    let seat = parse_seat_item(&subscription).ok_or_else(|| {
        ArgoError::Provider(format!(
            "subscription {subscription_id} has no licensed seat item"
        ))
    })?;
    let decision = plan_reconcile(seat.quantity, active);
    if !decision.needs_update() {
        return Ok(None);
    }
    set_seat_quantity(client, &seat.item_id, decision.target)
        .await
        .map_err(|e| ArgoError::Provider(format!("set seat quantity: {e}")))?;
    Ok(Some(QuantityChange {
        tenant_id: tenant,
        subscription_item_id: seat.item_id,
        old_quantity: seat.quantity,
        new_quantity: decision.target,
        reason,
    }))
}

#[cfg(test)]
#[path = "quantity_sync_tests.rs"]
mod tests;
