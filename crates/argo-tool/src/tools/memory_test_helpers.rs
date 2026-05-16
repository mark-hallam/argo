//! Shared helpers for the `memory_*_tests.rs` siblings.
//!
//! Both `memory_legacy_tests.rs` and `memory_namespace_tests.rs` reach
//! these through `use super::test_helpers::*;`.

#![allow(dead_code)]

use super::*;
use argo_identity::{MemoryNs, MemoryScope, WorkerId};
use argo_memory::{MemoryLayer, MemoryStore};
use argo_store::SessionStore;
use argo_types::{Principal, TenantId};

pub(super) async fn fresh_store() -> (Arc<MemoryStore>, WorkspaceId) {
    let store = SessionStore::open(":memory:").await.expect("open");
    store.migrate().await.expect("migrate");
    let (_tenant, workspace) = store
        .ensure_workspace("ws", "/tmp/ws")
        .await
        .expect("ensure_workspace");
    let memory = Arc::new(MemoryStore::new(Arc::new(store)));
    (memory, workspace)
}

pub(super) fn ctx() -> ToolContext {
    ToolContext::new(
        Principal::Local {
            os_user: "u".into(),
        },
        std::path::PathBuf::from("/tmp"),
    )
}

pub(super) fn first_text(result: &ToolCallResult) -> String {
    result
        .content
        .iter()
        .find_map(|b| match b {
            argo_types::ContentBlock::Text { text } => Some(text.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

pub(super) fn ns_for() -> MemoryNs {
    MemoryNs::new(TenantId::new(), WorkerId::new(), MemoryScope::WorkerSelf)
}

/// Seed `bodies` into the indexed layer of `ns` so `query_scored`
/// has rows to rank. Indexed-layer rows are persisted via the
/// public `append_in_namespace_with_layer` helper, identical to
/// the path the nightly promoter takes.
pub(super) async fn seed_indexed(
    store: &MemoryStore,
    ns: &MemoryNs,
    ws: WorkspaceId,
    bodies: &[&str],
) {
    for body in bodies {
        store
            .append_in_namespace_with_layer(
                ns,
                ws,
                MemoryFile::Memory,
                (*body).to_string(),
                None,
                MemoryLayer::Indexed,
            )
            .await
            .expect("seed");
    }
}
