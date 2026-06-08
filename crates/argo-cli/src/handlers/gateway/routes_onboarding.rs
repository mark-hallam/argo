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
//! storage. When no workspace root key is configured (dev /
//! fresh-checkout flow), the handler returns a friendly 503 and lets
//! the operator continue with the mock provider rather than leaking
//! key-source details in a raw 500.
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
    build_first_hire_draft, charter_hash, enforce_first_hire_gate, invoke_genesis,
};
use super::routes_onboarding_helpers::{
    bad_request, count_approval_gates, count_skill_files, count_workflow_files, emit_audit,
    infer_provider, is_workspace_root_key_error, load_vertical, map_workspace_error,
    mask_key, memory_schema_attached, micros_to_usd, name_from_description, not_found,
    principal_email, provider_key_storage_error, provider_validation_unconfigured,
    resolve_workspace, server_error, unauthorized, validate_key_shape, wrap_secret,
    HUMAN_PRINCIPAL_PREFIX,
};
use super::routes_onboarding_idempotency;
use super::routes_onboarding_live_key::LiveKeyOutcome;
use super::store_unavailable;
use argo_audit::{AuditKind, AuditSink};
use argo_identity::IdentityStore;
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

    // TIER GATE (sprint-ws4-3). Free tier is mock-only: it lacks the
    // `provider_real_keys` feature, so reject the real-key save with a
    // 403 upsell rather than let a Free tenant persist + later dispatch
    // through a real provider. This is the runtime reader that closes
    // the WS5-4 "Free→mock is SOFT" finding.
    if !super::tier_gate::tier_allows(store, tenant, "provider_real_keys").await {
        let tier = super::tier_gate::tenant_tier_or_free(store, tenant).await;
        return super::tier_gate::tier_forbidden_response(tier, "provider_real_keys");
    }

    let validation = validate_key_shape(&provider, &api_key);
    if !validation.ok {
        return bad_request(&validation.reason);
    }
    // Bounded live one-token validation against the REAL provider (gated
    // off by default; skipped in mock/test + for unmapped providers). NO
    // THEATER: a clear 401/403 rejects the save (`routes_onboarding_live_key`).
    if let LiveKeyOutcome::Invalid(reason) =
        super::routes_onboarding_live_key::validate_key_live(&provider, &api_key).await
    {
        return bad_request(&reason);
    }
    let masked = mask_key(&api_key);
    let ciphertext = match wrap_secret(&api_key, tenant) {
        Ok(c) => c,
        Err(e) if is_workspace_root_key_error(&e) => return provider_validation_unconfigured(),
        Err(e) => {
            tracing::warn!(
                target: "gateway::onboarding",
                error = %e,
                tenant = %tenant,
                "provider key wrap failed"
            );
            return provider_key_storage_error();
        }
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
    // Wave-T0119 receipt expansion: the wizard sub-copy promises
    // "skills, memory schema, and approval gates" — the operator's
    // receipt was confirming only skills. These two flags surface the
    // other two so the receipt matches the promise (or makes the gap
    // explicit when a pack ships without them).
    let memory_schema_attached = memory_schema_attached(&loaded);
    let approval_gates_added = count_approval_gates(&loaded);
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
            "memory_schema_attached": memory_schema_attached,
            "approval_gates_added": approval_gates_added,
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
            "memory_schema_attached": memory_schema_attached,
            "approval_gates_added": approval_gates_added,
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
    // The wizard now ships an explicit `name` field (F13 fix). When the
    // SPA supplies a non-empty trimmed name we trust it verbatim; the
    // first-capitalised-token heuristic is reserved for older clients
    // that still POST `{description, vertical_id}` without a `name`.
    let supplied_name = parsed
        .get("name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    // `resolve_workspace` still enforces the 401 / 412 provisioning gate
    // (the user must have provisioned before they can hire). But the
    // WRITE tenant is now the SAME canonical tenant the JWT-aware reads
    // resolve since #2676: `resolve_tenant_for_request` prefers the
    // `__Host-session` JWT's `tnt` claim (== the provisioned tenant).
    // Pre-#2676 this path deliberately wrote to the cwd tenant to match
    // the then-cwd-based reads; now that `/api/v1/team` &c. key off the
    // JWT tenant, a cwd write would orphan the hire — A hires, A's /team
    // shows empty. The anonymous/demo path (no session cookie) still
    // falls through to the cwd tenant inside the resolver, so the demo
    // walkthrough is unchanged.
    if let Err(e) = resolve_workspace(store, &email).await {
        return map_workspace_error(e);
    }
    let tenant = match super::resolve_tenant::resolve_tenant_for_request(req, store).await {
        Ok(t) => t,
        Err(e) => return server_error(&format!("resolve write tenant: {e}")),
    };
    let Some(picked) = load_vertical(&vertical_id) else {
        return not_found(&format!("vertical '{vertical_id}' not found"));
    };
    let worker_name = supplied_name.unwrap_or_else(|| name_from_description(&description));
    let principal_id = format!("{HUMAN_PRINCIPAL_PREFIX}{email}");
    let outcome = invoke_genesis(&description, &worker_name);
    // The operator's role-contract confirmation digest (DEC-025 #1 + #4),
    // echoed back from the contract this seam surfaces. `None` → unconfirmed →
    // the gate holds in `needs_review`, so the first hire is refused without a
    // confirmed contract. Accepts either field name (SPA / SDK parity).
    let confirmed_digest = ["confirmed_contract_digest", "contract_digest"]
        .iter()
        .find_map(|field| {
            parsed
                .get(*field)
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
        });

    // SAFETY GATE (DEC-025 / Genesis P0 + P4). `first-hire` is a Genesis-driven
    // hire seam — it synthesises an in-memory `GenesisPlan` and commits a live
    // worker — so it must clear the SAME fail-closed gate the SSE path
    // (`routes_genesis_stream`) and the hire-commit fence
    // (`POST /api/v1/workers`) enforce, BEFORE `create_worker_from_draft`. A
    // failing/thin/unsafe teammate (or a `plan()`-fail with no plan to grade),
    // OR one whose role contract the operator has not confirmed, is refused and
    // never activated. The on-disk fence in `routes_workers_hire_gate` cannot
    // catch this seam — first-hire's plan is in-memory only (no `.genesis/`
    // dir) and uses a catalog `vertical_id` — which is exactly the bypass this
    // closes. Extracted to a sibling helper to keep this file under the
    // per-file LOC budget.
    if let Err(blocked) = enforce_first_hire_gate(
        store,
        tenant,
        req,
        &outcome,
        &vertical_id,
        &worker_name,
        confirmed_digest.as_deref(),
    )
    .await
    {
        return blocked;
    }
    // Gate cleared. The `plan()`-fail fallback also returned above, so
    // `outcome.plan` is `Some` here; treat a `None` as an internal error
    // rather than panic (the gate's invariant makes it unreachable).
    let Some(plan) = outcome.plan.as_ref() else {
        return server_error("first-hire gate passed without a Genesis plan");
    };

    // Assemble the draft (charter + duties + persona overrides) in the
    // genesis sibling so this handler stays under the 500-LOC budget. The
    // returned `charter` + `duty_count` feed the audit row below.
    let assembled = build_first_hire_draft(plan, &picked, &description, &worker_name, &vertical_id);
    let draft = &assembled.draft;
    let charter = assembled.charter.as_str();
    let duty_count = assembled.duty_count;
    // DEFENSE-IN-DEPTH IDEMPOTENCY (mirrors `POST /api/v1/workers`, shares
    // its cache). Only the CONFIRM POST reaches here — the needs_review POST
    // returned 422 from `enforce_first_hire_gate` above and created no worker,
    // so we never record a 422 (which would wrongly short-circuit the later
    // confirm). The SPA carries the SAME `idempotency_key` on both submit and
    // confirm, so two tabs / a double-fired confirm collapse to one teammate.
    let idem = routes_onboarding_idempotency::IdemTicket::open(
        &parsed,
        &worker_name,
        &description,
        &vertical_id,
        confirmed_digest.as_deref(),
    );
    if let Some(short_circuit) = idem.guard(tenant, &worker_name, &vertical_id) {
        return short_circuit;
    }

    let identities = IdentityStore::new(store);
    let new_id = match identities
        .create_worker_from_draft(draft, tenant, &principal_id)
        .await
    {
        Ok(id) => id,
        Err(ArgoError::Schema(msg)) => return bad_request(&msg),
        Err(e) => return server_error(&format!("create_worker: {e}")),
    };
    idem.record(tenant, new_id);
    // The fallback (`plan()`-fail) path returned `422` above before reaching
    // here, so a worker is only ever committed on the gate-cleared Genesis
    // path — `genesis_invoked` is unconditionally `true` and there is no
    // fallback worker to audit.
    emit_audit(
        store,
        tenant,
        req,
        "onboarding.first_hire.genesis_invoked",
        json!({
            "tenant_id": tenant.0.to_string(),
            "worker_id": new_id.to_string(),
            "vertical_id": vertical_id,
            "name": worker_name,
            "genesis_invoked": true,
            "genesis_fallback": false,
            "plan_id": plan.vertical_id,
            "archetype": plan.archetype.slug(),
            "charter_hash": charter_hash(charter),
            "duty_count": duty_count,
        }),
    )
    .await;
    (
        "201 Created",
        json!({
            "worker_id": new_id.to_string(),
            "name": worker_name,
            "vertical_id": vertical_id,
            "genesis_invoked": true,
            "genesis_fallback": false,
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

#[cfg(test)]
#[path = "routes_onboarding_first_hire_contract_tests.rs"]
mod first_hire_contract_tests;

#[cfg(test)]
#[path = "routes_onboarding_first_hire_second_call_tests.rs"]
mod first_hire_second_call_tests;

#[cfg(test)]
#[path = "routes_onboarding_first_hire_idem_tests.rs"]
mod first_hire_idem_tests;

#[cfg(test)]
#[path = "routes_onboarding_install_vertical_receipt_tests.rs"]
mod install_vertical_receipt_tests;
