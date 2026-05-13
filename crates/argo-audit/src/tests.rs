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
        None,
    )
    .await
    .unwrap();
    sink.write(
        Some(session),
        AuditKind::AssistantMessage,
        serde_json::json!({"text_len": 4}),
        None,
    )
    .await
    .unwrap();
    sink.write(
        Some(session),
        AuditKind::ToolCall,
        serde_json::json!({"tool": "ls"}),
        None,
    )
    .await
    .unwrap();
    sink.write(Some(session), AuditKind::TurnEnd, serde_json::json!({}), None)
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
        sink.write(Some(session), k, serde_json::json!({"k": k.label()}), None)
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
        None,
    )
    .await
    .unwrap();
    sink.write(Some(session), AuditKind::TurnEnd, serde_json::json!({}), None)
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
        sink.write(Some(session), AuditKind::TurnStart, serde_json::json!({}), None)
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
    sink.write(Some(session), AuditKind::TurnStart, serde_json::json!({}), None)
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
            sink.write(Some(s), AuditKind::TurnStart, serde_json::json!({}), None)
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
        sink.write(Some(session), kind, serde_json::json!({"i": i}), None)
            .await
            .unwrap();
    }
    let report = verify_session(&store, session).await.unwrap();
    assert!(report.ok);
    assert_eq!(report.rows, 1_000);
}

#[tokio::test]
async fn broadcast_subscribers_receive_writes_in_order() {
    // Stream `claude-realtime-audit-stream`. Subscribers attached
    // before any writes see every row in order.
    let (store, session) = fresh().await;
    let sink = AuditSink::new(store.clone());
    let mut rx = sink.subscribe();
    assert_eq!(sink.subscriber_count(), 1);
    for i in 0..5 {
        sink.write(
            Some(session),
            AuditKind::AssistantMessage,
            serde_json::json!({"i": i}),
            None,
        )
        .await
        .unwrap();
    }
    let mut received = Vec::new();
    for _ in 0..5 {
        let ev = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
            .await
            .expect("recv timed out")
            .expect("recv ok");
        received.push(ev.payload.get("i").and_then(|v| v.as_u64()).unwrap());
    }
    assert_eq!(received, vec![0, 1, 2, 3, 4]);
}

#[tokio::test]
async fn broadcast_subscribers_added_late_only_see_subsequent_writes() {
    // Brief: live subscribers see new rows, not historical ones.
    let (store, session) = fresh().await;
    let sink = AuditSink::new(store.clone());
    sink.write(
        Some(session),
        AuditKind::TurnStart,
        serde_json::json!({"phase": "first"}),
        None,
    )
    .await
    .unwrap();
    let mut rx = sink.subscribe();
    sink.write(
        Some(session),
        AuditKind::TurnEnd,
        serde_json::json!({"phase": "second"}),
        None,
    )
    .await
    .unwrap();
    let ev = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
        .await
        .expect("recv timed out")
        .expect("recv ok");
    assert_eq!(ev.payload.get("phase").and_then(|v| v.as_str()), Some("second"));
}

#[tokio::test]
async fn broadcast_event_carries_kind_and_session_id() {
    let (store, session) = fresh().await;
    let sink = AuditSink::new(store.clone());
    let mut rx = sink.subscribe();
    sink.write(
        Some(session),
        AuditKind::ToolCall,
        serde_json::json!({"tool": "shell"}),
        None,
    )
    .await
    .unwrap();
    let ev = rx.recv().await.unwrap();
    assert_eq!(ev.kind, "tool_call");
    assert_eq!(ev.session_id, Some(session.0.to_string()));
    assert!(ev.payload.get("tool").is_some());
}

#[tokio::test]
async fn write_with_principal_stamps_audit_record_column() {
    // Stream `mvp-audit-principal-stamp` (Tier 1 MVP — closes
    // mission claim "everything is auditable"). Asserts the
    // resolved gateway caller lands in the new `principal_id`
    // column, that NULL stays NULL for system-internal writes,
    // and that the chain still verifies after both row kinds
    // intermix.
    let (store, session) = fresh().await;
    let sink = AuditSink::new(store.clone());
    sink.write(
        Some(session),
        AuditKind::TurnStart,
        serde_json::json!({}),
        Some("human:operator@example.com".to_string()),
    )
    .await
    .unwrap();
    sink.write(
        Some(session),
        AuditKind::TurnEnd,
        serde_json::json!({}),
        None,
    )
    .await
    .unwrap();

    let rows: Vec<(i64, Option<String>)> = sqlx::query_as(
        "SELECT seq, principal_id FROM audit_record \
         WHERE session_id = ? ORDER BY seq ASC",
    )
    .bind(session.0.as_bytes().as_slice())
    .fetch_all(store.pool())
    .await
    .unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].1.as_deref(), Some("human:operator@example.com"));
    assert!(rows[1].1.is_none(), "system-internal write must land NULL");

    let report = verify_session(&store, session).await.unwrap();
    assert!(report.ok, "{:?}", report.breaks);
}

