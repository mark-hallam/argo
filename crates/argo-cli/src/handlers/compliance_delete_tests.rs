//! Unit tests for `argo compliance delete`.
//!
//! Wired in from `compliance_delete.rs` via `#[path = ...] mod tests`
//! so they share the file's private surface.
use super::*;
use crate::cli::ComplianceDeleteArgs;
use argo_audit::compute_chain_hash;
use argo_store::SessionStore;
use argo_types::{SessionId, TenantId, WorkspaceId};
use chrono::Utc;
use uuid::Uuid;

async fn fresh_store() -> SessionStore {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    store
}

async fn seed_workspace(store: &SessionStore, tenant: TenantId, name: &str) -> WorkspaceId {
    let workspace = WorkspaceId::new();
    sqlx::query(
        "INSERT INTO workspace (id, tenant_id, name, root_path, created_at) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(workspace.0.as_bytes().as_slice())
    .bind(tenant.0.as_bytes().as_slice())
    .bind(name)
    .bind(format!("/tmp/{name}"))
    .bind(1_700_000_000_i64)
    .execute(store.pool())
    .await
    .unwrap();
    workspace
}

async fn seed_session(
    store: &SessionStore,
    tenant: TenantId,
    workspace: WorkspaceId,
) -> SessionId {
    let session = SessionId::new();
    sqlx::query(
        "INSERT INTO session (id, tenant_id, workspace_id, started_at, model, provider) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(session.0.as_bytes().as_slice())
    .bind(tenant.0.as_bytes().as_slice())
    .bind(workspace.0.as_bytes().as_slice())
    .bind(1_700_000_001_i64)
    .bind("gpt-x")
    .bind("openai")
    .execute(store.pool())
    .await
    .unwrap();
    session
}

async fn seed_message(store: &SessionStore, session: SessionId, body: &str, seq: i64) {
    sqlx::query(
        "INSERT INTO message (id, session_id, role, content_json, created_at, seq) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(Uuid::now_v7().as_bytes().as_slice())
    .bind(session.0.as_bytes().as_slice())
    .bind("user")
    .bind(body)
    .bind(1_700_000_002_i64)
    .bind(seq)
    .execute(store.pool())
    .await
    .unwrap();
}

async fn seed_audit_chain(store: &SessionStore, session: SessionId, count: usize) {
    let mut parent = [0_u8; 32];
    for i in 0..count {
        let payload = serde_json::json!({
            "actor": "alice",
            "target": format!("session:{}", session),
            "secret_email": "alice@example.com",
            "iter": i,
        })
        .to_string();
        let kind = "tool_call";
        let self_hash = compute_chain_hash(&parent, kind, &payload);
        sqlx::query(
            "INSERT INTO audit_record (id, session_id, kind, payload_json, parent_hash, self_hash, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(Uuid::now_v7().as_bytes().as_slice())
        .bind(session.0.as_bytes().as_slice())
        .bind(kind)
        .bind(payload)
        .bind(parent.as_slice())
        .bind(self_hash.as_slice())
        .bind(1_700_000_010_i64 + i as i64)
        .execute(store.pool())
        .await
        .unwrap();
        parent = self_hash;
    }
}

fn args(tenant: TenantId, confirm: &str, dry_run: bool) -> ComplianceDeleteArgs {
    ComplianceDeleteArgs {
        tenant: tenant.to_string(),
        confirm: confirm.to_string(),
        dry_run,
        json: false,
    }
}

#[tokio::test]
async fn confirm_mismatch_returns_none_and_makes_no_changes() {
    let store = fresh_store().await;
    let tenant = TenantId::new();
    seed_workspace(&store, tenant, "alpha").await;
    let mismatched = args(tenant, &Uuid::now_v7().to_string(), false);
    let result = validate_confirm(&mismatched).unwrap();
    assert!(
        result.is_none(),
        "mismatched --confirm must yield None (caller exits with EXIT_CONFIRM_MISMATCH)"
    );
    let still_there: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM workspace")
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(still_there, 1, "no DELETE should have run");
}

#[tokio::test]
async fn dry_run_reports_counts_without_modifying_data() {
    let store = fresh_store().await;
    let tenant = TenantId::new();
    let workspace = seed_workspace(&store, tenant, "alpha").await;
    let session = seed_session(&store, tenant, workspace).await;
    seed_message(&store, session, "hello", 0).await;
    seed_message(&store, session, "world", 1).await;
    seed_audit_chain(&store, session, 3).await;
    let summary = execute(&store, tenant, true).await.unwrap();
    assert_eq!(summary.deleted.get("message").copied(), Some(2));
    assert_eq!(summary.deleted.get("session").copied(), Some(1));
    assert_eq!(summary.deleted.get("workspace").copied(), Some(1));
    assert_eq!(summary.audit_rows_tombstoned, 3);
    let messages: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM message")
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(messages, 2, "dry-run must not delete messages");
    let workspaces: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM workspace")
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(workspaces, 1, "dry-run must not delete workspaces");
    let audit_payloads: Vec<String> =
        sqlx::query_scalar("SELECT payload_json FROM audit_record ORDER BY seq")
            .fetch_all(store.pool())
            .await
            .unwrap();
    assert!(
        audit_payloads
            .iter()
            .all(|p| p.contains("alice@example.com")),
        "dry-run must not redact audit payloads"
    );
}

#[tokio::test]
async fn real_delete_clears_every_tenant_table() {
    let store = fresh_store().await;
    let tenant = TenantId::new();
    let workspace = seed_workspace(&store, tenant, "alpha").await;
    let session = seed_session(&store, tenant, workspace).await;
    seed_message(&store, session, "hello", 0).await;
    seed_audit_chain(&store, session, 2).await;
    let summary = execute(&store, tenant, false).await.unwrap();
    assert!(summary.deleted.get("session").copied().unwrap_or(0) >= 1);
    let workspaces: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM workspace WHERE tenant_id = ?")
        .bind(tenant.0.as_bytes().as_slice())
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(workspaces, 0);
    let sessions: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM session WHERE tenant_id = ?")
        .bind(tenant.0.as_bytes().as_slice())
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(sessions, 0);
    let messages: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM message")
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(messages, 0);
}

#[tokio::test]
async fn cross_tenant_isolation_is_preserved() {
    let store = fresh_store().await;
    let alice = TenantId::new();
    let bob = TenantId::new();
    let alice_ws = seed_workspace(&store, alice, "alice").await;
    let bob_ws = seed_workspace(&store, bob, "bob").await;
    let alice_sess = seed_session(&store, alice, alice_ws).await;
    let bob_sess = seed_session(&store, bob, bob_ws).await;
    seed_message(&store, alice_sess, "alice msg", 0).await;
    seed_message(&store, bob_sess, "bob msg", 0).await;
    seed_audit_chain(&store, alice_sess, 2).await;
    seed_audit_chain(&store, bob_sess, 2).await;
    execute(&store, alice, false).await.unwrap();
    let alice_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM workspace WHERE tenant_id = ?")
        .bind(alice.0.as_bytes().as_slice())
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(alice_count, 0);
    let bob_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM workspace WHERE tenant_id = ?")
        .bind(bob.0.as_bytes().as_slice())
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(bob_count, 1, "bob's workspace must survive alice's delete");
    let bob_msgs: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM message m JOIN session s ON s.id = m.session_id WHERE s.tenant_id = ?",
    )
    .bind(bob.0.as_bytes().as_slice())
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert_eq!(bob_msgs, 1, "bob's messages must survive");
    let bob_audit: Vec<String> = sqlx::query_scalar(
        "SELECT payload_json FROM audit_record WHERE session_id = ? ORDER BY seq",
    )
    .bind(bob_sess.0.as_bytes().as_slice())
    .fetch_all(store.pool())
    .await
    .unwrap();
    assert_eq!(bob_audit.len(), 2, "bob's audit chain must survive");
    assert!(
        bob_audit.iter().all(|p| p.contains("alice@example.com")),
        "bob's audit must NOT have been redacted"
    );
}

#[tokio::test]
async fn audit_rows_are_tombstoned_and_chain_still_verifies() {
    let store = fresh_store().await;
    let tenant = TenantId::new();
    let workspace = seed_workspace(&store, tenant, "alpha").await;
    let session = seed_session(&store, tenant, workspace).await;
    seed_audit_chain(&store, session, 4).await;
    execute(&store, tenant, false).await.unwrap();
    let rows: Vec<(String, Vec<u8>, Vec<u8>, String)> = sqlx::query_as(
        "SELECT kind, parent_hash, self_hash, payload_json FROM audit_record \
         WHERE session_id = ? ORDER BY seq ASC",
    )
    .bind(session.0.as_bytes().as_slice())
    .fetch_all(store.pool())
    .await
    .unwrap();
    assert_eq!(rows.len(), 4, "tombstone preserves row count, never deletes");
    let mut prev_self: Option<[u8; 32]> = None;
    for (kind, parent_hash, self_hash, payload) in &rows {
        assert!(payload.contains("\"tombstone\":true"), "payload not tombstoned: {payload}");
        assert!(!payload.contains("alice@example.com"), "PII leaked: {payload}");
        let parent: [u8; 32] = parent_hash.as_slice().try_into().unwrap();
        let self_h: [u8; 32] = self_hash.as_slice().try_into().unwrap();
        if let Some(prev) = prev_self {
            assert_eq!(prev, parent, "chain link broken: parent != prev self_hash");
        }
        let recomputed = compute_chain_hash(&parent, kind, payload);
        assert_eq!(recomputed, self_h, "self_hash does not match recomputed chain hash");
        prev_self = Some(self_h);
    }
}

#[tokio::test]
async fn delete_audit_row_is_appended_with_tenant_target() {
    let store = fresh_store().await;
    let tenant = TenantId::new();
    let workspace = seed_workspace(&store, tenant, "alpha").await;
    let session = seed_session(&store, tenant, workspace).await;
    seed_audit_chain(&store, session, 1).await;
    execute(&store, tenant, false).await.unwrap();
    let (kind, payload): (String, String) = sqlx::query_as(
        "SELECT kind, payload_json FROM audit_record WHERE session_id IS NULL ORDER BY seq DESC LIMIT 1",
    )
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert_eq!(kind, "compliance.delete");
    let parsed: serde_json::Value = serde_json::from_str(&payload).unwrap();
    assert_eq!(parsed["action"], "compliance.delete");
    assert_eq!(parsed["target"], tenant.to_string());
    assert!(parsed["row_total"].as_u64().unwrap() >= 1);
}

#[tokio::test]
async fn mid_delete_failure_rolls_back_all_changes() {
    let store = fresh_store().await;
    let tenant = TenantId::new();
    let workspace = seed_workspace(&store, tenant, "alpha").await;
    let session = seed_session(&store, tenant, workspace).await;
    seed_message(&store, session, "hi", 0).await;
    seed_audit_chain(&store, session, 1).await;
    sqlx::query(
        "CREATE TRIGGER fail_workspace_delete BEFORE DELETE ON workspace \
         BEGIN SELECT RAISE(ABORT, 'simulated mid-delete failure'); END",
    )
    .execute(store.pool())
    .await
    .unwrap();
    let outcome = execute(&store, tenant, false).await;
    assert!(outcome.is_err(), "trigger must propagate as Err");
    let messages: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM message")
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(messages, 1, "rollback must restore messages");
    let sessions: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM session WHERE tenant_id = ?")
        .bind(tenant.0.as_bytes().as_slice())
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(sessions, 1, "rollback must restore sessions");
    let audit_payloads: Vec<String> =
        sqlx::query_scalar("SELECT payload_json FROM audit_record WHERE session_id = ?")
            .bind(session.0.as_bytes().as_slice())
            .fetch_all(store.pool())
            .await
            .unwrap();
    assert!(
        audit_payloads.iter().all(|p| p.contains("alice@example.com")),
        "rollback must restore original audit payloads"
    );
}

#[test]
fn redact_payload_keeps_actor_target_and_scrubs_pii() {
    let original = serde_json::json!({
        "actor": "alice",
        "target": "session:42",
        "secret_email": "alice@example.com",
        "ssn": "123-45-6789",
    })
    .to_string();
    let redacted = redact_payload(&original, "2026-05-11T00:00:00Z");
    let parsed: serde_json::Value = serde_json::from_str(&redacted).unwrap();
    assert_eq!(parsed["tombstone"], true);
    assert_eq!(parsed["actor"], "alice");
    assert_eq!(parsed["target"], "session:42");
    assert_eq!(parsed["secret_email"], "<erased>");
    assert_eq!(parsed["ssn"], "<erased>");
    assert_eq!(parsed["redacted_at"], "2026-05-11T00:00:00Z");
}

#[test]
fn parse_tenant_rejects_empty_and_garbage() {
    assert!(parse_tenant("").is_err());
    assert!(parse_tenant("   ").is_err());
    assert!(parse_tenant("not-a-uuid").is_err());
    let valid = Uuid::now_v7().to_string();
    assert!(parse_tenant(&valid).is_ok());
}

#[test]
fn validate_confirm_requires_non_empty_confirm() {
    let tenant = TenantId::new();
    let bad = ComplianceDeleteArgs {
        tenant: tenant.to_string(),
        confirm: String::new(),
        dry_run: false,
        json: false,
    };
    assert!(validate_confirm(&bad).is_err());
}

#[test]
fn iso_round_trip_in_audit_payload_uses_now() {
    // Smoke check that Utc::now() formats to RFC3339 — append_delete_audit
    // serializes this into the chain. Cheap regression guard against a
    // future chrono major-version surprise.
    let now = Utc::now().to_rfc3339();
    assert!(now.contains('T'));
    assert!(now.ends_with('Z') || now.contains('+') || now.contains('-'));
}
