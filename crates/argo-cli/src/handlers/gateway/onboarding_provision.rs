//! `POST /api/v1/onboarding/provision` — tenant + workspace seed for a
//! freshly-authenticated user. Stream `claude-tenant-provisioning-endpoint`.
//!
//! Bearer auth + principal-header parsing are upstream
//! (`routes_dispatch_post::dispatch_post`). This handler:
//!
//! 1. Pulls the human caller out of `X-Argo-Principal: human:<email>`
//!    (the contract the magic-link verify route stamps onto every
//!    authenticated request). Returns 401 if the header is missing or
//!    does not carry a `human:` principal — the brief requires "401
//!    without JWT" semantics, and the principal header is the
//!    `human:<email>` projection of the verified JWT claims today.
//! 2. Parses `{ workspace_name, company_name? }` from the JSON body.
//! 3. Finds (or inserts on first sight) the `user_account` row keyed
//!    by the verified email. `claude-magic-link-auth-backend` will
//!    insert this row on verify once it lands on main; until then the
//!    upsert here keeps the endpoint independently testable end-to-end.
//! 4. Calls [`argo_identity::provision_for_user`] — idempotent: a
//!    second call from the same user returns the existing tenant +
//!    workspace.
//! 5. Verifies the workspace's primary vertical (`gtm-engineer`) is in
//!    the on-disk catalog so a downstream worker hire can immediately
//!    reference it. The catalog is global, not per-workspace; the
//!    brief's "pre-install" requirement is met by recording the
//!    selection on the audit row plus a hard catalog lookup that
//!    fails the response with 500 when the pack is missing.
//! 6. Emits an `AuditKind::TenantProvisioned` row carrying the
//!    `tenant_id`, `workspace_id`, `user_id`, `workspace_name`, and
//!    whether this was a fresh insert or an idempotent re-call.

use super::body_from_request;
use super::onboarding_seeds;
use super::store_unavailable;
use argo_audit::{AuditKind, AuditSink};
use argo_identity::user::UserAccount;
use argo_identity::user_store::{get_user_by_email, insert_user};
use argo_identity::{provision_for_user, ProvisionOutcome, ProvisionRequest};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use serde_json::{json, Value};
use std::sync::Arc;

/// Principal handle stamped on seeded workplace-knowledge docs and on
/// their audit rows. Distinct from `human:<email>` so operators can
/// filter `/audit` by actor to see the initial seed event.
const SEED_ACTOR: &str = "system:onboarding-seed";

const DEFAULT_VERTICAL: &str = "gtm-engineer";
const HUMAN_PRINCIPAL_PREFIX: &str = "human:";
const PRINCIPAL_HEADER: &str = "X-Argo-Principal";

/// `POST /api/v1/onboarding/provision` — handler entry point.
///
/// On success returns `201 { tenant_id, workspace_id, next_step }` on a
/// fresh provision or `200 { …, idempotent: true }` on a repeat call.
pub(super) async fn provision_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let store = match store {
        Some(s) => s,
        None => return store_unavailable(),
    };
    let email = match principal_email(req) {
        Some(e) => e,
        None => return unauthorized(),
    };
    let body = body_from_request(req);
    let parsed: Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return bad_request(&format!("parse body: {e}")),
    };
    let workspace_name = match parsed.get("workspace_name").and_then(Value::as_str) {
        Some(s) if !s.trim().is_empty() => s.trim().to_string(),
        _ => return bad_request("`workspace_name` (non-empty string) is required"),
    };
    let company_name = parsed
        .get("company_name")
        .and_then(Value::as_str)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    if !vertical_exists(DEFAULT_VERTICAL) {
        return server_error(&format!(
            "default vertical '{DEFAULT_VERTICAL}' not in catalog"
        ));
    }

    let user = match upsert_user(store, &email).await {
        Ok(u) => u,
        Err(e) => return server_error(&format!("resolve user: {e}")),
    };

    let outcome = match provision_for_user(
        store,
        ProvisionRequest {
            user_id: user.id,
            user_email: &user.email,
            workspace_name: &workspace_name,
            company_name: company_name.as_deref(),
        },
    )
    .await
    {
        Ok(o) => o,
        Err(ArgoError::Schema(msg)) => return bad_request(&msg),
        Err(e) => return server_error(&format!("provision: {e}")),
    };

    emit_audit(store, &user, &workspace_name, &outcome).await;

    if let Err(e) = seed_and_audit(store, &outcome.tenant_id).await {
        // Seeds are best-effort polish: failing to seed shouldn't 500
        // a successful provision — the wiki can still be authored by
        // hand. We log + continue so the response stays consistent
        // with the rest of the post-provision audit emissions
        // (`emit_audit` also swallows + logs).
        tracing::warn!(
            target: "gateway::onboarding_provision::seed",
            error = %e,
            tenant_id = %outcome.tenant_id,
            "workplace-knowledge seed failed"
        );
    }

    let next_step = next_step_for(&outcome);
    // `created` is the positive form of `idempotent`; the wizard's
    // workspace step shows a celebration toast on `created=true` and
    // stays silent on `created=false`. Both fields are emitted so
    // existing callers (which already key off `idempotent`) keep
    // working and new callers can read the more direct flag.
    let payload = json!({
        "tenant_id": outcome.tenant_id.0.to_string(),
        "workspace_id": outcome.workspace_id.0.to_string(),
        "vertical_id": DEFAULT_VERTICAL,
        "next_step": next_step,
        "idempotent": !outcome.created,
        "created": outcome.created,
    });
    let status = if outcome.created {
        "201 Created"
    } else {
        "200 OK"
    };
    (status, payload.to_string())
}

