//! Gateway routes for the per-tenant white-label theme override.
//!
//!   - `GET  /api/v1/tenant/theme`  → the caller-tenant's persisted
//!     override merged over the locked SKYCOT default. The response
//!     carries `is_override` so the SPA knows whether to synthesize a
//!     tenant palette or leave the shipped brand tokens untouched.
//!   - `PATCH /api/v1/tenant/theme` → persist `{accent_hex, neutral_tint,
//!     radius}` for the caller's own tenant, append a hash-chained audit
//!     row, and echo back the merged object.
//!
//! ## Brand guardrail (DEC-027)
//!
//! The SKYCOT brand is LOCKED. This route does NOT touch the default
//! brand tokens — it only stores an *operator white-label override* for
//! a tenant who wants their own accent. With NO row persisted, GET yields
//! the locked SKYCOT default (`accent_hex = #8B5CF6`, `is_override =
//! false`), so a non-white-label tenant's brand is unchanged.
//!
//! ## Persistence — reuse, no new table
//!
//! The override persists into the SAME generic `workspace_settings` table
//! (migration v85) via [`argo_store::workspace_settings`], under
//! `group_key = "theme"`, keyed by `(tenant_id, group_key)`. Tenant
//! scoping is therefore structural: the composite primary key means one
//! tenant can never read or overwrite another's theme. No migration is
//! added.
//!
//! Tenant resolution uses the canonical
//! [`super::resolve_tenant::resolve_tenant_for_request`] (WS1), so a
//! caller can only read/write their OWN tenant's theme — the JWT-bound
//! tenant always wins over any override header for a signed-in user.

use argo_audit::compute_chain_hash;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::Utc;
use serde_json::{json, Map, Value};
use sqlx::{Sqlite, Transaction};
use std::sync::Arc;
use uuid::Uuid;

/// `group_key` the override persists under in `workspace_settings`.
const GROUP_THEME: &str = "theme";

/// Audit `kind` appended on a theme write.
const AUDIT_KIND: &str = "tenant_theme_update";

/// The locked SKYCOT default accent (purple end of the brand gradient,
/// DEC-027). Returned when a tenant has no override row.
const SKYCOT_ACCENT_HEX: &str = "#8B5CF6";

/// Allowed `neutral_tint` values (mirror `synthesizeTenantCss` in
/// `web/lib/theme/synthesize.ts`).
const NEUTRAL_TINTS: &[&str] = &["cool", "neutral", "warm"];

/// Allowed `radius` values (mirror `synthesizeTenantCss`).
const RADII: &[&str] = &["sharp", "default", "rounded"];

/// The locked-default theme object, served when no override row exists.
/// `is_override = false` tells the SPA to keep the shipped brand tokens.
fn default_theme() -> Value {
    json!({
        "accent_hex": SKYCOT_ACCENT_HEX,
        "neutral_tint": "cool",
        "radius": "default",
        "is_override": false,
    })
}

/// GET match arm — fanned out from `routes_dispatch_get_api_groups`.
/// Returns `Some(resp)` when the path matches, `None` otherwise.
pub(super) async fn dispatch_get(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    if !path_matches(req, "GET") {
        return None;
    }
    Some(get_response(req, store).await)
}

/// PATCH match arm — fanned out from `routes_dispatch_patch`.
pub(super) async fn dispatch_patch(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    if !path_matches(req, "PATCH") {
        return None;
    }
    Some(patch_response(req, store).await)
}

/// Match `<VERB> /api/v1/tenant/theme` exactly (followed by a space or
/// `?`) so sibling `/api/v1/tenant/...` routes are not swallowed.
fn path_matches(req: &str, verb: &str) -> bool {
    let with_space = format!("{verb} /api/v1/tenant/theme ");
    let with_query = format!("{verb} /api/v1/tenant/theme?");
    req.starts_with(&with_space) || req.starts_with(&with_query)
}

async fn get_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let tenant = match super::resolve_tenant::resolve_tenant_for_request(req, store).await {
        Ok(t) => t.to_string(),
        Err(e) => return server_error(&format!("resolve tenant: {e}")),
    };
    match current_theme(store, &tenant).await {
        Ok(v) => ("200 OK", v.to_string()),
        Err(e) => server_error(&format!("theme read: {e}")),
    }
}

