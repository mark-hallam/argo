//! DEC-041 piece 3b — auto-graduate a hired teammate's SAFE TIER to the
//! internal-document executor.
//!
//! After a hire commits, this writes a `document`-kind execution-promotion
//! grant (see [`argo_store::executor_promotion`]) for every manifest action
//! that is **low-risk AND requires no approval** — the act-by-default safe
//! tier (DEC-041 "A"). A granted safe action resolves, at dispatch, to the
//! side-effect-free `DocumentExecutor`: the teammate produces an internal
//! document/summary and the action self-completes, with **zero external
//! blast radius**.
//!
//! ## Dormant by default (load-bearing)
//!
//! The whole step is gated on `ARGO_EXECUTION_GRADUATION`
//! ([`crate::handlers::worker_actions_executor_promotion::execution_graduation_enabled`]).
//! With the flag unset — the shipped default — this is a **no-op**: no grant
//! is written, the hire is byte-for-byte unchanged, and every action stays
//! draft-only. The grants only ever appear once the operator has flipped the
//! flag live (after the blast-radius review re-run, DEC-041). Sensitive /
//! medium / approval-required actions are NEVER graduated here — they stay
//! human/draft. Best-effort: a write failure is logged, never surfaced (the
//! worker is already committed).

use super::super::worker_actions_executor_promotion::execution_graduation_enabled;
use argo_identity::WorkerId;
use argo_store::executor_promotion::{promote, PromotionGrant, PROMOTABLE_KIND_DOCUMENT};
use argo_store::SessionStore;
use argo_types::TenantId;
use argo_vertical::ActionRiskLevel;
use std::sync::Arc;

/// Principal recorded as the grantor for an auto-graduated safe-tier action.
const AUTO_GRADUATION_PRINCIPAL: &str = "auto:safe-tier-graduation";

/// Auto-graduate the worker's safe-tier (low-risk + no-approval) actions to
/// the internal `document` executor. No-op unless `ARGO_EXECUTION_GRADUATION`
/// is truthy. Returns the number of grants written.
pub(super) async fn auto_graduate_safe_tier(
    store: &Arc<SessionStore>,
    tenant_id: TenantId,
    worker_id: WorkerId,
    vertical_id: &str,
) -> usize {
    // Dormant default-OFF: write nothing until the operator flips graduation on.
    if !execution_graduation_enabled() {
        return 0;
    }
    let safe_actions = collect_safe_tier_actions(vertical_id);
    if safe_actions.is_empty() {
        return 0;
    }
    let mut written = 0usize;
    for action_type in safe_actions {
        let grant = PromotionGrant {
            tenant_id: tenant_id.to_string(),
            worker_id: worker_id.to_string(),
            action_type: action_type.clone(),
            executor_kind: PROMOTABLE_KIND_DOCUMENT.to_string(),
            mcp_server: None,
            mcp_tool: None,
            promoted_by: AUTO_GRADUATION_PRINCIPAL.to_string(),
        };
        match promote(store.pool(), &grant).await {
            Ok(_) => written += 1,
            Err(e) => tracing::warn!(
                target: "gateway::safe_tier_graduation",
                tenant_id = %tenant_id.0,
                worker_id = %worker_id,
                action_type = %action_type,
                error = %e,
                "auto safe-tier document graduation write failed; action stays draft-only"
            ),
        }
    }
    written
}

/// Load the vertical manifest and return the action types that are in the
/// safe tier (low risk AND `requires_approval = false`). Owned `String`s so
/// the manifest registry borrow is dropped before any `.await`. Returns an
/// empty vec on any load error (best-effort; the worker is already committed).
fn collect_safe_tier_actions(vertical_id: &str) -> Vec<String> {
    let dir = super::super::vertical::resolve_verticals_dir();
    let Ok((registry, _errors)) = argo_vertical::load_dir(&dir) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (_, loaded) in registry.list() {
        if loaded.manifest.vertical.id != vertical_id {
            continue;
        }
        if let Some(actions) = loaded.manifest.actions.as_ref() {
            for (action_type, decl) in &actions.actions {
                // SAFE TIER = low risk AND the manifest needs no approval.
                // medium/high or approval-required are never graduated here.
                if matches!(decl.risk, ActionRiskLevel::Low) && !decl.requires_approval {
                    out.push(action_type.clone());
                }
            }
        }
    }
    out
}
