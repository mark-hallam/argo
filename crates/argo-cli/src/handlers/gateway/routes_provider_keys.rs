//! `POST /api/v1/providers/keys` — store a tenant's OWN upstream
//! provider API key (BYOK, DEC-053 Enterprise tier).
//!
//! ## Why this exists
//!
//! Enterprise = Bring Your Own Key: the tenant's own Anthropic / OpenAI
//! key bills *their* provider account directly (zero token COGS). The
//! `/settings/providers` page already had a GET STATUS route
//! ([`super::routes_settings_credentials`]) and the SPA client
//! (`web/lib/api/provider-keys.ts`) already POSTs `{ provider, key }` to
//! `/api/v1/providers/keys` — but the write endpoint did not exist, so
//! the inline "Add key / Rotate" form 404'd against the live gateway.
//! This module is that write endpoint.
//!
//! ## Reuse, not fork (Parallel-surface rule)
//!
//! Credentials persist through the SAME [`argo_config::cred`] store the
//! CLI + connector flow use, via the **tenant-scoped**
//! [`argo_config::cred::set_tenant`] — so one tenant's provider key is
//! invisible to another. The transport-layer resolution that makes a
//! tenant key WIN over the platform key lives in
//! `argo_agent_runner::resolve_provider_api_key`; this route only
//! *stores* the key. The owner-RBAC gate reuses the existing
//! [`Action::ConnectorAuthorize`] capability row — storing a provider
//! credential is the same perimeter-loosening, owner-floor mutation.
//!
//! ## Safety invariants (NO THEATER · DEC-007 / DEC-029)
//!
//! - **Owner only** — a non-owner is refused 403 and audited.
//! - **Tenant-scoped** — the key is written under
//!   `[credentials.tenant.<tid>]`, never the process-global root.
//! - **Secret discipline** — the key arrives in the POST body (NEVER a
//!   query param), is handed straight to the credential store, and is
//!   NEVER echoed in the response or the audit payload. The response is
//!   metadata only (`configured: true`, `masked_preview: null`); the
//!   audit row records THAT a provider key was set + by whom, not the
//!   value.
//! - **CSRF + identity perimeter** — enforced centrally upstream of
//!   dispatch (`middleware_csrf` + `require_identity` in `routes.rs`).

use super::role_gate::{enforce_role, resolve_role, Action};
use super::routes_auth_principal::session_principal_id_for_audit;
use super::{body_from_request, store_unavailable};
use argo_audit::{AuditKind, AuditSink};
use argo_identity::user_tenant::UserTenantRole;
use argo_store::SessionStore;
use argo_types::TenantId;
use serde_json::{json, Value};
use std::sync::Arc;

const PROVIDER_KEYS_PATH: &str = "/api/v1/providers/keys";

/// The providers the `/settings/providers` page renders, paired with
/// the conventional credential env each uses. Mirrors the GET-status
/// route's `PROVIDER_ENV` and the SPA's `PROVIDER_OPTIONS`. Only
/// API-key families are BYOK-storable here; `bedrock` authenticates via
/// the AWS credential chain, so it is intentionally absent.
const PROVIDER_ENV: &[(&str, &str)] = &[
    ("anthropic", "ANTHROPIC_API_KEY"),
    ("openai", "OPENAI_API_KEY"),
    ("google", "GOOGLE_API_KEY"),
    ("azure", "AZURE_OPENAI_API_KEY"),
];

/// Dispatcher entry point. Returns `Some(resp)` for the provider-key
/// SET POST, `None` otherwise so the caller falls through.
pub(super) async fn dispatch_post(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    let rest = req.strip_prefix("POST ")?;
    let path = rest.split_whitespace().next().unwrap_or("");
    let path = path.split('?').next().unwrap_or(path);
    if path != PROVIDER_KEYS_PATH {
        return None;
    }
    Some(set_response(req, store).await)
}

