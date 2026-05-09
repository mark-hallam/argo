//! Tests for [`super::team_roster`] — sibling-tests pattern via
//! `#[path = "team_roster_tests.rs"] mod tests;` so the module
//! file stays under the 500-LOC ceiling while sharing access to
//! private helpers.
#![cfg(test)]

use super::{
    add_team_member, create_team, ensure_team_roster_columns, get_team, list_teams,
    remove_team_member, rename_team,
};
use crate::SessionStore;
use argo_types::TenantId;

async fn fresh() -> (SessionStore, TenantId) {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    (store, TenantId::new())
}

#[tokio::test]
async fn ensure_schema_is_idempotent() {
    let (store, _) = fresh().await;
    ensure_team_roster_columns(store.pool()).await.unwrap();
    ensure_team_roster_columns(store.pool()).await.unwrap();
    let rows = sqlx::query("PRAGMA table_info(roster_member)")
        .fetch_all(store.pool())
        .await
        .unwrap();
    let names: Vec<String> = rows
        .iter()
        .map(|r| {
            use sqlx::Row;
            r.get::<String, _>("name")
        })
        .collect();
    assert!(names.contains(&"role".to_string()));
    assert!(names.contains(&"removed_at".to_string()));
}

#[tokio::test]
async fn create_team_inserts_org_unit_and_roster() {
    let (store, tenant) = fresh().await;
    let outcome = create_team(store.pool(), tenant, "finance", None).await.unwrap();
    assert!(!outcome.id.is_empty());
    assert_eq!(outcome.name, "finance");
    let rows = list_teams(store.pool(), tenant).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].name, "finance");
    assert_eq!(rows[0].member_count, 0);
}

#[tokio::test]
async fn create_team_validates_parent() {
    let (store, tenant) = fresh().await;
    let err = create_team(store.pool(), tenant, "child", Some("nope-id"))
        .await
        .unwrap_err();
    assert!(err.to_string().contains("parent team"));
}

#[tokio::test]
async fn create_team_with_valid_parent() {
    let (store, tenant) = fresh().await;
    let parent = create_team(store.pool(), tenant, "engineering", None)
        .await
        .unwrap();
    let child = create_team(store.pool(), tenant, "platform", Some(&parent.id))
        .await
        .unwrap();
    let detail = get_team(store.pool(), tenant, &child.id).await.unwrap().unwrap();
    assert_eq!(detail.level, 1);
    assert_eq!(detail.parent_id.as_deref(), Some(parent.id.as_str()));
}

#[tokio::test]
async fn rename_team_updates_org_unit_and_roster() {
    let (store, tenant) = fresh().await;
    let team = create_team(store.pool(), tenant, "ops", None).await.unwrap();
    let renamed = rename_team(store.pool(), tenant, &team.id, "operations")
        .await
        .unwrap();
    assert!(renamed);
    let rows = list_teams(store.pool(), tenant).await.unwrap();
    assert_eq!(rows[0].name, "operations");
    let roster_name: String = sqlx::query_scalar("SELECT name FROM roster WHERE id = ?")
        .bind(&team.id)
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(roster_name, "operations");
}

#[tokio::test]
async fn rename_missing_team_returns_false() {
    let (store, tenant) = fresh().await;
    let renamed = rename_team(store.pool(), tenant, "missing", "newname")
        .await
        .unwrap();
    assert!(!renamed);
}

#[tokio::test]
async fn rename_rejects_empty_name() {
    let (store, tenant) = fresh().await;
    let team = create_team(store.pool(), tenant, "ops", None).await.unwrap();
    let err = rename_team(store.pool(), tenant, &team.id, "   ")
        .await
        .unwrap_err();
    assert!(err.to_string().contains("must not be empty"));
}

#[tokio::test]
async fn add_member_appears_in_show_and_count() {
    let (store, tenant) = fresh().await;
    let team = create_team(store.pool(), tenant, "support", None).await.unwrap();
    let worker = uuid::Uuid::now_v7().to_string();
    add_team_member(store.pool(), &team.id, &worker, Some("clerk"))
        .await
        .unwrap();
    let detail = get_team(store.pool(), tenant, &team.id).await.unwrap().unwrap();
    assert_eq!(detail.members.len(), 1);
    assert_eq!(detail.members[0].worker_id, worker);
    assert_eq!(detail.members[0].role.as_deref(), Some("clerk"));
    let rows = list_teams(store.pool(), tenant).await.unwrap();
    assert_eq!(rows[0].member_count, 1);
}