// ─────────────────────────────────────────────────────────────────
// Stream `mission-enterprise-cross-tenant-audit` — tenant_id denorm
// ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn write_with_tenant_stamps_column_and_payload() {
    let (store, session) = fresh().await;
    let sink = AuditSink::new_with_tenant(store.clone(), "tenant-acme".to_string());
    sink.write(Some(session), AuditKind::TurnStart, serde_json::json!({"a": 1}), None)
        .await
        .unwrap();
    let row: (String, String) = sqlx::query_as(
        "SELECT tenant_id, payload_json FROM audit_record \
         WHERE session_id = ? ORDER BY seq DESC LIMIT 1",
    )
    .bind(session.0.as_bytes().as_slice())
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert_eq!(row.0, "tenant-acme");
    let payload: serde_json::Value = serde_json::from_str(&row.1).unwrap();
    assert_eq!(payload["tenant_id"].as_str(), Some("tenant-acme"));
    assert_eq!(payload["a"].as_i64(), Some(1));
}

#[tokio::test]
async fn legacy_default_tenant_unchanged_for_existing_call_sites() {
    // `AuditSink::new(store)` keeps backwards-compat: every existing
    // call site continues to compile and stamps the documented
    // default tenant id so the audit feed's legacy-bucket union
    // surfaces these rows for single-tenant deployments.
    let (store, session) = fresh().await;
    let sink = AuditSink::new(store.clone());
    sink.write(Some(session), AuditKind::TurnStart, serde_json::json!({}), None)
        .await
        .unwrap();
    let tenant: String = sqlx::query_scalar(
        "SELECT tenant_id FROM audit_record \
         WHERE session_id = ? ORDER BY seq DESC LIMIT 1",
    )
    .bind(session.0.as_bytes().as_slice())
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert_eq!(tenant, "ws-default");
    let report = verify_session(&store, session).await.unwrap();
    assert!(report.ok, "{:?}", report.breaks);
}

#[tokio::test]
async fn chain_hash_unchanged_after_tenant_stamp() {
    // Verifying a new chain end-to-end keeps the structural
    // invariant: rows stamped with tenant_id still recompute to the
    // same self_hash from the stored payload_json.
    let (store, session) = fresh().await;
    let sink = AuditSink::new_with_tenant(store.clone(), "tenant-alpha".into());
    for i in 0..4 {
        sink.write(
            Some(session),
            AuditKind::ToolCall,
            serde_json::json!({"i": i, "tool": "ls"}),
            None,
        )
        .await
        .unwrap();
    }
    let report = verify_session(&store, session).await.unwrap();
    assert!(report.ok, "{:?}", report.breaks);
    assert_eq!(report.rows, 4);
}

#[tokio::test]
async fn verify_chain_flags_tenant_denorm_drift() {
    // The denorm-drift check in verify_chain: column and
    // payload_json.tenant_id must agree for new rows. A tampered
    // column without a matching payload rewrite must surface as a
    // ChainBreak.
    let (store, session) = fresh().await;
    let sink = AuditSink::new_with_tenant(store.clone(), "tenant-clean".into());
    sink.write(Some(session), AuditKind::TurnStart, serde_json::json!({}), None)
        .await
        .unwrap();
    // Tamper: rewrite only the column, leave payload_json.tenant_id alone.
    sqlx::query("UPDATE audit_record SET tenant_id = 'tenant-other' WHERE seq = 1")
        .execute(store.pool())
        .await
        .unwrap();
    let report = verify_session(&store, session).await.unwrap();
    assert!(!report.ok, "expected drift to break chain report");
    assert!(
        report
            .breaks
            .iter()
            .any(|b| b.reason.contains("tenant_id denorm drift")),
        "expected denorm drift break, got {:?}",
        report.breaks
    );
}
