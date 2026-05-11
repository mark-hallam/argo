//! Integration smoke for the memory tool's shared-team actions:
//! seed two workers in the SAME tenant (`shared_team` is tenant-
//! scoped), have worker A `write_team` a note, then have worker B
//! `list_team` + `read_team` and assert visibility across workers.
//!
//! Brief: `mission-memory-tool-namespace-action`.

use std::sync::Arc;

use argo_identity::{MemoryNs, MemoryScope, WorkerId};
use argo_memory::MemoryStore;
use argo_store::SessionStore;
use argo_tool::{Memory, Tool, ToolContext};
use argo_types::{ContentBlock, Principal, TenantId};

fn text_block(result: &argo_types::ToolCallResult) -> String {
    result
        .content
        .iter()
        .find_map(|b| match b {
            ContentBlock::Text { text } => Some(text.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

fn ctx() -> ToolContext {
    ToolContext::new(
        Principal::Local {
            os_user: "smoke".into(),
        },
        std::path::PathBuf::from("/tmp"),
    )
}

#[tokio::test]
async fn shared_team_write_visible_to_sibling_worker() {
    let session = SessionStore::open(":memory:").await.expect("open");
    session.migrate().await.expect("migrate");
    let (_tenant, ws) = session
        .ensure_workspace("ws", "/tmp/ws")
        .await
        .expect("ensure_workspace");
    let store = Arc::new(MemoryStore::new(Arc::new(session)));

    let tenant = TenantId::new();
    let worker_a = WorkerId::new();
    let worker_b = WorkerId::new();
    let ns_a = MemoryNs::new(tenant, worker_a, MemoryScope::SharedTeam);
    let ns_b = MemoryNs::new(tenant, worker_b, MemoryScope::SharedTeam);

    let tool_a = Memory::with_namespace(Arc::clone(&store), ws, None, ns_a);
    let tool_b = Memory::with_namespace(Arc::clone(&store), ws, None, ns_b);

    // Worker A writes a shared note.
    let w = tool_a
        .dispatch(
            serde_json::json!({"action": "write_team", "content": "handoff from A"}),
            &ctx(),
        )
        .await
        .expect("write_team");
    assert!(!w.is_error, "write_team: {:?}", w.content);
    let written = text_block(&w);
    let entry_id = written
        .split("id=")
        .nth(1)
        .and_then(|s| s.split(',').next())
        .expect("id= in write_team output")
        .to_string();

    // Worker B lists and sees A's row.
    let l = tool_b
        .dispatch(serde_json::json!({"action": "list_team"}), &ctx())
        .await
        .expect("list_team");
    let listed = text_block(&l);
    assert!(listed.contains(&entry_id), "B's list_team must see A's row:\n{listed}");
    assert!(
        listed.contains(&worker_a.0.to_string()),
        "author worker_id stamp must appear:\n{listed}",
    );

    // Worker B reads A's note by id.
    let r = tool_b
        .dispatch(
            serde_json::json!({"action": "read_team", "entry": entry_id}),
            &ctx(),
        )
        .await
        .expect("read_team");
    assert_eq!(text_block(&r), "handoff from A");
}
