use super::{save_edited_graph, validate_editable_topology};
use crate::ir::{
    Edge, EdgeCond, EdgeId, Node, NodeAnnotations, NodeId, NodeKind, SkillGraph, SkillIrStore as _,
    SqliteSkillIrStore,
};
use argo_store::SessionStore;
use argo_types::{SkillId, TenantId};
use sqlx::Row;

async fn store() -> SessionStore {
    let store = SessionStore::open(":memory:")
        .await
        .expect("open memory store");
    store.migrate().await.expect("migrate");
    crate::ir::ensure_schema(&store).await.expect("ir schema");
    crate::ab_test::ensure_ab_schema(&store)
        .await
        .expect("ab schema");
    store
}

fn node(id: &str) -> Node {
    Node {
        id: NodeId::from(id),
        label: id.to_string(),
        kind: NodeKind::Action {
            instruction: format!("do {id}"),
        },
        annotations: NodeAnnotations::default(),
    }
}

fn graph(nodes: Vec<Node>, edges: Vec<Edge>) -> SkillGraph {
    SkillGraph {
        entry: NodeId::from("a"),
        nodes,
        edges,
        variables: vec![],
        metadata: Default::default(),
    }
}

fn edge(id: &str, from: &str, to: &str) -> Edge {
    Edge {
        id: EdgeId::from(id),
        from: NodeId::from(from),
        to: NodeId::from(to),
        cond: EdgeCond::Always,
    }
}

#[test]
fn topology_rejects_orphaned_nodes() {
    let g = graph(
        vec![node("a"), node("b"), node("orphan")],
        vec![edge("ab", "a", "b")],
    );
    let err = validate_editable_topology(&g).unwrap_err();
    assert!(err.contains("orphaned"));
}

#[test]
fn topology_rejects_graph_without_terminal() {
    let g = graph(
        vec![node("a"), node("b")],
        vec![edge("ab", "a", "b"), edge("ba", "b", "a")],
    );
    let err = validate_editable_topology(&g).unwrap_err();
    assert!(err.contains("terminal"));
}

#[test]
fn topology_rejects_unknown_edge_endpoints() {
    let g = graph(vec![node("a")], vec![edge("missing", "a", "b")]);
    let err = validate_editable_topology(&g).unwrap_err();
    assert!(err.contains("unknown destination"));
}

#[test]
fn topology_treats_loop_and_try_bodies_as_reachable_edges() {
    let mut loop_node = node("b");
    loop_node.kind = NodeKind::Loop {
        iter_var: "item".into(),
        body: vec![NodeId::from("c")],
    };
    let mut try_node = node("d");
    try_node.kind = NodeKind::TryCatch {
        try_body: vec![NodeId::from("e")],
        catch_body: vec![NodeId::from("c")],
    };
    let g = graph(
        vec![node("a"), loop_node, node("c"), try_node, node("e")],
        vec![edge("ab", "a", "b"), edge("bd", "b", "d")],
    );
    validate_editable_topology(&g).expect("structural body nodes are reachable");
}

#[tokio::test]
async fn save_persists_first_version_without_ab_run() {
    let store = store().await;
    let tenant = TenantId::new();
    let skill = SkillId::new();
    let g = graph(vec![node("a"), node("b")], vec![edge("ab", "a", "b")]);

    let out = save_edited_graph(&store, tenant, skill, &g)
        .await
        .expect("save");
    assert!(out.version.starts_with("canvas-"));
    assert!(out.ab_test_run_id.is_none());

    let ir = SqliteSkillIrStore::new(&store);
    assert!(ir.get(tenant, skill, &out.version).await.unwrap().is_some());
}

#[tokio::test]
async fn save_second_version_schedules_ab_run() {
    let store = store().await;
    let tenant = TenantId::new();
    let skill = SkillId::new();
    let g = graph(vec![node("a"), node("b")], vec![edge("ab", "a", "b")]);

    let first = save_edited_graph(&store, tenant, skill, &g)
        .await
        .expect("first");
    tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    let second = save_edited_graph(&store, tenant, skill, &g)
        .await
        .expect("second");
    let run_id = second.ab_test_run_id.expect("ab run");

    let row = sqlx::query(
        "SELECT skill_name, variant_a, variant_b, status, tenant_id \
         FROM skill_ab_test_run WHERE id = ? AND tenant_id = ?",
    )
    .bind(&run_id)
    .bind(tenant.to_string())
    .fetch_one(store.pool())
    .await
    .expect("ab row");
    assert_eq!(row.get::<String, _>("skill_name"), skill.to_string());
    assert_eq!(row.get::<String, _>("variant_a"), first.version);
    assert_eq!(row.get::<String, _>("variant_b"), second.version);
    assert_eq!(row.get::<String, _>("status"), "running");
    assert_eq!(row.get::<String, _>("tenant_id"), tenant.to_string());
}
