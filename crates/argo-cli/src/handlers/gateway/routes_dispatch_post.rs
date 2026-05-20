//! POST-route dispatch lifted out of `routes.rs` to keep that
//! file under the 500-LOC budget. Pure router — every match
//! arm forwards to a handler defined in another sibling
//! module. Returns `Some(resp)` on a match, `None` otherwise so
//! `routes.rs` can fall through to its 404 default.
//!
//! Domain-grouped sub-dispatchers live in sibling files
//! (`routes_dispatch_post_workers`, `_audit`, `_memory`,
//! `_retros`, `_duties`, `_approvals`). Hub-local arms cover
//! cross-domain plumbing (sessions/verticals/skills/cron/forge/
//! db/metrics/judge/genesis/workspace) where a single sibling
//! grouping would not help.
use super::routes_auth::check_bearer_auth;
use super::routes_auth_principal::{parse_principal_header, principal_id_for_audit};
use super::routes_post::create_session_response;
use argo_store::tenant_quota::TenantQuotaStore;
use argo_store::tenant_quota_kinds::{QuotaActionKind, QuotaError};
use argo_store::SessionStore;
use std::sync::Arc;
use tracing::Instrument;

/// HTTP header carrying a caller-supplied request correlation id
/// for log/audit cross-referencing. Optional — emissions get an
/// empty string when the header is absent.
const REQUEST_ID_HEADER: &str = "X-Request-Id";

/// Extract the request line's `<verb> <path>` (no HTTP version).
/// `req` is the raw HTTP wire form; the first line is always the
/// request line per RFC 9112. Returns `""` on a malformed prefix.
fn request_route(req: &str) -> &str {
    let line = req.lines().next().unwrap_or("");
    line.rsplit_once(' ').map_or(line, |(head, _)| head)
}

/// Read `X-Request-Id` from the header block. Returns the trimmed
/// value or an empty string when the header is missing.
fn request_id_header(req: &str) -> &str {
    let headers = req
        .split_once("\r\n\r\n")
        .map_or(req, |(headers, _body)| headers);
    headers
        .lines()
        .skip(1)
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case(REQUEST_ID_HEADER)
                .then(|| value.trim())
        })
        .unwrap_or("")
}

/// Resolve the cwd-derived tenant id and ask the quota store
/// whether one more `kind` action would exceed the configured
/// cap. Returns `Some((status, body))` to short-circuit dispatch
/// with HTTP 429; returns `None` on pass or on internal error
/// (fail-open — the gate is not authoritative when the store is
/// unreachable; observability still records the underlying
/// failure via the existing summary read path).
pub(super) async fn check_quota_or_reject(
    store: &Arc<SessionStore>,
    kind: QuotaActionKind,
) -> Option<(&'static str, String)> {
    let cwd = std::env::current_dir().ok()?;
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, _ws) = store.ensure_workspace(name, &root).await.ok()?;
    let tenant_id = tenant.0.to_string();
    let quotas = TenantQuotaStore::new(store.pool());
    match quotas
        .check_or_reject(&tenant_id, kind, 1, chrono::Utc::now())
        .await
    {
        Ok(()) => None,
        Err(QuotaError::Exceeded {
            kind,
            current,
            limit,
        }) => Some((
            "429 Too Many Requests",
            format!(
                "{{\"error\":\"quota_exceeded\",\"kind\":\"{kind}\",\"current\":{current},\"limit\":{limit}}}"
            ),
        )),
        Err(QuotaError::Internal(_)) => None,
    }
}

/// Dispatch every POST verb. Returns `None` when no match,
/// letting `dispatch_route` fall through to its other branches
/// (and ultimately the 404 default).
///
/// Wrapped in the canonical gateway request span so every
/// `info!`/`warn!`/`error!` emission below this point inherits
/// `route`, `principal_id`, and `request_id` fields without a
/// per-callsite change (mvp-structured-logging).
pub(super) async fn dispatch_post(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    let span = tracing::info_span!(
        argo_telemetry::spans::GATEWAY_REQUEST_DISPATCH,
        route = %request_route(req),
        principal_id = %principal_id_for_audit(req).unwrap_or_default(),
        request_id = %request_id_header(req),
    );
    dispatch_post_inner(req, store).instrument(span).await
}

