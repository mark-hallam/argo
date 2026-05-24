//! Skill marketplace MVP (`claude-skill-marketplace-mvp`) — browse +
//! install skills cross-tenant via a server-rendered HTML UI. Builds
//! on the cross-tenant share PoC (data model + signing) and the
//! dashboard MVP (HTML rendering pattern).
//!
//! Mounted at `/marketplace/*`: GET browsing is anonymous, while
//! install POSTs carry their own bearer/JWT auth gate and derive the
//! target tenant from that auth context, never `x-tenant-id`.
//!
//! - `GET  /marketplace`                    — list view with search
//!   + vertical + executor filters.
//! - `GET  /marketplace/<skill_id>`         — detail view + install
//!   button.
//! - `POST /marketplace/<skill_id>/install` — delegates to the
//!   existing cross-tenant share helper. No duplicate logic.
//!
//! Data source is the in-memory manifest index from
//! [`argo_skill::marketplace::search`]; "installed for tenant X"
//! is checked via
//! [`argo_store::skill_share::list_shared_for_tenant`].
//!
//! ## Sensible deviations from the brief
//!
//! 1. Templates live at `marketplace_templates/marketplace_*.html`
//!    (mirrors the dashboard MVP's `dashboard_templates/`
//!    convention); the brief's `templates/` was inconsistent with
//!    that.
//! 2. The brief asked for `crates/argo-e2e/tests/marketplace_smoke.rs`
//!    but no `argo-e2e` crate exists — gateway smoke tests live
//!    under `crates/argo-cli/tests/`.
//! 3. The static marketplace index has no `vertical` / `executor`
//!    columns — filters are heuristic `contains()` matches against
//!    name + description + publisher. Real tagging lands in V3
//!    with the meilisearch swap.
//! 4. Install delegates to `argo_store::skill_share::set_shared`
//!    rather than the local `POST /skills` install path: the
//!    marketplace index is a manifest catalog, not raw SKILL.md,
//!    so the share helper is the correct install side-effect
//!    (record cluster authorization for the principal tenant).
//! 5. The boot dispatcher's response shape doesn't let us emit a
//!    real 303 redirect with a `Location` header. Install POST
//!    returns 200 OK with a `<meta http-equiv="refresh">` to the
//!    detail page — same UX, no header gymnastics.

use super::marketplace_render::{
    render_404, render_detail, render_install_ok, render_list, url_decode, HTML_HDR, HTML_HDR_404,
};
use super::routes_auth::check_bearer_auth;
use super::routes_auth_principal::principal_id_for_audit;
use argo_audit::{AuditKind, AuditSink};
use argo_skill::marketplace::{search, MarketplaceSkill, SearchOptions};
use argo_store::SessionStore;
use serde_json::json;
use std::sync::Arc;

/// Cheap pre-check used by [`super::routes::dispatch_route`] to
/// route marketplace requests ahead of the bearer-auth gate.
pub(super) fn is_marketplace(req: &str) -> bool {
    let Some(line) = req.lines().next() else {
        return false;
    };
    let Some((_method, target)) = method_target(line) else {
        return false;
    };
    is_marketplace_path(split_query(target).0)
}

/// True only for anonymous marketplace browse/detail requests.
pub(super) fn is_marketplace_get(req: &str) -> bool {
    let Some(line) = req.lines().next() else {
        return false;
    };
    let Some((method, target)) = method_target(line) else {
        return false;
    };
    method == "GET" && is_marketplace_path(split_query(target).0)
}

fn is_marketplace_path(path: &str) -> bool {
    path == "/marketplace" || path.starts_with("/marketplace/")
}

