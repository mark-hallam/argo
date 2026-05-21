//! `POST /api/v1/workspace/members/invite` — issue a workspace-member
//! invitation. Stream `web-settings-members-invite-real-wire`.
//!
//! The handler:
//!
//! 1. Pulls the `human:<email>` principal off the request and resolves
//!    the caller's `(tenant_id, workspace_id)` via the same idempotent
//!    re-call path the onboarding handlers use.
//! 2. RBAC-gates the call to owner or admin via [`super::role_gate::resolve_role`].
//!    The wave-3 RBAC `Action::WorkerHire` floor is admin+, and member
//!    invitations carry comparable blast radius, so the same floor
//!    applies here.
//! 3. Validates the request body (email shape + role in
//!    `{owner, admin, user, guest}`) and the absence of a duplicate
//!    pending invitation for the same email.
//! 4. Mints a fresh 32-byte token, SHA-256-hashes it, inserts the
//!    `workspace_member_invitation` row, and dispatches the
//!    `WorkspaceInviteEmail` template through the existing outbound
//!    pipeline (the dev `Noop` provider logs the link to stdout — see
//!    `auth_magic_link_delivery::deliver_magic_link` for the same
//!    pattern).
//! 5. Emits `AuditKind::WorkspaceMemberInvited` with the invitation
//!    metadata.

use super::auth_magic_link::{looks_like_email, mint_token, sha256_hex};
use super::body_from_request;
use super::role_gate::resolve_role;
use super::routes_onboarding_helpers::{
    bad_request, map_workspace_error, principal_email, resolve_workspace, server_error,
    unauthorized,
};
use super::store_unavailable;
use argo_audit::{AuditKind, AuditSink};
use argo_config::EmailOutboundConfig;
use argo_email_templates::WorkspaceInviteEmail;
use argo_identity::user_tenant::UserTenantRole;
use argo_store::workspace_member_invitation::{
    create_invitation, lookup_pending_by_email, INVITATION_TTL_SECS,
};
use argo_store::SessionStore;
use argo_types::TenantId;
use chrono::Utc;
use serde::Deserialize;
use serde_json::json;
use std::sync::{Arc, OnceLock};

#[derive(Debug, Deserialize)]
struct InviteBody {
    email: String,
    role: String,
    #[serde(default)]
    cohort_scope: Vec<String>,
}

const ALLOWED_ROLES: &[&str] = &["owner", "admin", "user", "guest"];

/// Handler entry — wired from [`super::routes_workspace_members::dispatch_post`].
pub(super) async fn invite_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let store = match store {
        Some(s) => s,
        None => return store_unavailable(),
    };
    let caller_email = match principal_email(req) {
        Some(e) => e,
        None => return unauthorized(),
    };
    if !caller_role_can_invite(req) {
        return forbidden();
    }
    let body = body_from_request(req);
    let parsed: InviteBody = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return bad_request(&format!("parse body: {e}")),
    };
    let invitee_email = parsed.email.trim().to_ascii_lowercase();
    if !looks_like_email(&invitee_email) {
        return bad_request("`email` must be a valid email address");
    }
    let role = parsed.role.trim().to_ascii_lowercase();
    if !ALLOWED_ROLES.contains(&role.as_str()) {
        return bad_request("`role` must be one of: owner, admin, user, guest");
    }
    let cohort_scope: Vec<String> = parsed
        .cohort_scope
        .into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let (tenant, _ws) = match resolve_workspace(store, &caller_email).await {
        Ok(pair) => pair,
        Err(e) => return map_workspace_error(e),
    };

    match lookup_pending_by_email(store, tenant, &invitee_email).await {
        Ok(Some(_)) => return conflict_duplicate_pending(),
        Ok(None) => {}
        Err(e) => return server_error(&format!("lookup pending invitation: {e}")),
    }

    let raw_token = mint_token();
    let token_hash = sha256_hex(&raw_token);
    let now = Utc::now().timestamp();
    let expires_at = now + INVITATION_TTL_SECS;
    let invited_by = format!("human:{caller_email}");

    let invitation = match create_invitation(
        store,
        tenant,
        &invitee_email,
        &role,
        &cohort_scope,
        &token_hash,
        &invited_by,
        now,
        expires_at,
    )
    .await
    {
        Ok(i) => i,
        Err(e) => return server_error(&format!("create invitation: {e}")),
    };

    let workspace_label = workspace_label_for(&caller_email);
    deliver_invite_email(
        &caller_email,
        &invitee_email,
        &role,
        &workspace_label,
        &raw_token,
    )
    .await;
    emit_invite_audit(store, tenant, &invited_by, &invitation, &cohort_scope).await;

    (
        "200 OK",
        json!({
            "invitation_id": invitation.id,
            "expires_at": expires_at,
            "email_sent_to": invitee_email,
        })
        .to_string(),
    )
}