async fn dispatch_post_inner(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    if let Some(resp) = preflight_dispatch(req, store).await {
        return Some(resp);
    }
    if let Some(resp) = api_v1_dispatch(req, store).await {
        return Some(resp);
    }
    if let Some(resp) = domain_dispatchers(req, store).await {
        return Some(resp);
    }
    if let Some(resp) = top_level_dispatch(req, store).await {
        return Some(resp);
    }
    if let Some(resp) = super::routes_dispatch_post_paths::dispatch_simple_admin(req, store).await {
        return Some(resp);
    }
    if let Some(resp) = super::routes_dispatch_post_paths::dispatch_forge(req, store).await {
        return Some(resp);
    }
    if let Some(resp) =
        super::routes_dispatch_post_paths::dispatch_session_end_all(req, store).await
    {
        return Some(resp);
    }
    if let Some(resp) = super::routes_dispatch_post_paths::dispatch_skill_named(req, store).await {
        return Some(resp);
    }
    if let Some(resp) = super::routes_dispatch_post_paths::dispatch_session_path(req, store).await {
        return Some(resp);
    }
    // mission-pm-workflow-edit-api — workflow-definition CRUD
    // mutations under `/api/v1/verticals/{vertical_id}/workflows/...`.
    // Covers POST (create version), PUT (set head), DELETE (archive
    // version). See `routes_workflows::dispatch_mutate`.
    if let Some(resp) = super::routes_workflows::dispatch_mutate(req, store).await {
        return Some(resp);
    }
    None
}

/// Pre-routing gates: SCIM bearer auth, the unauthenticated wizard
/// surface, then bearer + principal-header validation. Any non-`None`
/// return short-circuits the dispatcher.
async fn preflight_dispatch(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    // SCIM 2.0 endpoints carry their own bearer credential and must
    // bypass the operator bearer gate. The wizard surface is
    // intentionally unauthenticated.
    if let Some(resp) = super::routes_scim_users::dispatch(req, store).await {
        return Some(resp);
    }
    if let Some(resp) = super::routes_wizard::dispatch_post(req).await {
        return Some(resp);
    }
    if let Some(resp) = check_bearer_auth(req) {
        return Some(resp);
    }
    if parse_principal_header(req).is_err() {
        return Some((
            "400 Bad Request",
            r#"{"error":"bad principal"}"#.to_string(),
        ));
    }
    None
}

