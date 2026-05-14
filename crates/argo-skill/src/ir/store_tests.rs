//! Unit tests for the SQLite [`SkillIrStore`] (L2 brief).
//!
//! Each test spins up an in-memory [`SessionStore`], runs the
//! workspace migration registry, then applies [`ensure_schema`] for
//! the `skill_ir` table. Tests are independent and never share state.
use super::{ChainRow, SkillIrStore, SqliteSkillIrStore, ensure_schema, verify_chain};
use crate::ir::{Node, NodeAnnotations, NodeId, NodeKind, SkillGraph, canonical_hash};
use argo_store::SessionStore;
use argo_types::{SkillId, TenantId};

async fn open_store() -> SessionStore {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    ensure_schema(&store).await.unwrap();
    store
}

fn graph_with_label(label: &str) -> SkillGraph {
    SkillGraph {
        entry: NodeId::from("n1"),
        nodes: vec![Node {
            id: NodeId::from("n1"),
            label: label.into(),
            kind: NodeKind::Action {
                instruction: format!("do {label}"),
            },
            annotations: NodeAnnotations::default(),
        }],
        edges: vec![],
        variables: vec![],
        metadata: Default::default(),
    }
}

#[tokio::test]
async fn ensure_schema_is_idempotent_against_populated_db() {
    let store = open_store().await;
    // Apply once more on top of a populated DB — second apply must
    // be a no-op (covers L2 acceptance criterion 2).
    ensure_schema(&store).await.unwrap();
    ensure_schema(&store).await.unwrap();
    // Sanity: a write still succeeds after re-applying schema.
    let ir = SqliteSkillIrStore::new(&store);
    let tenant = TenantId::new();
    let skill = SkillId::new();
    let returned = ir
        .put(tenant, skill, "v1", &graph_with_label("a"))
        .await
        .unwrap();
    assert_eq!(returned.len(), 32);
}

#[tokio::test]
async fn put_returns_canonical_hash_and_roundtrips() {
    let store = open_store().await;
    let ir = SqliteSkillIrStore::new(&store);
    let tenant = TenantId::new();
    let skill = SkillId::new();
    let graph = graph_with_label("hello");
    let returned = ir.put(tenant, skill, "v1", &graph).await.unwrap();
    assert_eq!(returned, canonical_hash(&graph).unwrap());
    let fetched = ir.get(tenant, skill, "v1").await.unwrap().unwrap();
    assert_eq!(fetched, graph);
}

#[tokio::test]
async fn latest_returns_newest_version_and_list_is_newest_first() {
    let store = open_store().await;
    let ir = SqliteSkillIrStore::new(&store);
    let tenant = TenantId::new();
    let skill = SkillId::new();
    ir.put(tenant, skill, "v1", &graph_with_label("one"))
        .await
        .unwrap();
    // Force monotonic created_at by sleeping a microsecond; the
    // ORDER BY also falls back to version DESC so the test stays
    // robust even on coarse clocks.
    tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    ir.put(tenant, skill, "v2", &graph_with_label("two"))
        .await
        .unwrap();
    let (latest_version, latest_graph) = ir.latest(tenant, skill).await.unwrap().unwrap();
    assert_eq!(latest_version, "v2");
    assert_eq!(latest_graph, graph_with_label("two"));
    let versions = ir.list_versions(tenant, skill).await.unwrap();
    assert_eq!(versions, vec!["v2".to_string(), "v1".to_string()]);
}

#[tokio::test]
async fn prev_hash_chains_consecutive_versions() {
    let store = open_store().await;
    let ir = SqliteSkillIrStore::new(&store);
    let tenant = TenantId::new();
    let skill = SkillId::new();
    let h1 = ir
        .put(tenant, skill, "v1", &graph_with_label("one"))
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    ir.put(tenant, skill, "v2", &graph_with_label("two"))
        .await
        .unwrap();
    let chain: Vec<ChainRow> = verify_chain(&store, tenant, skill).await.unwrap();
    assert_eq!(chain.len(), 2);
    assert_eq!(chain[0].stored_prev_hash, None);
    assert_eq!(chain[1].stored_prev_hash, Some(h1));
    assert_eq!(chain[0].recomputed_hash, h1);
}

#[tokio::test]
async fn verify_chain_detects_in_place_tampering() {
    let store = open_store().await;
    let ir = SqliteSkillIrStore::new(&store);
    let tenant = TenantId::new();
    let skill = SkillId::new();
    ir.put(tenant, skill, "v1", &graph_with_label("one"))
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    ir.put(tenant, skill, "v2", &graph_with_label("two"))
        .await
        .unwrap();
    // Tamper with row 1's ir_json — the recompute will no longer
    // match row 2's stored prev_hash.
    let tampered = serde_json::to_string(&graph_with_label("hacked")).unwrap();
    sqlx::query(
        "UPDATE skill_ir SET ir_json = ? \
         WHERE tenant_id = ? AND skill_id = ? AND version = 'v1'",
    )
    .bind(&tampered)
    .bind(tenant.0.as_bytes().as_slice())
    .bind(skill.0.as_bytes().as_slice())
    .execute(store.pool())
    .await
    .unwrap();
    let err = verify_chain(&store, tenant, skill).await.unwrap_err();
    assert!(
        err.to_string().contains("chain broken at version v2"),
        "expected chain-broken error, got: {err}"
    );
}

#[tokio::test]
async fn tenant_isolation_blocks_cross_tenant_reads() {
    let store = open_store().await;
    let ir = SqliteSkillIrStore::new(&store);
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    let skill = SkillId::new();
    ir.put(tenant_a, skill, "v1", &graph_with_label("secret"))
        .await
        .unwrap();
    assert!(ir.get(tenant_b, skill, "v1").await.unwrap().is_none());
    assert!(ir.latest(tenant_b, skill).await.unwrap().is_none());
    assert!(ir.list_versions(tenant_b, skill).await.unwrap().is_empty());
}

#[tokio::test]
async fn put_rejects_duplicate_version() {
    let store = open_store().await;
    let ir = SqliteSkillIrStore::new(&store);
    let tenant = TenantId::new();
    let skill = SkillId::new();
    ir.put(tenant, skill, "v1", &graph_with_label("one"))
        .await
        .unwrap();
    let err = ir
        .put(tenant, skill, "v1", &graph_with_label("one-again"))
        .await
        .unwrap_err();
    assert!(
        err.to_string().contains("versions are immutable"),
        "expected immutability error, got: {err}"
    );
}
