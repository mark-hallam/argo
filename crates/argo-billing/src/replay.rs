//! Operator-driven replay of dead-lettered Stripe webhook events
//! (Stream `codex-billing-webhook-resilience`).
//!
//! Replay reads `raw_payload` from `billing_event_dead_letter`, runs
//! it back through the same classification + tier mutation path the
//! gateway uses, and stamps `resolved_at` on success. **No call to
//! Stripe** — the payload is already canonical and HMAC was verified
//! at original delivery.

use crate::tier_store::apply_subscription_event;
use crate::webhook::{classify_event, decode_event, WebhookOutcome};
use argo_store::{billing_dlq_query, SessionStore};
use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, Utc};

/// Outcome of a single replay attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplayOutcome {
    /// Event re-applied; `resolved_at` stamped on the DLQ row.
    Replayed {
        /// `tier`/`rows_touched` from the apply (zero rows is still
        /// success — the workspace may simply not exist yet).
        rows_touched: u64,
        /// Whether the event would have surfaced a past-due warning
        /// at original delivery.
        past_due_warning: bool,
    },
    /// Classifier marked the event Ignored (e.g. a `tenant_id` is
    /// missing) — DLQ row resolved with no tier side-effect.
    ResolvedAsIgnored,
    /// `event_id` had no DLQ row (already replayed and resolved, or
    /// never existed). Caller should treat as no-op.
    NotFound,
}

/// Replay a single DLQ entry by `event_id`. Resolves the row on
/// success.
pub async fn replay_event(
    store: &SessionStore,
    event_id: &str,
    now: DateTime<Utc>,
) -> ArgoResult<ReplayOutcome> {
    let Some(entry) = billing_dlq_query::fetch_dead_letter(store, event_id).await? else {
        return Ok(ReplayOutcome::NotFound);
    };
    if entry.resolved_at.is_some() {
        return Ok(ReplayOutcome::NotFound);
    }
    let event = decode_event(entry.raw_payload.as_bytes())
        .map_err(|e| ArgoError::Provider(format!("replay decode: {e}")))?;
    match classify_event(&event) {
        WebhookOutcome::Subscription(sub) => {
            let out = apply_subscription_event(store, &sub, now).await?;
            billing_dlq_query::mark_resolved(store, event_id, now).await?;
            Ok(ReplayOutcome::Replayed {
                rows_touched: out.rows_touched,
                past_due_warning: out.past_due_warning,
            })
        }
        WebhookOutcome::Duplicate | WebhookOutcome::Ignored => {
            billing_dlq_query::mark_resolved(store, event_id, now).await?;
            Ok(ReplayOutcome::ResolvedAsIgnored)
        }
    }
}

/// Replay every unresolved DLQ row whose `failed_at >= since`.
/// Returns one outcome per attempted row in failed-at-DESC order.
pub async fn replay_since(
    store: &SessionStore,
    since: DateTime<Utc>,
    limit: i64,
    now: DateTime<Utc>,
) -> ArgoResult<Vec<(String, ReplayOutcome)>> {
    let rows = billing_dlq_query::list_dead_letters(
        store,
        Some(since.timestamp()),
        false,
        limit,
    )
    .await?;
    let mut results = Vec::with_capacity(rows.len());
    for entry in rows {
        let outcome = replay_event(store, &entry.event_id, now).await?;
        results.push((entry.event_id, outcome));
    }
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tier_store::has_processed_event;
    use argo_types::TenantId;

    async fn fresh_store() -> SessionStore {
        let s = SessionStore::open(":memory:").await.expect("memory");
        s.migrate().await.expect("migrate");
        s
    }

    fn payload(tenant: TenantId, event_id: &str) -> String {
        serde_json::json!({
            "id": event_id,
            "type": "customer.subscription.updated",
            "data": { "object": {
                "id": "sub_a",
                "customer": "cus_a",
                "status": "active",
                "metadata": { "tenant_id": tenant.to_string(), "tier": "pro" }
            }}
        })
        .to_string()
    }

    #[tokio::test]
    async fn replay_event_applies_and_resolves() {
        let store = fresh_store().await;
        let tenant = TenantId::new();
        let now = Utc::now();
        let raw = payload(tenant, "evt_replay");
        billing_dlq_query::upsert_dead_letter(
            &store,
            "evt_replay",
            "customer.subscription.updated",
            Some(tenant),
            &raw,
            now,
            "tenant locked",
        )
        .await
        .expect("seed");

        match replay_event(&store, "evt_replay", now).await.expect("replay") {
            ReplayOutcome::Replayed { .. } => {}
            other => panic!("expected Replayed, got {other:?}"),
        }

        let entry = billing_dlq_query::fetch_dead_letter(&store, "evt_replay")
            .await
            .expect("fetch")
            .expect("row");
        assert!(entry.resolved_at.is_some());
        assert!(has_processed_event(&store, "evt_replay").await.expect("dedup"));
    }

    #[tokio::test]
    async fn replay_event_missing_row_is_not_found() {
        let store = fresh_store().await;
        let outcome = replay_event(&store, "evt_missing", Utc::now())
            .await
            .expect("replay");
        assert_eq!(outcome, ReplayOutcome::NotFound);
    }

    #[tokio::test]
    async fn replay_event_already_resolved_returns_not_found() {
        let store = fresh_store().await;
        let now = Utc::now();
        let tenant = TenantId::new();
        let raw = payload(tenant, "evt_done");
        billing_dlq_query::upsert_dead_letter(
            &store,
            "evt_done",
            "customer.subscription.updated",
            Some(tenant),
            &raw,
            now,
            "x",
        )
        .await
        .expect("seed");
        billing_dlq_query::mark_resolved(&store, "evt_done", now)
            .await
            .expect("resolve");
        let outcome = replay_event(&store, "evt_done", now).await.expect("replay");
        assert_eq!(outcome, ReplayOutcome::NotFound);
    }

    #[tokio::test]
    async fn replay_since_processes_unresolved_window() {
        let store = fresh_store().await;
        let tenant = TenantId::new();
        let now = Utc::now();
        let _ = tenant; // pacify if unused
        for id in ["evt_a", "evt_b"] {
            billing_dlq_query::upsert_dead_letter(
                &store, id, "customer.subscription.updated",
                Some(tenant), &payload(tenant, id), now, "x",
            ).await.expect("seed");
        }
        let results =
            replay_since(&store, now - chrono::Duration::hours(1), 100, now)
                .await
                .expect("replay window");
        assert_eq!(results.len(), 2);
        for (_, outcome) in results {
            assert!(matches!(outcome, ReplayOutcome::Replayed { .. }));
        }
    }
}
