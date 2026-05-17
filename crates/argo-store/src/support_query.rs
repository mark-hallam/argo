//! Inbound support-ticket storage backing the operator-triage inbox
//! (Stream `claude-support-inbox-spa-page`).
//!
//! Sister to the Next.js `POST /api/support/ticket` route landed in
//! #1762: that route forwards the ticket to the outbound webhook
//! (Plain / Linear); this module persists the same payload so the
//! in-app inbox at `/support/inbox` can surface and reply to tickets
//! independently of webhook delivery. If the webhook fails the
//! ticket still lands here — graceful degradation, per the brief.
//!
//! `tenant_id` is nullable: marketing-site contact-form submissions
//! arrive unauthenticated and only get a tenant once an operator
//! triages them. `assigned_to` exists for the future assignment
//! workflow brief; the v1 UI doesn't surface it.

use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Row, Sqlite};
use uuid::Uuid;

/// Stable status discriminator stored in `support_ticket.status`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SupportTicketStatus {
    /// New ticket awaiting triage.
    Open,
    /// Awaiting customer reply.
    Pending,
    /// Operator resolved; archived from the active board.
    Resolved,
    /// Closed after a resolution window with no further activity.
    Closed,
}

impl SupportTicketStatus {
    /// SQLite label.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Pending => "pending",
            Self::Resolved => "resolved",
            Self::Closed => "closed",
        }
    }

    /// Parse a SQLite label.
    pub fn parse(raw: &str) -> ArgoResult<Self> {
        match raw {
            "open" => Ok(Self::Open),
            "pending" => Ok(Self::Pending),
            "resolved" => Ok(Self::Resolved),
            "closed" => Ok(Self::Closed),
            other => Err(ArgoError::Schema(format!(
                "unknown support ticket status '{other}'"
            ))),
        }
    }
}

/// Stable priority discriminator stored in `support_ticket.priority`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SupportTicketPriority {
    /// Low priority.
    Low,
    /// Default normal priority for new tickets.
    Normal,
    /// High priority.
    High,
    /// Urgent — surfaced first in the operator inbox.
    Urgent,
}

impl SupportTicketPriority {
    /// SQLite label.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Normal => "normal",
            Self::High => "high",
            Self::Urgent => "urgent",
        }
    }

    /// Parse a SQLite label.
    pub fn parse(raw: &str) -> ArgoResult<Self> {
        match raw {
            "low" => Ok(Self::Low),
            "normal" => Ok(Self::Normal),
            "high" => Ok(Self::High),
            "urgent" => Ok(Self::Urgent),
            other => Err(ArgoError::Schema(format!(
                "unknown support ticket priority '{other}'"
            ))),
        }
    }
}

/// Stable author-kind discriminator stored in `support_ticket_reply.author_kind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SupportReplyAuthorKind {
    /// External user / customer.
    User,
    /// Internal operator answering the ticket.
    Operator,
    /// System-generated note (status change, webhook outcome, etc).
    System,
}

impl SupportReplyAuthorKind {
    /// SQLite label.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Operator => "operator",
            Self::System => "system",
        }
    }

    /// Parse a SQLite label.
    pub fn parse(raw: &str) -> ArgoResult<Self> {
        match raw {
            "user" => Ok(Self::User),
            "operator" => Ok(Self::Operator),
            "system" => Ok(Self::System),
            other => Err(ArgoError::Schema(format!(
                "unknown support reply author kind '{other}'"
            ))),
        }
    }
}

/// Insert payload for [`create_ticket`].
#[derive(Debug, Clone)]
pub struct NewSupportTicket<'a> {
    /// Optional tenant id; `None` for unauthenticated contact-form submissions.
    pub tenant_id: Option<TenantId>,
    /// Submitter email (validated by the calling Next.js route).
    pub user_email: &'a str,
    /// Ticket subject line.
    pub subject: &'a str,
    /// Ticket body — markdown allowed but rendered as text in v1.
    pub body: &'a str,
    /// Override priority; defaults to `Normal`.
    pub priority: Option<SupportTicketPriority>,
}

/// One persisted ticket row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupportTicketRow {
    /// Stable ticket id.
    pub id: Uuid,
    /// Tenant id, `None` until enriched.
    pub tenant_id: Option<TenantId>,
    /// Submitter email.
    pub user_email: String,
    /// Subject line.
    pub subject: String,
    /// Ticket body.
    pub body: String,
    /// Current status.
    pub status: SupportTicketStatus,
    /// Current priority.
    pub priority: SupportTicketPriority,
    /// Creation timestamp, epoch seconds.
    pub created_at: i64,
    /// Last mutation timestamp, epoch seconds.
    pub updated_at: i64,
    /// Operator currently handling the ticket, when assigned.
    pub assigned_to: Option<Uuid>,
}

/// One reply row in a ticket's thread.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupportReplyRow {
    /// Stable reply id.
    pub id: Uuid,
    /// Parent ticket id.
    pub ticket_id: Uuid,
    /// Author kind.
    pub author_kind: SupportReplyAuthorKind,
    /// Author id (operator), if known.
    pub author_id: Option<Uuid>,
    /// Reply body.
    pub body: String,
    /// Creation timestamp, epoch seconds.
    pub created_at: i64,
}

