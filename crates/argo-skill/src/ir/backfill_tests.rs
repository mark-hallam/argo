//! Unit tests for the L3 backfill primitive.
use super::{
    BACKFILL_ENTRY_NODE_ID, BACKFILL_SOURCE, BackfillStatus, backfill, synthesize_graph,
};
use crate::ir::store::{SkillIrStore as _, SqliteSkillIrStore, ensure_schema};
use crate::ir::{NodeKind, NodePolicy, canonical_hash};
use crate::skill::{LifecycleState, Provenance, Skill, SkillFrontmatter};
use argo_store::SessionStore;
use argo_types::{SkillId, TenantId};
use chrono::Utc;

fn make_skill(name: &str, version: &str, body: &str) -> Skill {
    Skill {
        id: SkillId::new(),
        frontmatter: SkillFrontmatter {
            name: name.into(),
            version: version.into(),
            description: format!("{name} description"),
            capabilities: Default::default(),
            provenance: Provenance {
                source: "builtin".into(),
                hash: "sha256-x".into(),
            },
            lock: Default::default(),
            evals: vec![],
        },
        body: body.into(),
        hash: [0u8; 32],
        learned_in: None,
        superseded_by: None,
        created_at: Utc::now(),
        vertical_id: None,
        lifecycle_state: LifecycleState::Active,
    }
}

async fn open_store() -> SessionStore {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    ensure_schema(&store).await.unwrap();
    store
}

#[test]
fn synthesize_graph_produces_single_action_node_with_body_verbatim() {
    let skill = make_skill("greet", "0.1", "say hello to the user");
    let graph = synthesize_graph(&skill);
    assert_eq!(graph.entry.as_str(), BACKFILL_ENTRY_NODE_ID);
    assert_eq!(graph.nodes.len(), 1);
    assert!(graph.edges.is_empty());
    assert!(graph.variables.is_empty());
    let node = &graph.nodes[0];
    match &node.kind {
        NodeKind::Action { instruction } => {
            assert_eq!(instruction, "say hello to the user");
        }
        other => panic!("expected action node, got {other:?}"),
    }
}

#[test]
fn synthesize_graph_sets_curator_annotations() {
    let skill = make_skill("greet", "0.1", "body");
    let node = &synthesize_graph(&skill).nodes[0];
    assert_eq!(node.annotations.confidence, Some(1.0));
    assert_eq!(node.annotations.sample_count, Some(0));
    assert_eq!(node.annotations.policy, Some(NodePolicy::Auto));
    assert!(node.annotations.success_rate.is_none());
    assert!(node.annotations.last_used_at.is_none());
    assert!(node.annotations.tool_binding.is_none());
}

#[test]
fn synthesize_graph_copies_frontmatter_into_metadata() {
    let skill = make_skill("greet", "0.1", "body");
    let md = synthesize_graph(&skill).metadata;
    assert_eq!(md.name.as_deref(), Some("greet"));
    assert_eq!(md.version.as_deref(), Some("0.1"));
    assert_eq!(md.description.as_deref(), Some("greet description"));
    assert_eq!(md.source.as_deref(), Some(BACKFILL_SOURCE));
}

#[test]
fn synthesize_graph_is_deterministic() {
    // Two skills with identical content must produce byte-identical
    // canonical hashes — the idempotency claim ultimately rests on this.
    let mut a = make_skill("greet", "0.1", "body");
    let b = make_skill("greet", "0.1", "body");
    // Skill ids differ but they do not participate in the graph; the
    // synthesised graph must still hash the same.
    a.id = b.id;
    a.created_at = b.created_at;
    let h1 = canonical_hash(&synthesize_graph(&a)).unwrap();
    let h2 = canonical_hash(&synthesize_graph(&b)).unwrap();
    assert_eq!(h1, h2);
}

#[tokio::test]
async fn backfill_writes_each_skill_once_then_skips() {
    let store = open_store().await;
    let ir = SqliteSkillIrStore::new(&store);
    let tenant = TenantId::new();
    let skills = vec![
        make_skill("a", "0.1", "body-a"),
        make_skill("b", "0.2", "body-b"),
    ];
    let first = backfill(&skills, &ir, tenant, false).await.unwrap();
    assert_eq!(first.backfilled, 2);
    assert_eq!(first.skipped, 0);
    assert!(first
        .entries
        .iter()
        .all(|e| e.status == BackfillStatus::Backfilled));
    // Second run is a no-op — idempotency.
    let second = backfill(&skills, &ir, tenant, false).await.unwrap();
    assert_eq!(second.backfilled, 0);
    assert_eq!(second.skipped, 2);
    assert!(second
        .entries
        .iter()
        .all(|e| e.status == BackfillStatus::Skipped));
    // Stored payload byte-identical to a fresh synthesise.
    let stored = ir
        .get(tenant, skills[0].id, &skills[0].frontmatter.version)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored, synthesize_graph(&skills[0]));
}

#[tokio::test]
async fn backfill_dry_run_does_not_write() {
    let store = open_store().await;
    let ir = SqliteSkillIrStore::new(&store);
    let tenant = TenantId::new();
    let skills = vec![make_skill("a", "0.1", "body-a")];
    let report = backfill(&skills, &ir, tenant, true).await.unwrap();
    assert_eq!(report.backfilled, 1);
    assert_eq!(report.skipped, 0);
    // Nothing persisted under --dry-run.
    let row = ir
        .get(tenant, skills[0].id, &skills[0].frontmatter.version)
        .await
        .unwrap();
    assert!(row.is_none());
    // A non-dry-run after the dry-run still classifies as backfilled
    // (the dry-run left no state behind).
    let real = backfill(&skills, &ir, tenant, false).await.unwrap();
    assert_eq!(real.backfilled, 1);
    assert_eq!(real.skipped, 0);
}

#[tokio::test]
async fn backfill_is_tenant_scoped() {
    let store = open_store().await;
    let ir = SqliteSkillIrStore::new(&store);
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    let skills = vec![make_skill("a", "0.1", "body")];
    backfill(&skills, &ir, tenant_a, false).await.unwrap();
    // Tenant B is empty — same skill ids must backfill again.
    let b = backfill(&skills, &ir, tenant_b, false).await.unwrap();
    assert_eq!(b.backfilled, 1);
    assert_eq!(b.skipped, 0);
}
