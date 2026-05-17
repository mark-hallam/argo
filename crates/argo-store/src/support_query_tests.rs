//! Unit tests for [`super::support_query`].
#![cfg(test)]

use super::{
    append_reply, create_ticket, get_ticket, list_replies, list_tickets, update_ticket,
    NewSupportTicket, SupportReplyAuthorKind, SupportTicketPriority, SupportTicketStatus,
    SupportTicketUpdate,
};
use crate::SessionStore;
use argo_types::TenantId;
use sqlx::{Pool, Sqlite};
use uuid::Uuid;

async fn fresh_pool() -> Pool<Sqlite> {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    store.pool().clone()
}

#[tokio::test]
async fn create_then_get_round_trips() {
    let pool = fresh_pool().await;
    let id = create_ticket(
        &pool,
        NewSupportTicket {
            tenant_id: None,
            user_email: "alice@example.com",
            subject: "cannot log in",
            body: "magic link bounces with 500",
            priority: None,
        },
    )
    .await
    .unwrap();

    let row = get_ticket(&pool, id).await.unwrap().expect("row exists");
    assert_eq!(row.user_email, "alice@example.com");
    assert_eq!(row.subject, "cannot log in");
    assert_eq!(row.status, SupportTicketStatus::Open);
    assert_eq!(row.priority, SupportTicketPriority::Normal);
    assert!(row.tenant_id.is_none());
    assert!(row.assigned_to.is_none());
    assert!(row.created_at > 0);
    assert_eq!(row.updated_at, row.created_at);
}

#[tokio::test]
async fn list_filters_by_status_and_orders_newest_first() {
    let pool = fresh_pool().await;
    for subject in ["old", "mid", "new"] {
        create_ticket(
            &pool,
            NewSupportTicket {
                tenant_id: None,
                user_email: "u@example.com",
                subject,
                body: "body",
                priority: None,
            },
        )
        .await
        .unwrap();
        // Tickets share a created_at when issued in the same second;
        // a tiny wait keeps the order assertion stable on fast machines.
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    }
    let open = list_tickets(&pool, Some(SupportTicketStatus::Open), 50)
        .await
        .unwrap();
    assert_eq!(open.len(), 3);
    assert_eq!(open[0].subject, "new");
    assert_eq!(open[2].subject, "old");

    let resolved = list_tickets(&pool, Some(SupportTicketStatus::Resolved), 50)
        .await
        .unwrap();
    assert!(resolved.is_empty());

    let all = list_tickets(&pool, None, 50).await.unwrap();
    assert_eq!(all.len(), 3);
}

#[tokio::test]
async fn update_changes_status_priority_and_assignee() {
    let pool = fresh_pool().await;
    let id = create_ticket(
        &pool,
        NewSupportTicket {
            tenant_id: None,
            user_email: "u@example.com",
            subject: "hi",
            body: "body",
            priority: None,
        },
    )
    .await
    .unwrap();
    let operator = Uuid::now_v7();
    let changed = update_ticket(
        &pool,
        id,
        SupportTicketUpdate {
            status: Some(SupportTicketStatus::Pending),
            priority: Some(SupportTicketPriority::High),
            assigned_to: Some(Some(operator)),
        },
    )
    .await
    .unwrap();
    assert!(changed);
    let row = get_ticket(&pool, id).await.unwrap().unwrap();
    assert_eq!(row.status, SupportTicketStatus::Pending);
    assert_eq!(row.priority, SupportTicketPriority::High);
    assert_eq!(row.assigned_to, Some(operator));
}

#[tokio::test]
async fn update_returns_false_for_missing_ticket() {
    let pool = fresh_pool().await;
    let changed = update_ticket(
        &pool,
        Uuid::now_v7(),
        SupportTicketUpdate {
            status: Some(SupportTicketStatus::Closed),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert!(!changed);
}

#[tokio::test]
async fn append_reply_orders_thread_and_touches_ticket() {
    let pool = fresh_pool().await;
    let ticket = create_ticket(
        &pool,
        NewSupportTicket {
            tenant_id: None,
            user_email: "u@example.com",
            subject: "thread",
            body: "first",
            priority: None,
        },
    )
    .await
    .unwrap();
    let initial = get_ticket(&pool, ticket).await.unwrap().unwrap().updated_at;

    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    let operator = Uuid::now_v7();
    append_reply(
        &pool,
        ticket,
        SupportReplyAuthorKind::Operator,
        Some(operator),
        "we're looking into this",
    )
    .await
    .unwrap();
    append_reply(
        &pool,
        ticket,
        SupportReplyAuthorKind::User,
        None,
        "thanks",
    )
    .await
    .unwrap();

    let thread = list_replies(&pool, ticket).await.unwrap();
    assert_eq!(thread.len(), 2);
    assert_eq!(thread[0].author_kind, SupportReplyAuthorKind::Operator);
    assert_eq!(thread[0].author_id, Some(operator));
    assert_eq!(thread[1].author_kind, SupportReplyAuthorKind::User);
    assert!(thread[1].author_id.is_none());

    let row = get_ticket(&pool, ticket).await.unwrap().unwrap();
    assert!(row.updated_at > initial);
}

#[tokio::test]
async fn tenant_id_round_trips() {
    let pool = fresh_pool().await;
    let tenant = TenantId(Uuid::now_v7());
    let id = create_ticket(
        &pool,
        NewSupportTicket {
            tenant_id: Some(tenant),
            user_email: "u@example.com",
            subject: "tenant",
            body: "body",
            priority: Some(SupportTicketPriority::Urgent),
        },
    )
    .await
    .unwrap();
    let row = get_ticket(&pool, id).await.unwrap().unwrap();
    assert_eq!(row.tenant_id, Some(tenant));
    assert_eq!(row.priority, SupportTicketPriority::Urgent);
}