/// Insert one ticket. Returns the generated id.
pub async fn create_ticket(
    pool: &Pool<Sqlite>,
    new: NewSupportTicket<'_>,
) -> ArgoResult<Uuid> {
    let id = Uuid::now_v7();
    let now = Utc::now().timestamp();
    let priority = new.priority.unwrap_or(SupportTicketPriority::Normal);
    sqlx::query(
        "INSERT INTO support_ticket \
            (id, tenant_id, user_email, subject, body, status, priority, \
             created_at, updated_at, assigned_to) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, NULL)",
    )
    .bind(id.as_bytes().as_slice())
    .bind(new.tenant_id.map(|t| t.0.as_bytes().to_vec()))
    .bind(new.user_email)
    .bind(new.subject)
    .bind(new.body)
    .bind(SupportTicketStatus::Open.as_str())
    .bind(priority.as_str())
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("support_ticket insert: {e}")))?;
    Ok(id)
}

/// List tickets, optionally filtered by status. Newest-first; `limit`
/// is clamped to `[1, 500]` so a runaway request can't drain the table.
pub async fn list_tickets(
    pool: &Pool<Sqlite>,
    status: Option<SupportTicketStatus>,
    limit: u32,
) -> ArgoResult<Vec<SupportTicketRow>> {
    let limit = limit.clamp(1, 500);
    let rows = if let Some(s) = status {
        sqlx::query(
            "SELECT id, tenant_id, user_email, subject, body, status, priority, \
                    created_at, updated_at, assigned_to \
             FROM support_ticket WHERE status = ? \
             ORDER BY created_at DESC LIMIT ?",
        )
        .bind(s.as_str())
        .bind(i64::from(limit))
        .fetch_all(pool)
        .await
    } else {
        sqlx::query(
            "SELECT id, tenant_id, user_email, subject, body, status, priority, \
                    created_at, updated_at, assigned_to \
             FROM support_ticket ORDER BY created_at DESC LIMIT ?",
        )
        .bind(i64::from(limit))
        .fetch_all(pool)
        .await
    }
    .map_err(|e| ArgoError::Provider(format!("support_ticket list: {e}")))?;
    rows.into_iter().map(decode_ticket).collect()
}

/// Look up a single ticket by id.
pub async fn get_ticket(pool: &Pool<Sqlite>, id: Uuid) -> ArgoResult<Option<SupportTicketRow>> {
    let row = sqlx::query(
        "SELECT id, tenant_id, user_email, subject, body, status, priority, \
                created_at, updated_at, assigned_to \
         FROM support_ticket WHERE id = ?",
    )
    .bind(id.as_bytes().as_slice())
    .fetch_optional(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("support_ticket get: {e}")))?;
    row.map(decode_ticket).transpose()
}

/// Append one reply to a ticket's thread. Bumps `support_ticket.updated_at`.
/// Returns the new reply id.
pub async fn append_reply(
    pool: &Pool<Sqlite>,
    ticket_id: Uuid,
    author_kind: SupportReplyAuthorKind,
    author_id: Option<Uuid>,
    body: &str,
) -> ArgoResult<Uuid> {
    let id = Uuid::now_v7();
    let now = Utc::now().timestamp();
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| ArgoError::Provider(format!("support reply tx: {e}")))?;
    sqlx::query(
        "INSERT INTO support_ticket_reply \
            (id, ticket_id, author_kind, author_id, body, created_at) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(id.as_bytes().as_slice())
    .bind(ticket_id.as_bytes().as_slice())
    .bind(author_kind.as_str())
    .bind(author_id.map(|u| u.as_bytes().to_vec()))
    .bind(body)
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(|e| ArgoError::Provider(format!("support reply insert: {e}")))?;
    sqlx::query("UPDATE support_ticket SET updated_at = ? WHERE id = ?")
        .bind(now)
        .bind(ticket_id.as_bytes().as_slice())
        .execute(&mut *tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("support ticket touch: {e}")))?;
    tx.commit()
        .await
        .map_err(|e| ArgoError::Provider(format!("support reply commit: {e}")))?;
    Ok(id)
}

/// Read every reply on a ticket, oldest-first (thread order).
pub async fn list_replies(
    pool: &Pool<Sqlite>,
    ticket_id: Uuid,
) -> ArgoResult<Vec<SupportReplyRow>> {
    let rows = sqlx::query(
        "SELECT id, ticket_id, author_kind, author_id, body, created_at \
         FROM support_ticket_reply WHERE ticket_id = ? \
         ORDER BY created_at ASC, id ASC",
    )
    .bind(ticket_id.as_bytes().as_slice())
    .fetch_all(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("support reply list: {e}")))?;
    rows.into_iter().map(decode_reply).collect()
}

