//! End-to-end `argo compliance delete` smoke test.
//!
//! Seeds two tenants in a fresh `ARGO_HOME`, deletes tenant A via the
//! real `argo` binary, and asserts:
//! - alice's workspace + session + message + audit rows are gone
//! - bob's data is untouched
//! - the audit chain shows tombstoned alice rows + a fresh
//!   `compliance.delete` global row
//! - confirm-mismatch exits 2 without touching any rows
mod common;

use argo_audit::compute_chain_hash;
use argo_store::SessionStore;
use argo_types::{SessionId, TenantId, WorkspaceId};
use std::path::Path;
use std::process::{Command, Output};
use uuid::Uuid;

#[tokio::test]
async fn compliance_delete_isolates_and_tombstones() {
    let home = tempfile::tempdir().unwrap();
    let store = SessionStore::open(home.path().join("sessions.db"))
        .await
        .unwrap();
    store.migrate().await.unwrap();

    let alice = TenantId::new();
    let bob = TenantId::new();
    let alice_ws = seed_workspace(&store, alice, "alice-ws").await;
    let bob_ws = seed_workspace(&store, bob, "bob-ws").await;
    let alice_sess = seed_session(&store, alice, alice_ws).await;
    let bob_sess = seed_session(&store, bob, bob_ws).await;
    seed_message(&store, alice_sess, "alice msg", 0).await;
    seed_message(&store, bob_sess, "bob msg", 0).await;
    seed_audit_chain(&store, alice_sess, 3).await;
    seed_audit_chain(&store, bob_sess, 2).await;

    let alice_id = alice.to_string();

    let bad = run(
        home.path(),
        &[
            "compliance",
            "delete",
            "--tenant",
            &alice_id,
            "--confirm",
            &Uuid::now_v7().to_string(),
        ],
    );
    assert_eq!(
        bad.status.code(),
        Some(2),
        "confirm mismatch must exit 2; stderr={}",
        String::from_utf8_lossy(&bad.stderr)
    );
    let alice_ws_after_bad: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM workspace WHERE tenant_id = ?")
            .bind(alice.0.as_bytes().as_slice())
            .fetch_one(store.pool())
            .await
            .unwrap();
    assert_eq!(alice_ws_after_bad, 1, "mismatch must not delete anything");

    let ok = run(
        home.path(),
        &[
            "compliance",
            "delete",
            "--tenant",
            &alice_id,
            "--confirm",
            &alice_id,
        ],
    );
    assert!(
        ok.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&ok.stderr)
    );

    let alice_ws_after: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM workspace WHERE tenant_id = ?")
            .bind(alice.0.as_bytes().as_slice())
            .fetch_one(store.pool())
            .await
            .unwrap();
    assert_eq!(alice_ws_after, 0, "alice's workspace must be gone");
    let bob_ws_after: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM workspace WHERE tenant_id = ?")
            .bind(bob.0.as_bytes().as_slice())
            .fetch_one(store.pool())
            .await
            .unwrap();
    assert_eq!(bob_ws_after, 1, "bob's workspace must survive");

    let bob_audit: Vec<String> = sqlx::query_scalar(
        "SELECT payload_json FROM audit_record WHERE session_id = ? ORDER BY seq",
    )
    .bind(bob_sess.0.as_bytes().as_slice())
    .fetch_all(store.pool())
    .await
    .unwrap();
    assert_eq!(bob_audit.len(), 2);
    assert!(
        bob_audit.iter().all(|p| p.contains("alice@example.com")),
        "bob's audit must NOT have been redacted"
    );

    let alice_audit: Vec<(String, Vec<u8>, Vec<u8>, String)> = sqlx::query_as(
        "SELECT kind, parent_hash, self_hash, payload_json FROM audit_record \
         WHERE session_id = ? ORDER BY seq ASC",
    )
    .bind(alice_sess.0.as_bytes().as_slice())
    .fetch_all(store.pool())
    .await
    .unwrap();
    assert_eq!(alice_audit.len(), 3, "alice audit rows tombstoned, not deleted");
    let mut prev: Option<[u8; 32]> = None;
    for (kind, parent_hash, self_hash, payload) in &alice_audit {
        assert!(payload.contains("\"tombstone\":true"), "payload not tombstoned");
        assert!(!payload.contains("alice@example.com"), "PII leaked: {payload}");
        let parent: [u8; 32] = parent_hash.as_slice().try_into().unwrap();
        let self_h: [u8; 32] = self_hash.as_slice().try_into().unwrap();
        if let Some(p) = prev {
            assert_eq!(p, parent, "broken chain link");
        }
        let recomputed = compute_chain_hash(&parent, kind, payload);
        assert_eq!(recomputed, self_h, "chain hash mismatch on tombstoned row");
        prev = Some(self_h);
    }

    let last_global_kind: String = sqlx::query_scalar(
        "SELECT kind FROM audit_record WHERE session_id IS NULL ORDER BY seq DESC LIMIT 1",
    )
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert_eq!(last_global_kind, "compliance.delete");
}

async fn seed_workspace(
    store: &SessionStore,
    tenant: TenantId,
    name: &str,
) -> WorkspaceId {
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

fn run(home: &Path, args: &[&str]) -> Output {
    Command::new(common::argo_bin())
        .env("ARGO_HOME", home)
        .args(args)
        .output()
        .unwrap()
}
