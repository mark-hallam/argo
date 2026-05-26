//! `POST /api/v1/onboarding/{provider-key, install-vertical, first-hire}`.
//!
//! Three sibling onboarding endpoints implemented alongside the already
//! shipped `onboarding_provision`. The wave-3 onboarding audit
//! (`docs/skycot/audit/onboarding/2026-05-20-wave-3-onboarding-e2e.md`)
//! flagged these as the missing backends; the Web UI silently treated
//! the gateway 404 as success, so users hard-stuck at step 5.
//!
//! Common shape (mirrors `onboarding_provision`):
//!
//! 1. Pull the `human:<email>` principal out of `X-Argo-Principal`. 401
//!    if the header is missing or doesn't carry a human principal.
//! 2. Resolve the caller's tenant + workspace via the existing
//!    `provision_for_user` idempotent re-call. Returns 412 if the user
//!    hasn't provisioned yet — these endpoints sit AFTER `/provision`.
//! 3. Do the work, scoped to that tenant + workspace.
//! 4. Emit an audit row for the accepted onboarding mutation.
//!
//! Provider keys are wrapped with `argo-credentials::age_wrap` before
//! storage. When no `MasterKey` is configured (dev / fresh-checkout
//! flow), the handler falls back to a process-ephemeral key —
//! encryption itself stays mandatory so the row never lands as
//! plaintext.
//!
//! Install-vertical reads the on-disk vertical manifest, counts skill
//! and workflow files under the loaded directory, and records the row.
//!
//! First-hire spawns one worker via `IdentityStore::create_worker_from_draft`
//! using a tight default duty so the operator can immediately observe
//! the worker on `/workers/<id>`.

use super::body_from_request;
use super::routes_auth_principal::principal_id_for_audit;
use super::routes_onboarding_genesis::{
    charter_hash, derive_duties, invoke_genesis, synthesise_charter,
};
use super::routes_onboarding_helpers::{
    bad_request, count_skill_files, count_workflow_files, emit_audit, infer_provider,
    load_vertical, map_workspace_error, mask_key, micros_to_usd, name_from_description, not_found,
    principal_email, resolve_workspace, server_error, unauthorized, validate_key_shape,
    wrap_secret, HUMAN_PRINCIPAL_PREFIX,
};
use super::store_unavailable;
use argo_audit::{AuditKind, AuditSink};
use argo_identity::{ApprovalPolicyChoice, HireDraft, IdentityStore};
use argo_store::workspace_keys::{
    ensure_schema as ensure_workspace_keys_schema, record_vertical_install, upsert_provider_key,
    ProviderKeyRecord,
};
use argo_store::SessionStore;
use argo_types::{ArgoError, TenantId};
use serde_json::{json, Value};
use std::sync::Arc;

/// `POST /api/v1/onboarding/provider-key` — validate + persist a
/// provider API key for the caller's workspace. The wire shape
/// matches the existing client at `web/lib/onboarding/use-provider-key.ts:20`.
pub(super) async fn provider_key_response(
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
    let api_key = match parsed.get("api_key").and_then(Value::as_str) {
        Some(s) if !s.trim().is_empty() => s.trim().to_string(),
        _ => return bad_request("`api_key` (non-empty string) is required"),
    };
    let provider = parsed
        .get("provider")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map_or_else(|| infer_provider(&api_key), str::to_string);

    let (tenant, workspace) = match resolve_workspace(store, &email).await {
        Ok(pair) => pair,
        Err(e) => return map_workspace_error(e),
    };

    let validation = validate_key_shape(&provider, &api_key);
    if !validation.ok {
        return bad_request(&validation.reason);
    }
    let masked = mask_key(&api_key);
    let ciphertext = match wrap_secret(&api_key, tenant) {
        Ok(c) => c,
        Err(e) => return server_error(&format!("wrap key: {e}")),
    };
    if let Err(e) = ensure_workspace_keys_schema(store.pool()).await {
        return server_error(&format!("ensure workspace_keys schema: {e}"));
    }
    if let Err(e) = upsert_provider_key(
        store.pool(),
        tenant,
        workspace,
        ProviderKeyRecord {
            provider: provider.clone(),
            masked_key: masked.clone(),
            ciphertext_b64: ciphertext,
            test_cost_micros: validation.test_cost_micros,
            test_tokens: validation.test_tokens,
        },
    )
    .await
    {
        return server_error(&format!("upsert provider key: {e}"));
    }

    emit_onboarding_audit(
        store,
        tenant,
        req,
        AuditKind::ProviderKeyConfigured,
        "onboarding.provider_key",
        json!({
            "tenant_id": tenant.0.to_string(),
            "workspace_id": workspace.0.to_string(),
            "provider": provider,
            "masked_key": masked,
            "test_cost_micros": validation.test_cost_micros,
            "test_tokens": validation.test_tokens,
        }),
    )
    .await;

    (
        "200 OK",
        json!({
            "validated": true,
            "provider": provider,
            "masked_key": masked,
            "test_cost": micros_to_usd(validation.test_cost_micros),
            "test_tokens": validation.test_tokens,
        })
        .to_string(),
    )
}

