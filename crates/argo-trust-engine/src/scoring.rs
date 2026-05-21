//! Signal aggregation. Read-only against `audit_record` and
//! `trust_observation`; no writes here.
//!
//! All queries filter by `tenant_id` AND the worker (either by the
//! denorm `worker_id` column on `audit_record` or by joining on the
//! source-tenant column of `trust_observation`). The cross-tenant
//! isolation guarantee depends on both filters — see
//! `tests/cross_tenant_isolation.rs` for the negative test.

use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

/// +0.5 per consecutive granted approval in the longest streak.
pub const WEIGHT_APPROVAL_STREAK: f32 = 0.5;
/// +1.0 per granted approval in the last 30 days.
pub const WEIGHT_APPROVAL_GRANTED: f32 = 1.0;
/// −2.0 per rejected approval reply in the last 30 days.
pub const WEIGHT_APPROVAL_REJECTED: f32 = -2.0;
/// −1.0 per escalation-required event in the last 30 days.
pub const WEIGHT_ESCALATION: f32 = -1.0;
/// −0.5 per RBAC-deny event in the last 30 days.
pub const WEIGHT_RBAC_DENY: f32 = -0.5;
/// +5.0 × mean cross-tenant quality observation in the last 90 days.
pub const WEIGHT_QUALITY_AVG: f32 = 5.0;

/// 30-day window for action-side signals.
pub const WINDOW_DAYS_ACTIONS: i64 = 30;
/// 90-day window for cross-tenant quality observations.
pub const WINDOW_DAYS_QUALITY: i64 = 90;

/// Per-signal contributions plus the total. Stored verbatim in the
/// `TrustGradeChanged` audit payload so an operator can answer
/// "why did this grade move?" without re-running the query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoreBreakdown {
    /// Worker the breakdown belongs to.
    pub worker_id: String,
    /// Longest consecutive `approval.granted` streak × [`WEIGHT_APPROVAL_STREAK`].
    pub streak_score: f32,
    /// Granted-count × [`WEIGHT_APPROVAL_GRANTED`] + rejected-count × [`WEIGHT_APPROVAL_REJECTED`].
    pub approval_score: f32,
    /// Escalation-count × [`WEIGHT_ESCALATION`].
    pub escalation_score: f32,
    /// RBAC-deny-count × [`WEIGHT_RBAC_DENY`].
    pub rbac_score: f32,
    /// Mean cross-tenant quality × [`WEIGHT_QUALITY_AVG`].
    pub quality_score: f32,
    /// Sum of all five subscores.
    pub total: f32,
    /// UTC wall-clock at which this score was computed.
    pub computed_at: DateTime<Utc>,
}

/// Compute one worker's score across the 5 signal streams. Pure
/// function over the store — no side effects.
///
/// # Errors
/// Returns [`ArgoError::Provider`] when any sqlx query fails.
pub async fn compute_score(
    store: &SessionStore,
    tenant: &TenantId,
    worker_id: &str,
) -> ArgoResult<ScoreBreakdown> {
    let now = Utc::now();
    let action_cutoff = micros(now - Duration::days(WINDOW_DAYS_ACTIONS));
    let quality_cutoff = micros(now - Duration::days(WINDOW_DAYS_QUALITY));

    let streak = longest_streak(store, tenant, worker_id).await?;
    let (granted, rejected) =
        approval_counts(store, tenant, worker_id, action_cutoff).await?;
    let escalations = escalation_count(store, tenant, worker_id, action_cutoff).await?;
    let rbac_denies = rbac_deny_count(store, tenant, worker_id, action_cutoff).await?;
    let quality_avg = cross_tenant_quality_avg(store, tenant, quality_cutoff).await?;

    let streak_score = streak as f32 * WEIGHT_APPROVAL_STREAK;
    let approval_score = granted as f32 * WEIGHT_APPROVAL_GRANTED
        + rejected as f32 * WEIGHT_APPROVAL_REJECTED;
    let escalation_score = escalations as f32 * WEIGHT_ESCALATION;
    let rbac_score = rbac_denies as f32 * WEIGHT_RBAC_DENY;
    let quality_score = quality_avg * WEIGHT_QUALITY_AVG;
    let total = streak_score + approval_score + escalation_score + rbac_score + quality_score;

    Ok(ScoreBreakdown {
        worker_id: worker_id.to_string(),
        streak_score,
        approval_score,
        escalation_score,
        rbac_score,
        quality_score,
        total,
        computed_at: now,
    })
}