/// Partial-update bag used by [`update_ticket`]. `None` fields leave
/// the existing column untouched.
#[derive(Debug, Clone, Default)]
pub struct SupportTicketUpdate {
    /// New status.
    pub status: Option<SupportTicketStatus>,
    /// New priority.
    pub priority: Option<SupportTicketPriority>,
    /// New assignee — `Some(None)` clears it, `Some(Some(uuid))` sets it,
    /// `None` leaves it untouched.
    pub assigned_to: Option<Option<Uuid>>,
}

/// Apply a partial update to a ticket. Returns `true` when the row
/// existed. Caller is responsible for any RBAC check; the storage
/// layer only enforces schema.
pub async fn update_ticket(
    pool: &Pool<Sqlite>,
    id: Uuid,
    update: SupportTicketUpdate,
) -> ArgoResult<bool> {
    let now = Utc::now().timestamp();
    let existing = get_ticket(pool, id).await?;
    let Some(row) = existing else {
        return Ok(false);
    };
    let next_status = update.status.unwrap_or(row.status);
    let next_priority = update.priority.unwrap_or(row.priority);
    let next_assigned = match update.assigned_to {
        Some(value) => value,
        None => row.assigned_to,
    };
    sqlx::query(
        "UPDATE support_ticket SET status = ?, priority = ?, assigned_to = ?, updated_at = ? \
         WHERE id = ?",
    )
    .bind(next_status.as_str())
    .bind(next_priority.as_str())
    .bind(next_assigned.map(|u| u.as_bytes().to_vec()))
    .bind(now)
    .bind(id.as_bytes().as_slice())
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("support_ticket update: {e}")))?;
    Ok(true)
}

fn decode_ticket(row: sqlx::sqlite::SqliteRow) -> ArgoResult<SupportTicketRow> {
    let id = decode_uuid(&row, "id")?;
    let tenant_raw: Option<Vec<u8>> = row.try_get("tenant_id").ok();
    let tenant_id = match tenant_raw {
        Some(b) if !b.is_empty() => Some(TenantId(decode_uuid_bytes(&b, "tenant_id")?)),
        _ => None,
    };
    let assigned_raw: Option<Vec<u8>> = row.try_get("assigned_to").ok();
    let assigned_to = match assigned_raw {
        Some(b) if !b.is_empty() => Some(decode_uuid_bytes(&b, "assigned_to")?),
        _ => None,
    };
    let status_raw: String = row
        .try_get("status")
        .map_err(|e| ArgoError::Provider(format!("support_ticket status: {e}")))?;
    let priority_raw: String = row
        .try_get("priority")
        .map_err(|e| ArgoError::Provider(format!("support_ticket priority: {e}")))?;
    Ok(SupportTicketRow {
        id,
        tenant_id,
        user_email: row
            .try_get("user_email")
            .map_err(|e| ArgoError::Provider(format!("support_ticket email: {e}")))?,
        subject: row
            .try_get("subject")
            .map_err(|e| ArgoError::Provider(format!("support_ticket subject: {e}")))?,
        body: row
            .try_get("body")
            .map_err(|e| ArgoError::Provider(format!("support_ticket body: {e}")))?,
        status: SupportTicketStatus::parse(&status_raw)?,
        priority: SupportTicketPriority::parse(&priority_raw)?,
        created_at: row
            .try_get("created_at")
            .map_err(|e| ArgoError::Provider(format!("support_ticket created_at: {e}")))?,
        updated_at: row
            .try_get("updated_at")
            .map_err(|e| ArgoError::Provider(format!("support_ticket updated_at: {e}")))?,
        assigned_to,
    })
}

fn decode_reply(row: sqlx::sqlite::SqliteRow) -> ArgoResult<SupportReplyRow> {
    let id = decode_uuid(&row, "id")?;
    let ticket_id = decode_uuid(&row, "ticket_id")?;
    let kind_raw: String = row
        .try_get("author_kind")
        .map_err(|e| ArgoError::Provider(format!("support reply kind: {e}")))?;
    let author_raw: Option<Vec<u8>> = row.try_get("author_id").ok();
    let author_id = match author_raw {
        Some(b) if !b.is_empty() => Some(decode_uuid_bytes(&b, "author_id")?),
        _ => None,
    };
    Ok(SupportReplyRow {
        id,
        ticket_id,
        author_kind: SupportReplyAuthorKind::parse(&kind_raw)?,
        author_id,
        body: row
            .try_get("body")
            .map_err(|e| ArgoError::Provider(format!("support reply body: {e}")))?,
        created_at: row
            .try_get("created_at")
            .map_err(|e| ArgoError::Provider(format!("support reply created_at: {e}")))?,
    })
}

fn decode_uuid(row: &sqlx::sqlite::SqliteRow, column: &str) -> ArgoResult<Uuid> {
    let bytes: Vec<u8> = row
        .try_get(column)
        .map_err(|e| ArgoError::Provider(format!("support {column}: {e}")))?;
    decode_uuid_bytes(&bytes, column)
}

fn decode_uuid_bytes(bytes: &[u8], column: &str) -> ArgoResult<Uuid> {
    Uuid::from_slice(bytes).map_err(|e| ArgoError::Schema(format!("support {column} uuid: {e}")))
}

#[cfg(test)]
#[path = "support_query_tests.rs"]
mod tests;