fn principal_email(req: &str) -> Option<String> {
    let headers = req
        .split_once("\r\n\r\n")
        .map_or(req, |(headers, _body)| headers);
    headers
        .lines()
        .skip(1)
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case(PRINCIPAL_HEADER)
                .then(|| value.trim().to_string())
        })
        .and_then(|raw| raw.strip_prefix(HUMAN_PRINCIPAL_PREFIX).map(str::to_string))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && s.contains('@'))
}

async fn upsert_user(store: &SessionStore, email: &str) -> ArgoResult<UserAccount> {
    if let Some(existing) = get_user_by_email(store, email).await? {
        return Ok(existing);
    }
    let user = UserAccount::new(email.to_string());
    insert_user(store, &user).await?;
    Ok(user)
}

fn vertical_exists(id: &str) -> bool {
    let dir = crate::handlers::vertical::resolve_verticals_dir();
    let Ok((registry, _errors)) = argo_vertical::load_dir(&dir) else {
        return false;
    };
    registry.get(id).is_some()
}

fn next_step_for(outcome: &ProvisionOutcome) -> &'static str {
    // v1 always lands the operator on the connect-provider step. Once
    // we persist onboarding state (provider key set / vertical chosen
    // / first hire created) the idempotent re-call branch can resume
    // mid-flow; the field is shaped to support that without a wire
    // change. Distinct const so the contract is easy to grep.
    let _ = outcome.created;
    "/onboarding/connect-provider"
}

async fn emit_audit(
    store: &Arc<SessionStore>,
    user: &UserAccount,
    workspace_name: &str,
    outcome: &ProvisionOutcome,
) {
    let payload = json!({
        "tenant_id": outcome.tenant_id.0.to_string(),
        "workspace_id": outcome.workspace_id.0.to_string(),
        "user_id": user.id.0.to_string(),
        "workspace_name": workspace_name,
        "vertical_id": DEFAULT_VERTICAL,
        "created": outcome.created,
    });
    let sink = AuditSink::new_with_tenant(Arc::clone(store), outcome.tenant_id.to_string());
    let principal = format!("{HUMAN_PRINCIPAL_PREFIX}{}", user.email);
    if let Err(e) = sink
        .write(None, AuditKind::TenantProvisioned, payload, Some(principal))
        .await
    {
        tracing::warn!(
            target: "gateway::onboarding_provision",
            error = %e,
            user_id = %user.id,
            tenant_id = %outcome.tenant_id,
            "tenant_provisioned audit write failed"
        );
    }
}

async fn seed_and_audit(store: &Arc<SessionStore>, tenant_id: &TenantId) -> ArgoResult<()> {
    let inserted =
        onboarding_seeds::seed_workplace_knowledge(store, tenant_id, SEED_ACTOR).await?;
    if inserted.is_empty() {
        return Ok(());
    }
    tracing::info!(
        target: "argo::onboarding::seed",
        tenant = %tenant_id,
        count = inserted.len(),
        "workplace knowledge seeded"
    );
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant_id.to_string());
    for slug in inserted {
        let payload = json!({
            "tenant_id": tenant_id.to_string(),
            "slug": slug,
            "created_by": SEED_ACTOR,
            "source": "onboarding-seed",
        });
        if let Err(e) = sink
            .write(
                None,
                AuditKind::WorkplaceKnowledgeDocCreated,
                payload,
                Some(SEED_ACTOR.to_string()),
            )
            .await
        {
            tracing::warn!(
                target: "gateway::onboarding_provision::seed",
                error = %e,
                tenant_id = %tenant_id,
                slug = slug,
                "workplace_knowledge_doc_created audit write failed"
            );
        }
    }
    Ok(())
}

fn unauthorized() -> (&'static str, String) {
    error_envelope(
        "401 Unauthorized",
        "unauthorized",
        "X-Argo-Principal header must carry a verified `human:<email>` value",
    )
}

fn bad_request(message: &str) -> (&'static str, String) {
    error_envelope("400 Bad Request", "bad_request", message)
}

fn server_error(message: &str) -> (&'static str, String) {
    error_envelope("500 Internal Server Error", "internal_error", message)
}

fn error_envelope(status: &'static str, code: &str, message: &str) -> (&'static str, String) {
    (
        status,
        json!({
            "error": {
                "code": code,
                "message": message,
            }
        })
        .to_string(),
    )
}

#[cfg(test)]
#[path = "onboarding_provision_tests.rs"]
mod tests;
