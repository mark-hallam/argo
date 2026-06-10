//! Legacy-action tests for the wired `Memory` tool — capability
//! requirements, entry resolution, and `write`/`read`/`list`
//! dispatch round-trips. Namespace-bound action tests
//! (`query_scored`, `*_team`) live in `memory_namespace_tests.rs`.

use super::test_helpers::*;
use super::*;
use argo_memory::PrincipalOverride;
use argo_types::Lock;

// ───────────────── back-compat ─────────────────

#[tokio::test]
async fn default_constructor_still_returns_not_wired_error() {
    let r = Memory::default()
        .dispatch(serde_json::json!({"action": "list"}), &ctx())
        .await
        .unwrap();
    assert!(r.is_error);
    let body = first_text(&r);
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
    assert_eq!(
        resolve_entry(Some("MEMORY.md")).unwrap(),
        MemoryFile::Memory
    );
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
async fn vertical_bound_read_excludes_other_verticals_rows() {
    // 3b regression pin (tool layer): a customer-support-bound tool must
    // not read a gtm-engineer row in the same workspace — the observed
    // 2026-06-10 leak. Pre-fix `list_active` returned all verticals' rows.
    let (store, ws) = fresh_store().await;
    store
        .append_for_vertical(
            ws,
            MemoryFile::Memory,
            "GTM-OUTREACH-DRAFT".to_string(),
            None,
            "gtm-engineer",
        )
        .await
        .unwrap();
    let tool = Memory::new(store.clone(), ws, Some("customer-support".to_string()));
    let list = tool
        .dispatch(serde_json::json!({"action": "list"}), &ctx())
        .await
        .unwrap();
    assert!(
        first_text(&list).contains("no memory entries"),
        "got: {}",
        first_text(&list)
    );
    let err = tool
        .dispatch(
            serde_json::json!({"action": "read", "entry": "MEMORY.md"}),
            &ctx(),
        )
        .await
        .expect_err("must not see gtm-engineer row");
    assert!(err.to_string().contains("no entries"), "got: {err}");
}

#[tokio::test]
async fn vertical_bound_write_does_not_supersede_other_verticals_row() {
    // 3b regression pin: writing customer-support's MEMORY.md must CREATE
    // (append for its vertical), never supersede gtm-engineer's same-named
    // row. Pre-fix the supersede check used `list_active`, so the write
    // could overwrite another vertical's entry (worse than a read leak).
    let (store, ws) = fresh_store().await;
    store
        .append_for_vertical(
            ws,
            MemoryFile::Memory,
            "GTM-OUTREACH-DRAFT".to_string(),
            None,
            "gtm-engineer",
        )
        .await
        .unwrap();
    let tool = Memory::new(store.clone(), ws, Some("customer-support".to_string()));
    tool.dispatch(
        serde_json::json!({"action": "write", "entry": "MEMORY.md", "content": "CS-NOTE"}),
        &ctx(),
    )
    .await
    .unwrap();
    let gtm = store
        .list_for_vertical(ws, MemoryFile::Memory, "gtm-engineer")
        .await
        .unwrap();
    assert_eq!(gtm.len(), 1, "gtm row must survive");
    assert_eq!(gtm[0].body, "GTM-OUTREACH-DRAFT");
    assert!(
        gtm[0].superseded_by.is_none(),
        "gtm row must not be superseded"
    );
    let cs = store
        .list_for_vertical(ws, MemoryFile::Memory, "customer-support")
        .await
        .unwrap();
    assert_eq!(cs.len(), 1);
    assert_eq!(cs[0].body, "CS-NOTE");
}

#[tokio::test]
async fn legacy_unbound_read_still_sees_workspace_global_rows() {
    // Criterion 3: a tool with no bound vertical (CLI without --vertical)
    // keeps the workspace-global `list_active` behaviour for back-compat.
    let (store, ws) = fresh_store().await;
    store
        .append(ws, MemoryFile::Memory, "GLOBAL-ROW".to_string(), None)
        .await
        .unwrap();
    let tool = Memory::new(store.clone(), ws, None);
    let r = tool
        .dispatch(
            serde_json::json!({"action": "read", "entry": "MEMORY.md"}),
            &ctx(),
        )
        .await
        .unwrap();
    assert_eq!(first_text(&r), "GLOBAL-ROW");
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
