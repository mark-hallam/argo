//! Shared demo-mode permission override for the worker **detail** and
//! **overview** routes.
//!
//! `GET /api/v1/workers/<id>` (detail) and `GET /api/v1/workers/<id>/overview`
//! (overview) must resolve the SAME permission block for the same principal —
//! the SPA's `useWorker` hook reads perms from the detail route, so a mismatch
//! left the Schedules tab stuck read-only. #2393
//! (`fix-gateway-demo-mode-worker-permissions-grant-all`) added the demo-mode
//! owner-permission override to the overview route only; this module is the
//! extracted single home for that logic so both routes call it (no
//! duplicated/parallel surface). When demo mode is enabled AND the request
//! carries a matching demo principal, the caller is granted full owner
//! permissions; otherwise the normal `resolve_permissions(role, is_hire_author)`
//! result stands.
//!
//! Nested under `routes_worker_detail` via `#[path]`, so within this file
//! `super` is the detail module (home of `Permissions` / `PrincipalRole` /
//! `resolve_permissions`) and `super::super` is the gateway parent (home of
//! the sibling `session_jwt` / `auth_magic_link` / `demo_mode` modules). This
//! keeps the change inside the brief's may-touch list without editing
//! `gateway/mod.rs`.

use super::{resolve_permissions, Permissions, PrincipalRole};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::Deserialize;

/// A demo-session principal resolved from a `demo:true` magic-link JWT.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::handlers::gateway) struct DemoPrincipal {
    pub principal_id: String,
}

#[derive(Debug, Deserialize)]
struct DemoSessionClaims {
    #[serde(default)]
    email: String,
    #[serde(default)]
    demo: bool,
    #[serde(rename = "iat")]
    _iat: i64,
    #[serde(rename = "exp")]
    _exp: i64,
}

/// Resolve permissions with the demo override applied. Returns the permission
/// set plus whether the override was granted (so the caller can emit the
/// `PermissionDemoOverrideGranted` audit row). When the override is not
/// allowed this is exactly `resolve_permissions(role, is_hire_author)`, so the
/// non-demo path is unchanged.
pub(in crate::handlers::gateway) fn compute_permissions_demo(
    role: PrincipalRole,
    is_hire_author: bool,
    demo_override_allowed: bool,
) -> (Permissions, bool) {
    if demo_override_allowed {
        return (
            Permissions {
                can_edit_charter: true,
                can_edit_duties: true,
                can_pause_resume: true,
                can_archive: true,
                can_delete: true,
                can_view_audit: true,
                can_view_settings: true,
            },
            true,
        );
    }
    (resolve_permissions(role, is_hire_author), false)
}

/// The override is allowed only in demo mode, only when the request carries a
/// valid demo principal, and only when the audit-header actor (if any) matches
/// that demo principal — a non-matching header actor is treated as a different
/// principal and denied.
pub(in crate::handlers::gateway) fn demo_permission_override_allowed(
    demo_mode_enabled: bool,
    header_principal: Option<&str>,
    demo_principal: Option<&DemoPrincipal>,
) -> bool {
    if !demo_mode_enabled {
        return false;
    }
    let Some(demo_principal) = demo_principal else {
        return false;
    };
    header_principal
        .map(|p| p == demo_principal.principal_id)
        .unwrap_or(true)
}

/// Extract a demo principal from the request's bearer token, falling back to
/// the session cookie.
pub(in crate::handlers::gateway) fn demo_principal_for_request(req: &str) -> Option<DemoPrincipal> {
    if let Some(token) = super::super::session_jwt::bearer_from_request(req) {
        if let Some(principal) = demo_principal_from_token(&token) {
            return Some(principal);
        }
    }
    super::super::session_jwt::session_cookie_from_request(req)
        .and_then(|token| demo_principal_from_token(&token))
}

