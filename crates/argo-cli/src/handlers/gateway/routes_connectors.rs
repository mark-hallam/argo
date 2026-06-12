//! `GET /api/v1/connectors` + `POST /api/v1/connectors/<id>/{authorize,credentials}`
//! — the gateway (HTTP) face of the connector-authorize subsystem that was
//! **CLI-only** until now (tg2, operator-directed tiered-gating · Tier-2).
//!
//! ## Why this exists
//!
//! The token/API-key connector flow already shipped, but only as
//! `argo connector authorize <kind>` on the command line
//! (`crate::handlers::connector_authorize`). The locked grant card's
//! **Connect** button — and the `/settings/connectors` catalog — had no HTTP
//! endpoint, so a non-technical operator could not connect a tool without a
//! terminal. This module exposes the same authorize over the gateway.
//!
//! ## Reuse, not fork (Parallel-surface rule)
//!
//! The connector spec table, the per-connector `token_key`, and the auth-kind
//! all come from the EXISTING [`crate::handlers::connector_authorize`] module
//! ([`CONNECTORS`], [`spec_for`], [`AuthKind`]) — this route never re-declares
//! them. Credentials persist through the SAME [`argo_config::cred`] store the
//! CLI uses, but via the **tenant-scoped** [`argo_config::cred::set_tenant`]
//! and a per-tenant read ([`argo_config::cred::load_tenant`]) rather than the
//! CLI's process-global root section — so one tenant's Slack token is invisible
//! to another.
//!
//! ## Safety invariants
//!
//! - **Owner only** ([`Action::ConnectorAuthorize`]) — storing a provider
//!   credential loosens the perimeter for that tenant, as sensitive as the
//!   other owner-floor mutations. A non-owner is refused 403 and audited.
//! - **Tenant-scoped, fail-closed** — the credential is written under
//!   `[credentials.tenant.<tid>]`; the status read consults ONLY that tenant's
//!   section (never the shared `auth.json` state file or process env), so a
//!   second tenant always reads the connector as unauthorized.
//! - **Secret discipline (NO THEATER · DEC-007)** — the secret arrives in the
//!   POST body (NEVER a query param), is handed straight to the credential
//!   store, and is NEVER echoed in the response or the audit payload. The
//!   audit row records THAT `<connector>` was authorized + by whom, not the
//!   secret.
//! - **CSRF + identity perimeter** — enforced centrally upstream of dispatch
//!   (`middleware_csrf::check_csrf` + `require_identity` in `routes.rs`); no
//!   per-route work needed here.
//! - **OAuth connectors are refused (409)** — the OAuth dance needs an
//!   interactive browser-redirect loop a single credential POST can't carry;
//!   it stays on the CLI / `window.open` connect flow until a dedicated
//!   gateway redirect route ships.

use super::role_gate::{enforce_role, resolve_role, Action};
use super::routes_auth_principal::session_principal_id_for_audit;
use super::{body_from_request, store_unavailable};
use crate::handlers::connector_authorize::{spec_for, workspace_dir, AuthKind, ConnectorSpec, CONNECTORS};
use argo_audit::{AuditKind, AuditSink};
use argo_identity::user_tenant::UserTenantRole;
use argo_store::SessionStore;
use argo_types::TenantId;
use serde_json::{json, Value};
use std::sync::Arc;

/// Route `POST /api/v1/connectors/<id>/authorize` and the SPA's API-key
/// alias `POST /api/v1/connectors/<id>/credentials` to [`authorize_response`].
/// Returns `None` for any non-matching path so the caller falls through.
///
/// Both verbs carry the same semantics (store a tenant-scoped credential for
/// the connector); `/credentials` is the path the shipped
/// `web/lib/api/connectors.ts` `saveTenantConnectorCredential` already calls,
/// `/authorize` is the name the brief specifies — supporting both makes the
/// locked UI work without a client change.
pub(super) async fn dispatch_post(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    let rest = req.strip_prefix("POST /api/v1/connectors/")?;
    let head = rest.split_whitespace().next().unwrap_or("");
    let head = head.split('?').next().unwrap_or(head);
    let (id, action) = head.split_once('/')?;
    if id.is_empty() || (action != "authorize" && action != "credentials") {
        return None;
    }
    Some(authorize_response(id, req, store).await)
}

/// Route the exact `GET /api/v1/connectors` (no sub-path) to [`list_response`].
/// The trailing-space / `?` guard keeps the worker-scoped
/// `GET /api/v1/workers/<id>/connectors/...` family from matching here.
pub(super) async fn dispatch_get(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    if req.starts_with("GET /api/v1/connectors ") || req.starts_with("GET /api/v1/connectors?") {
        return Some(list_response(req, store).await);
    }
    None
}

