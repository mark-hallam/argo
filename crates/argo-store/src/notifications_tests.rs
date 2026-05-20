//! Unit tests for the `notifications` store module.

use super::*;
use sqlx::sqlite::SqlitePoolOptions;

async fn pool() -> SqlitePool {
    SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("memory pool")
}

fn ins<'a>(
    id: &'a str,
    tenant: &'a str,
    principal: Option<&'a str>,
    created_at: i64,
) -> NotificationInsert<'a> {
    NotificationInsert {
        id,
        tenant_id: tenant,
        principal_id: principal,
        kind: "duty-output-ready",
        body: "Daily revenue digest is ready.",
        created_at,
        related_audit_id: None,
        severity: "info",
    }
}

#[tokio::test]
async fn insert_then_list_round_trips() {
    let p = pool().await;
    insert(&p, &ins("nt_a", "t1", Some("human:demo"), 100))
        .await
        .unwrap();
    let page = list_for_principal(
        &p,
        "t1",
        Some("human:demo"),
        &ListQuery {
            limit: None,
            before: None,
            exclude_dismissed: true,
        },
    )
    .await
    .unwrap();
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].id, "nt_a");
    assert!(!page.has_more);
    assert!(page.cursor.is_none());
}

#[tokio::test]
async fn list_excludes_cross_tenant_rows() {
    let p = pool().await;
    insert(&p, &ins("nt_a", "t1", Some("human:alice"), 100))
        .await
        .unwrap();
    insert(&p, &ins("nt_b", "t2", Some("human:alice"), 100))
        .await
        .unwrap();
    let page = list_for_principal(
        &p,
        "t1",
        Some("human:alice"),
        &ListQuery {
            exclude_dismissed: true,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].id, "nt_a");
}

#[tokio::test]
async fn list_excludes_other_principals_but_includes_broadcasts() {
    let p = pool().await;
    insert(&p, &ins("nt_a", "t1", Some("human:alice"), 100))
        .await
        .unwrap();
    insert(&p, &ins("nt_b", "t1", Some("human:bob"), 100))
        .await
        .unwrap();
    insert(&p, &ins("nt_c", "t1", None, 100)).await.unwrap();
    let alice = list_for_principal(
        &p,
        "t1",
        Some("human:alice"),
        &ListQuery {
            exclude_dismissed: true,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let mut alice_ids: Vec<&str> = alice.items.iter().map(|r| r.id.as_str()).collect();
    alice_ids.sort();
    assert_eq!(alice_ids, vec!["nt_a", "nt_c"]);
}

#[tokio::test]
async fn pagination_returns_cursor_and_has_more() {
    let p = pool().await;
    for n in 0..5 {
        insert(
            &p,
            &ins(
                &format!("nt_{n}"),
                "t1",
                Some("human:alice"),
                100 + i64::from(n),
            ),
        )
        .await
        .unwrap();
    }
    let page = list_for_principal(
        &p,
        "t1",
        Some("human:alice"),
        &ListQuery {
            limit: Some(2),
            exclude_dismissed: true,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(page.items.len(), 2);
    assert!(page.has_more);
    let cursor = page.cursor.expect("cursor present when has_more");
    assert_eq!(cursor, 103);

    let next = list_for_principal(
        &p,
        "t1",
        Some("human:alice"),
        &ListQuery {
            limit: Some(2),
            before: Some(cursor),
            exclude_dismissed: true,
        },
    )
    .await
    .unwrap();
    let ids: Vec<&str> = next.items.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(ids, vec!["nt_2", "nt_1"]);
    assert!(next.has_more);
}

#[tokio::test]
async fn dismiss_one_is_idempotent_and_scoped() {
    let p = pool().await;
    insert(&p, &ins("nt_a", "t1", Some("human:alice"), 100))
        .await
        .unwrap();
    insert(&p, &ins("nt_b", "t1", Some("human:bob"), 100))
        .await
        .unwrap();
    let first = dismiss_one(&p, "t1", Some("human:alice"), "nt_a", 999)
        .await
        .unwrap();
    assert!(first, "first dismiss returns true");
    let second = dismiss_one(&p, "t1", Some("human:alice"), "nt_a", 999)
        .await
        .unwrap();
    assert!(!second, "second dismiss is a no-op");

    let cross = dismiss_one(&p, "t1", Some("human:alice"), "nt_b", 999)
        .await
        .unwrap();
    assert!(!cross, "alice cannot dismiss bob's notification");
}

#[tokio::test]
async fn dismiss_one_refuses_cross_tenant() {
    let p = pool().await;
    insert(&p, &ins("nt_a", "t1", Some("human:alice"), 100))
        .await
        .unwrap();
    let result = dismiss_one(&p, "t2", Some("human:alice"), "nt_a", 999)
        .await
        .unwrap();
    assert!(!result);
}

#[tokio::test]
async fn dismiss_all_only_touches_visible_rows() {
    let p = pool().await;
    insert(&p, &ins("nt_a", "t1", Some("human:alice"), 100))
        .await
        .unwrap();
    insert(&p, &ins("nt_b", "t1", Some("human:alice"), 110))
        .await
        .unwrap();
    insert(&p, &ins("nt_c", "t1", Some("human:bob"), 110))
        .await
        .unwrap();
    insert(&p, &ins("nt_d", "t2", Some("human:alice"), 110))
        .await
        .unwrap();
    let touched = dismiss_all_for_principal(&p, "t1", Some("human:alice"), 999)
        .await
        .unwrap();
    assert_eq!(touched, 2, "alice's two t1 rows are dismissed");
    // bob's notification + alice's t2 notification still visible
    let bob = list_for_principal(
        &p,
        "t1",
        Some("human:bob"),
        &ListQuery {
            exclude_dismissed: true,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(bob.items.len(), 1);
}

#[tokio::test]
async fn unread_count_ignores_dismissed_and_cross_tenant() {
    let p = pool().await;
    insert(&p, &ins("nt_a", "t1", Some("human:alice"), 100))
        .await
        .unwrap();
    insert(&p, &ins("nt_b", "t1", Some("human:alice"), 110))
        .await
        .unwrap();
    insert(&p, &ins("nt_c", "t2", Some("human:alice"), 110))
        .await
        .unwrap();
    insert(&p, &ins("nt_d", "t1", Some("human:bob"), 110))
        .await
        .unwrap();
    assert_eq!(
        unread_count(&p, "t1", Some("human:alice")).await.unwrap(),
        2
    );
    dismiss_one(&p, "t1", Some("human:alice"), "nt_a", 999)
        .await
        .unwrap();
    assert_eq!(
        unread_count(&p, "t1", Some("human:alice")).await.unwrap(),
        1
    );
}

#[tokio::test]
async fn wire_json_uses_camel_case_keys() {
    let row = NotificationRow {
        id: "nt_a".into(),
        tenant_id: "t1".into(),
        principal_id: Some("human:alice".into()),
        kind: "budget-approaching".into(),
        body: "Hi".into(),
        created_at: 1_700_000_000,
        read_at: None,
        dismissed_at: None,
        related_audit_id: None,
        severity: "info".into(),
    };
    let v = row.to_wire_json();
    assert_eq!(v["id"], "nt_a");
    assert_eq!(v["title"], "Hi");
    assert_eq!(v["read"], false);
    assert!(v["createdAt"].as_str().unwrap().ends_with("Z"));
}