async fn patch_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let tenant = match super::resolve_tenant::resolve_tenant_for_request(req, store).await {
        Ok(t) => t.to_string(),
        Err(e) => return server_error(&format!("resolve tenant: {e}")),
    };
    let patch = match parse_patch_body(req) {
        Ok(p) => p,
        Err(resp) => return resp,
    };
    // Stored fields = the current persisted override (or empty), shallow-
    // merged with the validated patch. We persist ONLY the three theme
    // fields; `is_override` is derived on read, never stored.
    let mut stored = match stored_fields(store, &tenant).await {
        Ok(m) => m,
        Err(e) => return server_error(&format!("theme read: {e}")),
    };
    if let Err(resp) = apply_patch(&mut stored, &patch) {
        return resp;
    }
    let to_persist = Value::Object(stored.clone());
    let now = Utc::now().timestamp();
    if let Err(e) = argo_store::workspace_settings::upsert_group(
        store.pool(),
        &tenant,
        GROUP_THEME,
        &to_persist.to_string(),
        now,
    )
    .await
    {
        return server_error(&format!("theme upsert: {e}"));
    }
    if let Err(e) = append_audit(store, &tenant, &to_persist).await {
        // The override IS persisted; a failed audit append must not lose
        // it, but it must be visible. Surface as a 500 so the failure is
        // never silent (truth-bounded — DEC-029).
        return server_error(&format!("theme audit append: {e}"));
    }
    // `is_override` is derived the same way GET derives it (non-empty
    // stored fields), so an empty no-op PATCH stays `false` and a real
    // accent write reads back `true` — the two paths never disagree.
    let is_override = !stored.is_empty();
    let mut resp = stored;
    resp.insert("is_override".to_string(), Value::Bool(is_override));
    ("200 OK", Value::Object(resp).to_string())
}

/// The complete theme object for a tenant: the locked default with any
/// persisted override fields merged over it, plus a derived
/// `is_override` flag (`true` iff a row exists).
async fn current_theme(store: &SessionStore, tenant: &str) -> ArgoResult<Value> {
    let mut obj = match default_theme() {
        Value::Object(m) => m,
        _ => Map::new(),
    };
    let stored = stored_fields(store, tenant).await?;
    let is_override = !stored.is_empty();
    for (k, v) in stored {
        obj.insert(k, v);
    }
    obj.insert("is_override".to_string(), Value::Bool(is_override));
    Ok(Value::Object(obj))
}

/// The persisted override fields for a tenant (`{}` when no row). Never
/// includes `is_override` — that is derived, not stored.
async fn stored_fields(store: &SessionStore, tenant: &str) -> ArgoResult<Map<String, Value>> {
    let Some(raw) =
        argo_store::workspace_settings::get_group(store.pool(), tenant, GROUP_THEME).await?
    else {
        return Ok(Map::new());
    };
    match serde_json::from_str::<Value>(&raw) {
        Ok(Value::Object(mut m)) => {
            m.remove("is_override");
            Ok(m)
        }
        _ => Ok(Map::new()),
    }
}

/// Shallow-merge the validated patch keys over `current`. Rejects unknown
/// keys, wrong types, and out-of-range values with a 400 so the stored
/// object stays well-formed.
fn apply_patch(
    current: &mut Map<String, Value>,
    patch: &Map<String, Value>,
) -> Result<(), (&'static str, String)> {
    for (key, value) in patch {
        match key.as_str() {
            "accent_hex" => {
                let ok = value.as_str().is_some_and(is_hex_color);
                if !ok {
                    return Err(bad_request(
                        "field 'accent_hex' must be a '#rrggbb' hex color",
                    ));
                }
            }
            "neutral_tint" => {
                if !is_enum(value, NEUTRAL_TINTS) {
                    return Err(bad_request(
                        "field 'neutral_tint' must be one of cool|neutral|warm",
                    ));
                }
            }
            "radius" => {
                if !is_enum(value, RADII) {
                    return Err(bad_request(
                        "field 'radius' must be one of sharp|default|rounded",
                    ));
                }
            }
            other => return Err(bad_request(&format!("unknown field '{other}'"))),
        }
        current.insert(key.clone(), value.clone());
    }
    Ok(())
}

