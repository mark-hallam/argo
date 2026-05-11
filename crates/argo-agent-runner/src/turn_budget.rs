//! Per-turn hook into the subtask budget tree (Stream
//! `mission-delegation-budget-tree`).
//!
//! Two seams into the agent loop:
//!
//!   1. [`check_pre_turn_budget`] — runs before the first round of
//!      `run_turn_with_tools`. Finds the currently-running subtask
//!      (if any) for the session and aborts the turn cleanly when
//!      its remaining budget is `<= 0`. NULL / legacy budgets land
//!      on the unbounded path and never abort.
//!
//!   2. [`record_subtask_turn_spend`] — runs immediately after the
//!      existing `record_turn_spend` per-round call. Charges the
//!      same `(tokens × price)` math (converted from USD micros to
//!      USD cents) against the running subtask. Silent when no
//!      running subtask is bound to the session — preserving the
//!      legacy single-session-no-subtask hot path.
//!
//! Both helpers are deliberately tolerant of "no running subtask":
//! a session can outlive its decomposition, and legacy sessions
//! that never ran through the Planner have nothing in
//! `subtask_ledger` at all. In those cases the helpers no-op so
//! existing turn callers keep working unchanged.
use argo_audit::{AuditKind, AuditSink};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, BudgetKind, SessionId};
use sqlx::Row;
use std::sync::Arc;

use crate::helpers::SpendCtx;

/// Resolve the running subtask attached to `session_id`. Returns
/// the most-recently-started running row, or `None` when no row
/// is in the `running` lifecycle state.
async fn running_subtask_id(
    store: &Arc<SessionStore>,
    session_id: SessionId,
) -> ArgoResult<Option<String>> {
    let sid = session_id.0.to_string();
    let row = sqlx::query(
        "SELECT id FROM subtask_ledger \
         WHERE parent_session_id = ? AND state = 'running' \
         ORDER BY started_at DESC LIMIT 1",
    )
    .bind(&sid)
    .fetch_optional(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("running_subtask_id: {e}")))?;
    Ok(row.map(|r| r.get::<String, _>("id")))
}

/// Pre-turn budget gate. Surfaces an `ArgoError::BudgetExhausted`
/// and writes an audit row when the running subtask's
/// `get_remaining` returns `Some(x)` with `x <= 0`. No-op for the
/// unbounded or no-running-subtask paths.
pub async fn check_pre_turn_budget(
    store: &Arc<SessionStore>,
    session_id: SessionId,
    audit: &AuditSink,
) -> ArgoResult<()> {
    let Some(id) = running_subtask_id(store, session_id).await? else {
        return Ok(());
    };
    let remaining = argo_store::subtask_budget::get_remaining(store.pool(), &id).await?;
    if let Some(r) = remaining {
        if r <= 0 {
            let payload = serde_json::json!({
                "subtask_id": id,
                "remaining_usd_cents": r,
                "reason": "subtask_budget_exceeded",
            });
            let _ = audit
                .write(Some(session_id), AuditKind::Halt, payload, None)
                .await;
            return Err(ArgoError::BudgetExhausted {
                kind: BudgetKind::SpendUsdMicros,
            });
        }
    }
    Ok(())
}

/// Post-round subtask charge. Converts the same
/// `(tokens × price_per_1k)` math used by
/// [`crate::record_turn_spend`] into USD cents, then accumulates
/// onto the running subtask's `spent_usd_cents` column. Silent
/// when pricing or token counts are missing, or when no running
/// subtask is bound to the session.
pub async fn record_subtask_turn_spend(
    store: &Arc<SessionStore>,
    session_id: SessionId,
    spend: &SpendCtx<'_>,
    tokens_in: Option<u32>,
    tokens_out: Option<u32>,
) -> ArgoResult<()> {
    let (Some((p_in, p_out)), Some(ti), Some(to)) = (spend.pricing, tokens_in, tokens_out) else {
        return Ok(());
    };
    let cents = compute_cents(ti, to, p_in, p_out);
    if cents == 0 {
        return Ok(());
    }
    let Some(id) = running_subtask_id(store, session_id).await? else {
        return Ok(());
    };
    argo_store::subtask_budget::record_subtask_spend(store.pool(), &id, cents).await?;
    Ok(())
}

/// Pure helper — split out for testability. Mirrors the
/// helpers::record_turn_spend math (`tokens × price / 1000` USD)
/// but rounds straight to cents instead of micros.
pub fn compute_cents(tokens_in: u32, tokens_out: u32, price_in: f64, price_out: f64) -> i64 {
    let usd = (f64::from(tokens_in) * price_in + f64::from(tokens_out) * price_out) / 1000.0;
    let cents = (usd * 100.0).round().max(0.0);
    cents as i64
}

#[cfg(test)]
#[path = "turn_budget_tests.rs"]
mod tests;
