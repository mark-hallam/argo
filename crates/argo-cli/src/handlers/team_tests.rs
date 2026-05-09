//! CLI-handler tests for `argo team`.
//!
//! Covers two layers: rendering (markdown vs json formatters) and
//! charter hydration via `IdentityStore`. The end-to-end binary
//! tests live in `crates/argo-cli/tests/` so the in-process
//! handler is exercised here against an in-memory `SessionStore`.
#![cfg(test)]

use argo_identity::{IdentityStore, Persona, Role, Worker};
use argo_store::team_roster::{
    add_team_member, create_team, summarise_charter, TeamDetail, TeamMemberRow,
};
use argo_store::SessionStore;
use argo_types::TenantId;

use super::{__test_render_list, __test_render_show, __test_show_payload};

async fn fresh() -> (SessionStore, TenantId) {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    (store, TenantId::new())
}

#[tokio::test]
async fn list_markdown_snapshot() {
    let (store, tenant) = fresh().await;
    let _ = create_team(store.pool(), tenant, "alpha", None).await.unwrap();
    let _ = create_team(store.pool(), tenant, "beta", None).await.unwrap();
    let teams = argo_store::team_roster::list_teams(store.pool(), tenant)
        .await
        .unwrap();
    let rendered = __test_render_list(&teams);
    assert!(rendered.starts_with("# Teams\n"));
    assert!(rendered.contains("| alpha |"));
    assert!(rendered.contains("| beta |"));
    assert!(rendered.contains("| 0 |"));
}

#[tokio::test]
async fn list_markdown_empty_state() {
    let (store, tenant) = fresh().await;
    let teams = argo_store::team_roster::list_teams(store.pool(), tenant)
        .await
        .unwrap();
    let rendered = __test_render_list(&teams);
    assert_eq!(rendered.trim(), "(no teams)");
}

#[tokio::test]
async fn show_markdown_snapshot_with_charter_summary() {
    let (store, tenant) = fresh().await;
    let identities = IdentityStore::new(&store);
    identities.ensure_schema().await.unwrap();
    let mut worker = Worker::draft(
        tenant,
        "support",
        Persona::named("Maya", "op"),
        Role::new("Run point on customer escalations."),
    );
    worker.charter = Some(
        "Reply within 24h. Loop the AR clerk on any pricing question. \
         Escalate to Director within 4h on any privacy incident."
            .to_string(),
    );
    identities.create_worker(&worker).await.unwrap();
    let team = create_team(store.pool(), tenant, "support", None).await.unwrap();
    add_team_member(
        store.pool(),
        &team.id,
        &worker.id.to_string(),
        Some("lead"),
    )
    .await
    .unwrap();
    let detail = argo_store::team_roster::get_team(store.pool(), tenant, &team.id)
        .await
        .unwrap()
        .unwrap();
    let payload = __test_show_payload(detail, tenant, &store).await.unwrap();
    let rendered = __test_render_show(&payload);
    assert!(rendered.contains("# Team `support`"));
    assert!(rendered.contains("- level: 0"));
    assert!(rendered.contains("- members: 1"));
    assert!(rendered.contains("| lead |"));
    assert!(
        rendered.contains("Reply within 24h"),
        "expected charter summary in:\n{rendered}"
    );
}

#[tokio::test]
async fn show_markdown_no_active_members() {
    let (store, tenant) = fresh().await;
    let team = create_team(store.pool(), tenant, "ops", None).await.unwrap();
    let detail = argo_store::team_roster::get_team(store.pool(), tenant, &team.id)
        .await
        .unwrap()
        .unwrap();
    let payload = __test_show_payload(detail, tenant, &store).await.unwrap();
    let rendered = __test_render_show(&payload);
    assert!(rendered.contains("_no active members._"));
}

#[tokio::test]
async fn show_charter_summary_truncates_to_80_chars() {
    let body = "x".repeat(120);
    let summary = summarise_charter(&body);
    assert_eq!(summary.chars().count(), 80);
    assert!(summary.ends_with('…'));
}

#[tokio::test]
async fn show_charter_summary_short_passes_through() {
    let summary = summarise_charter("short charter");
    assert_eq!(summary, "short charter");
}

#[tokio::test]
async fn show_payload_handles_unparseable_worker_id() {
    let (store, tenant) = fresh().await;
    let detail = TeamDetail {
        id: "team-x".into(),
        name: "x".into(),
        parent_id: None,
        level: 0,
        members: vec![TeamMemberRow {
            worker_id: "not-a-uuid".into(),
            role: None,
            added_at: 0,
        }],
    };
    let payload = __test_show_payload(detail, tenant, &store).await.unwrap();
    assert_eq!(payload.members.len(), 1);
    assert!(payload.members[0].charter_summary.is_none());
}
