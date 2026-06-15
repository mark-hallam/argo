//! # MF-3 regression — tenant rename must NOT break the audit chain
//!
//! Pre-launch security blocker MF-3: `rename_tenant`
//! (`argo-store/src/tenant_query_inspect.rs`) used to run
//!
//! ```text
//! UPDATE audit_record SET payload_json = replace(payload_json, <old>, <new>)
//! ```
//!
//! WITHOUT recomputing each row's `self_hash`
//! (`= sha256(parent_hash || kind || 0x00 || payload_json)`, see
//! `argo_audit::compute_chain_hash` + `argo-audit/src/verify.rs`).
//! Because the audit sink stamps `payload_json.tenant_id = <tenant>`
//! on every row, a rename rewrote that string in every audit payload
//! and SILENTLY invalidated the tamper-evident hash chain — `argo
//! audit verify` then failed on every touched row, which is the exact
//! failure mode the ledger exists to detect.
//!
//! The fix makes `rename_tenant` REFUSE to touch `audit_record`:
//! the immutable hash chain is left exactly as written, so
//! verification stays clean after a rename. (Re-chaining in place is
//! also impossible from `argo-store`, which sits below `argo-audit`
//! in the crate graph — the canonical `chain_hash` is unreachable
//! there.)
//!
//! ## What this test proves
//!
//! 1. Seed a clean chain of audit rows under the OLD tenant id (the
//!    sink stamps `payload_json.tenant_id = <old>` into each row).
//! 2. Confirm `verify_all` is OK before the rename (rules out a
//!    plumbing failure).
//! 3. Rename the tenant old → new (with the bug, this rewrote every
//!    audit payload and broke the chain).
//! 4. Assert `verify_all` is STILL OK with **breaks: 0** afterwards.
//!
//! Before the fix, step 4 fails (`breaks` is non-empty). After the
//! fix it passes.

use std::sync::Arc;

use argo_audit::{verify_all, AuditKind, AuditSink};
use argo_store::tenant_query::rename_tenant;
use argo_store::SessionStore;
use argo_types::{SessionId, TenantId, WorkspaceId};
use serde_json::json;
use uuid::Uuid;

/// Seed enough rows that the chain genuinely links (each row's
/// `parent_hash` must equal the prior row's `self_hash`), so a
/// payload rewrite breaks the chain in multiple places.
const SEED_ROWS: usize = 5;

/// Insert a `workspace` + `session` row so `rename_tenant`'s
/// `tenant_exists(old)` gate (which scans the tenant-scoped tables)
/// returns true and the rename is accepted. The workspace is inserted
/// first because `session.workspace_id` is a foreign key onto it.
async fn seed_session(store: &SessionStore, tenant: TenantId, session_id: SessionId) {
    let workspace = WorkspaceId::new();
    sqlx::query(
        "INSERT INTO workspace (id, tenant_id, name, root_path, created_at) \
         VALUES (?, ?, 'mf3-regression', '.', 1)",
    )
    .bind(workspace.0.as_bytes().as_slice())
    .bind(tenant.0.as_bytes().as_slice())
    .execute(store.pool())
    .await
    .expect("seed workspace");
    sqlx::query(
        "INSERT INTO session (id, tenant_id, workspace_id, model, provider, started_at) \
         VALUES (?, ?, ?, 'mock', 'mock', 1)",
    )
    .bind(session_id.0.as_bytes().as_slice())
    .bind(tenant.0.as_bytes().as_slice())
    .bind(workspace.0.as_bytes().as_slice())
    .execute(store.pool())
    .await
    .expect("seed session");
}

#[tokio::test]
async fn tenant_rename_leaves_audit_chain_verifiable() {
    let store = Arc::new(SessionStore::open(":memory:").await.expect("open"));
    store.migrate().await.expect("migrate");

    let old = TenantId::new();
    let new = TenantId::new();
    let session_id = SessionId(Uuid::now_v7());
    seed_session(&store, old, session_id).await;

    // Sink bound to the OLD tenant id → every row's payload_json gets
    // `tenant_id: <old>` stamped in, so the buggy `replace(<old>,<new>)`
    // would have rewritten all of them.
    let sink = AuditSink::new_with_tenant(Arc::clone(&store), old.to_string());
    for i in 0..SEED_ROWS {
        sink.write(
            Some(session_id),
            AuditKind::ToolCall,
            json!({ "i": i, "tool": "echo", "marker": format!("mf3-row-{i}") }),
            Some("system:mf3-regression".into()),
        )
        .await
        .expect("audit write");
    }
    sink.flush_pending().await.expect("flush");

    // Phase 2: clean chain before the rename.
    let before = verify_all(store.as_ref()).await.expect("verify before");
    assert!(
        before.ok && before.breaks.is_empty(),
        "MF-3: chain must be clean BEFORE rename (test plumbing check); breaks={:?}",
        before.breaks
    );
    assert_eq!(before.rows, SEED_ROWS, "MF-3: expected {SEED_ROWS} seeded rows");

    // Phase 3: rename old → new. With the bug this rewrote every audit
    // payload_json (replacing the old tenant-id string with the new one)
    // without re-signing, breaking the hash chain.
    rename_tenant(store.as_ref(), old, new, 1)
        .await
        .expect("rename_tenant");

    // Phase 4: the audit chain MUST still verify cleanly. This is the
    // assertion that fails before the fix and passes after.
    let after = verify_all(store.as_ref()).await.expect("verify after");
    assert!(
        after.ok,
        "MF-3: audit chain must still verify after a tenant rename; got breaks={:?}",
        after.breaks
    );
    assert!(
        after.breaks.is_empty(),
        "MF-3: tenant rename must leave breaks:0 in the audit chain; got {:?}",
        after.breaks
    );
    assert_eq!(
        after.rows, SEED_ROWS,
        "MF-3: rename must not drop or add audit rows"
    );
}
