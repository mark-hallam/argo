//! Operator-side support inbox endpoints (Stream
//! `claude-support-inbox-spa-page`, Wave 4 P2 follow-up to #1762).
//!
//! Five HTTP routes backing the `/support/inbox` SPA page:
//!
//! - `POST   /api/v1/support/tickets` — internal create called by the
//!   Next.js `/api/support/ticket` proxy after the outbound webhook
//!   forward. Persists the ticket so the operator inbox surfaces it
//!   even when the webhook fails.
//! - `GET    /api/v1/support/tickets?status=open&limit=50` — paginated
//!   list, admin-only.
//! - `GET    /api/v1/support/tickets/<id>` — single ticket plus thread.
//! - `POST   /api/v1/support/tickets/<id>/reply` — operator reply.
//! - `PATCH  /api/v1/support/tickets/<id>` — status / priority /
//!   assignee update.
//!
//! Admin gating mirrors the existing `X-Argo-Role` shim (see
//! `routes_worker_detail::PrincipalRole`). v1 treats `Operator` as
//! the admin/owner role; native `role.owner`/`role.admin` lands when
//! `mission-enterprise-principal-roles` ships.
//!
//! The create endpoint (`POST .../tickets`) is intentionally NOT
//! admin-gated — it is the in-app side-channel for the unauthenticated
//! marketing-site contact form. Bearer auth still runs upstream.

use super::body_from_request;
use super::routes_auth_principal::principal_id_for_audit;
use argo_store::support_query::{
    append_reply, create_ticket, get_ticket, list_replies, list_tickets, update_ticket,
    NewSupportTicket, SupportReplyAuthorKind, SupportReplyRow, SupportTicketPriority,
    SupportTicketRow, SupportTicketStatus, SupportTicketUpdate,
};
use argo_store::SessionStore;
use argo_types::TenantId;
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

/// `POST /api/v1/support/tickets` — internal create (no admin gate;
/// Next.js proxy forwards the unauthenticated contact-form payload
/// here). Body: `{ user_email, subject, body, tenant_id?, priority? }`.
pub(super) async fn create_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let store = match store {
        Some(s) => s,
        None => return super::store_unavailable(),
    };
    let body = body_from_request(req);
    let parsed: Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return bad_request(&format!("parse body: {e}")),
    };
    let email = match parsed.get("user_email").and_then(Value::as_str) {
        Some(s) if !s.trim().is_empty() => s.trim().to_string(),
        _ => return bad_request("`user_email` (non-empty string) is required"),
    };
    let subject = match parsed.get("subject").and_then(Value::as_str) {
        Some(s) if !s.trim().is_empty() => s.trim().to_string(),
        _ => return bad_request("`subject` (non-empty string) is required"),
    };
    let body_text = match parsed.get("body").and_then(Value::as_str) {
        Some(s) if !s.trim().is_empty() => s.to_string(),
        _ => return bad_request("`body` (non-empty string) is required"),
    };
    let tenant_id = match parsed.get("tenant_id").and_then(Value::as_str) {
        Some(s) => match Uuid::parse_str(s) {
            Ok(u) => Some(TenantId(u)),
            Err(_) => return bad_request("`tenant_id` must be a valid uuid"),
        },
        None => None,
    };
    let priority = match parsed.get("priority").and_then(Value::as_str) {
        Some(s) => match SupportTicketPriority::parse(s) {
            Ok(p) => Some(p),
            Err(_) => return bad_request("`priority` must be low|normal|high|urgent"),
        },
        None => None,
    };
    let id = match create_ticket(
        store.pool(),
        NewSupportTicket {
            tenant_id,
            user_email: &email,
            subject: &subject,
            body: &body_text,
            priority,
        },
    )
    .await
    {
        Ok(id) => id,
        Err(e) => return server_error(&format!("create ticket: {e}")),
    };
    ("201 Created", json!({ "ticket_id": id.to_string() }).to_string())
}

