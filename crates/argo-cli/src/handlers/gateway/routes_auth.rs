//! Shared bearer-token gate for gateway dispatch + token-bound
//! principal/tenant derivation.
//!
//! `check_bearer_auth` is wired by `routes_dispatch_post::dispatch_post`
//! (every POST) and `routes::dispatch_route` (every non-allowlist GET).
//! `is_public_get` carries the curated read-only allowlist that stays
//! anonymous so liveness probes, load balancers, and catalog-discovery
//! flows keep working without an auth token. Mirrors
//! `ARGO_GATEWAY_BEARER_TOKEN` precedent landed by #946.
//!
//! [`authenticate_request`] is the richer entry point introduced by
//! `mission-audit-principal-tenant-from-token`: it returns an
//! [`AuthCheckOutcome`] whose `Pass` variant carries an
//! [`AuthenticatedPrincipal`] derived from the credential itself —
//! either a row in `service_token_binding` (operator-bound) or a
//! `service:<fingerprint>` synthetic identity for unbound tokens.
//! Callers that need to enforce the cross-check against caller-
//! supplied `X-Argo-Principal` / `X-Argo-Tenant-Id` headers run
//! [`enforce_header_match`] against the resolved principal before
//! any business logic.

use argo_store::service_token_binding::lookup_binding;
use argo_store::SessionStore;
use argo_types::TenantId;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use uuid::Uuid;

/// Workspace-default tenant id stamped on unbound bearer tokens.
pub(super) const WORKSPACE_DEFAULT_TENANT: &str = "00000000-0000-0000-0000-000000000000";

use super::routes_auth_production_mode::{
    bearer_token_configured, is_production_mode, BEARER_TOKEN_ENV,
};
use std::sync::atomic::{AtomicU64, Ordering};

static LAST_DEV_BYPASS_WARN_SECS: AtomicU64 = AtomicU64::new(0);

/// Bearer-token gate applied before dispatch.
///
/// Returns `None` (auth passes) when any of:
/// - `gateway.auth_bypass = true` is set in non-production mode
///   (integration-test escape hatch),
/// - `ARGO_GATEWAY_BEARER_TOKEN` is unset/empty in dev mode, or
/// - the request carries a matching `Authorization: Bearer <token>` header.
///
/// Returns `Some(401, body)` otherwise. Body is a small JSON envelope so
/// CLI clients can parse a stable error shape.
///
/// # Examples
///
/// ```ignore
/// // (Helper is `pub(super)`; example shown for documentation only.)
/// // No token set → auth passes for every caller.
/// std::env::remove_var("ARGO_GATEWAY_BEARER_TOKEN");
/// assert!(check_bearer_auth("GET /sessions HTTP/1.1\r\n\r\n").is_none());
/// ```
pub(super) fn check_bearer_auth(req: &str) -> Option<(&'static str, String)> {
    let production = is_production_mode();
    if argo_config::load().unwrap_or_default().gateway.auth_bypass && !production {
        warn_dev_bypass();
        return None;
    }
    let expected = match argo_config::cred::get("ARGO_GATEWAY_BEARER_TOKEN") {
        Ok(t) if !t.trim().is_empty() => t,
        _ if production => {
            return Some((
                "401 Unauthorized",
                r#"{"error":"unauthorized","reason":"gateway-bearer-token-required-in-production"}"#
                    .to_string(),
            ));
        }
        _ => {
            warn_dev_bypass();
            return None;
        }
    };
    if presented_bearer(req).as_deref() == Some(expected.as_str()) {
        None
    } else {
        Some((
            "401 Unauthorized",
            r#"{"error":"unauthorized","reason":"missing or invalid bearer token"}"#.to_string(),
        ))
    }
}

