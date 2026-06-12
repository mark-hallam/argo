//! Sub-dispatchers grouped by URL family for
//! [`super::routes_dispatch_get_api::dispatch_api_get`]. Each helper
//! owns a disjoint URL prefix and returns `Some(resp)` on a hit so
//! the parent dispatcher early-returns.
#[path = "routes_workers_list.rs"]
mod routes_workers_list;

use argo_store::SessionStore;
use serde_json::json;
use std::sync::Arc;

/// `GET /auth/status`, `/api/v1/meta/error-reporting`, workspace
/// members, `/api/v1/me/export/...`, and approval-bundles list.
pub(super) async fn dispatch_auth_and_meta(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    if req.starts_with("GET /auth/status ") || req.starts_with("GET /auth/status?") {
        return Some(auth_status_response());
    }
    if req.starts_with("GET /api/v1/meta/error-reporting ")
        || req.starts_with("GET /api/v1/meta/error-reporting?")
    {
        return Some(super::routes_meta_error_reporting::response());
    }
    if super::routes_workspace_members::is_list_request(req) {
        return Some(super::routes_workspace_members::list_response(req, store).await);
    }
    if req.starts_with("GET /api/v1/me/export/") {
        if let Some(resp) = super::routes_me_export::dispatch(req, store).await {
            return Some(resp);
        }
    }
    if req.starts_with("GET /api/v1/approvals/bundles ")
        || req.starts_with("GET /api/v1/approvals/bundles?")
    {
        return Some(super::routes_approval_bundles::list_response(req, store).await);
    }
    if req.starts_with("GET /api/v1/verticals ") || req.starts_with("GET /api/v1/verticals?") {
        return Some(super::routes_verticals_list_get::list_response(req, store).await);
    }
    if let Some(resp) = dispatch_vertical_detail_alias(req).await {
        return Some(resp);
    }
    None
}

/// `/api/v1/verticals/<id>` — singular detail alias that delegates
/// to the same handler as the bare `/verticals/<id>` route so the
/// api-prefixed and unprefixed surfaces stay symmetric. Mirrors the
/// workers-collection alias shipped in `9ea693db1`. Only matches
/// when `<id>` has no further `/` segment, so the per-vertical
/// `/workflows[...]`, `/lint`, `/doctor` paths fall through to
/// their existing dispatchers.
async fn dispatch_vertical_detail_alias(req: &str) -> Option<(&'static str, String)> {
    let rest = req.strip_prefix("GET /api/v1/verticals/")?;
    let (head, _) = rest.split_once(' ')?;
    let (id_with_query, query) = match head.split_once('?') {
        Some((i, q)) => (i, Some(q)),
        None => (head, None),
    };
    if id_with_query.is_empty() || id_with_query.contains('/') {
        return None;
    }
    let format = query
        .and_then(|q| q.split('&').find_map(|kv| kv.strip_prefix("format=")))
        .map(str::to_string);
    Some(super::routes_verticals::show_response(id_with_query, format.as_deref()).await)
}