/// `GET /api/v1/support/tickets?status=open&limit=50` — paginated
/// list, admin-only.
pub(super) async fn list_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    if let Some(resp) = require_admin(req) {
        return resp;
    }
    let store = match store {
        Some(s) => s,
        None => return super::store_unavailable(),
    };
    let status = match query_param(req, "status") {
        Some(raw) => match SupportTicketStatus::parse(&raw) {
            Ok(s) => Some(s),
            Err(_) => return bad_request("`status` must be open|pending|resolved|closed"),
        },
        None => None,
    };
    let limit = query_param(req, "limit")
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(50);
    let rows = match list_tickets(store.pool(), status, limit).await {
        Ok(v) => v,
        Err(e) => return server_error(&format!("list tickets: {e}")),
    };
    let items: Vec<Value> = rows.iter().map(ticket_to_json).collect();
    ("200 OK", json!({ "tickets": items }).to_string())
}

/// `GET /api/v1/support/tickets/<id>` — ticket + thread, admin-only.
pub(super) async fn detail_response(
    ticket_id: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    if let Some(resp) = require_admin(req) {
        return resp;
    }
    let store = match store {
        Some(s) => s,
        None => return super::store_unavailable(),
    };
    let id = match Uuid::parse_str(ticket_id) {
        Ok(u) => u,
        Err(_) => return bad_request("invalid ticket id"),
    };
    let ticket = match get_ticket(store.pool(), id).await {
        Ok(Some(t)) => t,
        Ok(None) => return not_found(ticket_id),
        Err(e) => return server_error(&format!("get ticket: {e}")),
    };
    let thread = match list_replies(store.pool(), id).await {
        Ok(v) => v,
        Err(e) => return server_error(&format!("list replies: {e}")),
    };
    let replies: Vec<Value> = thread.iter().map(reply_to_json).collect();
    (
        "200 OK",
        json!({
            "ticket": ticket_to_json(&ticket),
            "replies": replies,
        })
        .to_string(),
    )
}

/// `POST /api/v1/support/tickets/<id>/reply` — operator reply, admin-only.
/// Body: `{ body }`.
pub(super) async fn reply_response(
    ticket_id: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    if let Some(resp) = require_admin(req) {
        return resp;
    }
    let store = match store {
        Some(s) => s,
        None => return super::store_unavailable(),
    };
    let id = match Uuid::parse_str(ticket_id) {
        Ok(u) => u,
        Err(_) => return bad_request("invalid ticket id"),
    };
    let parsed: Value = match serde_json::from_str(body_from_request(req)) {
        Ok(v) => v,
        Err(e) => return bad_request(&format!("parse body: {e}")),
    };
    let body_text = match parsed.get("body").and_then(Value::as_str) {
        Some(s) if !s.trim().is_empty() => s.to_string(),
        _ => return bad_request("`body` (non-empty string) is required"),
    };
    match get_ticket(store.pool(), id).await {
        Ok(Some(_)) => {}
        Ok(None) => return not_found(ticket_id),
        Err(e) => return server_error(&format!("get ticket: {e}")),
    };
    let author_id = principal_id_for_audit(req).and_then(|p| Uuid::parse_str(&p).ok());
    let reply_id = match append_reply(
        store.pool(),
        id,
        SupportReplyAuthorKind::Operator,
        author_id,
        &body_text,
    )
    .await
    {
        Ok(rid) => rid,
        Err(e) => return server_error(&format!("append reply: {e}")),
    };
    (
        "201 Created",
        json!({ "reply_id": reply_id.to_string() }).to_string(),
    )
}

