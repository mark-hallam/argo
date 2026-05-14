//! Stream `claude-worker-promotion-workflow` (mission claim 2).
//!
//! Walks workers up the `entry → junior → senior → lead` tenure
//! ladder once `sessions / success-rate / tenure-days` thresholds are
//! cleared. Drives two CLI verbs:
//!
//! - `argo worker promote <id> [--to <level>] [--force] [--reason <t>]`
//! - `argo worker level <id>` — current level + numeric gap to next.
//!
//! Promotion writes a single string-literal `worker_promotion` audit
//! row tying old level, new level, the snapshot metrics, and the
//! optional operator reason / force flag into the chain (mirrors the
//! `worker_handoff` audit pattern from
//! `claude-worker-handoff-record`). Auto-approval downstream
//! consumption (riskier policies behind seniority) is a follow-up.

use crate::home::{db_path, ensure_dir, resolve_home};
use argo_identity::{IdentityStore, WorkerId, WorkerLevel};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::Utc;
use serde::Serialize;
use sqlx::{Pool, Sqlite};
use std::str::FromStr;
use uuid::Uuid;

/// Stable kind label for the promotion audit row. Not in the typed
/// `argo_audit::AuditKind` enum yet (handoff and other worker-
/// lifecycle events follow the same string-literal pattern today).
pub const WORKER_PROMOTION_KIND: &str = "worker_promotion";

// Promotion thresholds — declared as `pub const` so a follow-up brief
// can env-override them without re-exporting these symbols.
pub const ENTRY_TO_JUNIOR_SESSIONS: u64 = 25;
pub const ENTRY_TO_JUNIOR_SUCCESS_RATE: f64 = 0.80;
pub const ENTRY_TO_JUNIOR_TENURE_DAYS: i64 = 14;

pub const JUNIOR_TO_SENIOR_SESSIONS: u64 = 100;
pub const JUNIOR_TO_SENIOR_SUCCESS_RATE: f64 = 0.90;
pub const JUNIOR_TO_SENIOR_TENURE_DAYS: i64 = 60;

pub const SENIOR_TO_LEAD_SESSIONS: u64 = 500;
pub const SENIOR_TO_LEAD_SUCCESS_RATE: f64 = 0.92;
pub const SENIOR_TO_LEAD_TENURE_DAYS: i64 = 180;

/// Numeric thresholds the worker must clear to land at `target`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PromotionCriteria {
    pub sessions: u64,
    pub success_rate: f64,
    pub tenure_days: i64,
    /// `true` for the `senior → lead` transition: operator must
    /// explicitly pass `--force` even when the numeric thresholds
    /// are cleared.
    pub requires_force: bool,
}

/// Resolve the criteria gate that applies when promoting **into**
/// `target`. Returns `None` for `Entry` (no one promotes "into entry").
#[must_use]
pub fn criteria_for(target: WorkerLevel) -> Option<PromotionCriteria> {
    match target {
        WorkerLevel::Entry => None,
        WorkerLevel::Junior => Some(PromotionCriteria {
            sessions: ENTRY_TO_JUNIOR_SESSIONS,
            success_rate: ENTRY_TO_JUNIOR_SUCCESS_RATE,
            tenure_days: ENTRY_TO_JUNIOR_TENURE_DAYS,
            requires_force: false,
        }),
        WorkerLevel::Senior => Some(PromotionCriteria {
            sessions: JUNIOR_TO_SENIOR_SESSIONS,
            success_rate: JUNIOR_TO_SENIOR_SUCCESS_RATE,
            tenure_days: JUNIOR_TO_SENIOR_TENURE_DAYS,
            requires_force: false,
        }),
        WorkerLevel::Lead => Some(PromotionCriteria {
            sessions: SENIOR_TO_LEAD_SESSIONS,
            success_rate: SENIOR_TO_LEAD_SUCCESS_RATE,
            tenure_days: SENIOR_TO_LEAD_TENURE_DAYS,
            requires_force: true,
        }),
    }
}