/// SCIM, JSON approvals, skill IR, audit elevated-view, team/workers
/// roster, notifications, and tenant usage.
pub(super) async fn dispatch_scim_and_team(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    if let Some(resp) = super::routes_scim_users::dispatch(req, store).await {
        return Some(resp);
    }
    if req.starts_with("GET /api/v1/approvals ") || req.starts_with("GET /api/v1/approvals?") {
        return Some(super::routes_approvals_json::list_response(req, store).await);
    }
    if let Some(resp) = super::routes_skill_ir::dispatch(req, store).await {
        return Some(resp);
    }
    if req.starts_with("GET /api/v1/audit/elevated-view-grants ")
        || req.starts_with("GET /api/v1/audit/elevated-view-grants?")
    {
        return Some(super::routes_elevated_view::list_response(req, store).await);
    }
    if req.starts_with("GET /api/v1/workers ") || req.starts_with("GET /api/v1/workers?") {
        return Some(routes_workers_list::list_response(req, store).await);
    }
    if req.starts_with("GET /api/v1/team/scorecard ")
        || req.starts_with("GET /api/v1/team/scorecard?")
    {
        return Some(super::routes_worker_scorecard::team_scorecard_response(req, store).await);
    }
    if req.starts_with("GET /api/v1/team ") || req.starts_with("GET /api/v1/team?") {
        return Some(super::routes_team_list::deprecated_list_response(req, store).await);
    }
    if let Some(resp) = super::routes_notifications::dispatch_get(req, store).await {
        return Some(resp);
    }
    // theatre item T1 — workspace/providers/notifications settings panels.
    // Pinned after the bell-popover notifications GET so the exact
    // `/api/v1/notifications/preferences` match cannot be shadowed by
    // the `/api/v1/notifications` list prefix above.
    if let Some(resp) = super::routes_workspace_settings::dispatch_get(req, store).await {
        return Some(resp);
    }
    // Per-tenant white-label theme override (reuses the workspace_settings
    // `theme` group). Returns the locked SKYCOT default when unset.
    if let Some(resp) = super::routes_tenant_theme::dispatch_get(req, store).await {
        return Some(resp);
    }
    // Gap #1 move A — audit-readiness / governance report. Read-only
    // assembly over the audit chain + capability roles + truth boundary
    // + approval coverage for one tenant.
    if let Some(resp) = super::routes_governance::dispatch_get(req, store).await {
        return Some(resp);
    }
    if req.starts_with("GET /api/v1/tenant/usage ")
        || req.starts_with("GET /api/v1/tenant/usage?")
    {
        return Some(super::usage_endpoint::show_response(req, store).await);
    }
    // DEC-053 honest Pro meter: included vs used vs overage + the
    // effective hard cap (pricing-stripe-metered-overage-and-caps).
    if req.starts_with("GET /api/v1/billing/meter ")
        || req.starts_with("GET /api/v1/billing/meter?")
    {
        return Some(super::billing_meter::meter_response(req, store).await);
    }
    // UF A7 — tenant-scoped dollar surface. The `/tenant/costs/...`
    // prefix is disjoint from the rollups above; the more-specific
    // `by-worker` suffix is matched before the `total` read.
    if req.starts_with("GET /api/v1/tenant/costs/by-worker ")
        || req.starts_with("GET /api/v1/tenant/costs/by-worker?")
    {
        return Some(super::routes_tenant_costs::by_worker_response(req, store).await);
    }
    if req.starts_with("GET /api/v1/tenant/costs/total ")
        || req.starts_with("GET /api/v1/tenant/costs/total?")
    {
        return Some(super::routes_tenant_costs::total_response(req, store).await);
    }
    None
}

/// `/api/v1/cohorts[...]`, orchestration, dashboards, retros.
pub(super) async fn dispatch_cohorts_and_dashboards(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    if let Some(rest) = req.strip_prefix("GET /api/v1/cohorts/") {
        let segment = rest.split_whitespace().next().unwrap_or(rest);
        return Some(super::routes_cohorts::detail_response(req, segment, store).await);
    }
    if req.starts_with("GET /api/v1/cohorts ") || req.starts_with("GET /api/v1/cohorts?") {
        return Some(super::routes_cohorts::list_response(req, store).await);
    }
    if req.starts_with("GET /api/v1/orchestration/runs ")
        || req.starts_with("GET /api/v1/orchestration/runs?")
    {
        return Some(super::routes_orchestration::list_response(req, store).await);
    }
    if req.starts_with("GET /api/v1/dashboards/error-rate ")
        || req.starts_with("GET /api/v1/dashboards/error-rate?")
    {
        return Some(crate::handlers::errors_dashboard::gateway_response(req, store).await);
    }
    if req.starts_with("GET /api/v1/dashboards/sla ")
        || req.starts_with("GET /api/v1/dashboards/sla?")
    {
        return Some(crate::handlers::sla_report::gateway_response(req, store).await);
    }
    if req.starts_with("GET /api/v1/retros ") || req.starts_with("GET /api/v1/retros?") {
        return Some(super::routes_retros::index_response(req, store).await);
    }
    None
}

