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
use argo_types::ArgoResult;
use serde_json::{json, Map, Value};
use std::sync::Arc;

/// `group_key` value persisted in `workspace_settings` for each panel.
const GROUP_WORKSPACE: &str = "workspace";
const GROUP_PROVIDERS: &str = "providers";
const GROUP_NOTIFICATIONS: &str = "notifications";
/// The voice-provider selection group (`GET/PATCH /api/v1/voice/settings`),
/// read by the gateway voice routes' provider selector.
const GROUP_VOICE: &str = "voice";

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

/// The voice panel: a single `provider` choice ("mock" | "openai"). The
/// gateway voice routes read this via [`read_voice_provider`] to pick which
/// STT/TTS adapter backs a tenant's spoken turns.
fn voice_spec() -> GroupSpec {
    GroupSpec {
        group_key: GROUP_VOICE,
        default_obj: default_voice,
        fields: &[("provider", FieldKind::Enum(&["mock", "openai"]))],
    }
}

// Defaults mirror the `DEFAULT_*` constants in
// `web/lib/api/settings.ts`. Keep these in lockstep with the SPA.

fn default_workspace() -> Value {
    workspace_default_for(super::demo_mode::is_enabled())
}

/// The bundled `/api/v1/workspace` default, served when a tenant has no
/// saved `workspace_settings` row.
///
/// Real-mode walk 2026-06-07: a real signed-in stranger's GET returned
/// "SKYCOT Demo" because this default was demo-branded — so every fresh
/// real tenant saw "SKYCOT Demo" as their own workspace name, straight from
/// the gateway. Real tenants now get a NEUTRAL default (blank fields the SPA
/// shows for the user to fill in); only the demo showcase
/// (`ARGO_DEMO_MODE=1`) keeps the "SKYCOT Demo" branding. Mirrors
/// `NEUTRAL_WORKSPACE_SETTINGS` / `DEFAULT_WORKSPACE_SETTINGS` in
/// `web/lib/api/settings.ts` — keep in lockstep. Pure (`demo_mode` is a
/// parameter) so it is unit-tested without env-var manipulation.
fn workspace_default_for(demo_mode: bool) -> Value {
    if demo_mode {
        json!({
            "name": "SKYCOT Demo",
            "description": "Operator workspace for the SKYCOT demo tenant.",
            "default_vertical": "marketing",
            "timezone": "Australia/Melbourne",
            "billing_contact": "owner@skycot.example",
        })
    } else {
        json!({
            "name": "",
            "description": "",
            "default_vertical": "marketing",
            "timezone": "Australia/Melbourne",
            "billing_contact": "",
        })
    }
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

fn default_voice() -> Value {
    json!({ "provider": "mock" })
}

/// GET match arm — fanned out from
/// [`super::routes_dispatch_get_api_groups`]. Returns `Some(resp)` when
/// one of the three settings paths matches, `None` otherwise.
pub(super) async fn dispatch_get(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    let spec = match_path(req, "GET")?;
    Some(get_response(req, store, &spec).await)
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
        ("/api/v1/voice/settings", voice_spec),
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
    req: &str,
    store: Option<&Arc<SessionStore>>,
    spec: &GroupSpec,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let tenant = match super::resolve_tenant::resolve_tenant_for_request(req, store).await {
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
    let tenant = match super::resolve_tenant::resolve_tenant_for_request(req, store).await {
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

/// The tenant's chosen voice `provider` ("mock" | "openai"), read for the
/// gateway voice routes' provider selector. Reuses the same tenant
/// resolution + `voice` group read as the GET/PATCH handlers.
///
/// Returns `None` (so the caller falls back to `ARGO_VOICE_PROVIDER`) when
/// the tenant cannot be resolved, the group is unset, or the stored value is
/// not a string — i.e. a settings hiccup never hard-fails a voice request.
pub(super) async fn read_voice_provider(store: &SessionStore) -> Option<String> {
    // VOICE-CONFIG EXCEPTION (WS1): this is the *workspace-level* voice
    // provider, resolved from the gateway working dir — NOT a per-request JWT
    // tenant. Its callers (routes_voice_io: select_stt / select_tts ->
    // resolve_for_tenant) carry no HTTP request to resolve from, so unlike the
    // GET/PATCH settings handlers above it can't use resolve_tenant_for_request.
    // Threading the request tenant through the voice-IO chain is a separate
    // follow-up; until then voice config stays workspace-scoped.
    let cwd = std::env::current_dir().ok()?;
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let (tenant, _) = store
        .ensure_workspace(name, &cwd.to_string_lossy())
        .await
        .ok()?;
    let tenant = tenant.to_string();
    let stored =
        argo_store::workspace_settings::get_group(store.pool(), &tenant, GROUP_VOICE)
            .await
            .ok()??;
    serde_json::from_str::<Value>(&stored)
        .ok()?
        .get("provider")
        .and_then(Value::as_str)
        .map(str::to_string)
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
