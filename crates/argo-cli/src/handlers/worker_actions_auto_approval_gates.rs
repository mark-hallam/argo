//! Effective-policy gate layered on top of the H-1 budget verdict.
//!
//! Extracted from `worker_actions_auto_approval.rs` to keep that file
//! under the 500-LOC ceiling. The gate-layer surface owns:
//!
//! - [`SessionContext`] + [`resolve_session_context`] — the
//!   (tenant, role, worker, roster) tuple that drives
//!   `argo_identity::policy::resolve_effective_policy`.
//! - [`apply_effective_gates`] — the post-budget constraint pass
//!   (`require_signatures`, `quiet_hours`, `audit_chain_required`).
//!
//! Quiet-hours window detection lives in the sibling
//! `worker_actions_auto_approval_quiet_hours` module so the date /
//! timezone helpers can grow without bloating the gate logic.

use std::path::Path;
use std::sync::Arc;

use argo_config::LEGACY_AUDIT_TENANT_SENTINEL;
use argo_identity::auto_approval::AutoApprovalDecision;
use argo_identity::policy::EffectivePolicy;
use argo_identity::RosterStore;
use argo_store::SessionStore;
use argo_types::{ArgoResult, TenantId};
use chrono::{DateTime, Utc};

use super::worker_actions::ActionRecord;
use super::worker_actions_auto_approval::{block_to_error, worker_id_for_action};
use super::worker_actions_auto_approval_quiet_hours::quiet_hours_check;

/// Resolved (tenant, role, worker, roster) tuple consumed by
/// [`argo_identity::policy::resolve_effective_policy`] and
/// [`apply_effective_gates`]. Empty strings are tolerated — they fall
/// through the wildcard (`*`) matchers in policy YAMLs so missing
/// context only loses specificity, never correctness.
#[derive(Debug, Clone, Default)]
pub(crate) struct SessionContext {
    /// Workspace tenant id as the operator wrote it in policy YAML
    /// (`workspace_tenant_id(cwd)`); compared against
    /// `PolicyApplies::Tenant`. Empty only when cwd resolution fails.
    pub(crate) tenant_str: String,
    /// Worker's `role_name` (operator-set, see
    /// `argo_identity::Worker::role_name`); compared against
    /// `PolicyApplies::Role`. Empty when the worker is unknown or has
    /// no role_name.
    pub(crate) role_str: String,
    /// Stable worker id for budget / circuit / roster lookups.
    /// Populated so a future roster-name `PolicyApplies` variant can
    /// grow without re-threading this lookup.
    #[allow(dead_code)]
    pub(crate) worker_id_str: String,
    /// Most-relevant roster the worker belongs to (per
    /// `RosterStore::lookup_for_worker` priority order); empty when
    /// the worker has no roster. Resolved here so the next brief
    /// (`PolicyApplies::Roster`) can switch the resolver to consume
    /// it without re-threading the lookup.
    #[allow(dead_code)]
    pub(crate) roster_name: String,
}

/// Resolve the gate's session context (tenant + role + worker +
/// roster) so `resolve_effective_policy` sees real values instead of
/// the v0 empty-string placeholders. Falls back to empty strings on
/// missing pieces — the resolver treats those as "wildcard-only
/// matches" so the action still flows; only specificity is lost.
pub(crate) async fn resolve_session_context(
    store: Arc<SessionStore>,
    cwd: &Path,
    action: &ActionRecord,
) -> ArgoResult<SessionContext> {
    let tenant_str = argo_config::workspace_tenant_id(cwd);
    let worker_id_str = worker_id_for_action(action);
    let name = cwd.file_name().and_then(|s| s.to_str()).unwrap_or("workspace");
    let (tenant_uuid, _ws): (TenantId, _) =
        store.ensure_workspace(name, &cwd.to_string_lossy()).await?;
    let mut role_str = String::new();
    let mut roster_name = String::new();
    if !action.worker_id.0.is_nil() {
        if let Ok(Some(worker)) = argo_identity::IdentityStore::new(&store)
            .get_worker(tenant_uuid, action.worker_id)
            .await
        {
            role_str = worker.role_name.unwrap_or_default();
        }
        if let Ok(Some(assignment)) = RosterStore::new(Arc::clone(&store))
            .lookup_for_worker(action.worker_id)
            .await
        {
            roster_name = assignment.roster.name;
        }
    }
    Ok(SessionContext { tenant_str, role_str, worker_id_str, roster_name })
}

/// Layer `EffectivePolicy` constraints (`require_signatures`,
/// `quiet_hours`, `audit_chain_required`) on top of the H-1 budget
/// verdict. Only `AutoApprove` decisions are reconsidered; `Ask` /
/// `Block` already deny auto-approval so the policy-file constraints
/// don't loosen them.
///
/// Fail-closed semantics: every unsatisfied constraint downgrades the
/// decision (auto-approve → Ask, or → Block for
/// `QuietHoursBehavior::Block` and audit-chain failures). Nothing
/// silently no-ops.
pub(crate) fn apply_effective_gates(
    action_id: &str,
    effective: &EffectivePolicy,
    session_ctx: &SessionContext,
    decision: AutoApprovalDecision,
    now: DateTime<Utc>,
) -> ArgoResult<AutoApprovalDecision> {
    if !matches!(decision, AutoApprovalDecision::AutoApprove { .. }) {
        return Ok(decision);
    }
    if effective.require_signatures > 1 {
        return Ok(AutoApprovalDecision::Ask {
            channel: None,
            reason: format!(
                "require_signatures={} not satisfied by auto-approval",
                effective.require_signatures
            ),
        });
    }
    if let Some(forced) = quiet_hours_check(action_id, effective, now)? {
        return Ok(forced);
    }
    if effective.audit_chain_required && session_ctx.tenant_str == LEGACY_AUDIT_TENANT_SENTINEL {
        return Err(block_to_error(
            action_id,
            "audit_chain_required: workspace tenant is the legacy sentinel",
        ));
    }
    Ok(decision)
}