fn micros(ts: DateTime<Utc>) -> i64 {
    ts.timestamp_micros()
}

/// Length of the longest run of consecutive `approval.granted`
/// rows for this worker, scoped to the tenant. A `revoke` or a
/// `reply` with `reply_kind == "reject"` resets the active run.
async fn longest_streak(
    store: &SessionStore,
    tenant: &TenantId,
    worker_id: &str,
) -> ArgoResult<i64> {
    let rows: Vec<(String, Option<String>)> = sqlx::query_as(
        "SELECT kind, payload_json FROM audit_record \
         WHERE tenant_id = ? AND worker_id = ? \
           AND kind IN ('approval.granted', 'approval.revoked', 'approval.replied') \
         ORDER BY created_at ASC, seq ASC",
    )
    .bind(tenant.to_string())
    .bind(worker_id)
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("trust_engine streak query: {e}")))?;

    let mut current: i64 = 0;
    let mut longest: i64 = 0;
    for (kind, payload_json) in rows {
        match kind.as_str() {
            "approval.granted" => {
                current = current.saturating_add(1);
                if current > longest {
                    longest = current;
                }
            }
            "approval.revoked" => current = 0,
            "approval.replied" => {
                let reply_kind = payload_json
                    .as_deref()
                    .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
                    .and_then(|v| v.get("reply_kind").and_then(|x| x.as_str()).map(String::from))
                    .unwrap_or_default();
                if reply_kind == "reject" || reply_kind == "deny" {
                    current = 0;
                }
            }
            _ => {}
        }
    }
    Ok(longest)
}

/// Returns `(granted_count, rejected_count)` over the action
/// window. `rejected` counts `approval.replied` rows whose
/// `payload_json.reply_kind` is `reject` or `deny`.
async fn approval_counts(
    store: &SessionStore,
    tenant: &TenantId,
    worker_id: &str,
    cutoff_micros: i64,
) -> ArgoResult<(i64, i64)> {
    let granted: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_record \
         WHERE tenant_id = ? AND worker_id = ? \
           AND kind = 'approval.granted' AND created_at >= ?",
    )
    .bind(tenant.to_string())
    .bind(worker_id)
    .bind(cutoff_micros)
    .fetch_one(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("trust_engine granted query: {e}")))?;

    let rejected_rows: Vec<Option<String>> = sqlx::query_scalar(
        "SELECT payload_json FROM audit_record \
         WHERE tenant_id = ? AND worker_id = ? \
           AND kind = 'approval.replied' AND created_at >= ?",
    )
    .bind(tenant.to_string())
    .bind(worker_id)
    .bind(cutoff_micros)
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("trust_engine replied query: {e}")))?;

    let rejected = rejected_rows
        .into_iter()
        .flatten()
        .filter(|payload| {
            serde_json::from_str::<serde_json::Value>(payload)
                .ok()
                .and_then(|v| v.get("reply_kind").and_then(|x| x.as_str()).map(String::from))
                .is_some_and(|s| s == "reject" || s == "deny")
        })
        .count() as i64;

    Ok((granted, rejected))
}

async fn escalation_count(
    store: &SessionStore,
    tenant: &TenantId,
    worker_id: &str,
    cutoff_micros: i64,
) -> ArgoResult<i64> {
    let n: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_record \
         WHERE tenant_id = ? AND worker_id = ? \
           AND kind = 'escalation.required' AND created_at >= ?",
    )
    .bind(tenant.to_string())
    .bind(worker_id)
    .bind(cutoff_micros)
    .fetch_one(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("trust_engine escalation query: {e}")))?;
    Ok(n)
}

