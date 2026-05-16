//! Tenant + workspace provisioning orchestrator
//! (stream `claude-tenant-provisioning-endpoint`).
//!
//! Wraps the substrate writers landed by `mission-tenant-substrate-tables`
//! into one idempotent step that the gateway's
//! `POST /api/v1/onboarding/provision` endpoint calls. Behaviour:
//!
//! - First call for a `UserId`: inserts a `tenant` row, a `workspace`
//!   row anchored to it, and a `user_tenant` link with `role = owner`.
//! - Repeat calls: the user's existing tenant + first workspace are
//!   returned unchanged. The caller decides which onboarding step to
//!   resume; this layer never mutates on a re-call.
//!
//! Audit emission lives in the gateway handler — `argo-identity` does
//! not depend on `argo-audit`. The handler stamps an
//! `AuditKind::TenantProvisioned` row using the [`ProvisionOutcome`]
//! returned here.
//!
//! ```no_run
//! # async fn demo() -> argo_types::ArgoResult<()> {
//! use argo_identity::tenant_provision::{provision_for_user, ProvisionRequest};
//! use argo_identity::user::UserAccount;
//! use argo_identity::user_store::insert_user;
//! use argo_store::SessionStore;
//! let store = SessionStore::open(":memory:").await?;
//! store.migrate().await?;
//! let user = UserAccount::new("alice@acme.com".into());
//! insert_user(&store, &user).await?;
//! let outcome = provision_for_user(
//!     &store,
//!     ProvisionRequest {
//!         user_id: user.id,
//!         user_email: &user.email,
//!         workspace_name: "Acme",
//!         company_name: Some("Acme Co"),
//!     },
//! )
//! .await?;
//! assert!(outcome.created);
//! # Ok(()) }
//! ```

use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId, WorkspaceId};
use chrono::Utc;

use crate::tenant::Tenant;
use crate::tenant_store::{get_tenant, insert_tenant};
use crate::user::UserId;
use crate::user_tenant::UserTenantRole;
use crate::user_tenant_store::{link_user_to_tenant, list_tenants_for_user};

/// Inputs to [`provision_for_user`]. Borrows so the caller keeps
/// ownership of the strings — they're typically slices of an HTTP body.
pub struct ProvisionRequest<'a> {
    /// Verified caller. The user must already exist in `user_account`
    /// (the gateway handler inserts on first-time magic-link verify).
    pub user_id: UserId,
    /// Verified caller email — used as the tenant's `billing_email`
    /// when no other value is supplied by the body.
    pub user_email: &'a str,
    /// Operator-facing workspace label (from the request body).
    pub workspace_name: &'a str,
    /// Optional company name — when provided, becomes the tenant's
    /// `name`. Falls back to `workspace_name` when absent.
    pub company_name: Option<&'a str>,
}

/// What [`provision_for_user`] returns. `created` is `true` only on the
/// first call for a given `UserId`; idempotent re-calls return the
/// existing tenant + workspace with `created = false`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvisionOutcome {
    /// Tenant the user owns.
    pub tenant_id: TenantId,
    /// Workspace anchored to that tenant.
    pub workspace_id: WorkspaceId,
    /// `true` on the first-call insert path; `false` when an existing
    /// link was found and returned unchanged.
    pub created: bool,
}

/// Provision (or look up) a tenant + workspace for `req.user_id`. The
/// `user_account` row must already exist; tenant + workspace + link
/// rows are created on the first call and reused on subsequent calls.
pub async fn provision_for_user(
    store: &SessionStore,
    req: ProvisionRequest<'_>,
) -> ArgoResult<ProvisionOutcome> {
    if req.workspace_name.trim().is_empty() {
        return Err(ArgoError::Schema(
            "workspace_name must be non-empty".to_string(),
        ));
    }
    if let Some(existing) = first_link_for_user(store, req.user_id).await? {
        return Ok(existing);
    }
    let tenant_name = req
        .company_name
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| req.workspace_name.trim());
    let tenant = Tenant::new(tenant_name.to_string(), req.user_email.to_string());
    insert_tenant(store, &tenant).await?;
    let workspace_id =
        insert_workspace_for_tenant(store, tenant.id, req.workspace_name.trim()).await?;
    link_user_to_tenant(store, req.user_id, tenant.id, UserTenantRole::Owner).await?;
    Ok(ProvisionOutcome {
        tenant_id: tenant.id,
        workspace_id,
        created: true,
    })
}

async fn first_link_for_user(
    store: &SessionStore,
    user_id: UserId,
) -> ArgoResult<Option<ProvisionOutcome>> {
    let links = list_tenants_for_user(store, user_id).await?;
    let Some(link) = links.into_iter().min_by_key(|l| l.created_at) else {
        return Ok(None);
    };
    // Verify the tenant row still exists. If a tenant was soft-deleted
    // the user may need to be re-provisioned; surfacing `None` here lets
    // the caller create a fresh tenant rather than reuse a dead link.
    if get_tenant(store, link.tenant_id).await?.is_none() {
        return Ok(None);
    }
    let Some(workspace_id) = first_workspace_for_tenant(store, link.tenant_id).await? else {
        return Ok(None);
    };
    Ok(Some(ProvisionOutcome {
        tenant_id: link.tenant_id,
        workspace_id,
        created: false,
    }))
}

async fn insert_workspace_for_tenant(
    store: &SessionStore,
    tenant_id: TenantId,
    name: &str,
) -> ArgoResult<WorkspaceId> {
    let workspace_id = WorkspaceId::new();
    let root_path = format!("tenant://{}/{}", tenant_id.0, workspace_id.0);
    let created_at = Utc::now().timestamp();
    sqlx::query(
        "INSERT INTO workspace (id, tenant_id, name, root_path, created_at) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(workspace_id.0.as_bytes().as_slice())
    .bind(tenant_id.0.as_bytes().as_slice())
    .bind(name)
    .bind(root_path)
    .bind(created_at)
    .execute(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("provision insert workspace: {e}")))?;
    Ok(workspace_id)
}

async fn first_workspace_for_tenant(
    store: &SessionStore,
    tenant_id: TenantId,
) -> ArgoResult<Option<WorkspaceId>> {
    let row: Option<Vec<u8>> = sqlx::query_scalar(
        "SELECT id FROM workspace WHERE tenant_id = ? ORDER BY created_at ASC LIMIT 1",
    )
    .bind(tenant_id.0.as_bytes().as_slice())
    .fetch_optional(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("provision lookup workspace: {e}")))?;
    row.map(|b| {
        uuid::Uuid::from_slice(&b)
            .map(WorkspaceId)
            .map_err(|e| ArgoError::Provider(format!("workspace id decode: {e}")))
    })
    .transpose()
}

#[cfg(test)]
#[path = "tenant_provision_tests.rs"]
mod tests;
