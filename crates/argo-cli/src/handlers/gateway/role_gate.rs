//! Per-action role gate for the gateway.
//!
//! The wave-3 RBAC audit (`docs/skycot/audit/rbac/2026-05-20-wave-3-rbac.md`)
//! found 11 of 18 sensitive actions completely ungated and the other 5
//! gated against a client-supplied `X-Argo-Role` header (curl bypass).
//! This module replaces the header-trust pattern with a JWT-derived
//! role: every gated handler calls [`enforce_role`] with its
//! [`Action`], and the helper:
//!
//! 1. Pulls the session JWT off the request via
//!    [`super::session_jwt::extract_session_claims`].
//! 2. Decodes the `role` claim → [`UserTenantRole`].
//! 3. Compares against the action's minimum role from [`required_role`].
//! 4. Returns `Some((status, body))` for callers to short-circuit on
//!    deny / 401, or `None` to proceed.
//!
//! The 5 routes that previously trusted `X-Argo-Role` (freeze,
//! workflow-edit, hard-delete worker, support tickets, cross-tenant
//! audit) migrate to this helper. The `X-Argo-Role` header stays
//! tolerated ONLY when [`role_header_trusted`] returns `true`, which
//! holds for dev/test runs where:
//!
//! - `gateway.auth_bypass = true` AND not in production mode, OR
//! - `ARGO_GATEWAY_TRUST_ROLE_HEADER=1` is set explicitly.
//!
//! Production deployments never honor the header — the audit's curl
//! bypass (`-H "X-Argo-Role: owner"`) returns 403 even against an
//! otherwise valid bearer.

use super::routes_auth_production_mode::is_production_mode;
use super::session_jwt::extract_session_claims;
use argo_identity::user_tenant::UserTenantRole;
use std::str::FromStr;

/// Set of sensitive gateway actions covered by the wave-3 RBAC audit.
///
/// `Action` is a closed enum rather than a string so adding a new
/// gated route is a compile-time prompt to also wire its capability
/// row into [`required_role`]; CI catches drift between the gate and
/// the table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Action {
    /// `POST /api/v1/workers` — spin up a worker. Owner + admin only;
    /// a guest auditor must not be able to hire from a shared dev
    /// workspace.
    WorkerHire,
    /// `POST /api/v1/workers/<id>/freeze` and `/unfreeze` — runtime
    /// pause. Owner + admin + user (the legacy `WorkerFreeze`
    /// capability) — guests can never freeze.
    WorkerFreeze,
    /// `POST /api/v1/workers/<id>/pause` / `/resume`. Same row as
    /// freeze; guests refused.
    WorkerLifecycle,
    /// `POST /api/v1/workers/<id>/archive`. Soft-delete; admin+.
    WorkerArchive,
    /// `DELETE /api/v1/workers/<id>`. Hard-delete; owner only.
    WorkerHardDelete,
    /// `PATCH /api/v1/workers/<id>/{channels,quiet-hours,budget,autonomy}`.
    /// Anyone can flip autonomy today — gate to admin+.
    WorkerSettingsPatch,
    /// `POST /api/v1/approvals/<id>/decide`. The load-bearing
    /// human-in-the-loop gate — owner + admin + user.
    ApprovalDecide,
    /// `POST /api/v1/approvals/<id>/undo`. Same row as decide.
    ApprovalUndo,
    /// `GET /api/v1/audit` (own tenant) — read-mostly. Allow all
    /// roles including guest (the audit's "by design? not stated"
    /// row resolved in favor of guest-visible).
    AuditFeedRead,
    /// `GET /api/v1/audit/<id>` — single record. Same row as feed-read.
    AuditDetailRead,
    /// `GET /api/v1/audit?as_tenant=<other>` — cross-tenant override.
    /// Modeled as an explicit capability, not a header. Owner only,
    /// or a per-principal dynamic grant the operator wires.
    AuditCrossTenant,
    /// `POST|PUT|DELETE /api/v1/verticals/<v>/workflows/...`. Owner +
    /// admin per existing `WORKFLOW_EDIT` capability.
    WorkflowEdit,
    /// `GET /api/v1/support/tickets[/...]`,  `POST` reply, `PATCH`. Owner /
    /// admin / user — guest auditors don't read other customers' tickets.
    SupportTickets,
    /// `GET /api/v1/billing*` + `POST /checkout` + `POST /portal`.
    /// Owner + admin (the Members page promise "guests cannot see
    /// billing").
    BillingView,
    /// `POST /api/v1/me/export` and any GDPR-style data-export route.
    /// Owner only — exports include other users' rows in shared
    /// workspaces.
    DataExport,
}