async fn rbac_deny_count(
    store: &SessionStore,
    tenant: &TenantId,
    worker_id: &str,
    cutoff_micros: i64,
) -> ArgoResult<i64> {
    let n: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_record \
         WHERE tenant_id = ? AND worker_id = ? \
           AND kind = 'rbac.deny' AND created_at >= ?",
    )
    .bind(tenant.to_string())
    .bind(worker_id)
    .bind(cutoff_micros)
    .fetch_one(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("trust_engine rbac.deny query: {e}")))?;
    Ok(n)
}

/// Mean `trust_observation.quality_score` over the quality window
/// where this tenant is the SOURCE (i.e., observations OTHER tenants
/// made about THIS tenant's skills). Returns `0.0` when the row
/// set is empty — the column types in the store schema are
/// nullable on aggregate, hence the `Option` plumbing.
async fn cross_tenant_quality_avg(
    store: &SessionStore,
    tenant: &TenantId,
    cutoff_micros: i64,
) -> ArgoResult<f32> {
    let tenant_blob = tenant.0.as_bytes().to_vec();
    let avg: Option<f32> = sqlx::query_scalar(
        "SELECT AVG(quality_score) FROM trust_observation \
         WHERE source_tenant_id = ? AND observed_at >= ?",
    )
    .bind(tenant_blob)
    .bind(cutoff_micros)
    .fetch_one(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("trust_engine quality avg query: {e}")))?;
    Ok(avg.unwrap_or(0.0).clamp(0.0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_audit::{AuditKind, AuditSink};
    use argo_identity::{IdentityStore, Persona, Role, Worker, WorkerId};
    use serde_json::json;
    use std::sync::Arc;

    async fn fresh_store_with_tenant() -> (Arc<SessionStore>, TenantId, WorkerId) {
        let store = SessionStore::open(":memory:").await.unwrap();
        store.migrate().await.unwrap();
        let store = Arc::new(store);
        let tenant = TenantId::new();
        let identities = IdentityStore::new(&store);
        let worker = Worker::draft(
            tenant,
            "vert",
            Persona::named("Tester", "op"),
            Role::new("Run trust-engine tests."),
        );
        identities.create_worker(&worker).await.unwrap();
        (store, tenant, worker.id)
    }

    async fn emit(
        sink: &AuditSink,
        tenant: &TenantId,
        kind: AuditKind,
        worker_id: &WorkerId,
        extra: serde_json::Value,
    ) {
        let mut payload = json!({
            "worker_id": worker_id.to_string(),
        });
        if let Some(obj) = extra.as_object() {
            for (k, v) in obj {
                payload[k] = v.clone();
            }
        }
        sink.write_with_tenant(None, kind, payload, None, &tenant.to_string())
            .await
            .unwrap();
        sink.flush_pending().await.unwrap();
    }

    #[tokio::test]
    async fn empty_signals_returns_zero() {
        let (store, tenant, worker_id) = fresh_store_with_tenant().await;
        let s = compute_score(&store, &tenant, &worker_id.to_string())
            .await
            .unwrap();
        assert_eq!(s.total, 0.0);
        assert_eq!(s.streak_score, 0.0);
        assert_eq!(s.approval_score, 0.0);
        assert_eq!(s.escalation_score, 0.0);
        assert_eq!(s.rbac_score, 0.0);
        assert_eq!(s.quality_score, 0.0);
    }

    #[tokio::test]
    async fn three_approvals_gives_positive() {
        let (store, tenant, worker_id) = fresh_store_with_tenant().await;
        let sink = AuditSink::new_with_tenant(Arc::clone(&store), tenant.to_string());
        for _ in 0..3 {
            emit(&sink, &tenant, AuditKind::ApprovalGranted, &worker_id, json!({})).await;
        }
        let s = compute_score(&store, &tenant, &worker_id.to_string())
            .await
            .unwrap();
        // 3 grants * 1.0 (approval) + 3 (longest streak) * 0.5 = 4.5
        assert!(s.total > 0.0, "expected positive score, got {s:?}");
        assert_eq!(s.approval_score, 3.0);
        assert_eq!(s.streak_score, 1.5);
    }

    #[tokio::test]
    async fn one_rejection_overwhelms_streak() {
        let (store, tenant, worker_id) = fresh_store_with_tenant().await;
        let sink = AuditSink::new_with_tenant(Arc::clone(&store), tenant.to_string());
        emit(&sink, &tenant, AuditKind::ApprovalGranted, &worker_id, json!({})).await;
        emit(
            &sink,
            &tenant,
            AuditKind::ApprovalReplied,
            &worker_id,
            json!({ "reply_kind": "reject" }),
        )
        .await;
        let s = compute_score(&store, &tenant, &worker_id.to_string())
            .await
            .unwrap();
        // 1 grant (+1.0) + 1 streak (+0.5) − 1 reject (×−2.0) = −0.5
        assert!(s.approval_score <= -1.0, "got {s:?}");
        assert!(s.total < 0.0, "expected negative total, got {}", s.total);
    }

    #[tokio::test]
    async fn filters_old_events_outside_window() {
        let (store, tenant, worker_id) = fresh_store_with_tenant().await;
        let sink = AuditSink::new_with_tenant(Arc::clone(&store), tenant.to_string());
        // Within-window grant.
        emit(&sink, &tenant, AuditKind::ApprovalGranted, &worker_id, json!({})).await;
        // Direct-insert a row that's 100 days old (well outside the
        // 30-day action window). The sink can't backdate, so do
        // this with raw sql.
        let ancient = (Utc::now() - Duration::days(100)).timestamp_micros();
        sqlx::query(
            "INSERT INTO audit_record \
             (id, kind, payload_json, parent_hash, self_hash, created_at, tenant_id, worker_id) \
             VALUES (?, 'approval.granted', '{}', ?, ?, ?, ?, ?)",
        )
        .bind(uuid::Uuid::now_v7().as_bytes().as_slice())
        .bind(&[0_u8; 32][..])
        .bind(&[0_u8; 32][..])
        .bind(ancient)
        .bind(tenant.to_string())
        .bind(worker_id.to_string())
        .execute(store.pool())
        .await
        .unwrap();
        let s = compute_score(&store, &tenant, &worker_id.to_string())
            .await
            .unwrap();
        // Window-filtered counts use only the 1 recent grant.
        assert_eq!(s.approval_score, 1.0);
    }

    #[tokio::test]
    async fn cross_tenant_quality_weights_into_total() {
        let (store, tenant, worker_id) = fresh_store_with_tenant().await;
        let now_micros = Utc::now().timestamp_micros();
        let other_tenant_blob = TenantId::new().0.as_bytes().to_vec();
        let skill_blob = uuid::Uuid::now_v7().as_bytes().to_vec();
        // Two observations from this tenant's skills at quality 0.8 each.
        for _ in 0..2 {
            let id_blob = uuid::Uuid::now_v7().as_bytes().to_vec();
            sqlx::query(
                "INSERT INTO trust_observation \
                 (id, source_tenant_id, consumer_tenant_id, skill_id, quality_score, observed_at) \
                 VALUES (?, ?, ?, ?, ?, ?)",
            )
            .bind(id_blob)
            .bind(tenant.0.as_bytes().to_vec())
            .bind(other_tenant_blob.clone())
            .bind(skill_blob.clone())
            .bind(0.8_f32)
            .bind(now_micros)
            .execute(store.pool())
            .await
            .unwrap();
        }
        let s = compute_score(&store, &tenant, &worker_id.to_string())
            .await
            .unwrap();
        // avg = 0.8, weight = 5.0, expected quality_score = 4.0.
        assert!((s.quality_score - 4.0).abs() < 0.01, "got {s:?}");
        assert!(s.total >= 4.0, "expected total carrying quality, got {}", s.total);
    }
}
