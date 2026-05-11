//! Unit tests for the wired `Memory` tool.
//!
//! Each test stages an in-memory `SessionStore`, registers a
//! workspace so `MemoryStore::append` has a row to anchor to, and
//! drives `Memory::dispatch` via the public `Tool` trait. Splits
//! out of `memory.rs` keep that file under the 500-LOC cap.

use super::*;
use argo_identity::{MemoryNs, MemoryScope, WorkerId};
use argo_memory::{MemoryLayer, MemoryStore};
use argo_store::SessionStore;
use argo_types::{Lock, Principal, TenantId};
use uuid::Uuid;

async fn fresh_store() -> (Arc<MemoryStore>, WorkspaceId) {
    let store = SessionStore::open(":memory:").await.expect("open");
    store.migrate().await.expect("migrate");
    let (_tenant, workspace) = store
        .ensure_workspace("ws", "/tmp/ws")
        .await
        .expect("ensure_workspace");
    let memory = Arc::new(MemoryStore::new(Arc::new(store)));
    (memory, workspace)
}

fn ctx() -> ToolContext {
    ToolContext::new(
        Principal::Local {
            os_user: "u".into(),
        },
        std::path::PathBuf::from("/tmp"),
    )
}

// ───────────────── back-compat ─────────────────

#[tokio::test]
async fn default_constructor_still_returns_not_wired_error() {
    let r = Memory::default()
        .dispatch(serde_json::json!({"action": "list"}), &ctx())
        .await
        .unwrap();
    assert!(r.is_error);
    let body = r
        .content
        .iter()
        .filter_map(|b| match b {
            argo_types::ContentBlock::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .next()
        .unwrap_or("");
    assert!(body.contains("memory store not wired"), "got: {body}");
}

#[test]
fn write_action_requires_memory_write_cap() {
    let caps = Memory::default()
        .required_capabilities(
            &serde_json::json!({"action": "write", "entry": "memory", "content": "c"}),
        )
        .unwrap();
    assert!(caps.covers(&Capability::MemoryWrite));
}

#[test]
fn read_action_does_not_require_memory_write_cap() {
    let caps = Memory::default()
        .required_capabilities(&serde_json::json!({"action": "read", "entry": "memory"}))
        .unwrap();
    assert!(!caps.covers(&Capability::MemoryWrite));
}

// ───────────────── entry resolution ─────────────────

#[test]
fn resolve_entry_accepts_canonical_filename() {
    assert_eq!(resolve_entry(Some("MEMORY.md")).unwrap(), MemoryFile::Memory);
    assert_eq!(resolve_entry(Some("USER.md")).unwrap(), MemoryFile::User);
}

#[test]
fn resolve_entry_accepts_bare_lowercase_form() {
    assert_eq!(resolve_entry(Some("memory")).unwrap(), MemoryFile::Memory);
    assert_eq!(resolve_entry(Some("USER")).unwrap(), MemoryFile::User);
}

#[test]
fn resolve_entry_rejects_unknown_token() {
    let err = resolve_entry(Some("RANDOM")).expect_err("must reject");
    assert!(err.to_string().contains("RANDOM"), "got: {err}");
}

// ───────────────── wired dispatch ─────────────────

#[tokio::test]
async fn write_then_read_round_trips_through_dispatch() {
    let (store, ws) = fresh_store().await;
    let tool = Memory::new(store, ws, None);
    // Write
    let w = tool
        .dispatch(
            serde_json::json!({"action": "write", "entry": "MEMORY.md", "content": "hello"}),
            &ctx(),
        )
        .await
        .unwrap();
    assert!(!w.is_error, "write must succeed");
    // Read
    let r = tool
        .dispatch(
            serde_json::json!({"action": "read", "entry": "MEMORY.md"}),
            &ctx(),
        )
        .await
        .unwrap();
    let body = first_text(&r);
    assert_eq!(body, "hello");
}

#[tokio::test]
async fn list_action_reports_per_file_byte_counts() {
    let (store, ws) = fresh_store().await;
    let tool = Memory::new(store.clone(), ws, None);
    tool.dispatch(
        serde_json::json!({"action": "write", "entry": "MEMORY.md", "content": "abc"}),
        &ctx(),
    )
    .await
    .unwrap();
    tool.dispatch(
        serde_json::json!({"action": "write", "entry": "USER.md", "content": "user-bio"}),
        &ctx(),
    )
    .await
    .unwrap();
    let r = tool
        .dispatch(serde_json::json!({"action": "list"}), &ctx())
        .await
        .unwrap();
    let body = first_text(&r);
    assert!(body.contains("MEMORY.md"), "got: {body}");
    assert!(body.contains("USER.md"), "got: {body}");
    assert!(body.contains("3 bytes"));
    assert!(body.contains("8 bytes"));
}

#[tokio::test]
async fn vertical_bound_write_persists_with_vertical_id() {
    let (store, ws) = fresh_store().await;
    let vertical = "marketplace-seller";
    let tool = Memory::new(store.clone(), ws, Some(vertical.to_string()));
    tool.dispatch(
        serde_json::json!({"action": "write", "entry": "MEMORY.md", "content": "scoped"}),
        &ctx(),
    )
    .await
    .unwrap();
    let entries = store
        .list_for_vertical(ws, MemoryFile::Memory, vertical)
        .await
        .unwrap();
    assert_eq!(entries.len(), 1, "vertical-scoped read must see the row");
    assert_eq!(entries[0].vertical_id.as_deref(), Some(vertical));
    assert_eq!(entries[0].body, "scoped");
}

#[tokio::test]
async fn unbound_write_persists_with_null_vertical_id() {
    let (store, ws) = fresh_store().await;
    let tool = Memory::new(store.clone(), ws, None);
    tool.dispatch(
        serde_json::json!({"action": "write", "entry": "MEMORY.md", "content": "global"}),
        &ctx(),
    )
    .await
    .unwrap();
    let entries = store.list_active(ws, MemoryFile::Memory).await.unwrap();
    assert_eq!(entries.len(), 1);
    assert!(entries[0].vertical_id.is_none(), "vertical must be NULL");
}

#[tokio::test]
async fn write_against_user_protected_entry_rejects_with_force_hint() {
    let (store, ws) = fresh_store().await;
    let tool = Memory::new(store.clone(), ws, None);
    // Seed an entry, then lock it.
    tool.dispatch(
        serde_json::json!({"action": "write", "entry": "USER.md", "content": "v1"}),
        &ctx(),
    )
    .await
    .unwrap();
    let head = store.list_active(ws, MemoryFile::User).await.unwrap()[0].id;
    store
        .set_lock(head, Lock::UserProtected, PrincipalOverride::Force)
        .await
        .unwrap();
    // Agent retry must fail with the --force hint.
    let r = tool
        .dispatch(
            serde_json::json!({"action": "write", "entry": "USER.md", "content": "v2"}),
            &ctx(),
        )
        .await;
    let err = r.expect_err("must reject");
    let msg = err.to_string();
    assert!(msg.contains("user-protected"), "got: {msg}");
    assert!(msg.contains("--force"), "got: {msg}");
}

#[tokio::test]
async fn write_without_content_returns_clear_argument_error() {
    let (store, ws) = fresh_store().await;
    let tool = Memory::new(store, ws, None);
    let err = tool
        .dispatch(
            serde_json::json!({"action": "write", "entry": "MEMORY.md"}),
            &ctx(),
        )
        .await
        .expect_err("must reject");
    assert!(err.to_string().contains("content"), "got: {err}");
}

#[tokio::test]
async fn read_action_errors_when_no_entry_exists() {
    let (store, ws) = fresh_store().await;
    let tool = Memory::new(store, ws, None);
    let err = tool
        .dispatch(
            serde_json::json!({"action": "read", "entry": "MEMORY.md"}),
            &ctx(),
        )
        .await
        .expect_err("must reject");
    assert!(err.to_string().contains("no entries"), "got: {err}");
}

fn first_text(result: &ToolCallResult) -> String {
    result
        .content
        .iter()
        .find_map(|b| match b {
            argo_types::ContentBlock::Text { text } => Some(text.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

// ───────────────── query_scored ─────────────────

fn ns_for() -> MemoryNs {
    MemoryNs::new(TenantId::new(), WorkerId::new(), MemoryScope::WorkerSelf)
}

/// Seed `bodies` into the indexed layer of `ns` so `query_scored`
/// has rows to rank. Indexed-layer rows are persisted via the
/// public `append_in_namespace_with_layer` helper, identical to
/// the path the nightly promoter takes.
async fn seed_indexed(
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

#[tokio::test]
async fn query_scored_default_namespace_returns_hits_ranked_by_score() {
    let (store, ws) = fresh_store().await;
    let ns = ns_for();
    seed_indexed(
        &store,
        &ns,
        ws,
        &[
            "alpha bravo charlie",
            "delta echo foxtrot",
            "alpha alpha alpha",
        ],
    )
    .await;
    let tool = Memory::with_namespace(store, ws, None, ns.clone());
    let r = tool
        .dispatch(
            serde_json::json!({"action": "query_scored", "query": "alpha"}),
            &ctx(),
        )
        .await
        .unwrap();
    assert!(!r.is_error, "got: {:?}", r.content);
    let body = first_text(&r);
    let hits: Vec<serde_json::Value> = serde_json::from_str(&body).expect("json");
    assert_eq!(hits.len(), 3, "all 3 entries match (token overlap)");
    // First hit should be the entry with the most `alpha` tokens.
    assert_eq!(hits[0]["snippet"].as_str().unwrap(), "alpha alpha alpha");
    let score0 = hits[0]["score"].as_f64().unwrap();
    let score1 = hits[1]["score"].as_f64().unwrap();
    assert!(score0 >= score1, "scores must be descending: {score0} {score1}");
}

#[tokio::test]
async fn query_scored_explicit_namespace_defers_to_followup_brief() {
    let (store, ws) = fresh_store().await;
    let tool = Memory::with_namespace(store, ws, None, ns_for());
    let err = tool
        .dispatch(
            serde_json::json!({
                "action": "query_scored",
                "query": "alpha",
                "namespace": "worker:other-id",
            }),
            &ctx(),
        )
        .await
        .expect_err("explicit namespace not yet supported in v1");
    let msg = err.to_string();
    assert!(msg.contains("mission-memory-namespace-addressing"), "got: {msg}");
    assert!(msg.contains("#43"), "got: {msg}");
}

#[tokio::test]
async fn query_scored_respects_limit() {
    let (store, ws) = fresh_store().await;
    let ns = ns_for();
    seed_indexed(
        &store,
        &ns,
        ws,
        &["alpha one", "alpha two", "alpha three", "alpha four", "alpha five"],
    )
    .await;
    let tool = Memory::with_namespace(store, ws, None, ns);
    let r = tool
        .dispatch(
            serde_json::json!({"action": "query_scored", "query": "alpha", "limit": 2}),
            &ctx(),
        )
        .await
        .unwrap();
    let hits: Vec<serde_json::Value> = serde_json::from_str(&first_text(&r)).unwrap();
    assert_eq!(hits.len(), 2);
}

#[tokio::test]
async fn query_scored_clamps_limit_to_20() {
    let (store, ws) = fresh_store().await;
    let ns = ns_for();
    let bodies: Vec<String> = (0..25).map(|i| format!("alpha row {i}")).collect();
    let refs: Vec<&str> = bodies.iter().map(String::as_str).collect();
    seed_indexed(&store, &ns, ws, &refs).await;
    let tool = Memory::with_namespace(store, ws, None, ns);
    let r = tool
        .dispatch(
            serde_json::json!({"action": "query_scored", "query": "alpha", "limit": 100}),
            &ctx(),
        )
        .await
        .unwrap();
    let hits: Vec<serde_json::Value> = serde_json::from_str(&first_text(&r)).unwrap();
    assert_eq!(hits.len(), 20, "limit clamps to 20, never errors");
}

#[tokio::test]
async fn query_scored_zero_limit_returns_empty_vec() {
    let (store, ws) = fresh_store().await;
    let ns = ns_for();
    seed_indexed(&store, &ns, ws, &["alpha", "beta"]).await;
    let tool = Memory::with_namespace(store, ws, None, ns);
    let r = tool
        .dispatch(
            serde_json::json!({"action": "query_scored", "query": "alpha", "limit": 0}),
            &ctx(),
        )
        .await
        .unwrap();
    assert!(!r.is_error, "k=0 is not an error");
    let hits: Vec<serde_json::Value> = serde_json::from_str(&first_text(&r)).unwrap();
    assert!(hits.is_empty(), "k=0 returns empty Vec");
}

#[tokio::test]
async fn query_scored_unwired_errors_with_historical_message() {
    let r = Memory::default()
        .dispatch(
            serde_json::json!({"action": "query_scored", "query": "alpha"}),
            &ctx(),
        )
        .await
        .unwrap();
    assert!(r.is_error);
    let body = first_text(&r);
    assert!(body.contains("memory store not wired"), "got: {body}");
}

#[tokio::test]
async fn query_scored_wired_without_namespace_explains_binding() {
    let (store, ws) = fresh_store().await;
    let tool = Memory::new(store, ws, None);
    let err = tool
        .dispatch(
            serde_json::json!({"action": "query_scored", "query": "alpha"}),
            &ctx(),
        )
        .await
        .expect_err("must reject when no namespace bound");
    let msg = err.to_string();
    assert!(msg.contains("not bound to a worker namespace"), "got: {msg}");
    assert!(msg.contains("Memory::with_namespace"), "got: {msg}");
}

// ───────────────── shared-team actions ─────────────────

#[test]
fn write_team_action_requires_memory_write_team_cap() {
    let caps = Memory::default()
        .required_capabilities(&serde_json::json!({"action": "write_team", "content": "c"}))
        .unwrap();
    assert!(caps.covers(&Capability::MemoryWriteTeam));
    assert!(
        !caps.covers(&Capability::MemoryWrite),
        "write_team must NOT require MemoryWrite (private-namespace cap)",
    );
}

#[test]
fn list_team_and_read_team_actions_require_no_capability() {
    for action in ["list_team", "read_team"] {
        let caps = Memory::default()
            .required_capabilities(&serde_json::json!({
                "action": action, "entry": Uuid::nil().to_string(),
            }))
            .unwrap();
        assert!(
            !caps.covers(&Capability::MemoryWriteTeam),
            "{action} must NOT require write-team cap (reads are default-allowed)",
        );
        assert!(
            !caps.covers(&Capability::MemoryWrite),
            "{action} must NOT require private-namespace write cap",
        );
    }
}

#[tokio::test]
async fn list_team_returns_team_entries_in_chronological_order() {
    let (store, ws) = fresh_store().await;
    let ns = ns_for();
    let tool = Memory::with_namespace(store, ws, None, ns);
    let mut written_ids = Vec::new();
    for body in ["alpha-row", "bravo-row", "charlie-row"] {
        let w = tool
            .dispatch(
                serde_json::json!({"action": "write_team", "content": body}),
                &ctx(),
            )
            .await
            .unwrap();
        let body = first_text(&w);
        let id = body
            .split("id=")
            .nth(1)
            .and_then(|s| s.split(',').next())
            .map(str::to_string)
            .unwrap();
        written_ids.push(id);
    }
    let r = tool
        .dispatch(serde_json::json!({"action": "list_team"}), &ctx())
        .await
        .unwrap();
    assert!(!r.is_error, "got: {:?}", r.content);
    let body = first_text(&r);
    let pos0 = body.find(&written_ids[0]).unwrap();
    let pos1 = body.find(&written_ids[1]).unwrap();
    let pos2 = body.find(&written_ids[2]).unwrap();
    assert!(pos0 < pos1 && pos1 < pos2, "insertion order preserved:\n{body}");
}

#[tokio::test]
async fn read_team_reads_by_entry_id_uuid() {
    let (store, ws) = fresh_store().await;
    let ns = ns_for();
    let tool = Memory::with_namespace(store, ws, None, ns);
    let w = tool
        .dispatch(
            serde_json::json!({"action": "write_team", "content": "shared note"}),
            &ctx(),
        )
        .await
        .unwrap();
    let written = first_text(&w);
    let id = written
        .split("id=")
        .nth(1)
        .and_then(|s| s.split(',').next())
        .unwrap();
    let r = tool
        .dispatch(
            serde_json::json!({"action": "read_team", "entry": id}),
            &ctx(),
        )
        .await
        .unwrap();
    assert_eq!(first_text(&r), "shared note");
}

#[tokio::test]
async fn list_team_defaults_to_worker_bound_tenant() {
    let (store, ws) = fresh_store().await;
    let ns = ns_for();
    let tool = Memory::with_namespace(store.clone(), ws, None, ns.clone());
    tool.dispatch(
        serde_json::json!({"action": "write_team", "content": "x"}),
        &ctx(),
    )
    .await
    .unwrap();
    // No team_id → must resolve to the worker's bound tenant and see the row.
    let r = tool
        .dispatch(serde_json::json!({"action": "list_team"}), &ctx())
        .await
        .unwrap();
    assert!(first_text(&r).contains("bytes=1"), "got: {}", first_text(&r));
    // Explicit matching team_id → same result.
    let r = tool
        .dispatch(
            serde_json::json!({"action": "list_team", "team_id": ns.tenant_id.to_string()}),
            &ctx(),
        )
        .await
        .unwrap();
    assert!(first_text(&r).contains("bytes=1"));
}

#[tokio::test]
async fn read_team_cross_tenant_team_id_is_rejected() {
    let (store, ws) = fresh_store().await;
    let ns = ns_for();
    let tool = Memory::with_namespace(store, ws, None, ns);
    let other_tenant = TenantId::new();
    let err = tool
        .dispatch(
            serde_json::json!({
                "action": "read_team",
                "entry": Uuid::nil().to_string(),
                "team_id": other_tenant.to_string(),
            }),
            &ctx(),
        )
        .await
        .expect_err("cross-tenant team_id must be rejected");
    let msg = err.to_string();
    assert!(msg.contains("cross-team"), "got: {msg}");
}

#[tokio::test]
async fn team_actions_without_namespace_binding_explain_requirement() {
    let (store, ws) = fresh_store().await;
    let tool = Memory::new(store, ws, None);
    let err = tool
        .dispatch(
            serde_json::json!({"action": "list_team"}),
            &ctx(),
        )
        .await
        .expect_err("must require a bound namespace");
    let msg = err.to_string();
    assert!(msg.contains("not bound to a worker namespace"), "got: {msg}");
    assert!(msg.contains("team actions"), "got: {msg}");
}