/// `/api/v1/workers/<id>/...` — per-worker GETs (retros, policies,
/// trust-grade, evals, trust-streaks, overview, activity, memory,
/// settings, skills, plus the bare worker-detail show).
pub(super) async fn dispatch_workers_family(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    let rest = req.strip_prefix("GET /api/v1/workers/")?;
    if let Some(resp) = dispatch_worker_two_segment(rest, req, store).await {
        return Some(resp);
    }
    if let Some(resp) = dispatch_worker_subpath(rest, req, store).await {
        return Some(resp);
    }
    // Bare /api/v1/workers/<id> — matches only when the path has no
    // further slash before the space/query, so sibling sub-path arms
    // above keep priority.
    if let Some((head, _)) = rest.split_once(' ') {
        let id_only = head.split('?').next().unwrap_or(head);
        if !id_only.is_empty() && !id_only.contains('/') {
            return Some(super::routes_worker_detail::show_response(id_only, req, store).await);
        }
    }
    None
}

/// `/api/v1/workers/<id>/{retros,policies,duties,escalations}` — exact
/// two-segment paths. Pinned first so the more-specific match wins.
async fn dispatch_worker_two_segment(
    rest: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    let (head, _) = rest.split_once(' ')?;
    let parts: Vec<&str> = head.split('/').collect();
    if parts.len() != 2 {
        return None;
    }
    match parts[1] {
        "retros" => {
            Some(super::routes_retros::list_for_worker_response(parts[0], req, store).await)
        }
        "policies" => {
            Some(super::routes_worker_policies::list_response(parts[0], req, store).await)
        }
        "duties" => Some(super::routes_duties::list_response(parts[0], req, store).await),
        "escalations" => {
            Some(super::routes_worker_escalations::list_response(parts[0], req, store).await)
        }
        _ => None,
    }
}

/// `/api/v1/workers/<id>/<verb>...` — single-slash sub-paths under
/// a worker id (trust-grade, evals, trust-streaks, overview,
/// activity, memory, settings, skills, skills/<sid>/history).
async fn dispatch_worker_subpath(
    rest: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    let (id_with_suffix, tail) = rest.split_once('/')?;
    // `runs/<job_id>/output` — a single teammate run's artifact bundle.
    // Pinned first among the sub-paths since its three-segment shape is
    // disjoint from every single-verb arm below.
    if let Some(after) = tail.strip_prefix("runs/") {
        if let Some((job_seg, rest_after)) = after.split_once('/') {
            if rest_after.starts_with("output ") || rest_after.starts_with("output?") {
                return Some(
                    super::routes_worker_outputs::output_response(
                        id_with_suffix,
                        job_seg,
                        req,
                        store,
                    )
                    .await,
                );
            }
        }
    }
    // `skills/<sid>/history` — the other multi-segment sub-path. Disjoint
    // from the single-verb arms, so checking it here (before the simple-verb
    // helper, which also owns the `skills` list verb) is order-preserving.
    if tail.starts_with("skills/") {
        let after = tail.trim_start_matches("skills/");
        if let Some((sid, rest_after)) = after.split_once('/') {
            if rest_after.starts_with("history ") || rest_after.starts_with("history?") {
                return Some(
                    super::routes_worker_skills::history_response(
                        id_with_suffix,
                        sid,
                        req,
                        store,
                    )
                    .await,
                );
            }
        }
    }
    dispatch_worker_simple_verb(id_with_suffix, tail, req, store).await
}

