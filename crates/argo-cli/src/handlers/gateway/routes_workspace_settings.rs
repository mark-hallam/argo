//! Gateway routes for the `/settings` SPA surface (theatre item `T1`).
//!
//! Three panels — workspace, providers, notifications — each have a
//! GET (read the persisted object, or bundled defaults if unseeded)
//! and a PATCH (shallow-merge a partial body over the current object,
//! validate the shape, upsert, echo back the merged object):
//!
//!   - `GET/PATCH /api/v1/workspace`                 → workspace panel
//!   - `GET/PATCH /api/v1/providers/policy`          → providers panel
//!   - `GET/PATCH /api/v1/notifications/preferences` → notifications panel
//!
//! All three persist into the one generic `workspace_settings` table
//! (migration v85) via [`argo_store::workspace_settings`], keyed by
//! `(tenant_id, group_key)`. The default objects mirror the
//! `DEFAULT_*` constants in `web/lib/api/settings.ts` so a GET always
//! returns a complete shape even before the tenant has saved anything.
//!
//! Bearer auth is applied upstream (`routes::dispatch_route` for GET,
//! `routes_dispatch_post::dispatch_post_inner` for PATCH); this module
//! resolves the workspace tenant from cwd the same way
//! `routes_notifications` does, so every read/write is tenant-scoped.

use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use serde_json::{json, Map, Value};
use std::sync::Arc;

/// `group_key` value persisted in `workspace_settings` for each panel.
const GROUP_WORKSPACE: &str = "workspace";
const GROUP_PROVIDERS: &str = "providers";
const GROUP_NOTIFICATIONS: &str = "notifications";

/// Field kinds a panel key may hold. Used to validate a PATCH body and
/// reject unknown keys or wrong-typed values before they reach the DB.
#[derive(Clone, Copy)]
enum FieldKind {
    /// Any JSON string.
    Str,
    /// A string constrained to a fixed allow-list.
    Enum(&'static [&'static str]),
    Bool,
    /// A non-negative integer.
    NonNegInt,
}

/// One panel's contract: its persisted `group_key`, the default object
/// (returned when no row exists), and the `(key, kind)` schema used to
/// validate PATCH bodies.
struct GroupSpec {
    group_key: &'static str,
    default_obj: fn() -> Value,
    fields: &'static [(&'static str, FieldKind)],
}

fn workspace_spec() -> GroupSpec {
    GroupSpec {
        group_key: GROUP_WORKSPACE,
        default_obj: default_workspace,
        fields: &[
            ("name", FieldKind::Str),
            ("description", FieldKind::Str),
            ("default_vertical", FieldKind::Str),
            ("timezone", FieldKind::Str),
            ("billing_contact", FieldKind::Str),
        ],
    }
}

fn providers_spec() -> GroupSpec {
    GroupSpec {
        group_key: GROUP_PROVIDERS,
        default_obj: default_providers,
        fields: &[
            (
                "primary_provider",
                FieldKind::Enum(&["anthropic", "openai", "google", "azure", "bedrock"]),
            ),
            (
                "fallback_behavior",
                FieldKind::Enum(&["fail_closed", "fail_open", "retry_alt"]),
            ),
            ("allow_worker_overrides", FieldKind::Bool),
        ],
    }
}

fn notifications_spec() -> GroupSpec {
    GroupSpec {
        group_key: GROUP_NOTIFICATIONS,
        default_obj: default_notifications,
        fields: &[
            ("approval_escalation_minutes", FieldKind::NonNegInt),
            ("daily_digest_time", FieldKind::Str),
            ("quiet_hours_start", FieldKind::Str),
            ("quiet_hours_end", FieldKind::Str),
            ("email_enabled", FieldKind::Bool),
            ("push_enabled", FieldKind::Bool),
        ],
    }
}

// Defaults mirror the `DEFAULT_*` constants in
// `web/lib/api/settings.ts`. Keep these in lockstep with the SPA.

fn default_workspace() -> Value {
    json!({
        "name": "SKYCOT Demo",
        "description": "Operator workspace for the SKYCOT demo tenant.",
        "default_vertical": "marketing",
        "timezone": "Australia/Melbourne",
        "billing_contact": "owner@skycot.example",
    })
}

fn default_providers() -> Value {
    json!({
        "primary_provider": "anthropic",
        "fallback_behavior": "fail_closed",
        "allow_worker_overrides": true,
    })
}

fn default_notifications() -> Value {
    json!({
        "approval_escalation_minutes": 60,
        "daily_digest_time": "08:30",
        "quiet_hours_start": "18:00",
        "quiet_hours_end": "08:00",
        "email_enabled": true,
        "push_enabled": false,
    })
}

/// GET match arm — fanned out from
/// [`super::routes_dispatch_get_api_groups`]. Returns `Some(resp)` when
/// one of the three settings paths matches, `None` otherwise.
pub(super) async fn dispatch_get(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    let spec = match_path(req, "GET")?;
    Some(get_response(store, &spec).await)
}