fn demo_principal_from_token(token: &str) -> Option<DemoPrincipal> {
    if token.split('.').count() != 3 {
        return None;
    }
    let key = DecodingKey::from_secret(&super::super::auth_magic_link::jwt_secret());
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;
    validation.required_spec_claims.clear();
    validation.required_spec_claims.insert("exp".into());
    validation.required_spec_claims.insert("iat".into());
    validation.leeway = 60;
    let claims = decode::<DemoSessionClaims>(token, &key, &validation)
        .ok()?
        .claims;
    if !claims.demo {
        return None;
    }
    let email = claims.email.trim().to_ascii_lowercase();
    if !super::super::auth_magic_link::looks_like_email(&email) {
        return None;
    }
    Some(DemoPrincipal {
        principal_id: format!("human:{email}"),
    })
}

/// Whether demo-mode permission grants are enabled. Tests drive it through the
/// `DEMO_MODE_ENV` env var; production reads the demo-mode gate.
pub(in crate::handlers::gateway) fn demo_mode_enabled_for_permissions() -> bool {
    #[cfg(test)]
    {
        std::env::var(super::super::demo_mode::DEMO_MODE_ENV)
            .map(|v| v == "1")
            .unwrap_or(false)
    }
    #[cfg(not(test))]
    {
        super::super::demo_mode::is_enabled()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn demo(email: &str) -> DemoPrincipal {
        DemoPrincipal {
            principal_id: format!("human:{email}"),
        }
    }

    #[test]
    fn override_denied_when_demo_mode_off() {
        let p = demo("ceo@acme.test");
        assert!(!demo_permission_override_allowed(false, None, Some(&p)));
    }

    #[test]
    fn override_denied_without_demo_principal() {
        assert!(!demo_permission_override_allowed(true, None, None));
    }

    #[test]
    fn override_allowed_for_matching_or_absent_header_actor() {
        let p = demo("ceo@acme.test");
        // No audit-header actor → the demo principal is the only identity.
        assert!(demo_permission_override_allowed(true, None, Some(&p)));
        // Header actor matches the demo principal.
        assert!(demo_permission_override_allowed(
            true,
            Some("human:ceo@acme.test"),
            Some(&p)
        ));
    }

    #[test]
    fn override_denied_for_mismatched_header_actor() {
        let p = demo("ceo@acme.test");
        assert!(!demo_permission_override_allowed(
            true,
            Some("human:intruder@acme.test"),
            Some(&p)
        ));
    }

    #[test]
    fn compute_grants_all_when_allowed() {
        let (perms, granted) =
            compute_permissions_demo(PrincipalRole::Delegator, false, true);
        assert!(granted);
        assert!(perms.can_edit_charter);
        assert!(perms.can_edit_duties);
        assert!(perms.can_pause_resume);
        assert!(perms.can_archive);
        assert!(perms.can_delete);
        assert!(perms.can_view_audit);
        assert!(perms.can_view_settings);
    }

    #[test]
    fn compute_falls_back_to_resolve_permissions_when_not_allowed() {
        // Non-author delegator with no override gets the plain resolver
        // result (all-false) — the non-demo path is unchanged.
        let (perms, granted) =
            compute_permissions_demo(PrincipalRole::Delegator, false, false);
        assert!(!granted);
        assert_eq!(perms, resolve_permissions(PrincipalRole::Delegator, false));
        assert!(!perms.can_edit_duties);

        // And an operator still gets the operator set, not the demo set.
        let (op, op_granted) =
            compute_permissions_demo(PrincipalRole::Operator, false, false);
        assert!(!op_granted);
        assert_eq!(op, resolve_permissions(PrincipalRole::Operator, false));
    }

    #[test]
    fn detail_matches_overview_under_demo_override() {
        // Both routes call compute_permissions_demo with the same inputs, so
        // an allowed override yields identical owner permissions regardless of
        // role / authorship — this is the consistency the brief pins.
        let (a, _) = compute_permissions_demo(PrincipalRole::Delegator, false, true);
        let (b, _) = compute_permissions_demo(PrincipalRole::Auditor, true, true);
        assert_eq!(a, b);
    }
}