/// `PATCH /api/v1/support/tickets/<id>` — status / priority / assignee
/// update, admin-only. Body: any subset of
/// `{ status, priority, assigned_to }`.
pub(super) async fn update_response(
    ticket_id: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    if let Some(resp) = require_admin(req) {
        return resp;
    }
    let store = match store {
        Some(s) => s,
        None => return super::store_unavailable(),
    };
    let id = match Uuid::parse_str(ticket_id) {
        Ok(u) => u,
        Err(_) => return bad_request("invalid ticket id"),
    };
    let parsed: Value = match serde_json::from_str(body_from_request(req)) {
        Ok(v) => v,
        Err(e) => return bad_request(&format!("parse body: {e}")),
    };
    let status = match parsed.get("status").and_then(Value::as_str) {
        Some(s) => match SupportTicketStatus::parse(s) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("`status` must be open|pending|resolved|closed"),
        },
        None => None,
    };
    let priority = match parsed.get("priority").and_then(Value::as_str) {
        Some(s) => match SupportTicketPriority::parse(s) {
            Ok(v) => Some(v),
            Err(_) => return bad_request("`priority` must be low|normal|high|urgent"),
        },
        None => None,
    };
    // `assigned_to` is tri-state: absent leaves untouched, null clears,
    // a string sets a new uuid.
    let assigned_to = match parsed.get("assigned_to") {
        Some(Value::Null) => Some(None),
        Some(Value::String(s)) => match Uuid::parse_str(s) {
            Ok(u) => Some(Some(u)),
            Err(_) => return bad_request("`assigned_to` must be a valid uuid"),
        },
        Some(_) => return bad_request("`assigned_to` must be a uuid string or null"),
        None => None,
    };
    let changed = match update_ticket(
        store.pool(),
        id,
        SupportTicketUpdate {
            status,
            priority,
            assigned_to,
        },
    )
    .await
    {
        Ok(v) => v,
        Err(e) => return server_error(&format!("update ticket: {e}")),
    };
    if !changed {
        return not_found(ticket_id);
    }
    let row = match get_ticket(store.pool(), id).await {
        Ok(Some(r)) => r,
        Ok(None) => return not_found(ticket_id),
        Err(e) => return server_error(&format!("get ticket: {e}")),
    };
    ("200 OK", json!({ "ticket": ticket_to_json(&row) }).to_string())
}

fn ticket_to_json(row: &SupportTicketRow) -> Value {
    json!({
        "id": row.id.to_string(),
        "tenant_id": row.tenant_id.map(|t| t.0.to_string()),
        "user_email": row.user_email,
        "subject": row.subject,
        "body": row.body,
        "status": row.status.as_str(),
        "priority": row.priority.as_str(),
        "created_at": row.created_at,
        "updated_at": row.updated_at,
        "assigned_to": row.assigned_to.map(|u| u.to_string()),
    })
}

fn reply_to_json(row: &SupportReplyRow) -> Value {
    json!({
        "id": row.id.to_string(),
        "ticket_id": row.ticket_id.to_string(),
        "author_kind": row.author_kind.as_str(),
        "author_id": row.author_id.map(|u| u.to_string()),
        "body": row.body,
        "created_at": row.created_at,
    })
}

/// Return `Some(resp)` to short-circuit dispatch with 403 when the
/// caller's verified session-JWT role lacks the support-tickets
/// capability. Falls back to the legacy `X-Argo-Role` header trust
/// path only in dev mode (see [`super::role_gate`] for the trust
/// policy). `None` ⇒ caller is authorized. Failure mode is fail-closed:
/// missing JWT → effective role guest → 403.
fn require_admin(req: &str) -> Option<(&'static str, String)> {
    super::role_gate::enforce_role(req, super::role_gate::Action::SupportTickets)
}

fn query_param(req: &str, key: &str) -> Option<String> {
    let line = req.lines().next()?;
    let path = line.split_whitespace().nth(1)?;
    let qs = path.split_once('?').map(|(_, q)| q)?;
    qs.split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        (k == key).then(|| v.to_string())
    })
}

fn bad_request(message: &str) -> (&'static str, String) {
    error_envelope("400 Bad Request", "bad_request", message)
}

fn not_found(id: &str) -> (&'static str, String) {
    error_envelope(
        "404 Not Found",
        "ticket_not_found",
        &format!("no ticket with id '{id}'"),
    )
}

fn server_error(message: &str) -> (&'static str, String) {
    error_envelope("500 Internal Server Error", "internal_error", message)
}

fn error_envelope(status: &'static str, code: &str, message: &str) -> (&'static str, String) {
    (
        status,
        json!({
            "error": {
                "code": code,
                "message": message,
            }
        })
        .to_string(),
    )
}

#[cfg(test)]
#[path = "routes_support_tickets_tests.rs"]
mod tests;
