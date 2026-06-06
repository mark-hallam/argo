//! Unit coverage for the per-action execution-graduation store
//! (UF Phase-B · B3, DEC-037).

use super::*;
use crate::SessionStore;

async fn migrated_pool() -> SessionStore {
    let store = SessionStore::open(":memory:").await.expect("open store");
    store.migrate().await.expect("migrate");
    store
}

fn mcp_grant(tenant: &str, worker: &str, action: &str) -> PromotionGrant {
    PromotionGrant {
        tenant_id: tenant.into(),
        worker_id: worker.into(),
        action_type: action.into(),
        executor_kind: PROMOTABLE_KIND_MCP.into(),
        mcp_server: Some("gmail".into()),
        mcp_tool: Some("drafts.send".into()),
        promoted_by: "principal:owner".into(),
    }
}

#[tokio::test]
async fn promote_then_get_roundtrips() {
    let store = migrated_pool().await;
    let rec = promote(store.pool(), &mcp_grant("tenant-a", "worker-1", "send_email"))
        .await
        .expect("promote");
    assert_eq!(rec.executor_kind, "mcp");
    assert_eq!(rec.mcp_server.as_deref(), Some("gmail"));
    assert_eq!(rec.mcp_tool.as_deref(), Some("drafts.send"));
    assert!(!rec.promoted_at.is_empty());

    let fetched = get(store.pool(), "tenant-a", "worker-1", "send_email")
        .await
        .expect("get")
        .expect("row present");
    assert_eq!(fetched, rec);
}

#[tokio::test]
async fn document_grant_persists_with_null_mcp_binding() {
    // DEC-041 A: an internal-document graduation is side-effect-free and
    // needs NO mcp server/tool — it persists with NULL bindings.
    let store = migrated_pool().await;
    let grant = PromotionGrant {
        tenant_id: "tenant-a".into(),
        worker_id: "worker-1".into(),
        action_type: "summarize_inbox".into(),
        executor_kind: PROMOTABLE_KIND_DOCUMENT.into(),
        mcp_server: None,
        mcp_tool: None,
        promoted_by: "principal:owner".into(),
    };
    let rec = promote(store.pool(), &grant)
        .await
        .expect("document promote succeeds without an mcp binding");
    assert_eq!(rec.executor_kind, "document");
    assert!(rec.mcp_server.is_none());
    assert!(rec.mcp_tool.is_none());

    let fetched = get(store.pool(), "tenant-a", "worker-1", "summarize_inbox")
        .await
        .expect("get")
        .expect("row present");
    assert_eq!(fetched, rec);
}

#[tokio::test]
async fn unknown_executor_kind_is_rejected() {
    // Only mcp + document are promotable; anything else fails closed.
    let store = migrated_pool().await;
    let mut grant = mcp_grant("tenant-a", "worker-1", "x");
    grant.executor_kind = "browser".into();
    assert!(
        promote(store.pool(), &grant).await.is_err(),
        "an un-promotable executor kind must be rejected"
    );
}

#[tokio::test]
async fn absent_promotion_is_none() {
    let store = migrated_pool().await;
    let none = get(store.pool(), "tenant-a", "worker-1", "send_email")
        .await
        .expect("get");
    assert!(none.is_none(), "un-promoted action stays draft-only (None)");
}

#[tokio::test]
async fn re_promote_overwrites_in_place() {
    let store = migrated_pool().await;
    promote(store.pool(), &mcp_grant("tenant-a", "worker-1", "send_email"))
        .await
        .expect("first promote");
    let mut grant = mcp_grant("tenant-a", "worker-1", "send_email");
    grant.mcp_tool = Some("messages.send".into());
    grant.promoted_by = "principal:admin".into();
    promote(store.pool(), &grant).await.expect("re-promote");

    let rows = list_for_worker(store.pool(), "tenant-a", "worker-1")
        .await
        .expect("list");
    assert_eq!(rows.len(), 1, "re-promotion overwrites, not appends");
    assert_eq!(rows[0].mcp_tool.as_deref(), Some("messages.send"));
    assert_eq!(rows[0].promoted_by, "principal:admin");
}

#[tokio::test]
async fn promotion_is_tenant_scoped() {
    let store = migrated_pool().await;
    promote(store.pool(), &mcp_grant("tenant-a", "worker-1", "send_email"))
        .await
        .expect("promote tenant-a");

    // A different tenant asking for the same worker/action sees nothing.
    let cross = get(store.pool(), "tenant-b", "worker-1", "send_email")
        .await
        .expect("get cross-tenant");
    assert!(cross.is_none(), "promotion must not bleed across tenants");

    let cross_list = list_for_worker(store.pool(), "tenant-b", "worker-1")
        .await
        .expect("list cross-tenant");
    assert!(cross_list.is_empty(), "cross-tenant list is empty");
}

#[tokio::test]
async fn revoke_demotes_back_to_draft_only() {
    let store = migrated_pool().await;
    promote(store.pool(), &mcp_grant("tenant-a", "worker-1", "send_email"))
        .await
        .expect("promote");
    let removed = revoke(store.pool(), "tenant-a", "worker-1", "send_email")
        .await
        .expect("revoke");
    assert!(removed, "revoke returns true when a row was removed");
    assert!(
        get(store.pool(), "tenant-a", "worker-1", "send_email")
            .await
            .expect("get")
            .is_none(),
        "action is draft-only again after revoke"
    );
    // Idempotent: revoking an absent grant returns false, not an error.
    let again = revoke(store.pool(), "tenant-a", "worker-1", "send_email")
        .await
        .expect("revoke again");
    assert!(!again, "revoking an absent promotion returns false");
}

#[tokio::test]
async fn non_mcp_kind_is_rejected() {
    let store = migrated_pool().await;
    let mut grant = mcp_grant("tenant-a", "worker-1", "send_email");
    grant.executor_kind = "computer_use".into();
    let err = promote(store.pool(), &grant).await.unwrap_err();
    assert!(
        format!("{err}").contains("are promotable") && format!("{err}").contains("computer_use"),
        "non-mcp kind rejected: {err}"
    );
}

#[tokio::test]
async fn mcp_grant_without_binding_is_rejected() {
    let store = migrated_pool().await;
    let mut grant = mcp_grant("tenant-a", "worker-1", "send_email");
    grant.mcp_server = None;
    let err = promote(store.pool(), &grant).await.unwrap_err();
    assert!(
        format!("{err}").contains("mcp_server is required"),
        "incomplete mcp binding rejected: {err}"
    );
}

#[tokio::test]
async fn empty_required_field_is_rejected() {
    let store = migrated_pool().await;
    let mut grant = mcp_grant("tenant-a", "worker-1", "send_email");
    grant.worker_id = "  ".into();
    let err = promote(store.pool(), &grant).await.unwrap_err();
    assert!(
        format!("{err}").contains("worker_id must not be empty"),
        "blank worker_id rejected: {err}"
    );
}