/// `/api/v1/*` family routes: genesis, ghostwriter, skill IR,
/// retrain, PATCH/DELETE dispatchers, approval bundles, worker
/// lifecycle. Bearer + principal validation are guaranteed by
/// [`preflight_dispatch`] before this is called.
async fn api_v1_dispatch(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    if req.starts_with("POST /api/v1/genesis/classify ") {
        return Some(super::routes_genesis::classify_response(req, store).await);
    }
    if req.starts_with("POST /api/v1/genesis/draft-worker ") {
        return Some(super::routes_genesis::draft_worker_response(req, store).await);
    }
    if req.starts_with("POST /api/v1/skills/draft-from-prompt ")
        || req.starts_with("POST /api/v1/skills/draft-from-prompt?")
    {
        return Some(super::routes_ghostwriter::draft_from_prompt_response(req, store).await);
    }
    // claude-tenant-provisioning-endpoint — self-serve onboarding.
    // Bearer auth + principal-header validation applied above; the
    // handler also requires a `human:<email>` principal to scope the
    // provisioned tenant to the magic-link-verified caller.
    if req.starts_with("POST /api/v1/onboarding/provision ")
        || req.starts_with("POST /api/v1/onboarding/provision?")
    {
        return Some(super::onboarding_provision::provision_response(req, store).await);
    }
    // mission-data-export-customer-flow — customer-facing GDPR/CCPA
    // Article 15 export. Scoped to the X-Argo-Principal human caller.
    if let Some(resp) = super::routes_me_export::dispatch(req, store).await {
        return Some(resp);
    }
    if let Some(resp) = super::routes_skill_ir::dispatch_post(req, store).await {
        return Some(resp);
    }
    if let Some(resp) = super::routes_retrain::dispatch(req, store).await {
        return Some(resp);
    }
    if let Some(resp) = super::routes_dispatch_patch::dispatch_patch(req, store).await {
        return Some(resp);
    }
    if let Some(resp) = super::routes_dispatch_delete::dispatch_delete(req, store).await {
        return Some(resp);
    }
    if let Some(resp) =
        super::routes_dispatch_post_paths::dispatch_approval_bundles(req, store).await
    {
        return Some(resp);
    }
    if let Some(resp) =
        super::routes_dispatch_post_paths::dispatch_worker_lifecycle(req, store).await
    {
        return Some(resp);
    }
    // web-gateway-endpoints-notifications-batch — bell-popover backing
    // API. Routes `POST /api/v1/notifications/<id>/dismiss` and
    // `POST /api/v1/notifications/dismiss-all`.
    if let Some(resp) = super::routes_notifications::dispatch_post(req, store).await {
        return Some(resp);
    }
    None
}

/// Domain-grouped sub-dispatchers (siblings). Each owns a disjoint
/// URL space; order between them is not significant. Orchestration
/// is included here because its match key is `/api/v1/...` too.
async fn domain_dispatchers(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    if let Some(resp) = super::routes_dispatch_post_workers::dispatch_workers(req, store).await {
        return Some(resp);
    }
    if let Some(resp) = super::routes_dispatch_post_audit::dispatch_audit(req, store).await {
        return Some(resp);
    }
    if let Some(resp) = super::routes_dispatch_post_memory::dispatch_memory(req, store).await {
        return Some(resp);
    }
    // web-gateway-endpoints-worker-skills-batch — POST/DELETE for the
    // per-worker Skills tab (add / remove / promote). Sibling
    // dispatcher lives on the handler module rather than a separate
    // `_skills` file so the URL pattern stays one hop away from its
    // handler set.
    if let Some(resp) = super::routes_worker_skills::dispatch_post(req, store).await {
        return Some(resp);
    }
    if let Some(resp) = super::routes_dispatch_post_retros::dispatch_retros(req, store).await {
        return Some(resp);
    }
    if let Some(resp) = super::routes_dispatch_post_duties::dispatch_duties(req, store).await {
        return Some(resp);
    }
    if let Some(resp) = super::routes_dispatch_post_paths::dispatch_orchestration(req, store).await
    {
        return Some(resp);
    }
    if let Some(resp) = super::routes_dispatch_post_approvals::dispatch_approvals(req).await {
        return Some(resp);
    }
    if let Some(resp) =
        super::routes_dispatch_post_paths::dispatch_support_tickets(req, store).await
    {
        return Some(resp);
    }
    None
}

/// `POST /sessions` (the collection-level create). Quota-gated, and
/// a 2xx response fans out a peer-poke for fast resync.
async fn top_level_dispatch(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    if !req.starts_with("POST /sessions ") {
        return None;
    }
    if let Some(s) = store {
        if let Some(resp) = check_quota_or_reject(s, QuotaActionKind::Sessions).await {
            return Some(resp);
        }
    }
    let resp = create_session_response(req, store).await;
    if resp.0.starts_with("200") || resp.0.starts_with("201") {
        super::peer::notify_peers_of_local_change();
    }
    Some(resp)
}

#[cfg(test)]
#[path = "routes_dispatch_post_tests.rs"]
mod tests;