#[tokio::test]
async fn add_member_is_idempotent_and_updates_role() {
    let (store, tenant) = fresh().await;
    let team = create_team(store.pool(), tenant, "support", None).await.unwrap();
    let worker = uuid::Uuid::now_v7().to_string();
    add_team_member(store.pool(), &team.id, &worker, Some("clerk"))
        .await
        .unwrap();
    add_team_member(store.pool(), &team.id, &worker, Some("lead"))
        .await
        .unwrap();
    let detail = get_team(store.pool(), tenant, &team.id).await.unwrap().unwrap();
    assert_eq!(detail.members.len(), 1);
    assert_eq!(detail.members[0].role.as_deref(), Some("lead"));
}

#[tokio::test]
async fn remove_member_is_soft_delete() {
    let (store, tenant) = fresh().await;
    let team = create_team(store.pool(), tenant, "support", None).await.unwrap();
    let worker = uuid::Uuid::now_v7().to_string();
    add_team_member(store.pool(), &team.id, &worker, None).await.unwrap();
    let removed = remove_team_member(store.pool(), &team.id, &worker)
        .await
        .unwrap();
    assert!(removed);
    let detail = get_team(store.pool(), tenant, &team.id).await.unwrap().unwrap();
    assert!(detail.members.is_empty());
    let row: Option<i64> = sqlx::query_scalar(
        "SELECT removed_at FROM roster_member WHERE roster_id = ? AND worker_id = ?",
    )
    .bind(&team.id)
    .bind(&worker)
    .fetch_optional(store.pool())
    .await
    .unwrap()
    .flatten();
    assert!(row.is_some(), "soft-delete should preserve the row");
}

#[tokio::test]
async fn re_add_after_remove_clears_removed_at() {
    let (store, tenant) = fresh().await;
    let team = create_team(store.pool(), tenant, "support", None).await.unwrap();
    let worker = uuid::Uuid::now_v7().to_string();
    add_team_member(store.pool(), &team.id, &worker, Some("clerk"))
        .await
        .unwrap();
    remove_team_member(store.pool(), &team.id, &worker)
        .await
        .unwrap();
    add_team_member(store.pool(), &team.id, &worker, Some("clerk"))
        .await
        .unwrap();
    let detail = get_team(store.pool(), tenant, &team.id).await.unwrap().unwrap();
    assert_eq!(detail.members.len(), 1);
    assert_eq!(detail.members[0].role.as_deref(), Some("clerk"));
}

#[tokio::test]
async fn remove_missing_returns_false() {
    let (store, tenant) = fresh().await;
    let team = create_team(store.pool(), tenant, "support", None).await.unwrap();
    let worker = uuid::Uuid::now_v7().to_string();
    let removed = remove_team_member(store.pool(), &team.id, &worker)
        .await
        .unwrap();
    assert!(!removed);
}

#[tokio::test]
async fn list_teams_orders_by_name_and_isolates_by_tenant() {
    let (store, tenant_a) = fresh().await;
    let tenant_b = TenantId::new();
    create_team(store.pool(), tenant_a, "zeta", None).await.unwrap();
    create_team(store.pool(), tenant_a, "alpha", None).await.unwrap();
    create_team(store.pool(), tenant_b, "outsider", None).await.unwrap();
    let rows_a = list_teams(store.pool(), tenant_a).await.unwrap();
    assert_eq!(rows_a.len(), 2);
    assert_eq!(rows_a[0].name, "alpha");
    assert_eq!(rows_a[1].name, "zeta");
    let rows_b = list_teams(store.pool(), tenant_b).await.unwrap();
    assert_eq!(rows_b.len(), 1);
    assert_eq!(rows_b[0].name, "outsider");
}

#[tokio::test]
async fn get_team_returns_none_for_unknown_id() {
    let (store, tenant) = fresh().await;
    let detail = get_team(store.pool(), tenant, "no-such-id").await.unwrap();
    assert!(detail.is_none());
}
