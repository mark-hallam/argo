//! `POST /api/v1/workspace/members/accept` — accept a workspace-member
//! invitation. Stream `web-settings-members-invite-real-wire`.
//!
//! The handler:
//!
//! 1. Reads the raw token from the JSON body, hashes it, and looks up
//!    the matching invitation row.
//! 2. Atomically marks the invitation accepted (matching the
//!    single-use guarantee that the login `magic_link_tokens` table
//!    provides). The store helper's `UPDATE … WHERE accepted_at IS
//!    NULL` guards the concurrent-accept race.
//! 3. Upserts the `user_account` row for the invitee's email
//!    (find-or-create — invitations may go to non-customers).
//! 4. Links the user to the tenant via
//!    [`argo_identity::user_tenant_store::link_user_to_tenant`].
//!    Existing-link collisions are tolerated (an already-member who
//!    accepts a duplicate invite is idempotent rather than fatal).
//! 5. Emits `AuditKind::WorkspaceMemberJoined`.
//!
//! Sessions are NOT minted here — the existing `__Host-session` cookie
//! discipline requires server-side `Set-Cookie`, which the
//! `(status, body)` dispatch tuple cannot carry. The response carries
//! `next_step: "/signin"` instead so the SPA hands the invitee off to
//! the magic-link login flow; the `user_account` + `user_tenant` rows
//! are already in place, so that login lands them in the right
//! workspace seamlessly. Sibling brief (TODO) can promote the accept
//! route to a stream-owner alongside `auth_magic_link_verify`.

use super::auth_magic_link::sha256_hex;
use super::body_from_request;
use super::store_unavailable;
use argo_audit::{AuditKind, AuditSink};
use argo_identity::user::UserAccount;
use argo_identity::user_store::{get_user_by_email, insert_user};
use argo_identity::user_tenant::UserTenantRole;
use argo_identity::user_tenant_store::link_user_to_tenant;
use argo_store::workspace_member_invitation::{accept_invitation, AcceptOutcome, Invitation};
use argo_store::SessionStore;
use argo_types::ArgoResult;
use chrono::Utc;
use serde::Deserialize;
use serde_json::json;
use std::str::FromStr;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
struct AcceptBody {
    token: String,
}

/// `POST /api/v1/workspace/members/accept` — handler entry point.
pub(super) async fn accept_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let store = match store {
        Some(s) => s,
        None => return store_unavailable(),
    };
    let parsed: AcceptBody = match serde_json::from_str(body_from_request(req)) {
        Ok(v) => v,
        Err(e) => return error_envelope("400 Bad Request", &format!("parse body: {e}")),
    };
    let token = parsed.token.trim();
    if token.is_empty() {
        return error_envelope("400 Bad Request", "`token` is required");
    }
    let token_hash = sha256_hex(token);
    let now = Utc::now().timestamp();

    // The store helper does its own atomic UPDATE WHERE accepted_at IS
    // NULL; the user row + link rows happen after a successful claim so
    // a token race cannot create orphaned memberships.
    let placeholder_user_id = uuid::Uuid::nil();
    let outcome = match accept_invitation(store, &token_hash, placeholder_user_id, now).await {
        Ok(o) => o,
        Err(e) => return error_envelope("500 Internal Server Error", &format!("accept: {e}")),
    };
    let invitation = match outcome {
        AcceptOutcome::Ok(invitation) => *invitation,
        AcceptOutcome::NotFound => {
            return error_envelope("404 Not Found", "Invitation not found.");
        }
        AcceptOutcome::Expired => {
            return error_envelope("410 Gone", "This invitation has expired.");
        }
        AcceptOutcome::AlreadyAccepted => {
            return error_envelope("409 Conflict", "This invitation has already been accepted.");
        }
    };

    // Resolve / mint the user, then write the real user_id back to the
    // invitation row and link the user to the tenant.
    let user = match upsert_user(store, &invitation.email).await {
        Ok(u) => u,
        Err(e) => return error_envelope("500 Internal Server Error", &format!("user: {e}")),
    };
    if let Err(e) = write_accepted_by(store, &invitation.id, user.id.0).await {
        tracing::warn!(
            target: "gateway::workspace_members_accept",
            error = %e,
            invitation_id = %invitation.id,
            "failed to backfill accepted_by_user_id; continuing",
        );
    }
    let role = UserTenantRole::from_str(&invitation.role).unwrap_or(UserTenantRole::Guest);
    if let Err(e) = link_user_to_tenant(store, user.id, invitation.tenant_id, role).await {
        tracing::warn!(
            target: "gateway::workspace_members_accept",
            error = %e,
            user_id = %user.id,
            tenant_id = %invitation.tenant_id,
            "link_user_to_tenant failed (already member?); continuing",
        );
    }

    emit_joined_audit(store, &invitation, &user).await;

    (
        "200 OK",
        json!({
            "user_id": user.id.to_string(),
            "tenant_id": invitation.tenant_id.to_string(),
            "role": invitation.role,
            "email": invitation.email,
            "next_step": "/signin",
        })
        .to_string(),
    )
}

pub(super) fn is_accept_request(req: &str) -> bool {
    req.starts_with("POST /api/v1/workspace/members/accept ")
        || req.starts_with("POST /api/v1/workspace/members/accept?")
}

async fn upsert_user(store: &SessionStore, email: &str) -> ArgoResult<UserAccount> {
    if let Some(existing) = get_user_by_email(store, email).await? {
        return Ok(existing);
    }
    let user = UserAccount::new(email.to_string());
    insert_user(store, &user).await?;
    Ok(user)
}

async fn write_accepted_by(
    store: &SessionStore,
    invitation_id: &str,
    user_id: uuid::Uuid,
) -> ArgoResult<()> {
    sqlx::query("UPDATE workspace_member_invitation SET accepted_by_user_id = ? WHERE id = ?")
        .bind(user_id.as_bytes().as_slice())
        .bind(invitation_id)
        .execute(store.pool())
        .await
        .map_err(|e| argo_types::ArgoError::Provider(format!("backfill accepted_by: {e}")))?;
    Ok(())
}

async fn emit_joined_audit(
    store: &Arc<SessionStore>,
    invitation: &Invitation,
    user: &UserAccount,
) {
    let payload = json!({
        "tenant_id": invitation.tenant_id.to_string(),
        "user_id": user.id.to_string(),
        "role": invitation.role,
        "invited_by": invitation.invited_by,
        "invitation_id": invitation.id,
    });
    let sink = AuditSink::new_with_tenant(Arc::clone(store), invitation.tenant_id.to_string());
    let actor = format!("human:{}", user.email);
    if let Err(e) = sink
        .write(None, AuditKind::WorkspaceMemberJoined, payload, Some(actor))
        .await
    {
        tracing::warn!(
            target: "gateway::workspace_members_accept",
            error = %e,
            tenant_id = %invitation.tenant_id,
            invitation_id = %invitation.id,
            "workspace_member_joined audit write failed",
        );
    }
}

fn error_envelope(status: &'static str, message: &str) -> (&'static str, String) {
    (
        status,
        json!({
            "error": {
                "code": "accept_invitation_failed",
                "message": message,
            }
        })
        .to_string(),
    )
}