fn caller_role_can_invite(req: &str) -> bool {
    match resolve_role(req).unwrap_or(UserTenantRole::Guest) {
        UserTenantRole::Owner | UserTenantRole::Admin => true,
        UserTenantRole::User | UserTenantRole::Guest => false,
    }
}

fn forbidden() -> (&'static str, String) {
    (
        "403 Forbidden",
        json!({
            "error": {
                "code": "forbidden",
                "message": "Only owners and admins can invite new members.",
            }
        })
        .to_string(),
    )
}

fn conflict_duplicate_pending() -> (&'static str, String) {
    (
        "409 Conflict",
        json!({
            "error": {
                "code": "duplicate_pending",
                "message": "An invitation for this email is already pending.",
            }
        })
        .to_string(),
    )
}

fn workspace_label_for(email: &str) -> String {
    email
        .split('@')
        .next()
        .filter(|s| !s.is_empty())
        .map_or_else(|| "your workspace".to_string(), str::to_string)
}

async fn deliver_invite_email(
    inviter_email: &str,
    invitee_email: &str,
    role: &str,
    workspace_label: &str,
    raw_token: &str,
) {
    let base = std::env::var("SKYCOT_PUBLIC_BASE_URL")
        .unwrap_or_else(|_| "https://app.skycot.com".to_string());
    let accept_url = format!("{base}/accept-invite?token={raw_token}");
    let rendered = WorkspaceInviteEmail {
        inviter: inviter_email.to_string(),
        workspace_name: workspace_label.to_string(),
        role: role.to_string(),
        email: invitee_email.to_string(),
        accept_url: accept_url.clone(),
        expires_in: "7 days".to_string(),
    }
    .render();
    tracing::warn!(
        target: "gateway::workspace_members_invite",
        recipient = %invitee_email,
        inviter = %inviter_email,
        role = %role,
        link = %accept_url,
        "workspace invitation issued",
    );
    let cfg = outbound_config();
    let from = if cfg.from_address.is_empty() {
        "noreply@skycot.com"
    } else {
        cfg.from_address.as_str()
    };
    let msg = super::email_outbound::OutboundMessage {
        to: invitee_email,
        from,
        rendered: &rendered,
    };
    if let Err(reason) = super::email_outbound::send(cfg, msg).await {
        tracing::warn!(
            target: "gateway::workspace_members_invite",
            recipient = %invitee_email,
            provider = cfg.provider.as_str(),
            reason = %reason,
            "outbound email failed; invite link still in trace log for operator copy",
        );
    }
}

fn outbound_config() -> &'static EmailOutboundConfig {
    static CFG: OnceLock<EmailOutboundConfig> = OnceLock::new();
    CFG.get_or_init(|| argo_config::load().unwrap_or_default().email.outbound)
}

async fn emit_invite_audit(
    store: &Arc<SessionStore>,
    tenant: TenantId,
    actor_principal: &str,
    invitation: &argo_store::workspace_member_invitation::Invitation,
    cohort_scope: &[String],
) {
    let payload = json!({
        "tenant_id": tenant.to_string(),
        "invitation_id": invitation.id,
        "invitee_email": invitation.email,
        "role": invitation.role,
        "cohort_scope": cohort_scope,
        "expires_at": invitation.expires_at,
    });
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant.to_string());
    if let Err(e) = sink
        .write(
            None,
            AuditKind::WorkspaceMemberInvited,
            payload,
            Some(actor_principal.to_string()),
        )
        .await
    {
        tracing::warn!(
            target: "gateway::workspace_members_invite",
            error = %e,
            tenant = %tenant,
            invitation_id = %invitation.id,
            "workspace_member_invited audit write failed",
        );
    }
}