/// The single-verb sub-paths under a worker id (no nested segments):
/// trust-grade, evals(+replay), trust-streaks, overview, activity,
/// scorecard, memory, settings, retrain/recent, skills (list). Split out
/// of [`dispatch_worker_subpath`] to keep each function under the 80-LOC
/// invariant; the arms are mutually disjoint so the order is preserved.
async fn dispatch_worker_simple_verb(
    id_with_suffix: &str,
    tail: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    if tail.starts_with("trust-grade ") || tail.starts_with("trust-grade?") {
        return Some(
            super::routes_worker_trust_grade::show_response(id_with_suffix, req, store).await,
        );
    }
    if tail.starts_with("evals/replay ") || tail.starts_with("evals/replay?") {
        return Some(
            super::routes_worker_evals_replay::replay_response(id_with_suffix, req, store).await,
        );
    }
    if tail.starts_with("evals ") || tail.starts_with("evals?") {
        return Some(super::routes_evals::list_response(id_with_suffix, req, store).await);
    }
    if tail.starts_with("trust-streaks ") || tail.starts_with("trust-streaks?") {
        return Some(
            super::routes_trust_streaks::list_response(id_with_suffix, req, store).await,
        );
    }
    if tail.starts_with("overview ") || tail.starts_with("overview?") {
        return Some(
            super::routes_worker_overview::show_response(id_with_suffix, req, store).await,
        );
    }
    if tail.starts_with("activity ") || tail.starts_with("activity?") {
        return Some(
            super::routes_worker_activity::list_response(id_with_suffix, req, store).await,
        );
    }
    if tail.starts_with("scorecard ") || tail.starts_with("scorecard?") {
        return Some(
            super::routes_worker_scorecard::show_response(id_with_suffix, req, store).await,
        );
    }
    if tail.starts_with("memory ") || tail.starts_with("memory?") {
        return Some(
            super::routes_worker_memory::list_response(id_with_suffix, req, store).await,
        );
    }
    if tail.starts_with("settings ") || tail.starts_with("settings?") {
        return Some(
            super::routes_worker_settings::get_settings(id_with_suffix, req, store).await,
        );
    }
    if tail.starts_with("retrain/recent ") || tail.starts_with("retrain/recent?") {
        return Some(super::routes_retrain::recent_response(id_with_suffix, req, store).await);
    }
    if tail.starts_with("skills ") || tail.starts_with("skills?") {
        return Some(
            super::routes_worker_skills::list_response(id_with_suffix, req, store).await,
        );
    }
    // launch-cloud-file-ingress — list the teammate's fs_root (names + sizes).
    if tail.starts_with("files ") || tail.starts_with("files?") {
        return Some(
            super::routes_worker_files::list_response(id_with_suffix, req, store).await,
        );
    }
    // tg1 — pending capability requests (fence denies) backing the
    // connect-on-demand grant card.
    if tail.starts_with("capability-requests ") || tail.starts_with("capability-requests?") {
        return Some(
            super::routes_worker_capability_requests::list_response(id_with_suffix, req, store)
                .await,
        );
    }
    None
}

/// `/api/v1/audit/...` (detail + screenshots + feed), support
/// tickets list/detail, and knowledge sub-dispatcher.
pub(super) async fn dispatch_audit_support_knowledge(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
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
                if is_audit_detail_id(segments[0]) {
                    return Some(super::routes_audit_detail::detail_response(req, segments[0], store).await);
                }
                return Some(audit_endpoint_not_found(path));
            }
        }
    }
    if req.starts_with("GET /api/v1/audit ") || req.starts_with("GET /api/v1/audit?") {
        return Some(super::routes_audit_feed::list_response(req, store).await);
    }
    // Past live-voice sessions (transcripts grouped by session), for the history view.
    if req.starts_with("GET /api/v1/voice/sessions ") || req.starts_with("GET /api/v1/voice/sessions?") {
        return Some(super::routes_voice_realtime::voice_sessions_response(req, store).await);
    }
    if let Some(rest) = req.strip_prefix("GET /api/v1/support/tickets/") {
        if let Some((head, _)) = rest.split_once(' ') {
            let id = head.split('?').next().unwrap_or(head);
            if !id.is_empty() && !id.contains('/') {
                return Some(super::routes_support_tickets::detail_response(id, req, store).await);
            }
        }
    }
    if req.starts_with("GET /api/v1/support/tickets ")
        || req.starts_with("GET /api/v1/support/tickets?")
    {
        return Some(super::routes_support_tickets::list_response(req, store).await);
    }
    if let Some(resp) = super::routes_knowledge::dispatch_get_api(req, store).await {
        return Some(resp);
    }
    None
}

fn is_audit_detail_id(segment: &str) -> bool {
    segment.len() == 32 && segment.bytes().all(|b| b.is_ascii_hexdigit())
}

fn audit_endpoint_not_found(path_tail: &str) -> (&'static str, String) {
    (
        "404 Not Found",
        json!({
            "error": "endpoint_not_found",
            "path": format!("/api/v1/audit/{path_tail}"),
            "hint": "see /api/v1/audit or /api/v1/audit/events",
        })
        .to_string(),
    )
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
