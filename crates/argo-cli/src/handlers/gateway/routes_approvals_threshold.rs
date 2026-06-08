//! N-of-M threshold approvals (Stream `n-of-m-multi-step-approval`,
//! v1.1 governance) — the per-approver **vote ledger** plus the
//! settle-path orchestration that keeps an action HELD until a quorum
//! of distinct approvers vote.
//!
//! ## Where the threshold comes from
//!
//! An approval *request* (the `ActionRecord`) declares its rule on the
//! action payload under an `approval` node:
//!
//! ```json
//! {
//!   "approval": {
//!     "threshold": 2,
//!     "approvers": ["a@x", "b@x", "c@x"],
//!     "requester": "a@x"
//!   }
//! }
//! ```
//!
//! [`ThresholdRule::from_payload`] parses it; absence — or a
//! `threshold` of `0`/`1` — means single-approver, so the caller
//! ([`super::routes_channels::settle`]) keeps its existing one-reply
//! path untouched (back-compat).
//!
//! ## Separation of duties (Stream
//! `n-of-m-bind-approver-identity-and-self-approval-exclusion`)
//!
//! A threshold quorum is only trustworthy if its N approvers are
//! cryptographically-distinct authenticated humans, so a counted vote
//! must clear three gates:
//!   * **identity-bound token** — the settle path
//!     ([`super::routes_channels::settle`]) rejects any threshold vote
//!     whose token carries no bound approver, and derives the voter from
//!     the token (not the spoofable channel-supplied actor string);
//!   * **named allowlist** — [`ThresholdRule::is_authorized`] requires a
//!     non-empty `approvers` list (an empty "anyone" allowlist
//!     authorizes no one);
//!   * **requester excluded** — a vote whose approver equals
//!     `approval.requester` is never counted
//!     ([`ThresholdRule::is_self_approval`]).
//!
//! ## The vote ledger (`approval_vote`, migration v92)
//!
//! Each authorized approver's decision is one row. Tenant isolation +
//! per-approver idempotency are both enforced by the table's
//! `UNIQUE (tenant_id, approval_request_id, approver)` constraint:
//!   * the same approver voting twice is a no-op `INSERT OR IGNORE`, so
//!     it never counts twice toward the threshold; and
//!   * every query is tenant-scoped, so tenant A's votes are invisible
//!     to (and uncountable by) tenant B even on a colliding
//!     `approval_request_id`.
//!
//! ## Reject rule (documented denial semantics)
//!
//! **Any single reject from an authorized approver denies the whole
//! request.** A reject finalizes the action to `blocked` immediately;
//! subsequent votes hit the settle-path `state != Pending` guard and
//! short-circuit. Approvals must reach the full `threshold` of distinct
//! approve votes to finalize as granted.

use super::routes_channels::Decision;
use crate::handlers::worker_actions_transitions::LocatedAction;
use argo_audit::{AuditKind, AuditSink};
use argo_identity::auto_approval::ThresholdRule;
use argo_types::{ArgoError, ArgoResult};
use serde_json::json;
use sqlx::{Pool, Sqlite};

/// Outcome of recording one threshold vote. The settle path turns this
/// into the durable state transition + terminal audit event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ThresholdOutcome {
    /// The voter is not on the approval's `approvers` allowlist; no
    /// vote was recorded and the action stays HELD.
    Unauthorized,
    /// Vote recorded; the action stays HELD pending more approvals.
    Held {
        /// Distinct approve votes recorded so far.
        distinct: usize,
        /// Approvals required to finalize.
        threshold: u32,
    },
    /// The threshold of distinct approve votes was reached — finalize
    /// the action as granted.
    FinalizeApprove,
    /// An authorized approver rejected — deny the whole request.
    FinalizeReject,
}

