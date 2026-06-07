//! Helpers for the first-class `/settings/api-keys` routes
//! (`routes_api_keys.rs`, Frontier-B depth SPEC 2) — split out so the
//! dispatch file stays well under the 500-LOC budget.
//!
//! Carries: secure token minting + SHA-256 hashing + masked-prefix
//! derivation, the create-request body parser, expiry→ISO resolution,
//! tenant + principal resolution, row→JSON serialization (metadata
//! only), audit emission, and the shared error envelopes.
//!
//! **Secret discipline (NO THEATER):** the raw token is minted once in
//! [`mint_token`], hashed immediately, and only ever returned to the
//! caller in the create response — it is never stored, logged, or put on
//! a list/get row. [`row_to_json`] projects metadata only; the masked
//! `prefix` was computed at create time and stored as plain text, never
//! re-derived from a secret on read.

use super::body_from_request;
use super::routes_auth_principal::principal_id_for_audit;
use argo_audit::{AuditKind, AuditSink};
use argo_store::api_key::ApiKeyRow;
use argo_store::SessionStore;
use argo_types::{ArgoResult, TenantId};
use base64::Engine;
use rand_core::{OsRng, RngCore};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use uuid::Uuid;

/// Max accepted `name` length — keeps the roster legible and the column
/// bounded.
pub(super) const NAME_MAX_LEN: usize = 120;

/// Wire labels the SPA's `ApiKeyScope` union may send. The create route
/// rejects any scope outside this set so a stored key can never carry a
/// capability the product does not define.
pub(super) const ALLOWED_SCOPES: &[&str] = &[
    "read:workers",
    "write:workers",
    "read:workflows",
    "write:workflows",
    "read:audit",
    "write:audit",
    "read:skills",
    "write:skills",
    "read:members",
    "write:members",
    "run:workflows",
];

/// A validated create request, parsed from the POST body.
#[derive(Debug)]
pub(super) struct CreateRequest {
    pub name: String,
    pub scopes: Vec<String>,
    pub cohort_ids: Vec<String>,
    pub all_cohorts: bool,
    /// Resolved ISO-8601 expiry, or `None` for a non-expiring key.
    pub expires_at: Option<String>,
}

/// Parse + validate `CreateApiKeyRequest` (`web/lib/api/api-keys.ts`).
/// Returns a `400` envelope on any shape error.
pub(super) fn parse_create(req: &str) -> Result<CreateRequest, (&'static str, String)> {
    let parsed: Value = serde_json::from_str(body_from_request(req))
        .map_err(|e| bad_request(&format!("parse body: {e}")))?;
    let name = parsed
        .get("name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| bad_request("`name` (non-empty string) is required"))?
        .to_string();
    if name.len() > NAME_MAX_LEN {
        return Err(bad_request(&format!("`name` must be ≤ {NAME_MAX_LEN} chars")));
    }
    let scopes = parse_string_array(parsed.get("scopes"), "scopes")?;
    for s in &scopes {
        if !ALLOWED_SCOPES.contains(&s.as_str()) {
            return Err(bad_request(&format!("unknown scope '{s}'")));
        }
    }
    let cohort_ids = parse_string_array(parsed.get("cohort_ids"), "cohort_ids")?;
    let all_cohorts = parsed
        .get("all_cohorts")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let expires_at = resolve_expiry(parsed.get("expiry"))?;
    Ok(CreateRequest {
        name,
        scopes,
        cohort_ids,
        all_cohorts,
        expires_at,
    })
}

/// Parse a JSON array of strings; an absent value is treated as empty.
/// A present-but-non-array, or a non-string element, is a `400`.
fn parse_string_array(
    value: Option<&Value>,
    field: &str,
) -> Result<Vec<String>, (&'static str, String)> {
    let Some(v) = value else {
        return Ok(Vec::new());
    };
    if v.is_null() {
        return Ok(Vec::new());
    }
    let arr = v
        .as_array()
        .ok_or_else(|| bad_request(&format!("`{field}` must be an array of strings")))?;
    let mut out = Vec::with_capacity(arr.len());
    for el in arr {
        let s = el
            .as_str()
            .ok_or_else(|| bad_request(&format!("`{field}` entries must be strings")))?;
        out.push(s.to_string());
    }
    Ok(out)
}

/// Resolve the SPA's `ApiKeyExpiry` (`"never" | "30d" | "90d" | "1y" |
/// { custom: ISO }`) to an absolute ISO-8601 timestamp (or `None`).
fn resolve_expiry(value: Option<&Value>) -> Result<Option<String>, (&'static str, String)> {
    let now = chrono::Utc::now();
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => match s.as_str() {
            "never" => Ok(None),
            "30d" => Ok(Some((now + chrono::Duration::days(30)).to_rfc3339())),
            "90d" => Ok(Some((now + chrono::Duration::days(90)).to_rfc3339())),
            "1y" => Ok(Some((now + chrono::Duration::days(365)).to_rfc3339())),
            other => Err(bad_request(&format!("unknown expiry '{other}'"))),
        },
        Some(Value::Object(map)) => map
            .get("custom")
            .and_then(Value::as_str)
            .map(|s| Some(s.to_string()))
            .ok_or_else(|| bad_request("`expiry.custom` must be an ISO-8601 string")),
        Some(_) => Err(bad_request("`expiry` must be a string or { custom }")),
    }
}

