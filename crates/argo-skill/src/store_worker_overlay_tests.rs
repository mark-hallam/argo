//! Coverage for [`SkillStore::list_for_worker`]
//! (mission-pm-skill-overlay-load).
//!
//! The brief calls for tests in `store_tests.rs` (NEW or existing
//! sibling); this file is the focused new sibling, mirroring the
//! pattern of `store_available_tests.rs` / `store_trust_tests.rs`.

use crate::store::SkillStore;
use argo_store::{SessionStore, WorkerId};
use std::sync::Arc;

async fn fresh_store() -> SkillStore {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    SkillStore::new(Arc::new(store))
}

fn skill_doc(name: &str, version: &str, body: &str) -> String {
    format!(
        "---\nname: {name}\nversion: \"{version}\"\ndescription: d\n\
         provenance:\n  source: builtin\n  hash: sha256-x\n---\n{body}"
    )
}

async fn attach_overlay(store: &SkillStore, worker_id: WorkerId, skill_id: argo_types::SkillId) {
    sqlx::query("INSERT INTO worker_skill (worker_id, skill_id, proficiency) VALUES (?, ?, 5)")
        .bind(worker_id.to_string())
        .bind(skill_id.to_string())
        .execute(store.store_arc().pool())
        .await
        .unwrap();
}

#[tokio::test]
async fn list_for_worker_returns_attached_active_overlays() {
    let store = fresh_store().await;
    let worker = WorkerId::new();
    let skill = store
        .install(&skill_doc("overlay-a", "0.1.0", "overlay-body"), None)
        .await
        .unwrap();
    attach_overlay(&store, worker, skill.id).await;

    let listed = store.list_for_worker(worker).await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].frontmatter.name, "overlay-a");
    assert_eq!(listed[0].body, "overlay-body");
}

#[tokio::test]
async fn list_for_worker_filters_other_workers() {
    let store = fresh_store().await;
    let worker_a = WorkerId::new();
    let worker_b = WorkerId::new();
    let skill_a = store
        .install(&skill_doc("for-a", "0.1.0", "body-a"), None)
        .await
        .unwrap();
    let skill_b = store
        .install(&skill_doc("for-b", "0.1.0", "body-b"), None)
        .await
        .unwrap();
    attach_overlay(&store, worker_a, skill_a.id).await;
    attach_overlay(&store, worker_b, skill_b.id).await;

    let listed_a = store.list_for_worker(worker_a).await.unwrap();
    assert_eq!(listed_a.len(), 1);
    assert_eq!(listed_a[0].frontmatter.name, "for-a");

    let listed_b = store.list_for_worker(worker_b).await.unwrap();
    assert_eq!(listed_b.len(), 1);
    assert_eq!(listed_b[0].frontmatter.name, "for-b");
}

#[tokio::test]
async fn list_for_worker_skips_non_active_lifecycle_states() {
    let store = fresh_store().await;
    let worker = WorkerId::new();
    let skill = store
        .install(&skill_doc("overlay-pending", "0.1.0", "p-body"), None)
        .await
        .unwrap();
    attach_overlay(&store, worker, skill.id).await;
    sqlx::query("UPDATE skill SET lifecycle_state = 'pending_review' WHERE id = ?")
        .bind(skill.id.0.as_bytes().as_slice())
        .execute(store.store_arc().pool())
        .await
        .unwrap();

    let listed = store.list_for_worker(worker).await.unwrap();
    assert!(listed.is_empty(), "pending_review overlays must be hidden");
}

#[tokio::test]
async fn list_for_worker_skips_superseded_rows() {
    let store = fresh_store().await;
    let worker = WorkerId::new();
    let skill = store
        .install(&skill_doc("overlay-old", "0.1.0", "old-body"), None)
        .await
        .unwrap();
    attach_overlay(&store, worker, skill.id).await;
    // Mark superseded by pointing at itself — the SQL filter only
    // checks IS NOT NULL, so any non-null value triggers the skip.
    sqlx::query("UPDATE skill SET superseded_by = ? WHERE id = ?")
        .bind(skill.id.0.as_bytes().as_slice())
        .bind(skill.id.0.as_bytes().as_slice())
        .execute(store.store_arc().pool())
        .await
        .unwrap();

    let listed = store.list_for_worker(worker).await.unwrap();
    assert!(listed.is_empty(), "superseded overlays must be hidden");
}

#[tokio::test]
async fn list_for_worker_returns_empty_when_no_overlays_attached() {
    let store = fresh_store().await;
    let worker = WorkerId::new();
    let listed = store.list_for_worker(worker).await.unwrap();
    assert!(listed.is_empty());
}
