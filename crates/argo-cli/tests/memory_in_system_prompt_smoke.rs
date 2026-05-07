//! mission-wire-memory-compose-context (Phase B) — integration smoke.
//!
//! Verifies the public-facing surface of the memory-into-system-prompt
//! wire: writing a workspace-shared memory row via the live store +
//! reading it back via `MemoryNs::compose_context` produces the
//! markdown block that `build_system_prompt` would inject.
//!
//! The full agent-loop wire (skill prompt + memory block ordering) is
//! covered by `argo-agent-runner::turn_tests` — this smoke is the
//! integration-layer sibling that proves the CLI-visible store +
//! identity APIs compose.

use argo_identity::{MemoryNs, MemoryScope, WorkerId};
use argo_memory::MemoryStore;
use argo_store::SessionStore;
use argo_types::TenantId;
use std::sync::Arc;

#[tokio::test]
async fn shared_team_rows_render_into_memory_compose_block() {
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let memory = MemoryStore::new(Arc::clone(&store));

    let tenant = TenantId::new();
    let ns = argo_memory::shared_team(tenant);

    memory
        .write_shared_team(&ns, WorkerId::new(), "monday standup notes".into())
        .await
        .unwrap();
    memory
        .write_shared_team(&ns, WorkerId::new(), "follow-up with operator".into())
        .await
        .unwrap();
    memory
        .write_shared_team(&ns, WorkerId::new(), "tuesday handoff".into())
        .await
        .unwrap();

    let entries = memory.read_shared_team(&ns).await.unwrap();
    assert_eq!(entries.len(), 3, "three rows persisted");

    let rows: Vec<_> = entries
        .into_iter()
        .map(|e| (e.written_at, e.content))
        .collect();

    let block = MemoryNs::new(tenant, WorkerId::nil(), MemoryScope::SharedTeam).compose_context(&rows);
    assert!(
        block.starts_with("## Memory (workspace-shared, last 3)\n"),
        "expected memory header; got:\n{block}"
    );
    assert!(block.contains("monday standup notes"), "got:\n{block}");
    assert!(block.contains("follow-up with operator"), "got:\n{block}");
    assert!(block.contains("tuesday handoff"), "got:\n{block}");
}

#[tokio::test]
async fn empty_shared_team_yields_no_memory_block() {
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let memory = MemoryStore::new(Arc::clone(&store));
    let tenant = TenantId::new();

    let entries = memory
        .read_shared_team(&argo_memory::shared_team(tenant))
        .await
        .unwrap();
    let rows: Vec<_> = entries
        .into_iter()
        .map(|e| (e.written_at, e.content))
        .collect();
    let block = MemoryNs::new(tenant, WorkerId::nil(), MemoryScope::SharedTeam).compose_context(&rows);
    assert!(
        block.is_empty(),
        "expected empty block for no rows; got: {block}"
    );
}
