//! Platform-staff authorization gate (DEC-057 Phase 1).
//!
//! Sibling to [`super::role_gate`], but for the *platform-operator* plane:
//! `role_gate` checks a caller's tenant role (`UserTenantRole`) on the session
//! JWT; this gate checks whether the caller is a Skycot **platform-staff**
//! operator and whether their [`PlatformRole`] is sufficient for a given
//! [`StaffAction`].
//!
//! Trust rules (non-negotiable, first-principles):
//! - **Default-deny.** The caller's `user_id` comes from the *verified*
//!   session JWT `sub` ([`super::session_jwt::extract_session_claims`]) — never
//!   a client header. No staff row, a disabled row, or no session → 403.
//! - The action→role floor lives in [`StaffAction::permitted`] so every
//!   authorization decision is made in one place.

use super::session_jwt::extract_session_claims;
use argo_identity::platform_staff::{PlatformStaff, StaffAction};
use argo_identity::platform_staff_store::get_staff;
use argo_identity::user::UserId;
use argo_store::SessionStore;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

/// Resolve + authorize the platform-staff caller of `req` for `need`.
/// Returns the [`PlatformStaff`] grant on success, or a ready-to-return
/// `(status, body)` deny tuple. The `user_id` is taken from the verified
/// session JWT; a forged or absent session can only ever fail closed.
pub(super) async fn enforce_staff(
    req: &str,
    store: &Arc<SessionStore>,
    need: StaffAction,
) -> Result<PlatformStaff, (&'static str, String)> {
    authorize_staff(store, staff_user_id(req), need).await
}

/// Testable core: given an already-resolved `user_id`, decide whether the
/// caller is active staff with sufficient authority for `need`. Split out so
/// the authorization matrix can be unit-tested without minting a JWT.
pub(super) async fn authorize_staff(
    store: &Arc<SessionStore>,
    user_id: Option<UserId>,
    need: StaffAction,
) -> Result<PlatformStaff, (&'static str, String)> {
    let Some(uid) = user_id else {
        return Err(staff_forbidden("not platform staff"));
    };
    let staff = match get_staff(store, uid).await {
        Ok(Some(s)) if s.is_active() => s,
        // Absent row OR disabled grant → default-deny. Identical opaque
        // message so a probe can't distinguish "no row" from "disabled".
        Ok(_) => return Err(staff_forbidden("not platform staff")),
        Err(e) => return Err(staff_server_error(&format!("staff lookup: {e}"))),
    };
    if !need.permitted(staff.role) {
        return Err(staff_forbidden("insufficient platform-staff role"));
    }
    Ok(staff)
}

/// Extract the verified caller's [`UserId`] from the session JWT `sub`.
/// `None` when there is no decodable session or the `sub` is not a UUID.
fn staff_user_id(req: &str) -> Option<UserId> {
    let claims = extract_session_claims(req)?;
    Uuid::parse_str(&claims.sub).ok().map(UserId)
}

/// 403 deny body, matching the gateway's `{ error: { code, message } }` shape.
pub(super) fn staff_forbidden(message: &str) -> (&'static str, String) {
    (
        "403 Forbidden",
        json!({ "error": { "code": "forbidden", "message": message } }).to_string(),
    )
}

fn staff_server_error(message: &str) -> (&'static str, String) {
    (
        "500 Internal Server Error",
        json!({ "error": { "code": "internal_error", "message": message } }).to_string(),
    )
}

#[cfg(test)]
#[path = "platform_staff_gate_tests.rs"]
mod tests;
