//! GET dispatcher for operator API resources under `/api/v1`, plus
//! public auth and SCIM status/list routes.
use argo_store::SessionStore;
use std::sync::Arc;

pub(super) async fn dispatch_api_get(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    // mission-sso-saml-oidc-gateway — `GET /auth/status` returns
    // the in-memory `[sso]` config (without secrets) so operators
    // can `curl` to confirm the wire-up. The OIDC + SAML
    // login/callback/ACS routes are dispatched upstream in
    // `boot.rs` (they need stream ownership for `Location:` +
    // `Set-Cookie:` headers).
    if req.starts_with("GET /auth/status ") || req.starts_with("GET /auth/status?") {
        return Some(auth_status_response());
    }
    // `mission-approval-bundle-ui` — server-defined approval bundles.
    // MUST come before the J1 `GET /api/v1/approvals` arm so the more
    // specific `/bundles` path wins prefix-match dispatch.
    if req.starts_with("GET /api/v1/approvals/bundles ")
        || req.starts_with("GET /api/v1/approvals/bundles?")
    {
        return Some(super::routes_approval_bundles::list_response(req, store).await);
    }
    // SCIM 2.0 endpoints (`mission-scim-provisioning`) carry their own
    // bearer credential; mounted here so list/read calls reach the
    // dedicated `scim_auth` gate. The operator-facing bearer gate in
    // `routes::dispatch_route` allowlists `/scim/v2/` paths via
    // `is_public_get` so the credential split is end-to-end.
    if let Some(resp) = super::routes_scim_users::dispatch(req, store).await {
        return Some(resp);
    }
    // J1 — JSON approvals API. The HTML `/inbox` route stays mounted
    // in `routes_channels_webinbox` for legacy operators.
    if req.starts_with("GET /api/v1/approvals ") || req.starts_with("GET /api/v1/approvals?") {
        return Some(super::routes_approvals_json::list_response(req).await);
    }
    // J16 — caller's elevated-view grants (active-only by default).
    // Must come BEFORE the J14 `/api/v1/audit` arm — more-specific
    // path wins so J14's prefix-match doesn't swallow this traffic.
    if req.starts_with("GET /api/v1/audit/elevated-view-grants ")
        || req.starts_with("GET /api/v1/audit/elevated-view-grants?")
    {
        return Some(super::routes_elevated_view::list_response(req, store).await);
    }
    // J6 — tenant-scoped Team-page roster.
    if req.starts_with("GET /api/v1/team ") || req.starts_with("GET /api/v1/team?") {
        return Some(super::routes_team_list::list_response(req, store).await);
    }
    // mission-skill-cohort-ui — fleet-wide cohort fan-out. Detail
    // path (`/api/v1/cohorts/<id>`) must come BEFORE the bare list
    // prefix so the longer route wins.
    if let Some(rest) = req.strip_prefix("GET /api/v1/cohorts/") {
        let segment = rest.split_whitespace().next().unwrap_or(rest);
        return Some(super::routes_cohorts::detail_response(req, segment, store).await);
    }
    if req.starts_with("GET /api/v1/cohorts ") || req.starts_with("GET /api/v1/cohorts?") {
        return Some(super::routes_cohorts::list_response(req, store).await);
    }
    // mission-retro-review-ui — tenant-scoped retro index.
    if req.starts_with("GET /api/v1/retros ") || req.starts_with("GET /api/v1/retros?") {
        return Some(super::routes_retros::index_response(req, store).await);
    }
    // Per-worker GETs under `/api/v1/workers/<id>/...`:
    //   • trust-grade  (mission-trust-grade-ui)
    //   • retros       (mission-retro-review-ui)
    // Pinned ahead of the J14 audit-feed arm so the more-specific
    // prefix wins.
    if let Some(rest) = req.strip_prefix("GET /api/v1/workers/") {
        if let Some((head, _)) = rest.split_once(' ') {
            let parts: Vec<&str> = head.split('/').collect();
            if parts.len() == 2 && parts[1] == "retros" {
                return Some(
                    super::routes_retros::list_for_worker_response(parts[0], req, store).await,
                );
            }
            // mission-approval-policy-ui — per-worker policy index.
            if parts.len() == 2 && parts[1] == "policies" {
                return Some(
                    super::routes_worker_policies::list_response(parts[0], req, store).await,
                );
            }
        }
        if let Some((id_with_suffix, tail)) = rest.split_once('/') {
            if tail.starts_with("trust-grade ") || tail.starts_with("trust-grade?") {
                return Some(
                    super::routes_worker_trust_grade::show_response(id_with_suffix, req, store)
                        .await,
                );
            }
            // J8 — /api/v1/workers/<id>/overview bundles charter,
            // duties, kpis, recent_activity for the Overview tab.
            if tail.starts_with("overview ") || tail.starts_with("overview?") {
                return Some(
                    super::routes_worker_overview::show_response(id_with_suffix, req, store).await,
                );
            }
            // J9 — per-worker audit timeline. Pinned alongside trust-grade
            // so the `/api/v1/workers/<id>/<verb>` family stays grouped.
            if tail.starts_with("activity ") || tail.starts_with("activity?") {
                return Some(
                    super::routes_worker_activity::list_response(id_with_suffix, req, store).await,
                );
            }
            // J10 — Memory-tab bundle (facts + voice-profile + namespace_id).
            // Sub-paths under `/memory/facts/<id>` write through the POST
            // dispatcher (POST/PUT/DELETE).
            if tail.starts_with("memory ") || tail.starts_with("memory?") {
                return Some(
                    super::routes_worker_memory::list_response(id_with_suffix, req, store).await,
                );
            }
            // J11 — bundled settings (channels / quiet / budget / autonomy
            // + operator-only advanced block).
            if tail.starts_with("settings ") || tail.starts_with("settings?") {
                return Some(
                    super::routes_worker_settings::get_settings(id_with_suffix, req, store).await,
                );
            }
        }
        // J7 — bare /api/v1/workers/<id> for the worker profile page.
        // Matches only when the path has no further slash before the
        // space/query, so sibling sub-path arms above (retros,
        // trust-grade) keep priority.
        if let Some((head, _)) = rest.split_once(' ') {
            let id_only = head.split('?').next().unwrap_or(head);
            if !id_only.is_empty() && !id_only.contains('/') {
                return Some(super::routes_worker_detail::show_response(id_only, req, store).await);
            }
        }
    }
    // J15 — audit detail + signed-URL screenshot. Pinned ahead of
    // the J14 audit-feed arm so the more-specific `/api/v1/audit/<id>`
    // path wins over the prefix-match list endpoint.
    if let Some(rest) = req.strip_prefix("GET /api/v1/audit/") {
        if let Some((head, _)) = rest.split_once(' ') {
            let (path, query) = head.split_once('?').unwrap_or((head, ""));
            let segments: Vec<&str> = path.split('/').collect();
            if segments.len() == 3 && segments[1] == "screenshots" {
                return Some(
                    super::routes_audit_screenshot::screenshot_response(
                        segments[0],
                        segments[2],
                        query,
                        store,
                    )
                    .await,
                );
            }
            if segments.len() == 1 {
                return Some(
                    super::routes_audit_detail::detail_response(req, segments[0], store).await,
                );
            }
        }
    }
    // J14 — audit feed with 7 filter dims; unblocks E5-a from mock.
    if req.starts_with("GET /api/v1/audit ") || req.starts_with("GET /api/v1/audit?") {
        return Some(super::routes_audit_feed::list_response(req, store).await);
    }
    None
}

/// `GET /auth/status` — read-only summary of which IdPs are
/// registered under `[sso]`. Returns the master enable flag plus the
/// tenant-id keys for each mechanism; never echoes the
/// `client_secret_env` value or any other potentially sensitive
/// string. Used by operators (and the runbook
/// `argo doctor` follow-up) to confirm SSO is wired correctly.
fn auth_status_response() -> (&'static str, String) {
    let cwd = match std::env::current_dir() {
        Ok(c) => c,
        Err(_) => return ("500 Internal Server Error", r#"{"error":"cwd"}"#.into()),
    };
    let cfg = argo_config::ArgoConfig::load(&cwd.join("argo.toml")).unwrap_or_default();
    let oidc_tenants: Vec<&String> = cfg.sso.oidc.keys().collect();
    let saml_tenants: Vec<&String> = cfg.sso.saml.keys().collect();
    let body = serde_json::json!({
        "enabled": cfg.sso.enabled,
        "session_ttl_seconds": cfg.sso.session_ttl_seconds,
        "cookie_name": cfg.sso.cookie_name,
        "oidc_tenants": oidc_tenants,
        "saml_tenants": saml_tenants,
    });
    ("200 OK", body.to_string())
}