/// True for the curated read-only allowlist that bypasses bearer auth
/// even when `ARGO_GATEWAY_BEARER_TOKEN` is configured.
///
/// Members:
/// - `GET /` — root index
/// - `GET /health`, `/health/live`, `/health/ready`, `/health/full`
/// - `GET /livez`, `/readyz` — k8s-convention probes (mission-health-check-endpoint)
/// - `GET /version`
/// - `GET /routes`
///
/// Anything else (including `/doctor`, `/db/*`, `/audit/*`, `/workspace/*`,
/// `/search*`, and every per-resource GET) requires auth when configured.
///
/// # Examples
///
/// ```ignore
/// assert!(is_public_get("GET /health/ready HTTP/1.1\r\n\r\n"));
/// assert!(!is_public_get("GET /sessions HTTP/1.1\r\n\r\n"));
/// ```
pub(super) fn is_public_get(req: &str) -> bool {
    // We only allow GET; POST always goes through `dispatch_post`'s gate.
    if !req.starts_with("GET ") {
        return false;
    }
    // Strip "GET " then peel off whichever terminator follows the path
    // (a trailing space marks "no query", '?' marks the start of one).
    let rest = req.strip_prefix("GET ").unwrap_or(req);
    let path_end = rest.find([' ', '?']).unwrap_or(rest.len());
    let path = &rest[..path_end];
    let exact = matches!(
        path,
        "/" | "/health"
            | "/health/live"
            | "/health/ready"
            | "/health/full"
            | "/livez"
            | "/readyz"
            | "/version"
            | "/routes"
            | "/openmetrics"
    );
    // SCIM 2.0 endpoints (`mission-scim-provisioning`) carry their own
    // bearer credential (`ARGO_SCIM_BEARER_TOKEN`); the dedicated
    // `scim_auth` gate inside the SCIM dispatcher enforces it. Treat
    // the path as "public" here so the operator-facing gate skips it.
    exact || path.starts_with("/scim/v2/")
}

pub(super) fn production_bearer_missing_message() -> String {
    format!(
        "{BEARER_TOKEN_ENV} required when binding non-loopback or \
         ARGO_GATEWAY_PROFILE=production; see docs/skycot/operator/security.md"
    )
}

pub(super) fn production_bearer_missing_for_boot() -> bool {
    !bearer_token_configured()
}

fn warn_dev_bypass() {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let last = LAST_DEV_BYPASS_WARN_SECS.load(Ordering::Relaxed);
    if now.saturating_sub(last) < 60 {
        return;
    }
    if LAST_DEV_BYPASS_WARN_SECS
        .compare_exchange(last, now, Ordering::Relaxed, Ordering::Relaxed)
        .is_ok()
    {
        tracing::warn!("auth bypass: bearer token unset, dev mode only");
    }
}

fn presented_bearer(req: &str) -> Option<String> {
    req.lines()
        .find_map(|l| {
            l.split_once(':')
                .and_then(|(k, v)| k.eq_ignore_ascii_case("Authorization").then(|| v.trim()))
        })
        .and_then(|h| h.strip_prefix("Bearer "))
        .map(str::to_string)
}

/// Where the [`AuthenticatedPrincipal`] was resolved from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AuthSource {
    /// `service_token_binding` row matched the bearer token's SHA-256 hash.
    Bound,
    /// Bearer is valid but unbound; principal is the synthetic
    /// `service:<fingerprint>` value and tenant is the workspace default.
    TokenSyntheticPrincipal,
    /// `gateway.auth_bypass` is on (integration-test escape hatch); no
    /// real credential was presented.
    DevBypass,
}

/// Principal + tenant derived from the request's authenticated
/// credential. Caller-supplied `X-Argo-Principal` /
/// `X-Argo-Tenant-Id` headers are compared against this value;
/// mismatch → 403 via [`enforce_header_match`].
#[derive(Debug, Clone)]
pub(crate) struct AuthenticatedPrincipal {
    pub(crate) principal: String,
    pub(crate) tenant_id: TenantId,
    /// Surface kept for diagnostics + future per-source policy
    /// (e.g. denying dev-bypass in production). Not yet consumed by
    /// callers — silence dead-code without dropping the field.
    #[allow(dead_code)]
    pub(crate) source: AuthSource,
}