/// Dispatcher for `/marketplace/*`. Returns `None` when the prefix
/// matches but no specific handler matches (caller falls through to
/// the 404 default).
pub(super) async fn dispatch(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    let line = req.lines().next()?;
    let (method, target) = method_target(line)?;
    let (path, query) = split_query(target);

    if method == "GET" {
        if path == "/marketplace" || path == "/marketplace/" {
            return Some(list_page(req, store, query).await);
        }
        if let Some(rest) = path.strip_prefix("/marketplace/") {
            let id = rest.trim_end_matches('/');
            if !id.is_empty() && !id.contains('/') {
                return Some(detail_page(req, store, id, query).await);
            }
        }
    }
    if method == "POST" {
        if let Some(rest) = path.strip_prefix("/marketplace/") {
            if let Some(id) = rest.strip_suffix("/install") {
                let id = id.trim_end_matches('/');
                if !id.is_empty() && !id.contains('/') {
                    return Some(install_action(req, store, id, query).await);
                }
            }
        }
    }
    None
}

fn method_target(line: &str) -> Option<(&'static str, &str)> {
    if let Some(rest) = line.strip_prefix("GET ") {
        return Some(("GET", rest));
    }
    if let Some(rest) = line.strip_prefix("POST ") {
        return Some(("POST", rest));
    }
    None
}

fn split_query(target: &str) -> (&str, &str) {
    let target = target.split(' ').next().unwrap_or(target);
    match target.split_once('?') {
        Some((p, q)) => (p, q),
        None => (target, ""),
    }
}

fn body_of(req: &str) -> &str {
    req.split_once("\r\n\r\n").map(|(_h, b)| b).unwrap_or("")
}

fn header_value<'a>(req: &'a str, name: &str) -> Option<&'a str> {
    for line in req.lines().skip(1) {
        if line.is_empty() {
            break;
        }
        if let Some((k, v)) = line.split_once(':') {
            if k.trim().eq_ignore_ascii_case(name) {
                return Some(v.trim());
            }
        }
    }
    None
}

fn query_param<'a>(query: &'a str, key: &str) -> Option<&'a str> {
    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        if k == key {
            return Some(v);
        }
    }
    None
}

fn body_tenant_id(body: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(body).ok()?;
    v.get("tenant_id")
        .and_then(|x| x.as_str())
        .map(str::to_string)
}

async fn list_page(
    req: &str,
    store: Option<&Arc<SessionStore>>,
    query: &str,
) -> (&'static str, String) {
    let q = query_param(query, "q").map(url_decode).unwrap_or_default();
    let vertical = query_param(query, "vertical")
        .map(url_decode)
        .unwrap_or_default();
    let executor = query_param(query, "executor")
        .map(url_decode)
        .unwrap_or_default();
    let tenant = header_value(req, "x-tenant-id");
    let body = render_list(store, tenant, &q, &vertical, &executor).await;
    (HTML_HDR, body)
}

async fn detail_page(
    req: &str,
    store: Option<&Arc<SessionStore>>,
    skill_id: &str,
    query: &str,
) -> (&'static str, String) {
    let id = url_decode(skill_id);
    let tenant = header_value(req, "x-tenant-id");
    let just_installed = query_param(query, "installed").is_some();

    let entries = search("", &SearchOptions::default());
    let Some(skill) = entries.iter().find(|s| s.name == id) else {
        return (HTML_HDR_404, render_404(&id));
    };
    let body = render_detail(store, tenant, skill, just_installed).await;
    (HTML_HDR, body)
}

