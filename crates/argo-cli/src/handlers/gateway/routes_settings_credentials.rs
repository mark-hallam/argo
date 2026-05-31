//! Settings → credential-metadata read routes (Frontier-B
//! backend-route batch, `fix-frontier-b-backend-routes`).
//!
//! Mounts two metadata-only GET endpoints the `/settings/api-keys` and
//! `/settings/providers` SPA pages call. Both are **METADATA ONLY** —
//! they NEVER return a secret value (no bearer token, no provider API
//! key, not even a masked slice derived from a live secret):
//!
//! * `GET /api/v1/api-keys` — the SKYCOT service-token roster
//!   (`ApiKeyRow[]`: id, name, masked prefix, scopes, created/last-used).
//!   There is no first-class api-key store yet (the SPA's scope/cohort
//!   model has no Rust home — `service_token_binding` only stores a
//!   token *hash* + principal, not a named, scoped key), so this returns
//!   an empty array (`200 []`) and the page renders its empty state.
//!   Follow-up: a dedicated `api_key` store (see
//!   `mission-gateway-api-keys-endpoint`).
//!
//! * `GET /api/v1/providers/keys` — BYOK upstream-provider key STATUS
//!   (`ProviderKeyStatus[]`: provider, configured, masked_preview,
//!   updated_at). Real status is derived by probing whether each
//!   provider's conventional credential env resolves non-empty through
//!   the existing `argo_config::cred` layer — `configured: true/false`
//!   only. `masked_preview` is always `null`: a masked slice still
//!   derives from the secret, so we don't emit one here.
//!
//! Both responses are bare JSON arrays (not `{ items: [...] }`) to match
//! the SPA clients (`lib/api/api-keys.ts`, `lib/api/provider-keys.ts`),
//! which `apiGet<ApiKeyRow[]>` / `apiGet<ProviderKeyStatus[]>`. CORS
//! preflight is stamped by `super::routes::dispatch_route`.

use argo_store::SessionStore;
use argo_types::TenantId;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

const API_KEYS_PATH: &str = "/api/v1/api-keys";
const PROVIDER_KEYS_PATH: &str = "/api/v1/providers/keys";

/// The five providers the `/settings/providers` page renders, paired
/// with the conventional credential env each uses. Mirrors the SPA's
/// `PROVIDER_OPTIONS` (`web/lib/api/settings.ts`) + the provider
/// manifests' `auth.api_key_env` (`manifests/providers/*.toml`).
const PROVIDER_ENV: &[(&str, &str)] = &[
    ("anthropic", "ANTHROPIC_API_KEY"),
    ("openai", "OPENAI_API_KEY"),
    ("google", "GOOGLE_API_KEY"),
    ("azure", "AZURE_OPENAI_API_KEY"),
    ("bedrock", "AWS_ACCESS_KEY_ID"),
];

/// Dispatcher entry point. Returns `Some(resp)` for the two credential-
/// metadata GETs, `None` otherwise so the caller falls through.
pub(super) async fn dispatch_get(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    if !req.starts_with("GET ") {
        return None;
    }
    let bare = bare_path(req);
    if bare == API_KEYS_PATH {
        return Some(api_keys_response());
    }
    if bare == PROVIDER_KEYS_PATH {
        return Some(provider_keys_response(req, store).await);
    }
    None
}

/// `GET /api/v1/api-keys` — service-token roster, metadata only.
///
/// No api-key store exists yet, so the honest answer is an empty list:
/// the page shows its empty state rather than fabricating rows. Never
/// returns a token value.
fn api_keys_response() -> (&'static str, String) {
    ("200 OK", "[]".to_string())
}

/// `GET /api/v1/providers/keys` — BYOK provider-key STATUS, metadata
/// only. Probes each provider's conventional credential env via the
/// tenant-scoped `argo_config::cred` layer and reports `configured`
/// (boolean). Never returns a key value or a secret-derived preview.
async fn provider_keys_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let tenant = match resolve_tenant(req, store).await {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    let rows: Vec<serde_json::Value> = PROVIDER_ENV
        .iter()
        .map(|(provider, env_key)| {
            json!({
                "provider": provider,
                "configured": credential_present(&tenant, env_key),
                // Never emit a secret-derived value, even masked.
                "masked_preview": serde_json::Value::Null,
                "updated_at": serde_json::Value::Null,
            })
        })
        .collect();
    ("200 OK", serde_json::Value::Array(rows).to_string())
}

/// True when `env_key` resolves to a non-empty credential for `tenant`
/// through the configured credential backend (keyring / encrypted file /
/// env / legacy). Only the presence boolean is used by the caller — the
/// resolved value is dropped immediately and never logged or returned.
fn credential_present(tenant: &TenantId, env_key: &str) -> bool {
    argo_config::cred::get(tenant, env_key).is_ok_and(|v| !v.trim().is_empty())
}

/// Resolve the caller's tenant. Prefers the `X-Argo-Tenant-Id` header
/// (set by the SPA proxy after JWT verification); falls back to the
/// cwd-bound workspace tenant for CLI / fixture callers — the same
/// pattern the sibling `/api/v1/skills` + `/api/v1/workflows` reads use.
async fn resolve_tenant(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Result<TenantId, (&'static str, String)> {
    if let Some(raw) = header_value(req, "X-Argo-Tenant-Id").filter(|s| !s.is_empty()) {
        if let Ok(uuid) = Uuid::parse_str(&raw) {
            return Ok(TenantId(uuid));
        }
    }
    let Some(store) = store else {
        return Err(super::store_unavailable());
    };
    let cwd = std::env::current_dir().map_err(|e| internal(&format!("cwd: {e}")))?;
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    store
        .ensure_workspace(name, &root)
        .await
        .map(|(t, _)| t)
        .map_err(|e| internal(&format!("resolve tenant: {e}")))
}

fn bare_path(req: &str) -> &str {
    let after = req.strip_prefix("GET ").unwrap_or(req);
    let path = after.split_whitespace().next().unwrap_or("");
    path.split('?').next().unwrap_or(path)
}

fn header_value(req: &str, name: &str) -> Option<String> {
    let headers = req
        .split_once("\r\n\r\n")
        .map_or(req, |(headers, _body)| headers);
    headers.lines().skip(1).find_map(|line| {
        let (k, v) = line.split_once(':')?;
        k.eq_ignore_ascii_case(name).then(|| v.trim().to_string())
    })
}

fn internal(message: &str) -> (&'static str, String) {
    (
        "500 Internal Server Error",
        json!({"error": {"code": "internal", "message": message}}).to_string(),
    )
}

#[cfg(test)]
#[path = "routes_settings_credentials_tests.rs"]
mod tests;