/// PATCH match arm — fanned out from
/// [`super::routes_dispatch_patch`].
pub(super) async fn dispatch_patch(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    let spec = match_path(req, "PATCH")?;
    Some(patch_response(req, store, &spec).await)
}

/// Map a request line (`<VERB> <path> ...`) to the matching panel
/// spec. The path is matched exactly (followed by a space or `?`) so
/// sibling routes under the same prefix do not get swallowed.
fn match_path(req: &str, verb: &str) -> Option<GroupSpec> {
    for (path, spec) in [
        ("/api/v1/workspace", workspace_spec as fn() -> GroupSpec),
        ("/api/v1/providers/policy", providers_spec),
        ("/api/v1/notifications/preferences", notifications_spec),
    ] {
        let with_space = format!("{verb} {path} ");
        let with_query = format!("{verb} {path}?");
        if req.starts_with(&with_space) || req.starts_with(&with_query) {
            return Some(spec());
        }
    }
    None
}

async fn get_response(
    store: Option<&Arc<SessionStore>>,
    spec: &GroupSpec,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let tenant = match resolve_workspace_tenant(store).await {
        Ok(t) => t.to_string(),
        Err(e) => return server_error(&format!("resolve tenant: {e}")),
    };
    let current = match current_object(store, &tenant, spec).await {
        Ok(v) => v,
        Err(e) => return server_error(&format!("settings read: {e}")),
    };
    ("200 OK", current.to_string())
}

async fn patch_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
    spec: &GroupSpec,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let tenant = match resolve_workspace_tenant(store).await {
        Ok(t) => t.to_string(),
        Err(e) => return server_error(&format!("resolve tenant: {e}")),
    };
    let patch = match parse_patch_body(req) {
        Ok(p) => p,
        Err(resp) => return resp,
    };
    let mut current = match current_object(store, &tenant, spec).await {
        Ok(Value::Object(m)) => m,
        Ok(_) => Map::new(),
        Err(e) => return server_error(&format!("settings read: {e}")),
    };
    if let Err(resp) = apply_patch(&mut current, &patch, spec) {
        return resp;
    }
    let merged = Value::Object(current);
    let now = chrono::Utc::now().timestamp();
    match argo_store::workspace_settings::upsert_group(
        store.pool(),
        &tenant,
        spec.group_key,
        &merged.to_string(),
        now,
    )
    .await
    {
        Ok(()) => ("200 OK", merged.to_string()),
        Err(e) => server_error(&format!("settings upsert: {e}")),
    }
}

/// The current complete panel object: the stored row merged over the
/// defaults (so a stored-but-partial row still returns a full shape),
/// or the bare defaults when no row exists.
async fn current_object(
    store: &SessionStore,
    tenant: &str,
    spec: &GroupSpec,
) -> ArgoResult<Value> {
    let mut obj = match (spec.default_obj)() {
        Value::Object(m) => m,
        _ => Map::new(),
    };
    if let Some(stored) = argo_store::workspace_settings::get_group(store.pool(), tenant, spec.group_key).await? {
        if let Ok(Value::Object(saved)) = serde_json::from_str::<Value>(&stored) {
            for (k, v) in saved {
                obj.insert(k, v);
            }
        }
    }
    Ok(Value::Object(obj))
}

/// Shallow-merge the validated patch keys over `current`. Rejects
/// unknown keys and wrong-typed values with a 400 so `value_json`
/// stays a well-formed panel object.
fn apply_patch(
    current: &mut Map<String, Value>,
    patch: &Map<String, Value>,
    spec: &GroupSpec,
) -> Result<(), (&'static str, String)> {
    for (key, value) in patch {
        let Some((_, kind)) = spec.fields.iter().find(|(k, _)| k == key) else {
            return Err(bad_request(&format!("unknown field '{key}'")));
        };
        if !value_matches_kind(value, *kind) {
            return Err(bad_request(&format!("field '{key}' has the wrong type")));
        }
        current.insert(key.clone(), value.clone());
    }
    Ok(())
}

fn value_matches_kind(value: &Value, kind: FieldKind) -> bool {
    match kind {
        FieldKind::Str => value.is_string(),
        FieldKind::Enum(allowed) => value
            .as_str()
            .is_some_and(|s| allowed.contains(&s)),
        FieldKind::Bool => value.is_boolean(),
        FieldKind::NonNegInt => value.as_i64().is_some_and(|n| n >= 0),
    }
}

/// Parse the PATCH body as a JSON object. An empty body is treated as
/// an empty patch (no-op merge that returns the current object).
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

/// Resolve the workspace tenant from cwd — identical convention to
/// `routes_notifications::resolve_workspace_tenant`.
async fn resolve_workspace_tenant(store: &SessionStore) -> ArgoResult<TenantId> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, _ws) = store.ensure_workspace(name, &root).await?;
    Ok(tenant)
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
#[path = "routes_workspace_settings_tests.rs"]
mod tests;
