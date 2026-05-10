//! Integration smoke for `memory.query_scored` — seed 5 indexed
//! rows in a worker namespace, dispatch the action through the
//! public `Tool` trait, assert the hit envelope is ranked by
//! score and capped at the caller's `limit`.
//!
//! Brief: `mission-memory-query-scored-action`.

use std::sync::Arc;

use argo_identity::{MemoryNs, MemoryScope, WorkerId};
use argo_memory::{MemoryFile, MemoryLayer, MemoryStore};
use argo_store::SessionStore;
use argo_tool::{Memory, Tool, ToolContext};
use argo_types::{ContentBlock, Principal, TenantId};

#[tokio::test]
async fn query_scored_ranks_and_caps_via_tool_surface() {
    let session = SessionStore::open(":memory:").await.expect("open");
    session.migrate().await.expect("migrate");
    let (_tenant, ws) = session
        .ensure_workspace("ws", "/tmp/ws")
        .await
        .expect("ensure_workspace");
    let store = Arc::new(MemoryStore::new(Arc::new(session)));

    let ns = MemoryNs::new(TenantId::new(), WorkerId::new(), MemoryScope::WorkerSelf);

    // Five distinct rows with varying token overlap against the
    // query "alpha". The first row repeats "alpha" most often and
    // must score highest.
    for body in [
        "alpha alpha alpha alpha",
        "alpha beta gamma",
        "delta epsilon",
        "alpha zeta",
        "phi chi psi",
    ] {
        store
            .append_in_namespace_with_layer(
                &ns,
                ws,
                MemoryFile::Memory,
                body.to_string(),
                None,
                MemoryLayer::Indexed,
            )
            .await
            .expect("seed");
    }

    let tool = Memory::with_namespace(Arc::clone(&store), ws, None, ns);
    let ctx = ToolContext::new(
        Principal::Local {
            os_user: "smoke".into(),
        },
        std::path::PathBuf::from("/tmp"),
    );

    let result = tool
        .dispatch(
            serde_json::json!({
                "action": "query_scored",
                "query": "alpha",
                "limit": 3,
            }),
            &ctx,
        )
        .await
        .expect("dispatch ok");
    assert!(!result.is_error, "expected success, got {:?}", result.content);

    let body = result
        .content
        .iter()
        .find_map(|b| match b {
            ContentBlock::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .expect("text block");
    let hits: Vec<serde_json::Value> = serde_json::from_str(body).expect("json envelope");

    assert_eq!(hits.len(), 3, "limit honoured: {hits:?}");
    assert_eq!(
        hits[0]["snippet"].as_str().unwrap(),
        "alpha alpha alpha alpha",
        "top hit must be the most alpha-dense row",
    );
    // Scores must be monotonically non-increasing.
    let s0 = hits[0]["score"].as_f64().unwrap();
    let s1 = hits[1]["score"].as_f64().unwrap();
    let s2 = hits[2]["score"].as_f64().unwrap();
    assert!(s0 >= s1, "rank: {s0} >= {s1}");
    assert!(s1 >= s2, "rank: {s1} >= {s2}");

    // Each hit must carry the envelope keys (entry_id, snippet,
    // score, namespace, written_at).
    for hit in &hits {
        assert!(hit.get("entry_id").is_some(), "entry_id key: {hit}");
        assert!(hit.get("snippet").is_some(), "snippet key: {hit}");
        assert!(hit.get("score").is_some(), "score key: {hit}");
        assert!(hit.get("namespace").is_some(), "namespace key: {hit}");
        assert!(hit.get("written_at").is_some(), "written_at key: {hit}");
    }
}