/// `#rrggbb` — a `#` followed by exactly six hex digits.
fn is_hex_color(s: &str) -> bool {
    let Some(rest) = s.strip_prefix('#') else {
        return false;
    };
    rest.len() == 6 && rest.bytes().all(|b| b.is_ascii_hexdigit())
}

fn is_enum(value: &Value, allowed: &[&str]) -> bool {
    value.as_str().is_some_and(|s| allowed.contains(&s))
}

/// Parse the PATCH body as a JSON object. An empty body is an empty patch
/// (a no-op that round-trips the current object).
fn parse_patch_body(req: &str) -> Result<Map<String, Value>, (&'static str, String)> {
    let raw = super::body_from_request(req);
    if raw.trim().is_empty() {
        return Ok(Map::new());
    }
    match serde_json::from_str::<Value>(raw) {
        Ok(Value::Object(m)) => Ok(m),
        Ok(_) => Err(bad_request("body must be a JSON object")),
        Err(e) => Err(bad_request(&format!("invalid JSON body: {e}"))),
    }
}

/// Append a hash-chained `tenant_theme_update` row to the global audit
/// chain (same pattern as the HIPAA posture breadcrumb). The payload
/// carries the tenant id + the persisted theme so the write is auditable.
async fn append_audit(store: &SessionStore, tenant: &str, theme: &Value) -> ArgoResult<()> {
    let payload = json!({
        "tenant_id": tenant,
        "theme": theme,
        "at": Utc::now().to_rfc3339(),
    });
    let canonical = serde_json::to_string(&payload)
        .map_err(|e| ArgoError::Schema(format!("theme audit serialize: {e}")))?;
    // Serialize the read-then-insert with `BEGIN IMMEDIATE` so two
    // concurrent theme writes can never read the same chain tail and
    // fork the audit chain (see fix-audit-chain-append-serialization).
    let mut tx = store
        .pool()
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(|e| ArgoError::Provider(format!("theme audit tx begin: {e}")))?;
    let parent_hash = read_global_parent(&mut tx).await?;
    let self_hash = compute_chain_hash(&parent_hash, AUDIT_KIND, &canonical);
    sqlx::query(
        "INSERT INTO audit_record (id, session_id, kind, payload_json, \
         parent_hash, self_hash, created_at, principal_id) \
         VALUES (?, NULL, ?, ?, ?, ?, ?, NULL)",
    )
    .bind(Uuid::now_v7().as_bytes().as_slice())
    .bind(AUDIT_KIND)
    .bind(canonical)
    .bind(parent_hash.as_slice())
    .bind(self_hash.as_slice())
    .bind(Utc::now().timestamp_micros())
    .execute(&mut *tx)
    .await
    .map_err(|e| ArgoError::Provider(format!("theme audit insert: {e}")))?;
    tx.commit()
        .await
        .map_err(|e| ArgoError::Provider(format!("theme audit tx commit: {e}")))?;
    Ok(())
}

/// Read the global (`session_id IS NULL`) chain tail inside the
/// caller's transaction so the parent-read shares the write lock with
/// the subsequent insert.
async fn read_global_parent(tx: &mut Transaction<'_, Sqlite>) -> ArgoResult<[u8; 32]> {
    let bytes: Option<Vec<u8>> = sqlx::query_scalar(
        "SELECT self_hash FROM audit_record WHERE session_id IS NULL ORDER BY seq DESC LIMIT 1",
    )
    .fetch_optional(&mut **tx)
    .await
    .map_err(|e| ArgoError::Provider(format!("theme audit parent read: {e}")))?;
    let mut out = [0u8; 32];
    if let Some(b) = bytes {
        if b.len() == 32 {
            out.copy_from_slice(&b);
        }
    }
    Ok(out)
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
#[path = "routes_tenant_theme_tests.rs"]
mod tests;