/// Snapshot of the metrics consulted at gate time. Materialised into
/// the audit payload so future `argo audit show` rows reproduce the
/// promotion decision.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct PromotionMetrics {
    pub sessions: u64,
    pub granted: u64,
    pub revoked: u64,
    pub success_rate: Option<f64>,
    pub tenure_days: i64,
}

/// Per-axis shortfall report. Negative `*_short` values mean the
/// threshold is already met (overshoot is fine). `all_met` is `true`
/// only when every axis is non-positive-short.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct PromotionGap {
    pub sessions_short: i64,
    pub success_rate_short: f64,
    pub tenure_days_short: i64,
    pub all_met: bool,
}

/// Compute the per-axis gap between observed metrics and the gate
/// criteria. Pure function; trivially testable.
#[must_use]
pub fn evaluate_gap(metrics: &PromotionMetrics, criteria: &PromotionCriteria) -> PromotionGap {
    let sessions_short = i64::try_from(criteria.sessions).unwrap_or(i64::MAX)
        - i64::try_from(metrics.sessions).unwrap_or(i64::MAX);
    let observed_rate = metrics.success_rate.unwrap_or(0.0);
    let success_rate_short = criteria.success_rate - observed_rate;
    let tenure_days_short = criteria.tenure_days - metrics.tenure_days;
    let all_met = sessions_short <= 0 && success_rate_short <= 0.0 && tenure_days_short <= 0;
    PromotionGap {
        sessions_short,
        success_rate_short,
        tenure_days_short,
        all_met,
    }
}

/// Render a one-line operator-readable shortfall summary mirroring
/// the brief: `"23 sessions short, 4d tenure short"`.
#[must_use]
pub fn format_gap(gap: &PromotionGap) -> String {
    let mut parts: Vec<String> = Vec::new();
    if gap.sessions_short > 0 {
        parts.push(format!("{} sessions short", gap.sessions_short));
    }
    if gap.success_rate_short > 0.0 {
        parts.push(format!(
            "{:.1}pp success-rate short",
            gap.success_rate_short * 100.0
        ));
    }
    if gap.tenure_days_short > 0 {
        parts.push(format!("{}d tenure short", gap.tenure_days_short));
    }
    if parts.is_empty() {
        "criteria met".to_string()
    } else {
        parts.join(", ")
    }
}

// ─────────────────────────── CLI entry ───────────────────────────

/// `argo worker promote <id> [--tenant <t>] [--to <level>] [--force]
/// [--reason <text>] [--json]`.
pub async fn promote(
    worker_id: &str,
    tenant: &str,
    to: Option<&str>,
    force: bool,
    reason: Option<&str>,
    json: bool,
) -> ArgoResult<()> {
    let tenant_id = parse_tenant(tenant)?;
    let parsed_worker = parse_worker(worker_id)?;
    let owned = open_store().await?;
    let worker = owned
        .identity()
        .get_worker(tenant_id, parsed_worker)
        .await?
        .ok_or_else(|| {
            ArgoError::Provider(format!(
                "worker {worker_id} not found in tenant {tenant}"
            ))
        })?;
    let target = match to {
        Some(label) => WorkerLevel::from_str(label).map_err(ArgoError::Provider)?,
        None => worker.level.next().ok_or_else(|| {
            ArgoError::Provider(format!(
                "worker is already at the top level ({})",
                worker.level
            ))
        })?,
    };
    if target.rank() <= worker.level.rank() {
        return Err(ArgoError::Provider(format!(
            "promotion target {} is not above current level {}",
            target, worker.level
        )));
    }
    if target.rank() != worker.level.rank() + 1 {
        return Err(ArgoError::Provider(format!(
            "skip-level promotion {} → {} is not allowed (promote one rung at a time)",
            worker.level, target
        )));
    }
    let criteria = criteria_for(target).ok_or_else(|| {
        ArgoError::Provider(format!("no criteria gate defined for {target}"))
    })?;
    if criteria.requires_force && !force {
        return Err(ArgoError::Provider(format!(
            "promotion to {target} requires --force (operator confirmation)"
        )));
    }
    let created_at = worker_created_at_micros(owned.pool(), parsed_worker).await?;
    let metrics = load_metrics(owned.pool(), parsed_worker, created_at).await?;
    let gap = evaluate_gap(&metrics, &criteria);
    if !gap.all_met && !force {
        return Err(ArgoError::Provider(format!(
            "{worker_id} not eligible for {target}: {}",
            format_gap(&gap)
        )));
    }
    owned
        .identity()
        .update_level(tenant_id, parsed_worker, target)
        .await?;
    write_audit_row(
        owned.pool(),
        parsed_worker,
        worker.level,
        target,
        &metrics,
        &gap,
        reason,
        force,
    )
    .await?;
    print_promote_outcome(worker_id, worker.level, target, &metrics, &gap, force, json);
    Ok(())
}