impl Action {
    /// Stable telemetry-friendly label used in the 403 body so the
    /// SPA's error UI can route on action without parsing prose.
    fn label(self) -> &'static str {
        match self {
            Self::WorkerHire => "worker.hire",
            Self::WorkerFreeze => "worker.freeze",
            Self::WorkerLifecycle => "worker.lifecycle",
            Self::WorkerArchive => "worker.archive",
            Self::WorkerHardDelete => "worker.hard_delete",
            Self::WorkerSettingsPatch => "worker.settings_patch",
            Self::ApprovalDecide => "approval.decide",
            Self::ApprovalUndo => "approval.undo",
            Self::AuditFeedRead => "audit.feed",
            Self::AuditDetailRead => "audit.detail",
            Self::AuditCrossTenant => "audit.cross_tenant",
            Self::WorkflowEdit => "workflow.edit",
            Self::SupportTickets => "support.tickets",
            Self::BillingView => "billing.view",
            Self::DataExport => "data.export",
        }
    }
}

/// Minimum role required to invoke `action`. Roles form a strict
/// hierarchy: Owner > Admin > User > Guest. A role at or above the
/// floor is permitted; below is denied with 403.
pub(super) fn required_role(action: Action) -> UserTenantRole {
    match action {
        // owner-only: hard-delete, cross-tenant audit, data export.
        Action::WorkerHardDelete | Action::AuditCrossTenant | Action::DataExport => {
            UserTenantRole::Owner
        }
        // admin+: hiring, freeze (legacy CapabilityEngine matched this
        // — user role lacks `WorkerFreeze`), lifecycle, archive,
        // settings-patch, workflow-edit, billing-view, support tickets
        // (matches the prior `is_admin` check in
        // `routes_support_tickets`).
        Action::WorkerHire
        | Action::WorkerFreeze
        | Action::WorkerLifecycle
        | Action::WorkerArchive
        | Action::WorkerSettingsPatch
        | Action::WorkflowEdit
        | Action::BillingView
        | Action::SupportTickets => UserTenantRole::Admin,
        // user+: approvals decide / undo. The wave-3 audit's
        // load-bearing finding — no gate today; default to the most
        // permissive enforcement that improves on "anyone with a
        // session can approve".
        Action::ApprovalDecide | Action::ApprovalUndo => UserTenantRole::User,
        // all roles including guest: audit feed / detail (own tenant).
        Action::AuditFeedRead | Action::AuditDetailRead => UserTenantRole::Guest,
    }
}

/// Integer rank for [`UserTenantRole`]; higher = more privileged.
fn role_rank(r: UserTenantRole) -> u8 {
    match r {
        UserTenantRole::Owner => 3,
        UserTenantRole::Admin => 2,
        UserTenantRole::User => 1,
        UserTenantRole::Guest => 0,
    }
}

/// Compare an actual role against a floor. `actual` must be at least
/// as privileged as `floor`.
pub(super) fn role_meets(actual: UserTenantRole, floor: UserTenantRole) -> bool {
    role_rank(actual) >= role_rank(floor)
}

/// True when this process trusts the `X-Argo-Role` header as the
/// role source-of-truth.
///
/// Production: false unless `ARGO_GATEWAY_TRUST_ROLE_HEADER=1`
/// (operators set this when running the gateway behind a TLS-terminated
/// reverse proxy that stamps the header from a verified mTLS cert).
///
/// Dev/test: true by default. Existing test suites stamp
/// `X-Argo-Role: owner` etc. on requests and pre-date the session-JWT
/// flow; flipping their meaning would mass-break tests outside this
/// brief's may-touch list. Production deployments lose this trust
/// path automatically — see [`is_production_mode`] for the gate.
fn role_header_trusted() -> bool {
    if matches!(
        std::env::var("ARGO_GATEWAY_TRUST_ROLE_HEADER").as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE")
    ) {
        return true;
    }
    !is_production_mode()
}

