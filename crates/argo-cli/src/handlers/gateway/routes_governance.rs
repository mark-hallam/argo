//! Gateway route for the audit-readiness / governance report
//! (competitive gap #1, move A — *package the moat we already have*).
//!
//! `GET /api/v1/governance/readiness[?tenant=<id>]` assembles a single
//! read-only report for one tenant from data the platform already
//! captures — it is **assembly, not new capture**:
//!
//!   - **audit** — runs the hash-chain verifier
//!     ([`argo_audit::verify_session`]) over every session the tenant
//!     owns and reports `chain_intact` (every chain verifies cleanly)
//!     plus `action_count` (total audit rows examined). This is the
//!     tamper-evidence claim made legible.
//!   - **capabilities** — the capability-RBAC role presets the engine
//!     provisions for a workspace
//!     ([`argo_capability::default_roles_for_workspace`]). Reported as a
//!     role-name summary; per-principal dynamic grants are not persisted
//!     in a countable table (see `// follow-up:` on [`capability_summary`]).
//!   - **truth_boundary** — whether a truth-boundary policy is configured
//!     for this process and resolves to the requested tenant
//!     (env `ARGO_TRUTH_BOUNDARY_POLICY_PATH`, loaded via
//!     [`argo_capability::load_policy`]).
//!   - **approvals** — decided vs. pending counts for the tenant from
//!     `approval_request`, plus a coverage percentage.
//!
//! **NO THEATER.** Every value is read straight from the source; a
//! source that is not readily countable is reported `null` with a `note`
//! rather than fabricated.
//!
//! Tenant scope: an explicit `?tenant=<uuid>` pins the report to that
//! tenant; without it the report resolves the workspace tenant from cwd
//! the same way [`super::routes_workspace_settings`] does. A tenant with
//! no rows returns a well-formed, clean report (no cross-tenant leak).
//!
//! Bearer auth is applied upstream by the GET dispatcher; this module is
//! a pure read.

use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId, TenantId};
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

/// GET match arm — fanned out from
/// [`super::routes_dispatch_get_api_groups`]. Returns `Some(resp)` when
/// the readiness path matches, `None` otherwise.
pub(super) async fn dispatch_get(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    if !(req.starts_with("GET /api/v1/governance/readiness ")
        || req.starts_with("GET /api/v1/governance/readiness?"))
    {
        return None;
    }
    Some(readiness_response(req, store).await)
}

async fn readiness_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let tenant = match resolve_tenant(req, store).await {
        Ok(t) => t,
        Err(e) => return server_error(&format!("resolve tenant: {e}")),
    };
    match build_report(store, tenant).await {
        Ok(report) => ("200 OK", report.to_string()),
        Err(e) => server_error(&format!("governance report: {e}")),
    }
}

/// Assemble the full report object for `tenant`. Each section is built
/// independently from its own source so a thin section never blocks the
/// rest of the report.
async fn build_report(store: &SessionStore, tenant: TenantId) -> ArgoResult<Value> {
    let audit = audit_summary(store, tenant).await?;
    let approvals = approval_summary(store, tenant).await?;
    Ok(json!({
        "tenant_id": tenant.to_string(),
        "generated_at": chrono::Utc::now().to_rfc3339(),
        "audit": audit,
        "capabilities": capability_summary(),
        "truth_boundary": truth_boundary_summary(tenant),
        "approvals": approvals,
    }))
}

/// `{ chain_intact, action_count }` — verify the hash chain for every
/// session the tenant owns (chains do not connect across sessions, so
/// each is verified independently, mirroring `verify_all`) and sum the
/// rows examined. `chain_intact` is the AND of every session's verdict;
/// a tenant with zero rows is vacuously intact with `action_count = 0`.
async fn audit_summary(store: &SessionStore, tenant: TenantId) -> ArgoResult<Value> {
    let session_ids = tenant_session_ids(store, tenant).await?;
    let mut action_count: usize = 0;
    let mut chain_intact = true;
    for sid in session_ids {
        let report = argo_audit::verify_session(store, sid).await?;
        action_count += report.rows;
        chain_intact = chain_intact && report.ok;
    }
    Ok(json!({
        "chain_intact": chain_intact,
        "action_count": action_count,
    }))
}

/// Distinct, non-NULL `session_id`s in `audit_record` owned by `tenant`.
/// The `tenant_id` column is the denormalized scope the verifier also
/// checks against `payload_json`, so this query is the tenant boundary.
async fn tenant_session_ids(
    store: &SessionStore,
    tenant: TenantId,
) -> ArgoResult<Vec<SessionId>> {
    let rows: Vec<Vec<u8>> = sqlx::query_scalar(
        "SELECT DISTINCT session_id FROM audit_record \
         WHERE tenant_id = ? AND session_id IS NOT NULL \
         ORDER BY session_id ASC",
    )
    .bind(tenant.to_string())
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("governance session ids: {e}")))?;
    Ok(rows
        .into_iter()
        .filter_map(|b| Uuid::from_slice(&b).ok().map(SessionId))
        .collect())
}