/// `argo worker level <id> [--tenant <t>] [--json]` — current level
/// plus numeric gap to next.
pub async fn level(worker_id: &str, tenant: &str, json: bool) -> ArgoResult<()> {
    let tenant_id = parse_tenant(tenant)?;
    let parsed_worker = parse_worker(worker_id)?;
    let owned = open_store().await?;
    let worker = owned
        .identity()
        .get_worker(tenant_id, parsed_worker)
        .await?
        .ok_or_else(|| {
            ArgoError::Provider(format!(
                "worker {worker_id} not found in tenant {tenant}"
            ))
        })?;
    let next = worker.level.next();
    let created_at = worker_created_at_micros(owned.pool(), parsed_worker).await?;
    let metrics = load_metrics(owned.pool(), parsed_worker, created_at).await?;
    let gap_info = match next {
        Some(target) => criteria_for(target).map(|c| (target, c, evaluate_gap(&metrics, &c))),
        None => None,
    };
    if json {
        let payload = serde_json::json!({
            "ok": true,
            "worker_id": worker_id,
            "level": worker.level.as_str(),
            "next_level": next.map(|n| n.as_str()),
            "metrics": metrics,
            "gap": gap_info.map(|(_, _, g)| g),
            "summary": gap_info.as_ref()
                .map(|(_, _, g)| format_gap(g))
                .unwrap_or_else(|| "at top level".to_string()),
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".into())
        );
    } else {
        println!("worker {worker_id}: level={}", worker.level);
        match gap_info {
            Some((target, _, g)) => println!("  → next ({target}): {}", format_gap(&g)),
            None => println!("  → at top level"),
        }
    }
    Ok(())
}

// ─────────────────────────── helpers ─────────────────────────────

fn parse_tenant(tenant: &str) -> ArgoResult<TenantId> {
    Uuid::parse_str(tenant)
        .map(TenantId)
        .map_err(|e| ArgoError::Schema(format!("invalid tenant id '{tenant}': {e}")))
}

fn parse_worker(worker_id: &str) -> ArgoResult<WorkerId> {
    WorkerId::from_str(worker_id)
        .map_err(|e| ArgoError::Schema(format!("invalid worker id '{worker_id}': {e}")))
}

async fn open_store() -> ArgoResult<OwnedIdentityStore> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(db_path(&home)).await?;
    store.migrate().await?;
    Ok(OwnedIdentityStore { store })
}

/// Owns the `SessionStore` for the duration of one CLI invocation so
/// the borrowing `IdentityStore` stays valid. Mirrors the wrapper in
/// `worker_charter.rs`.
pub(crate) struct OwnedIdentityStore {
    store: SessionStore,
}

impl OwnedIdentityStore {
    pub(crate) fn identity(&self) -> IdentityStore<'_> {
        IdentityStore::new(&self.store)
    }

    pub(crate) fn pool(&self) -> &Pool<Sqlite> {
        self.store.pool()
    }
}

async fn worker_created_at_micros(
    pool: &Pool<Sqlite>,
    worker_id: WorkerId,
) -> ArgoResult<i64> {
    let raw: Option<i64> = sqlx::query_scalar(
        "SELECT created_at FROM worker WHERE id = ?",
    )
    .bind(worker_id.to_string())
    .fetch_optional(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("worker created_at: {e}")))?;
    raw.ok_or_else(|| ArgoError::Provider(format!("worker {worker_id} missing created_at")))
}