/// `POST /api/v1/connectors/<id>/authorize` — store the tenant-scoped
/// credential for one token/API-key connector. Owner-only; OAuth connectors
/// are refused. Returns `200 { connectorId, ok, status, credential_key }` —
/// never the secret.
async fn authorize_response(
    kind: &str,
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
                "endpoint": "settings.connectors",
                "event_type": "connector.authorize_denied",
                "connector": kind,
                "role": role.as_str(),
            }),
        )
        .await;
        return resp;
    }
    let spec = match spec_for(kind) {
        Ok(spec) => spec,
        Err(_) => return not_found(&format!("unknown connector '{kind}'")),
    };
    if spec.auth == AuthKind::OAuth {
        return oauth_not_supported(kind);
    }
    let credential = match parse_credential(req, spec) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    let workspace = match workspace_dir() {
        Ok(w) => w,
        Err(e) => return server_error(&format!("resolve workspace: {e}")),
    };
    // Tenant-scoped write — `[credentials.tenant.<tid>]`, never the CLI's
    // process-global root section.
    if let Err(e) = argo_config::cred::set_tenant(&workspace, &tenant, spec.token_key, &credential) {
        return server_error(&format!("store credential: {e}"));
    }
    emit_audit(
        store,
        tenant,
        &principal,
        AuditKind::HttpRequest,
        json!({
            "endpoint": "settings.connectors",
            "event_type": "connector.authorized",
            "connector": kind,
            "credential_key": spec.token_key,
        }),
    )
    .await;
    (
        "200 OK",
        json!({
            "connectorId": kind,
            "ok": true,
            "status": "connected",
            "credential_key": spec.token_key,
        })
        .to_string(),
    )
}

/// `GET /api/v1/connectors` — tenant-scoped roster: every known connector with
/// a `connected` / `not_connected` status, derived ONLY from this tenant's
/// credential section. Shape matches `TenantConnectorListResponse` in
/// `web/lib/api/connectors.ts`.
async fn list_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let tenant = match super::resolve_tenant::resolve_tenant_for_request(req, store).await {
        Ok(t) => t,
        Err(e) => return server_error(&format!("resolve tenant: {e}")),
    };
    let workspace = match workspace_dir() {
        Ok(w) => w,
        Err(e) => return server_error(&format!("resolve workspace: {e}")),
    };
    // Per-tenant section ONLY — no env-var precedence, no legacy-root fallback,
    // so another tenant's (or the process's) credential can never read as
    // connected for this tenant.
    let creds = argo_config::cred::load_tenant(&workspace, &tenant).unwrap_or_default();
    let rows: Vec<Value> = CONNECTORS
        .iter()
        .map(|spec| {
            let status = if creds.contains_key(spec.token_key) {
                "connected"
            } else {
                "not_connected"
            };
            json!({ "connectorId": spec.kind, "status": status })
        })
        .collect();
    ("200 OK", json!({ "connectors": rows }).to_string())
}

/// Resolve `(tenant, principal)` for the caller. Tenant is identity-first
/// (`__Host-session` JWT → explicit override → cwd workspace) via the shared
/// [`super::resolve_tenant::resolve_tenant_for_request`]; principal is the
/// verified audit principal (falls back to a system sentinel).
async fn resolve(
    store: &Arc<SessionStore>,
    req: &str,
) -> Result<(TenantId, String), (&'static str, String)> {
    let principal =
        session_principal_id_for_audit(req).unwrap_or_else(|| "system:connectors".to_string());
    let tenant = super::resolve_tenant::resolve_tenant_for_request(req, store)
        .await
        .map_err(|e| server_error(&format!("resolve tenant: {e}")))?;
    Ok((tenant, principal))
}

/// Extract the connector secret from the POST body. Accepts the connector's
/// canonical `token_key` field (the shape `saveTenantConnectorCredential`
/// sends) and the common aliases, so both the `/credentials` flat-field client
/// and the brief's `{ "credential": "..." }` shape work. The value is NEVER
/// logged — only its presence is checked.
fn parse_credential(req: &str, spec: &ConnectorSpec) -> Result<String, (&'static str, String)> {
    let body = body_from_request(req).trim();
    if body.is_empty() {
        return Err(bad_request("missing JSON body with the connector credential"));
    }
    let parsed: Value = serde_json::from_str(body)
        .map_err(|e| bad_request(&format!("invalid JSON body: {e}")))?;
    let candidates = [
        spec.token_key,
        "token",
        "api_key",
        "apiKey",
        "credential",
        "secret",
        "value",
    ];
    for key in candidates {
        if let Some(v) = parsed.get(key).and_then(Value::as_str) {
            let v = v.trim();
            if !v.is_empty() {
                return Ok(v.to_string());
            }
        }
    }
    Err(bad_request(&format!(
        "missing credential: provide '{}' (or 'token' / 'api_key' / 'credential') as a non-empty string in the body",
        spec.token_key
    )))
}

/// Best-effort tenant-scoped audit write. A sink failure is logged, never
/// echoed (the row carries no secret regardless).
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
            target: "gateway::connectors",
            error = %e,
            "connector audit write failed"
        );
    }
}

fn oauth_not_supported(kind: &str) -> (&'static str, String) {
    (
        "409 Conflict",
        json!({ "error": {
            "code": "oauth_not_supported_over_http",
            "message": format!(
                "'{kind}' authenticates via OAuth; complete it through the browser \
                 connect flow, not a credential POST"
            ),
        }})
        .to_string(),
    )
}

fn bad_request(message: &str) -> (&'static str, String) {
    error_envelope("400 Bad Request", "bad_request", message)
}

fn not_found(message: &str) -> (&'static str, String) {
    error_envelope("404 Not Found", "not_found", message)
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
#[path = "routes_connectors_tests.rs"]
mod tests;