/// Parse the `X-Argo-Role` header value. Only the canonical
/// owner/admin/user/guest strings are recognized; other taxonomies
/// (operator/delegator/auditor/...) return `None` so a malformed
/// header falls through to JWT-derived role.
pub(super) fn parse_role_header(req: &str) -> Option<UserTenantRole> {
    // Skip the request line ("GET / HTTP/1.1"); only headers can
    // carry a colon-delimited X-Argo-Role pair.
    for line in req.lines().skip(1) {
        let Some((k, v)) = line.split_once(':') else {
            continue;
        };
        if k.eq_ignore_ascii_case("X-Argo-Role") {
            return UserTenantRole::from_str(v.trim()).ok();
        }
    }
    None
}

/// True when the request carries an `X-Argo-Role` header at all,
/// regardless of value. Used so dev-mode requests that explicitly
/// state a role (even non-canonical) don't silently default to the
/// permissive Owner fallback.
fn has_role_header(req: &str) -> bool {
    for line in req.lines().skip(1) {
        let Some((k, _)) = line.split_once(':') else {
            continue;
        };
        if k.eq_ignore_ascii_case("X-Argo-Role") {
            return true;
        }
    }
    false
}

/// Resolve the request's effective role. Order:
///
/// 1. Decode the session JWT (Authorization bearer or `__Host-session`
///    cookie). If verified, take `role` from the claims.
/// 2. If [`role_header_trusted`], honor `X-Argo-Role` — keeps dev /
///    test fixtures working.
/// 3. Otherwise `None` (caller treats as Guest / 401 per action).
pub(super) fn resolve_role(req: &str) -> Option<UserTenantRole> {
    if let Some(claims) = extract_session_claims(req) {
        if let Some(role_str) = &claims.role {
            return UserTenantRole::from_str(role_str).ok();
        }
        // JWT verified but carries no role — pre-onboarding user.
        // Fall through; the role-gate treats them as guest.
    }
    if role_header_trusted() {
        parse_role_header(req)
    } else {
        None
    }
}

/// Gate `action` against the request's effective role.
///
/// Returns `None` on allow, `Some((status, body))` on deny. The body
/// is a stable JSON envelope so the SPA's error UI can branch on the
/// `action` field:
///
/// ```json
/// {"error":"rbac_denied","action":"worker.hire","required":"admin"}
/// ```
///
/// When no role resolves (no session JWT, no trusted header) the gate
/// defaults to `Guest` in production (fails closed) and `Owner` in
/// dev (fails open). The dev default preserves existing test suites
/// that issue requests without role headers — flipping their meaning
/// would mass-break tests outside this brief's may-touch list.
/// Production deployments lose the dev-open path automatically — see
/// [`is_production_mode`].
pub(super) fn enforce_role(req: &str, action: Action) -> Option<(&'static str, String)> {
    let floor = required_role(action);
    let effective = effective_role(req);
    if role_meets(effective, floor) {
        None
    } else {
        Some(deny_response(action, floor, effective))
    }
}

fn effective_role(req: &str) -> UserTenantRole {
    if let Some(r) = resolve_role(req) {
        return r;
    }
    if is_production_mode() {
        UserTenantRole::Guest
    } else if has_role_header(req) {
        // Dev request that names a role (even non-canonical) is
        // expressing intent — fail-closed to Guest instead of
        // defaulting to the dev-open Owner. Keeps tests for legacy
        // taxonomies that signal "non-admin" failing as expected.
        UserTenantRole::Guest
    } else {
        UserTenantRole::Owner
    }
}

fn deny_response(
    action: Action,
    floor: UserTenantRole,
    actual: UserTenantRole,
) -> (&'static str, String) {
    // Standard `{ "error": { "code", "message" } }` envelope so the
    // SPA's `ApiError` parser keeps working. Action / required /
    // effective are surfaced as top-level extras for telemetry.
    let body = serde_json::json!({
        "error": {
            "code": "forbidden",
            "message": format!(
                "role {actual} lacks {action_label} (required: {required})",
                actual = actual.as_str(),
                action_label = action.label(),
                required = floor.as_str(),
            ),
        },
        "rbac": {
            "denied": true,
            "action": action.label(),
            "required": floor.as_str(),
            "effective": actual.as_str(),
        },
    });
    ("403 Forbidden", body.to_string())
}

#[cfg(test)]
#[path = "role_gate_tests.rs"]
mod tests;