/// Result of [`authenticate_request`]. `Pass` carries the resolved
/// principal; `Reject` carries the verbatim 401 response so the
/// caller can return it as-is.
#[derive(Debug)]
pub(super) enum AuthCheckOutcome {
    Pass(AuthenticatedPrincipal),
    Reject((&'static str, String)),
}

/// Resolve the authenticated principal + tenant for `req`.
///
/// 1. `gateway.auth_bypass = true` → `Pass(DevBypass)` deriving the
///    principal + tenant from caller-supplied headers (or defaults
///    when absent). This preserves the existing dev/test ergonomic
///    where the caller stamps their own identity.
/// 2. `ARGO_GATEWAY_BEARER_TOKEN` unset/empty → same as #1 (dev
///    default; existing `check_bearer_auth` semantics).
/// 3. Bearer presented but doesn't match → `Reject(401)`.
/// 4. Bearer matches:
///    - SHA-256 the raw token → look up in `service_token_binding`.
///    - Row present → `Pass(Bound)` with the stored principal + tenant.
///    - Row absent → `Pass(TokenSyntheticPrincipal)` with
///      `service:<first-16-hex>` + workspace-default tenant.
pub(super) async fn authenticate_request(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> AuthCheckOutcome {
    let cfg = argo_config::load().unwrap_or_default();
    if cfg.gateway.auth_bypass {
        return AuthCheckOutcome::Pass(dev_bypass_principal(req));
    }
    let expected = match argo_config::cred::get("ARGO_GATEWAY_BEARER_TOKEN") {
        Ok(t) if !t.is_empty() => t,
        _ => return AuthCheckOutcome::Pass(dev_bypass_principal(req)),
    };
    let Some(presented) = presented_bearer(req) else {
        return AuthCheckOutcome::Reject(reject_401());
    };
    if presented != expected {
        return AuthCheckOutcome::Reject(reject_401());
    }
    let hash = sha256_hex(&presented);
    if let Some(store) = store {
        if let Ok(Some(row)) = lookup_binding(store.pool(), &hash).await {
            if let Ok(tenant) = Uuid::parse_str(&row.tenant_id).map(TenantId) {
                return AuthCheckOutcome::Pass(AuthenticatedPrincipal {
                    principal: row.principal_id,
                    tenant_id: tenant,
                    source: AuthSource::Bound,
                });
            }
        }
    }
    AuthCheckOutcome::Pass(synthetic_principal_for(&hash))
}

/// Dev-bypass principal — caller's headers are authoritative (with
/// sensible defaults) so existing test harnesses keep working without
/// reaching for `service_token_binding`. In production the bypass is
/// off and the bearer-token path is exercised instead.
fn dev_bypass_principal(req: &str) -> AuthenticatedPrincipal {
    let principal = header_value(req, "x-argo-principal")
        .unwrap_or_else(|| "system:dev-bypass".to_string());
    let tenant_id = header_value(req, "x-argo-tenant-id")
        .and_then(|raw| Uuid::parse_str(&raw).ok())
        .map_or_else(workspace_default_tenant, TenantId);
    AuthenticatedPrincipal {
        principal,
        tenant_id,
        source: AuthSource::DevBypass,
    }
}

fn synthetic_principal_for(hash: &str) -> AuthenticatedPrincipal {
    let fp: String = hash.chars().take(16).collect();
    AuthenticatedPrincipal {
        principal: format!("service:{fp}"),
        tenant_id: workspace_default_tenant(),
        source: AuthSource::TokenSyntheticPrincipal,
    }
}

fn workspace_default_tenant() -> TenantId {
    TenantId(Uuid::parse_str(WORKSPACE_DEFAULT_TENANT).unwrap_or(Uuid::nil()))
}

fn reject_401() -> (&'static str, String) {
    (
        "401 Unauthorized",
        r#"{"error":"unauthorized","reason":"missing or invalid bearer token"}"#.to_string(),
    )
}

/// SHA-256 hex of `s`, lowercase. Token fingerprints / lookup keys
/// flow through this helper.
pub(super) fn sha256_hex(s: &str) -> String {
    let digest = Sha256::digest(s.as_bytes());
    let mut out = String::with_capacity(digest.len() * 2);
    for b in digest {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

/// Enforce that any caller-supplied `X-Argo-Principal` /
/// `X-Argo-Tenant-Id` headers match the [`AuthenticatedPrincipal`].
/// Absent headers are accepted (the route consumes `auth.principal`
/// / `auth.tenant_id` as the source of truth). Mismatch → `403
/// Forbidden` with a stable JSON envelope.
pub(super) fn enforce_header_match(
    req: &str,
    auth: &AuthenticatedPrincipal,
) -> Result<(), (&'static str, String)> {
    if let Some(value) = header_value(req, "x-argo-principal") {
        if value != auth.principal {
            return Err(forbidden("principal-header mismatch"));
        }
    }
    if let Some(value) = header_value(req, "x-argo-tenant-id") {
        if !tenant_header_matches(&value, &auth.tenant_id) {
            return Err(forbidden("tenant-header mismatch"));
        }
    }
    Ok(())
}

fn tenant_header_matches(raw: &str, tenant: &TenantId) -> bool {
    Uuid::parse_str(raw).is_ok_and(|u| u == tenant.0)
}

fn header_value(req: &str, name_lower: &str) -> Option<String> {
    let headers = req
        .split_once("\r\n\r\n")
        .map_or(req, |(headers, _body)| headers);
    headers.lines().skip(1).find_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.trim()
            .eq_ignore_ascii_case(name_lower)
            .then(|| value.trim().to_string())
    })
}

fn forbidden(message: &str) -> (&'static str, String) {
    (
        "403 Forbidden",
        format!(r#"{{"error":"forbidden","reason":"{message}"}}"#),
    )
}

#[cfg(test)]
#[path = "routes_auth_tests.rs"]
mod tests;