/// Mint a fresh, high-entropy bearer token (`sk_live_<43-char base64url>`
/// from 32 random bytes). Cryptographically random via `OsRng`, the same
/// source the magic-link minter uses.
pub(super) fn mint_token() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    let body = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes);
    format!("sk_live_{body}")
}

/// SHA-256 hex digest of the raw token — the only secret-derived value
/// stored. Mirrors `routes_auth::sha256_hex`.
pub(super) fn sha256_hex(s: &str) -> String {
    hex::encode(Sha256::digest(s.as_bytes()))
}

/// Masked 12-ish-char preview stored at create time
/// (`sk_live_a4b8...c2e1`). Derived ONCE here at create; never recomputed
/// from a secret on a read path.
pub(super) fn masked_prefix(token: &str) -> String {
    let head: String = token.chars().take(12).collect();
    let tail: String = token
        .chars()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("{head}...{tail}")
}

/// Mint a new opaque key id (`key_<uuid-v7>`).
pub(super) fn new_key_id() -> String {
    format!("key_{}", Uuid::now_v7())
}

/// Serialize one stored row to the SPA's `ApiKeyRow` JSON. Metadata
/// only — there is no token-hash field on the input row, so none can
/// leak. `scopes` / `cohort_ids` are stored as JSON text and re-parsed
/// to arrays (falling back to `[]` on any malformed legacy value).
pub(super) fn row_to_json(row: &ApiKeyRow) -> Value {
    json!({
        "id": row.id,
        "name": row.name,
        "prefix": row.prefix,
        "scopes": json_array_or_empty(&row.scopes_json),
        "cohort_ids": json_array_or_empty(&row.cohort_ids_json),
        "all_cohorts": row.all_cohorts,
        "created_at": row.created_at,
        "last_used_at": row.last_used_at,
        "expires_at": row.expires_at,
        "revoked": row.revoked,
    })
}

fn json_array_or_empty(raw: &str) -> Value {
    serde_json::from_str::<Value>(raw)
        .ok()
        .filter(Value::is_array)
        .unwrap_or_else(|| Value::Array(Vec::new()))
}

/// Resolve `(tenant, principal)` for the caller, IDENTITY-FIRST (WS1):
/// the signed-in user's `__Host-session` JWT tenant wins, then an explicit
/// `?tenant=` / `X-Tenant-Id` / `X-Argo-Tenant-Id` override, then the
/// cwd-bound workspace tenant for CLI / fixture callers — via the shared
/// `resolve_tenant::resolve_tenant_for_request`.
///
/// Previously this read ONLY the `X-Argo-Tenant-Id` header and otherwise
/// fell straight to cwd. The SPA sends no tenant header in single-workspace
/// mode, so a real signed-in user leaked the cwd/default tenant's API-key
/// metadata — a cross-tenant leak the real-mode walk caught (2026-06-07).
pub(super) async fn resolve_tenant_principal(
    store: &SessionStore,
    req: &str,
) -> ArgoResult<(TenantId, String)> {
    let principal =
        principal_id_for_audit(req).unwrap_or_else(|| "system:api-keys".to_string());
    let tenant = super::resolve_tenant::resolve_tenant_for_request(req, store).await?;
    Ok((tenant, principal))
}

/// Emit a tenant-scoped audit row for an api-key lifecycle event. The
/// token + hash are NEVER part of the payload — only the key id + name +
/// scope/cohort metadata.
pub(super) async fn emit_audit(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    principal_id: &str,
    event_type: &str,
    extras: Value,
) {
    let mut payload = json!({ "endpoint": "settings.api_keys", "event_type": event_type });
    if let (Value::Object(head), Value::Object(rest)) = (&mut payload, extras) {
        for (k, v) in rest {
            head.insert(k, v);
        }
    }
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant.to_string());
    if let Err(e) = sink
        .write(
            None,
            AuditKind::HttpRequest,
            payload,
            Some(principal_id.to_string()),
        )
        .await
    {
        tracing::warn!(
            target: "gateway::api_keys",
            error = %e,
            event_type = event_type,
            "api-key audit write failed"
        );
    }
}

pub(super) fn bad_request(message: &str) -> (&'static str, String) {
    (
        "400 Bad Request",
        json!({ "error": { "code": "bad_request", "message": message } }).to_string(),
    )
}

pub(super) fn server_error(message: &str) -> (&'static str, String) {
    (
        "500 Internal Server Error",
        json!({ "error": { "code": "internal", "message": message } }).to_string(),
    )
}

pub(super) fn not_found(id: &str) -> (&'static str, String) {
    (
        "404 Not Found",
        json!({ "error": { "code": "not_found", "message": format!("api key not found: {id}") } })
            .to_string(),
    )
}

#[cfg(test)]
#[path = "routes_api_keys_helpers_tests.rs"]
mod tests;