/// `POST /api/v1/onboarding/install-vertical` — record the operator's
/// vertical pick + report workflow/skill counts from the on-disk pack.
pub(super) async fn install_vertical_response(
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
    let vertical_id = match parsed.get("vertical_id").and_then(Value::as_str) {
        Some(s) if !s.trim().is_empty() => s.trim().to_string(),
        _ => return bad_request("`vertical_id` (non-empty string) is required"),
    };
    let (tenant, workspace) = match resolve_workspace(store, &email).await {
        Ok(pair) => pair,
        Err(e) => return map_workspace_error(e),
    };
    let Some(loaded) = load_vertical(&vertical_id) else {
        return not_found(&format!("vertical '{vertical_id}' not found"));
    };
    let workflows_added = count_workflow_files(&loaded);
    let skills_added = count_skill_files(&loaded);
    if let Err(e) = ensure_workspace_keys_schema(store.pool()).await {
        return server_error(&format!("ensure workspace_keys schema: {e}"));
    }
    if let Err(e) = record_vertical_install(
        store.pool(),
        tenant,
        workspace,
        &vertical_id,
        workflows_added,
        skills_added,
    )
    .await
    {
        return server_error(&format!("record install: {e}"));
    }
    emit_onboarding_audit(
        store,
        tenant,
        req,
        AuditKind::VerticalInstalled,
        "onboarding.install_vertical",
        json!({
            "tenant_id": tenant.0.to_string(),
            "workspace_id": workspace.0.to_string(),
            "vertical_id": vertical_id,
            "workflows_added": workflows_added,
            "skills_added": skills_added,
        }),
    )
    .await;
    (
        "200 OK",
        json!({
            "installed": true,
            "vertical_id": vertical_id,
            "workflows_added": workflows_added,
            "skills_added": skills_added,
        })
        .to_string(),
    )
}

/// `POST /api/v1/onboarding/first-hire` — spawn the workspace's first
/// worker. The handler synthesises a minimal `HireDraft` from the
/// operator-supplied description + vertical id and forwards to the
/// shared `IdentityStore::create_worker_from_draft` path used by the
/// `POST /api/v1/workers` hire-commit endpoint.
pub(super) async fn first_hire_response(
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
    let description = match parsed.get("description").and_then(Value::as_str) {
        Some(s) if !s.trim().is_empty() => s.trim().to_string(),
        _ => return bad_request("`description` (non-empty string) is required"),
    };
    let vertical_id = match parsed.get("vertical_id").and_then(Value::as_str) {
        Some(s) if !s.trim().is_empty() => s.trim().to_string(),
        _ => return bad_request("`vertical_id` (non-empty string) is required"),
    };
    let (tenant, _workspace) = match resolve_workspace(store, &email).await {
        Ok(pair) => pair,
        Err(e) => return map_workspace_error(e),
    };
    let Some(picked) = load_vertical(&vertical_id) else {
        return not_found(&format!("vertical '{vertical_id}' not found"));
    };
    let worker_name = name_from_description(&description);
    let principal_id = format!("{HUMAN_PRINCIPAL_PREFIX}{email}");
    let outcome = invoke_genesis(&description, &worker_name);
    let charter = synthesise_charter(&description, outcome.plan.as_ref());
    let duties = derive_duties(outcome.plan.as_ref(), &picked);
    let duty_count = duties.len();
    let draft = HireDraft {
        suggested_name: worker_name.clone(),
        charter: charter.clone(),
        suggested_vertical: vertical_id.clone(),
        approval_policy: ApprovalPolicyChoice {
            level: "balanced".into(),
        },
        duties,
        voice_profile: None,
        tagline: None,
    };
    let identities = IdentityStore::new(store);
    let new_id = match identities
        .create_worker_from_draft(&draft, tenant, &principal_id)
        .await
    {
        Ok(id) => id,
        Err(ArgoError::Schema(msg)) => return bad_request(&msg),
        Err(e) => return server_error(&format!("create_worker: {e}")),
    };
    let endpoint = if outcome.is_fallback() {
        "onboarding.first_hire.genesis_fallback"
    } else {
        "onboarding.first_hire.genesis_invoked"
    };
    emit_audit(
        store,
        tenant,
        req,
        endpoint,
        json!({
            "tenant_id": tenant.0.to_string(),
            "worker_id": new_id.to_string(),
            "vertical_id": vertical_id,
            "name": worker_name,
            "genesis_invoked": !outcome.is_fallback(),
            "genesis_fallback": outcome.is_fallback(),
            "plan_id": outcome.plan_id(),
            "archetype": outcome.archetype().map(|a| a.slug()),
            "charter_hash": charter_hash(&charter),
            "duty_count": duty_count,
            "fallback_reason": outcome.fallback_reason,
        }),
    )
    .await;
    (
        "201 Created",
        json!({
            "worker_id": new_id.to_string(),
            "name": worker_name,
            "vertical_id": vertical_id,
            "genesis_invoked": !outcome.is_fallback(),
            "genesis_fallback": outcome.is_fallback(),
        })
        .to_string(),
    )
}

async fn emit_onboarding_audit(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    req: &str,
    kind: AuditKind,
    endpoint: &str,
    payload: Value,
) {
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant.to_string());
    let mut body = payload;
    if let Some(obj) = body.as_object_mut() {
        obj.insert("endpoint".into(), Value::String(endpoint.into()));
    }
    if let Err(e) = sink
        .write(None, kind, body, principal_id_for_audit(req))
        .await
    {
        tracing::warn!(
            target: "gateway::onboarding",
            error = %e,
            kind = %kind.label(),
            endpoint = %endpoint,
            tenant = %tenant,
            "onboarding audit write failed"
        );
    }
}

#[cfg(test)]
#[path = "routes_onboarding_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "routes_onboarding_first_hire_genesis_tests.rs"]
mod first_hire_genesis_tests;
