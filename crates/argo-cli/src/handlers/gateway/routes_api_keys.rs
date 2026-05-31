//! First-class `/settings/api-keys` routes (Frontier-B depth, SPEC 2) —
//! the named, scoped, tenant-isolated API-key surface backing the SPA's
//! `web/lib/api/api-keys.ts` client.
//!
//! Replaces the prior hard-coded `200 []` stub with a real
//! `argo_store::api_key`-backed store (migration v86):
//!
//! * `GET    /api/v1/api-keys`        — tenant-scoped roster (metadata
//!   only; NEVER a token or token-hash).
//! * `POST   /api/v1/api-keys`        — create: mint a token, hash it,
//!   store the hash + metadata in `api_key`, ALSO write a
//!   `service_token_binding` row keyed on the same hash so the token
//!   authenticates, and return the full token EXACTLY ONCE.
//! * `DELETE /api/v1/api-keys/<id>`   — revoke: tenant-verified, so a
//!   caller can never revoke another tenant's key.
//!
//! Secret discipline (NO THEATER): the raw token exists only inside
//! [`create_response`], is hashed immediately, and is returned to the
//! caller once. It is never logged, never stored, and never on a
//! list/get row. The masked `prefix` is computed at create time and
//! stored as plain metadata. Helpers (mint/hash/parse/audit/envelopes)
//! live in `routes_api_keys_helpers.rs` to keep this file lean.

use super::routes_api_keys_helpers as h;
use argo_store::api_key::{self, ApiKeyInsert};
use argo_store::service_token_binding;
use argo_store::SessionStore;
use serde_json::json;
use std::sync::Arc;

/// `GET /api/v1/api-keys` — tenant-scoped key roster, metadata only.
/// Returns a bare JSON array (`ApiKeyRow[]`) to match the SPA client.
/// Never returns a token or token-hash.
pub(super) async fn list_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let (tenant, _principal) = match h::resolve_tenant_principal(store, req).await {
        Ok(pair) => pair,
        Err(e) => return h::server_error(&format!("resolve tenant: {e}")),
    };
    let rows = match api_key::list_for_tenant(store.pool(), &tenant.to_string()).await {
        Ok(rows) => rows,
        Err(e) => return h::server_error(&format!("list keys: {e}")),
    };
    let items: Vec<serde_json::Value> = rows.iter().map(h::row_to_json).collect();
    ("200 OK", serde_json::Value::Array(items).to_string())
}

/// `POST /api/v1/api-keys` — create one named, scoped key. Returns
/// `201 { key: ApiKeyRow, token }` where `token` is the full bearer
/// shown EXACTLY ONCE.
pub(super) async fn create_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let create = match h::parse_create(req) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let (tenant, principal) = match h::resolve_tenant_principal(store, req).await {
        Ok(pair) => pair,
        Err(e) => return h::server_error(&format!("resolve tenant: {e}")),
    };

    // Mint → hash → mask. The raw token never leaves this fn except in
    // the single response below.
    let token = h::mint_token();
    let token_hash = h::sha256_hex(&token);
    let prefix = h::masked_prefix(&token);
    let id = h::new_key_id();
    let now = chrono::Utc::now().to_rfc3339();
    let tenant_str = tenant.to_string();
    let scopes_json = serde_json::to_string(&create.scopes).unwrap_or_else(|_| "[]".to_string());
    let cohort_ids_json =
        serde_json::to_string(&create.cohort_ids).unwrap_or_else(|_| "[]".to_string());

    if let Err(e) = api_key::insert(
        store.pool(),
        &ApiKeyInsert {
            id: &id,
            tenant_id: &tenant_str,
            name: &create.name,
            token_hash: &token_hash,
            prefix: &prefix,
            scopes_json: &scopes_json,
            cohort_ids_json: &cohort_ids_json,
            all_cohorts: create.all_cohorts,
            created_at: &now,
            expires_at: create.expires_at.as_deref(),
        },
    )
    .await
    {
        return h::server_error(&format!("store key: {e}"));
    }

    // ALSO bind the token hash → (principal, tenant) so the credential
    // authenticates through the existing gateway bearer path. The
    // principal is the key's id so audit traces the acting key.
    let key_principal = format!("apikey:{id}");
    if let Err(e) = service_token_binding::insert_binding(
        store.pool(),
        &token_hash,
        &key_principal,
        &tenant_str,
        Some(&create.name),
        &now,
    )
    .await
    {
        return h::server_error(&format!("bind token: {e}"));
    }

    h::emit_audit(
        store,
        tenant,
        &principal,
        "api_key.created",
        json!({ "key_id": id, "name": create.name, "all_cohorts": create.all_cohorts }),
    )
    .await;

    // Read the row straight back so the response shape is exactly what a
    // subsequent list returns (single source of serialization truth).
    let key_json = match api_key::get_for_tenant(store.pool(), &tenant_str, &id).await {
        Ok(Some(row)) => h::row_to_json(&row),
        Ok(None) => return h::server_error("created key vanished on readback"),
        Err(e) => return h::server_error(&format!("readback key: {e}")),
    };
    (
        "201 Created",
        json!({ "key": key_json, "token": token }).to_string(),
    )
}

/// `DELETE /api/v1/api-keys/<id>` — revoke one key. Tenant-verified:
/// the `(tenant, id)` scoped UPDATE matches no row for a foreign tenant,
/// so a cross-tenant revoke 404s rather than touching another tenant's
/// key. Returns `200 { ok: true, id }` to match the SPA client.
pub(super) async fn revoke_response(
    id: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let (tenant, principal) = match h::resolve_tenant_principal(store, req).await {
        Ok(pair) => pair,
        Err(e) => return h::server_error(&format!("resolve tenant: {e}")),
    };
    let tenant_str = tenant.to_string();
    match api_key::mark_revoked(store.pool(), &tenant_str, id).await {
        Ok(true) => {}
        // No row for this (tenant, id) — unknown id OR a cross-tenant
        // attempt. Both surface as 404; a foreign tenant's key is never
        // revealed as "exists but forbidden".
        Ok(false) => return h::not_found(id),
        Err(e) => return h::server_error(&format!("revoke key: {e}")),
    }
    h::emit_audit(
        store,
        tenant,
        &principal,
        "api_key.revoked",
        json!({ "key_id": id }),
    )
    .await;
    ("200 OK", json!({ "ok": true, "id": id }).to_string())
}

/// DELETE dispatcher helper: extract `<id>` from
/// `DELETE /api/v1/api-keys/<id>` and route to [`revoke_response`].
/// Returns `None` for any non-matching path so the caller falls through.
pub(super) async fn dispatch_delete(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    let rest = req.strip_prefix("DELETE /api/v1/api-keys/")?;
    let head = rest.split_whitespace().next().unwrap_or("");
    let id = head.split('?').next().unwrap_or(head);
    if id.is_empty() || id.contains('/') {
        return None;
    }
    Some(revoke_response(id, req, store).await)
}

/// POST dispatcher helper: route an exact `POST /api/v1/api-keys` to
/// [`create_response`]. Returns `None` otherwise.
pub(super) async fn dispatch_post(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    if req.starts_with("POST /api/v1/api-keys ") || req.starts_with("POST /api/v1/api-keys?") {
        return Some(create_response(req, store).await);
    }
    None
}

#[cfg(test)]
#[path = "routes_api_keys_tests.rs"]
mod tests;
