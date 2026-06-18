//! Tenant-wide APPROVAL RATE read-side over the durable audit chain.
//!
//! ## The broken metric this closes (2026-06-14 25x-soak finding #2)
//!
//! The `/approvals` APPROVAL RATE KPI used to derive the rate from the
//! CURRENT pending list (`computeApprovalRate(items)` web-side). But a
//! resolved hold/approval DROPS OUT of the `/approvals` list (the
//! probation read-side filters resolved `(session_id, tool)` keys; the
//! memory/seeded arms surface only pending). So on any fresh load
//! `decided.length === 0` → the KPI reset to "—" even after the operator
//! had approved many actions. The metric was structurally unable to
//! reflect history because its source was the EPHEMERAL pending queue.
//!
//! ## No parallel store — read-side union over the audit chain
//!
//! Like the seeded-approval + probation-hold bridges
//! ([`super::routes_approvals_json_seeded`] /
//! [`super::routes_approvals_probation`]), this is purely a read-side
//! count: the rate is sourced from the hash-chained `audit_record` rows
//! the system already writes on every decide — `approval.granted` (the
//! operator approved) vs `approval.declined` (refused) — the honest
//! source, never the pending list. Tenant fail-closed: another tenant's
//! rows are never counted.
//!
//! ## Counting convention — reused, not reinvented
//!
//! Mirrors the per-worker `approval_rate` in
//! `crate::handlers::worker_metrics` (`granted / (granted + <other>)`):
//! the denominator is the count of resolved decisions, the numerator the
//! granted subset. There is no third definition.

use argo_store::SessionStore;
use serde_json::{json, Value};
use std::sync::Arc;

/// A tenant-wide approval-rate summary derived from the audit chain.
struct ApprovalRateSummary {
    granted: u64,
    declined: u64,
}

impl ApprovalRateSummary {
    /// Total resolved decisions (granted + declined).
    fn total(&self) -> u64 {
        self.granted + self.declined
    }

    /// Shape the summary as the `approval_rate` envelope object, matching
    /// the SPA `ApprovalRateSummary` TS type (snake_case server fields).
    /// `None` when no decisions have landed — divide-by-zero safe — so the
    /// caller omits the field entirely and the web layer keeps its
    /// back-compat client compute.
    fn to_json(&self) -> Option<Value> {
        let total = self.total();
        if total == 0 {
            return None;
        }
        let denom = total as f64;
        Some(json!({
            "total": total,
            "approved_fraction": self.granted as f64 / denom,
            "refused_fraction": self.declined as f64 / denom,
            // No durable "expired" audit kind feeds this metric; the slice
            // stays 0 so the ring's three slices still sum to 1.
            "expired_fraction": 0.0,
        }))
    }
}

/// Compute the durable tenant-wide approval rate for the request's tenant,
/// shaped as the `approval_rate` envelope object. Returns `None` on any
/// tenant-resolver / read miss OR when no decisions have landed yet —
/// never an error from the caller's view — because the field is additive
/// and the web layer falls back to its client compute when it is absent.
pub(super) async fn approval_rate_for_request(
    req: &str,
    store: &Arc<SessionStore>,
) -> Option<Value> {
    let tenant = super::resolve_tenant::resolve_tenant_for_request(req, store)
        .await
        .ok()?;
    let summary = count_decisions(store.pool(), &tenant.to_string())
        .await
        .ok()?;
    summary.to_json()
}

/// Count `approval.granted` vs `approval.declined` audit rows for the
/// tenant. Tenant fail-closed — the `tenant_id = ?` predicate never
/// counts another tenant's decisions.
async fn count_decisions(
    pool: &sqlx::SqlitePool,
    tenant_id_text: &str,
) -> Result<ApprovalRateSummary, sqlx::Error> {
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT kind, COUNT(*) FROM audit_record \
         WHERE kind IN ('approval.granted', 'approval.declined') AND tenant_id = ? \
         GROUP BY kind",
    )
    .bind(tenant_id_text)
    .fetch_all(pool)
    .await?;
    let mut granted = 0u64;
    let mut declined = 0u64;
    for (kind, count) in rows {
        let count = count.max(0) as u64;
        match kind.as_str() {
            "approval.granted" => granted = count,
            "approval.declined" => declined = count,
            _ => {}
        }
    }
    Ok(ApprovalRateSummary { granted, declined })
}

#[cfg(test)]
#[path = "routes_approvals_rate_tests.rs"]
mod tests;
