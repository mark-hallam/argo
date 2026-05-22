//! `GET /api/v1/workspace/members` — list current members and pending
//! invitations for the caller's workspace. Stream
//! `web-settings-members-invite-real-wire`.
//!
//! Wraps the substrate `user_tenant` rows (the canonical "who is a
//! member?" table) with the in-flight `workspace_member_invitation`
//! rows so the `/settings/members` page can render both lists from a
//! single fetch.
//!
//! The companion POST endpoints live in sibling modules:
//! - `routes_workspace_members_invite::invite_response` — issue invite
//! - `routes_workspace_members_accept::serve_accept_stream` — accept token
//!
//! This file holds the GET handler plus a small dispatch helper used
//! by `routes_dispatch_get_api`. PATCH (role change) and DELETE
//! (remove member) are intentionally deferred to a sibling brief —
//! the demo-blocker for `web-settings-members-invite-real-wire` is
//! the invite + accept flow, not the existing-member edit surface.

use super::routes_onboarding_helpers::{
    map_workspace_error, principal_email, resolve_workspace, server_error, unauthorized,
};
use super::store_unavailable;
use argo_identity::user_store::get_user;
use argo_identity::user_tenant_store::list_users_in_tenant;
use argo_store::workspace_member_invitation::list_pending_for_tenant;
use argo_store::SessionStore;
use serde_json::{json, Value};
use std::sync::Arc;

/// `GET /api/v1/workspace/members` handler.
pub(super) async fn list_response(
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
    let (tenant, _ws) = match resolve_workspace(store, &caller_email).await {
        Ok(pair) => pair,
        Err(e) => return map_workspace_error(e),
    };

    let links = match list_users_in_tenant(store, tenant).await {
        Ok(rows) => rows,
        Err(e) => return server_error(&format!("list members: {e}")),
    };
    let mut members: Vec<Value> = Vec::with_capacity(links.len());
    for link in links {
        let (email, display_name, last_login_at) = match get_user(store, link.user_id).await {
            Ok(Some(u)) => (u.email, u.display_name, u.last_login_at),
            Ok(None) => (String::new(), None, None),
            Err(e) => return server_error(&format!("read user row: {e}")),
        };
        members.push(json!({
            "user_id": link.user_id.to_string(),
            "email": email,
            "display_name": display_name,
            "role": link.role.as_str(),
            "joined_at": link.created_at,
            "last_login_at": last_login_at,
        }));
    }

    let pending = match list_pending_for_tenant(store, tenant).await {
        Ok(rows) => rows,
        Err(e) => return server_error(&format!("list pending invitations: {e}")),
    };
    let pending_json: Vec<Value> = pending
        .into_iter()
        .map(|inv| {
            json!({
                "invitation_id": inv.id,
                "email": inv.email,
                "role": inv.role,
                "cohort_scope": inv.cohort_scope,
                "invited_by": inv.invited_by,
                "invited_at": inv.invited_at,
                "expires_at": inv.expires_at,
            })
        })
        .collect();

    (
        "200 OK",
        json!({
            "tenant_id": tenant.to_string(),
            "members": members,
            "pending_invitations": pending_json,
        })
        .to_string(),
    )
}

/// Recognise the GET route prefix. Bound from
/// [`super::routes_dispatch_get_api::dispatch_api_get`].
pub(super) fn is_list_request(req: &str) -> bool {
    req.starts_with("GET /api/v1/workspace/members ")
        || req.starts_with("GET /api/v1/workspace/members?")
}

/// Recognise the POST invite route. Bound from
/// [`super::routes_dispatch_post::api_v1_dispatch`].
pub(super) fn is_invite_request(req: &str) -> bool {
    req.starts_with("POST /api/v1/workspace/members/invite ")
        || req.starts_with("POST /api/v1/workspace/members/invite?")
}

#[cfg(test)]
#[path = "routes_workspace_members_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "routes_workspace_members_cross_tenant_tests.rs"]
mod cross_tenant_tests;