/// `{ decided, pending, coverage_pct }` for the tenant's approval
/// requests. `decided` counts any terminal status (granted / denied /
/// expired); `coverage_pct` is `decided / (decided + pending) * 100`,
/// rounded to one decimal, and `null` when the tenant has no requests
/// (no denominator — reported with a `note`, never fabricated as 100).
async fn approval_summary(store: &SessionStore, tenant: TenantId) -> ArgoResult<Value> {
    let tenant_bytes = tenant.0.as_bytes().to_vec();
    let pending: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM approval_request \
         WHERE tenant_id = ? AND status = 'pending'",
    )
    .bind(&tenant_bytes)
    .fetch_one(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("governance pending count: {e}")))?;
    let decided: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM approval_request \
         WHERE tenant_id = ? AND status != 'pending'",
    )
    .bind(&tenant_bytes)
    .fetch_one(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("governance decided count: {e}")))?;
    let total = decided + pending;
    let coverage: Value = if total == 0 {
        Value::Null
    } else {
        // round to one decimal place
        let pct = (decided as f64) * 100.0 / (total as f64);
        json!((pct * 10.0).round() / 10.0)
    };
    let mut obj = json!({
        "decided": decided,
        "pending": pending,
        "coverage_pct": coverage,
    });
    if total == 0 {
        obj["note"] = json!("no approval requests recorded for this tenant");
    }
    Ok(obj)
}

/// `{ roles: [..] }` — the capability-RBAC role presets the engine
/// provisions for a workspace. These are the named capability sets a
/// fresh workspace ships with; reporting their names makes the RBAC
/// model legible without claiming a count we don't persist.
///
// follow-up: per-principal *dynamic* grants
// (`CapabilityEngine::grant_dynamic`) and any operator-added roles are
// held in-memory on the running engine, not in a tenant-scoped DB
// table, so a per-tenant grant count is not assemblable here. When a
// persisted grant store lands, add `grant_count` to this section.
fn capability_summary() -> Value {
    // `default_roles_for_workspace` keys role presets off the workspace
    // root only for the secret-scope path; the role *names* are
    // workspace-independent, so cwd is a safe, side-effect-free arg.
    let roles: Vec<String> = argo_capability::default_roles_for_workspace(
        &std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
    )
    .into_iter()
    .map(|r| r.name)
    .collect();
    json!({
        "model": "capability-rbac",
        "default_roles": roles,
        "note": "default workspace role presets; per-principal dynamic grants are not separately counted",
    })
}

/// `{ enabled }` — whether a truth-boundary policy is configured for
/// this process (`ARGO_TRUTH_BOUNDARY_POLICY_PATH`) **and** that policy
/// resolves to `tenant` (cross-tenant policies do not count, mirroring
/// the runtime's `active_policy_for` scope check). When no policy is
/// configured the boundary is reported `enabled: false` with a `note` —
/// the factual state, not an over-claim.
fn truth_boundary_summary(tenant: TenantId) -> Value {
    let Some(path) = std::env::var_os("ARGO_TRUTH_BOUNDARY_POLICY_PATH") else {
        return json!({
            "enabled": false,
            "note": "no truth-boundary policy configured for this process",
        });
    };
    match argo_capability::load_policy(std::path::Path::new(&path)) {
        Ok(policy) if policy.tenant_id == tenant => json!({
            "enabled": true,
            "policy_id": policy.id,
            "claim_classes": policy.claim_classes.len(),
        }),
        Ok(_) => json!({
            "enabled": false,
            "note": "a truth-boundary policy is configured but belongs to a different tenant",
        }),
        Err(e) => json!({
            "enabled": false,
            "note": format!("truth-boundary policy failed to load: {e}"),
        }),
    }
}

/// Resolve the report's target tenant. An explicit `?tenant=<uuid>`
/// query pins it to that tenant (the buyer/auditor download path); a
/// malformed value is a 400-worthy error surfaced by the caller. Absent
/// the query, the workspace tenant is resolved from cwd exactly as
/// [`super::routes_workspace_settings::resolve_workspace_tenant`] does,
/// so an unscoped call reports the operator's own workspace.
async fn resolve_tenant(req: &str, store: &SessionStore) -> ArgoResult<TenantId> {
    let tail = req.strip_prefix("GET ").unwrap_or(req);
    if let Some(raw) = super::routes_query::parse_query_kv(tail, "tenant") {
        let uuid = Uuid::parse_str(&raw)
            .map_err(|e| ArgoError::Schema(format!("invalid tenant id '{raw}': {e}")))?;
        return Ok(TenantId(uuid));
    }
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, _ws) = store.ensure_workspace(name, &root).await?;
    Ok(tenant)
}

fn server_error(message: &str) -> (&'static str, String) {
    (
        "500 Internal Server Error",
        json!({ "error": { "code": "internal_error", "message": message } }).to_string(),
    )
}

#[cfg(test)]
#[path = "routes_governance_tests.rs"]
mod tests;