pub(crate) async fn load_metrics(
    pool: &Pool<Sqlite>,
    worker_id: WorkerId,
    created_at_micros: i64,
) -> ArgoResult<PromotionMetrics> {
    let worker_blob = worker_id.0.as_bytes().to_vec();
    let sessions: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM session WHERE worker_id = ?",
    )
    .bind(worker_blob.as_slice())
    .fetch_one(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("session count: {e}")))?;
    let granted: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_record a JOIN session s ON s.id = a.session_id \
         WHERE s.worker_id = ? AND a.kind = 'approval.granted'",
    )
    .bind(worker_blob.as_slice())
    .fetch_one(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("granted count: {e}")))?;
    let revoked: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_record a JOIN session s ON s.id = a.session_id \
         WHERE s.worker_id = ? AND a.kind = 'approval.revoked'",
    )
    .bind(worker_blob.as_slice())
    .fetch_one(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("revoked count: {e}")))?;
    let denom = granted + revoked;
    let success_rate = if denom > 0 {
        #[allow(clippy::cast_precision_loss)]
        Some(granted as f64 / denom as f64)
    } else {
        None
    };
    let now_micros = Utc::now().timestamp_micros();
    let tenure_days = (now_micros - created_at_micros) / 86_400_000_000;
    Ok(PromotionMetrics {
        sessions: u64::try_from(sessions).unwrap_or(0),
        granted: u64::try_from(granted).unwrap_or(0),
        revoked: u64::try_from(revoked).unwrap_or(0),
        success_rate,
        tenure_days,
    })
}

#[allow(clippy::too_many_arguments)]
async fn write_audit_row(
    pool: &Pool<Sqlite>,
    worker_id: WorkerId,
    from_level: WorkerLevel,
    to_level: WorkerLevel,
    metrics: &PromotionMetrics,
    gap: &PromotionGap,
    reason: Option<&str>,
    force: bool,
) -> ArgoResult<()> {
    let payload = serde_json::json!({
        "worker_id": worker_id.to_string(),
        "from_level": from_level.as_str(),
        "to_level": to_level.as_str(),
        "metrics": metrics,
        "gap": gap,
        "reason": reason,
        "forced": force,
        "force_reason": if force { reason } else { None },
    });
    let canonical = serde_json::to_string(&payload)
        .map_err(|e| ArgoError::Schema(format!("worker_promotion payload: {e}")))?;
    let parent_hash = [0u8; 32];
    let self_hash =
        argo_audit::compute_chain_hash(&parent_hash, WORKER_PROMOTION_KIND, &canonical);
    let id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO audit_record (id, session_id, kind, payload_json, \
         parent_hash, self_hash, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id.as_bytes().as_slice())
    .bind(Option::<&[u8]>::None)
    .bind(WORKER_PROMOTION_KIND)
    .bind(canonical)
    .bind(parent_hash.as_slice())
    .bind(self_hash.as_slice())
    .bind(Utc::now().timestamp_micros())
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("audit insert: {e}")))?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn print_promote_outcome(
    worker_id: &str,
    from_level: WorkerLevel,
    to_level: WorkerLevel,
    metrics: &PromotionMetrics,
    gap: &PromotionGap,
    force: bool,
    json: bool,
) {
    if json {
        let payload = serde_json::json!({
            "ok": true,
            "worker_id": worker_id,
            "from_level": from_level.as_str(),
            "to_level": to_level.as_str(),
            "forced": force,
            "metrics": metrics,
            "gap": gap,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".into())
        );
    } else {
        let suffix = if force { " (forced)" } else { "" };
        println!("worker {worker_id}: {from_level} → {to_level}{suffix}");
        println!(
            "  sessions={} success_rate={} tenure_days={}",
            metrics.sessions,
            match metrics.success_rate {
                Some(r) => format!("{:.2}%", r * 100.0),
                None => "n/a".to_string(),
            },
            metrics.tenure_days,
        );
    }
}

#[cfg(test)]
#[path = "worker_promote_tests.rs"]
mod tests;