/// `POST /api/v1/providers/keys` — store the tenant-scoped provider key.
/// Owner-only. Returns `200 { provider, configured, masked_preview,
/// updated_at }` — never the secret.
async fn set_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let (tenant, principal) = match resolve(store, req).await {
        Ok(pair) => pair,
        Err(resp) => return resp,
    };
    // Owner-only — refuse + audit before touching any credential store.
    if let Some(resp) = enforce_role(req, Action::ConnectorAuthorize) {
        let role = resolve_role(req).unwrap_or(UserTenantRole::Guest);
        emit_audit(
            store,
            tenant,
            &principal,
            AuditKind::RbacDeny,
            json!({
                "endpoint": "settings.providers",
                "event_type": "provider_key.set_denied",
                "role": role.as_str(),
            }),
        )
        .await;
        return resp;
    }
    let (provider, env_key, key) = match parse_set(req) {
        Ok(triple) => triple,
        Err(resp) => return resp,
    };
    let workspace = match crate::handlers::connector_authorize::workspace_dir() {
        Ok(w) => w,
        Err(e) => return server_error(&format!("resolve workspace: {e}")),
    };
    // Tenant-scoped write — `[credentials.tenant.<tid>]`, never the
    // process-global root section.
    if let Err(e) = argo_config::cred::set_tenant(&workspace, &tenant, env_key, &key) {
        return server_error(&format!("store provider key: {e}"));
    }
    emit_audit(
        store,
        tenant,
        &principal,
        AuditKind::HttpRequest,
        json!({
            "endpoint": "settings.providers",
            "event_type": "provider_key.set",
            "provider": provider,
            "credential_key": env_key,
        }),
    )
    .await;
    (
        "200 OK",
        json!({
            "provider": provider,
            "configured": true,
            // Never emit a secret-derived value, even masked.
            "masked_preview": Value::Null,
            "updated_at": chrono::Utc::now().to_rfc3339(),
        })
        .to_string(),
    )
}

/// Parse + validate the `{ provider, key }` body. Returns the canonical
/// provider id, its credential env-var name, and the trimmed secret.
/// The secret is NEVER logged — only emptiness is checked.
fn parse_set(req: &str) -> Result<(&'static str, &'static str, String), (&'static str, String)> {
    let body = body_from_request(req).trim();
    if body.is_empty() {
        return Err(bad_request("missing JSON body with `provider` and `key`"));
    }
    let parsed: Value =
        serde_json::from_str(body).map_err(|e| bad_request(&format!("invalid JSON body: {e}")))?;
    let provider = parsed
        .get("provider")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| bad_request("`provider` (non-empty string) is required"))?;
    let (canonical, env_key) = PROVIDER_ENV
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(provider))
        .ok_or_else(|| {
            bad_request(&format!(
                "unknown provider '{provider}': expected one of anthropic, openai, google, azure"
            ))
        })?;
    let key = parsed
        .get("key")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| bad_request("`key` (non-empty string) is required"))?
        .to_string();
    Ok((canonical, env_key, key))
}

/// Resolve `(tenant, principal)` for the caller. Tenant is identity-first
/// (`__Host-session` JWT → explicit override → cwd workspace) via the
/// shared [`super::resolve_tenant::resolve_tenant_for_request`]; principal
/// is the verified audit principal (falls back to a system sentinel).
async fn resolve(
    store: &Arc<SessionStore>,
    req: &str,
) -> Result<(TenantId, String), (&'static str, String)> {
    let principal =
        session_principal_id_for_audit(req).unwrap_or_else(|| "system:provider-keys".to_string());
    let tenant = super::resolve_tenant::resolve_tenant_for_request(req, store)
        .await
        .map_err(|e| server_error(&format!("resolve tenant: {e}")))?;
    Ok((tenant, principal))
}

/// Best-effort tenant-scoped audit write. A sink failure is logged,
/// never echoed (the row carries no secret regardless).
async fn emit_audit(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    principal: &str,
    kind: AuditKind,
    payload: Value,
) {
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant.to_string());
    if let Err(e) = sink
        .write(None, kind, payload, Some(principal.to_string()))
        .await
    {
        tracing::warn!(
            target: "gateway::provider_keys",
            error = %e,
            "provider-key audit write failed"
        );
    }
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
        json!({ "error": { "code": code, "message": message } }).to_string(),
    )
}

#[cfg(test)]
#[path = "routes_provider_keys_tests.rs"]
mod tests;