/// Pure decision: given the rule, the vote, whether the voter is
/// authorized, and the current distinct-approval count, decide the
/// outcome. IO-free so the quorum / reject semantics are unit-testable
/// without a store.
pub(super) fn decide_outcome(
    rule: &ThresholdRule,
    decision: Decision,
    authorized: bool,
    distinct_approvals: usize,
) -> ThresholdOutcome {
    if !authorized {
        return ThresholdOutcome::Unauthorized;
    }
    match decision {
        // Any reject denies — pinned denial semantics.
        Decision::Reject => ThresholdOutcome::FinalizeReject,
        Decision::Approve => {
            if rule.is_met(distinct_approvals) {
                ThresholdOutcome::FinalizeApprove
            } else {
                ThresholdOutcome::Held {
                    distinct: distinct_approvals,
                    threshold: rule.threshold,
                }
            }
        }
    }
}

/// Record one approver's threshold vote, emit the per-vote audit row
/// (only for a genuinely new vote), and return the resulting outcome.
///
/// Unauthorized voters record nothing. Authorized re-votes are
/// idempotent (UNIQUE no-op) and do not emit a second audit row.
pub(super) async fn record_threshold_vote(
    audit: &AuditSink,
    pool: &Pool<Sqlite>,
    located: &LocatedAction,
    actor: &str,
    decision: Decision,
    source: &str,
    rule: &ThresholdRule,
) -> ArgoResult<ThresholdOutcome> {
    // Separation of duties: the voter must be a NAMED approver (an empty
    // allowlist authorizes no one) AND must not be the action's
    // requester. Either failure records nothing and keeps the action
    // HELD — fail closed.
    if !rule.is_authorized(actor) || rule.is_self_approval(actor) {
        return Ok(ThresholdOutcome::Unauthorized);
    }
    // The vote ledger is tenant-scoped to the sink's bound tenant — the
    // same tenant that owns the approval's other audit rows.
    let tenant_id = audit.tenant_id();
    let approval_id = located.record.action_id.as_str();
    let newly = record_vote(pool, tenant_id, approval_id, actor, decision).await?;
    let distinct = distinct_approvals(pool, tenant_id, approval_id).await?;
    if newly {
        let payload = json!({
            "action_id": approval_id,
            "approver": actor,
            "decision": decision_label(decision),
            "distinct_approvals": distinct,
            "threshold": rule.threshold,
            "source": source,
        });
        if let Err(e) = audit
            .write(None, AuditKind::ApprovalVoteRecorded, payload, None)
            .await
        {
            tracing::warn!(
                approval_id = %approval_id,
                error = %e,
                "approval.vote_recorded audit write failed; continuing",
            );
        }
    }
    Ok(decide_outcome(rule, decision, true, distinct))
}

/// Insert one vote idempotently. Returns `true` when it is a NEW
/// distinct vote (row inserted), `false` when the approver had already
/// voted (UNIQUE collision → no-op). Tenant-scoped via the UNIQUE key.
pub(super) async fn record_vote(
    pool: &Pool<Sqlite>,
    tenant_id: &str,
    approval_request_id: &str,
    approver: &str,
    decision: Decision,
) -> ArgoResult<bool> {
    let res = sqlx::query(
        "INSERT OR IGNORE INTO approval_vote \
         (tenant_id, approval_request_id, approver, decision, voted_at) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(tenant_id)
    .bind(approval_request_id)
    .bind(approver)
    .bind(decision_label(decision))
    .bind(chrono::Utc::now().to_rfc3339())
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("approval_vote insert: {e}")))?;
    Ok(res.rows_affected() == 1)
}

/// Count DISTINCT approvers who cast an *approve* vote for this
/// approval, scoped to `tenant_id`. This is the number compared against
/// the threshold.
pub(super) async fn distinct_approvals(
    pool: &Pool<Sqlite>,
    tenant_id: &str,
    approval_request_id: &str,
) -> ArgoResult<usize> {
    let n: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT approver) FROM approval_vote \
         WHERE tenant_id = ? AND approval_request_id = ? AND decision = 'approve'",
    )
    .bind(tenant_id)
    .bind(approval_request_id)
    .fetch_one(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("approval_vote tally: {e}")))?;
    Ok(usize::try_from(n.max(0)).unwrap_or(0))
}

fn decision_label(d: Decision) -> &'static str {
    match d {
        Decision::Approve => "approve",
        Decision::Reject => "reject",
    }
}

#[cfg(test)]
#[path = "routes_approvals_threshold_tests.rs"]
mod tests;
