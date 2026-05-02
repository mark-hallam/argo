use super::*;
use argo_store::SessionStore;
use argo_types::{SessionId, TenantId, WorkspaceId};
use std::sync::Arc;

async fn fresh() -> (Arc<SessionStore>, SessionId) {
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let tenant = TenantId::new();
    let ws = WorkspaceId::new();
    sqlx::query(
        "INSERT INTO workspace (id, tenant_id, name, root_path, created_at)
         VALUES (?, ?, 'test', '.', strftime('%s','now'))",
    )
    .bind(ws.0.as_bytes().as_slice())
    .bind(tenant.0.as_bytes().as_slice())
    .execute(store.pool())
    .await
    .unwrap();
    let session = store
        .create_session(tenant, ws, "model", "provider", Some("t"), None)
        .await
        .unwrap();
    (store, session)
}

#[tokio::test]
async fn write_then_verify_clean_chain() {
    let (store, session) = fresh().await;
    let sink = AuditSink::new(store.clone());
    sink.write(
        Some(session),
        AuditKind::TurnStart,
        serde_json::json!({"a": 1}),
    )
    .await
    .unwrap();
    sink.write(
        Some(session),
        AuditKind::AssistantMessage,
        serde_json::json!({"text_len": 4}),
    )
    .await
    .unwrap();
    sink.write(
        Some(session),
        AuditKind::ToolCall,
        serde_json::json!({"tool": "ls"}),
    )
    .await
    .unwrap();
    sink.write(Some(session), AuditKind::TurnEnd, serde_json::json!({}))
        .await
        .unwrap();
    let report = verify_session(&store, session).await.unwrap();
    assert!(report.ok, "{:?}", report.breaks);
    assert_eq!(report.rows, 4);
}

#[tokio::test]
async fn write_emits_each_v02_kind_and_chain_verifies() {
    let (store, session) = fresh().await;
    let sink = AuditSink::new(store.clone());
    for k in AuditKind::all() {
        sink.write(Some(session), k, serde_json::json!({"k": k.label()}))
            .await
            .unwrap();
    }
    let report = verify_session(&store, session).await.unwrap();
    assert!(report.ok);
    assert_eq!(report.rows, AuditKind::all().len());
    // §17.2 acceptance: every emitted row had a known label.
    for k in AuditKind::all() {
        assert!(
            report.kind_counts.contains_key(k.label()),
            "missing kind {}",
            k.label()
        );
    }
}

#[tokio::test]
async fn tampered_payload_breaks_chain() {
    let (store, session) = fresh().await;
    let sink = AuditSink::new(store.clone());
    sink.write(
        Some(session),
        AuditKind::TurnStart,
        serde_json::json!({"a": 1}),
    )
    .await
    .unwrap();
    sink.write(Some(session), AuditKind::TurnEnd, serde_json::json!({}))
        .await
        .unwrap();
    // Tamper with the first row's payload.
    sqlx::query("UPDATE audit_record SET payload_json = '{\"a\": 999}' WHERE seq = 1")
        .execute(store.pool())
        .await
        .unwrap();
    let report = verify_session(&store, session).await.unwrap();
    assert!(!report.ok);
    assert!(!report.breaks.is_empty());
    let reasons: Vec<&str> = report.breaks.iter().map(|b| b.reason.as_str()).collect();
    assert!(
        reasons.iter().any(|r| r.contains("self_hash")),
        "expected self_hash mismatch, got {reasons:?}"
    );
}

#[tokio::test]
async fn tampered_parent_hash_breaks_chain() {
    let (store, session) = fresh().await;
    let sink = AuditSink::new(store.clone());
    for _ in 0..3 {
        sink.write(Some(session), AuditKind::TurnStart, serde_json::json!({}))
            .await
            .unwrap();
    }
    sqlx::query("UPDATE audit_record SET parent_hash = randomblob(32) WHERE seq = 2")
        .execute(store.pool())
        .await
        .unwrap();
    let report = verify_session(&store, session).await.unwrap();
    assert!(!report.ok);
    assert!(report.breaks.iter().any(|b| b.seq == 2));
}

#[tokio::test]
async fn unknown_kind_label_is_flagged() {
    let (store, session) = fresh().await;
    let sink = AuditSink::new(store.clone());
    sink.write(Some(session), AuditKind::TurnStart, serde_json::json!({}))
        .await
        .unwrap();
    // Manually rewrite the kind to something the enum doesn't know.
    sqlx::query("UPDATE audit_record SET kind = 'mysterious_kind' WHERE seq = 1")
        .execute(store.pool())
        .await
        .unwrap();
    let report = verify_session(&store, session).await.unwrap();
    assert!(!report.ok);
    let any_unknown = report
        .breaks
        .iter()
        .any(|b| b.reason.contains("unknown audit kind"));
    assert!(any_unknown, "{:?}", report.breaks);
}

#[tokio::test]
async fn verify_all_walks_every_session() {
    let (store, session_a) = fresh().await;
    // Reuse the same store; create a second session in it.
    let tenant = TenantId::new();
    let ws = WorkspaceId::new();
    sqlx::query(
        "INSERT INTO workspace (id, tenant_id, name, root_path, created_at)
         VALUES (?, ?, 'b', '.', strftime('%s','now'))",
    )
    .bind(ws.0.as_bytes().as_slice())
    .bind(tenant.0.as_bytes().as_slice())
    .execute(store.pool())
    .await
    .unwrap();
    let session_b = store
        .create_session(tenant, ws, "m", "p", Some("b"), None)
        .await
        .unwrap();
    let sink = AuditSink::new(store.clone());
    for s in [session_a, session_b] {
        for _ in 0..3 {
            sink.write(Some(s), AuditKind::TurnStart, serde_json::json!({}))
                .await
                .unwrap();
        }
    }
    let report = verify_all(&store).await.unwrap();
    assert!(report.ok);
    assert_eq!(report.rows, 6);
}

#[tokio::test]
async fn rapid_writes_one_thousand_rows_verify_clean() {
    // §17.2 mini-load test: a thousand rows complete cleanly. The
    // canonical 10k corpus is the CLI test; this is the in-process
    // smoke test.
    let (store, session) = fresh().await;
    let sink = AuditSink::new(store.clone());
    for i in 0..1_000 {
        let kind = if i % 3 == 0 {
            AuditKind::ToolCall
        } else if i % 3 == 1 {
            AuditKind::ToolResult
        } else {
            AuditKind::AssistantMessage
        };
        sink.write(Some(session), kind, serde_json::json!({"i": i}))
            .await
            .unwrap();
    }
    let report = verify_session(&store, session).await.unwrap();
    assert!(report.ok);
    assert_eq!(report.rows, 1_000);
}