async fn install_action(
    req: &str,
    store: Option<&Arc<SessionStore>>,
    skill_id: &str,
    query: &str,
) -> (&'static str, String) {
    let id = url_decode(skill_id);

    let auth = match authenticated_caller(req) {
        Ok(auth) => auth,
        Err(resp) => return resp,
    };
    let Some(store) = store else {
        return crate::handlers::gateway::store_unavailable();
    };

    // Authz: a target tenant on the URL or in the body must equal the
    // authenticated tenant. `x-tenant-id` remains display-only for GET
    // rendering and is never trusted for installs.
    let body = body_of(req);
    let target = query_param(query, "tenant")
        .map(url_decode)
        .or_else(|| body_tenant_id(body));
    if let Some(target) = target.as_deref() {
        if !target.is_empty() && target != auth.tenant_id {
            return (
                "403 Forbidden",
                "{\"error\":\"tenant mismatch — cannot install into another tenant\"}".to_string(),
            );
        }
    }

    let entries = search("", &SearchOptions::default());
    let Some(skill) = entries.iter().find(|s| s.name == id) else {
        return (
            "404 Not Found",
            format!(
                "{{\"error\":\"unknown skill `{}`\"}}",
                id.replace('"', "\\\"")
            ),
        );
    };

    if let Err(e) = argo_store::skill_share::set_shared_for_tenant(
        store,
        &auth.tenant_id,
        &skill.name,
        &skill.version,
        true,
        Some(&auth.tenant_id),
    )
    .await
    {
        tracing::warn!(
            target: "gateway::marketplace",
            skill = %skill.name,
            tenant = %auth.tenant_id,
            principal = %auth.principal_id,
            error = %e,
            "marketplace install: set_shared failed"
        );
        return (
            "500 Internal Server Error",
            format!("{{\"error\":\"install: {e}\"}}"),
        );
    }
    audit_install(store, &auth, skill).await;
    (HTML_HDR, render_install_ok(&skill.name))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AuthContext {
    tenant_id: String,
    principal_id: String,
}

fn authenticated_caller(req: &str) -> Result<AuthContext, (&'static str, String)> {
    let Some(token) = bearer_token(req) else {
        return Err((
            "401 Unauthorized",
            r#"{"error":"unauthorized","reason":"missing bearer token"}"#.to_string(),
        ));
    };
    if let Ok(claims) = super::jwt::verify_jwt(&token, chrono::Utc::now().timestamp()) {
        if !claims.tnt.is_empty() && !claims.sub.is_empty() {
            return Ok(AuthContext {
                tenant_id: claims.tnt,
                principal_id: claims.sub,
            });
        }
    }
    if let Some(resp) = check_bearer_auth(req) {
        return Err(resp);
    }
    let Some(principal_id) = principal_id_for_audit(req) else {
        return Err((
            "401 Unauthorized",
            r#"{"error":"unauthorized","reason":"missing authenticated principal"}"#.to_string(),
        ));
    };
    Ok(AuthContext {
        tenant_id: configured_tenant_id(),
        principal_id,
    })
}

fn bearer_token(req: &str) -> Option<String> {
    header_value(req, "Authorization")
        .and_then(|h| h.strip_prefix("Bearer "))
        .filter(|t| !t.trim().is_empty())
        .map(|t| t.trim().to_string())
}

fn configured_tenant_id() -> String {
    let cfg = argo_config::load().unwrap_or_default();
    let explicit = cfg.tenant.default_id.trim();
    if !explicit.is_empty()
        && explicit != argo_config::defaults::DEFAULT_TENANT_ID
        && explicit != argo_config::LEGACY_AUDIT_TENANT_SENTINEL
    {
        return explicit.to_string();
    }
    std::env::current_dir()
        .map(|cwd| argo_config::workspace_tenant_id(&cwd))
        .unwrap_or_else(|_| argo_config::defaults::DEFAULT_TENANT_ID.to_string())
}

async fn audit_install(store: &Arc<SessionStore>, auth: &AuthContext, skill: &MarketplaceSkill) {
    let payload = json!({
        "event_type": "marketplace.skill_installed",
        "skill_name": skill.name,
        "skill_version": skill.version,
        "tenant_id": auth.tenant_id,
    });
    if let Err(e) = AuditSink::new_with_tenant(Arc::clone(store), auth.tenant_id.clone())
        .write(
            None,
            AuditKind::HttpRequest,
            payload,
            Some(auth.principal_id.clone()),
        )
        .await
    {
        tracing::warn!(
            target: "gateway::marketplace",
            tenant = %auth.tenant_id,
            principal = %auth.principal_id,
            error = %e,
            "marketplace install: audit write failed"
        );
    }
}

#[cfg(test)]
#[path = "marketplace_authz_tests.rs"]
mod authz_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_marketplace_matches_root_and_install() {
        assert!(is_marketplace("GET /marketplace HTTP/1.1\r\n\r\n"));
        assert!(is_marketplace("GET /marketplace/foo HTTP/1.1\r\n\r\n"));
        assert!(is_marketplace(
            "POST /marketplace/foo/install HTTP/1.1\r\n\r\n"
        ));
        assert!(!is_marketplace("GET /marketplaces HTTP/1.1\r\n\r\n"));
        assert!(!is_marketplace("PUT /marketplace HTTP/1.1\r\n\r\n"));
        assert!(is_marketplace("GET /marketplace?q=foo HTTP/1.1\r\n\r\n"));
    }

    #[test]
    fn header_value_is_case_insensitive() {
        let req = "GET / HTTP/1.1\r\nX-Tenant-Id: alpha\r\nHost: x\r\n\r\n";
        assert_eq!(header_value(req, "x-tenant-id"), Some("alpha"));
        assert_eq!(header_value(req, "X-TENANT-ID"), Some("alpha"));
        assert_eq!(header_value(req, "missing"), None);
    }

    #[test]
    fn split_query_separates_path_and_query() {
        assert_eq!(
            split_query("/marketplace?q=foo HTTP/1.1"),
            ("/marketplace", "q=foo")
        );
        assert_eq!(split_query("/marketplace HTTP/1.1"), ("/marketplace", ""));
    }

    #[test]
    fn body_tenant_id_extracts_when_present() {
        let b = r#"{"tenant_id":"alpha","other":1}"#;
        assert_eq!(body_tenant_id(b), Some("alpha".to_string()));
        assert_eq!(body_tenant_id("{}"), None);
        assert_eq!(body_tenant_id("not json"), None);
    }

    #[tokio::test]
    async fn dispatch_list_returns_html_with_skills() {
        let (status, body) = dispatch("GET /marketplace HTTP/1.1\r\n\r\n", None)
            .await
            .expect("list route");
        assert!(status.contains("text/html"));
        assert!(body.contains("Skill marketplace"));
        assert!(body.contains("marketplace-listing-draft"));
        assert!(body.contains("support-ticket-summarizer"));
    }

    #[tokio::test]
    async fn dispatch_list_filters_by_query() {
        let (_, body) = dispatch("GET /marketplace?q=recruiter HTTP/1.1\r\n\r\n", None)
            .await
            .expect("list route");
        assert!(body.contains("recruiter-reply-draft"));
        assert!(!body.contains("support-ticket-summarizer"));
    }

    #[tokio::test]
    async fn dispatch_detail_renders_install_button() {
        let (status, body) = dispatch(
            "GET /marketplace/marketplace-listing-draft HTTP/1.1\r\n\r\n",
            None,
        )
        .await
        .expect("detail route");
        assert!(status.contains("text/html"));
        assert!(body.contains("marketplace-listing-draft"));
        assert!(body.contains("type=\"submit\""));
        assert!(body.contains("Install"));
    }

    #[tokio::test]
    async fn dispatch_detail_404_for_unknown_skill() {
        let (status, body) = dispatch("GET /marketplace/does-not-exist HTTP/1.1\r\n\r\n", None)
            .await
            .expect("detail route");
        assert!(status.starts_with("404"));
        assert!(body.contains("No skill named"));
    }

    #[tokio::test]
    async fn install_requires_authenticated_principal() {
        let (status, _body) = dispatch(
            "POST /marketplace/marketplace-listing-draft/install HTTP/1.1\r\n\r\n",
            None,
        )
        .await
        .expect("install route");
        assert!(status.starts_with("401"));
    }
}
