//! Integration test for `argo_audit::render_replay` (Stream
//! `claude-audit-replay`). Seeds 5 audit rows on one session,
//! reads them back via `rows_for_session`, then asserts every
//! row appears in seq order in both `pretty` and `raw` output.
use argo_audit::{
    filter_replay_rows, render_replay, rows_for_session, AuditKind, AuditSink, ReplayFormat,
};
use argo_store::SessionStore;
use argo_types::{TenantId, WorkspaceId};
use std::sync::Arc;

async fn fresh() -> (Arc<SessionStore>, argo_types::SessionId) {
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let tenant = TenantId::new();
    let ws = WorkspaceId::new();
    sqlx::query(
        "INSERT INTO workspace (id, tenant_id, name, root_path, created_at)
         VALUES (?, ?, 'replay', '.', strftime('%s','now'))",
    )
    .bind(ws.0.as_bytes().as_slice())
    .bind(tenant.0.as_bytes().as_slice())
    .execute(store.pool())
    .await
    .unwrap();
    let session = store
        .create_session(tenant, ws, "model", "provider", Some("replay"), None)
        .await
        .unwrap();
    (store, session)
}

#[tokio::test]
async fn five_rows_round_trip_in_seq_order() {
    let (store, session) = fresh().await;
    let sink = AuditSink::new(Arc::clone(&store));
    for i in 0..5 {
        sink.write(
            Some(session),
            AuditKind::AssistantMessage,
            serde_json::json!({"i": i, "phase": format!("step-{i}")}),
        )
        .await
        .unwrap();
    }
    let rows = rows_for_session(&store, session).await.unwrap();
    assert_eq!(rows.len(), 5);
    let seqs: Vec<i64> = rows.iter().map(|r| r.seq).collect();
    let mut sorted = seqs.clone();
    sorted.sort();
    assert_eq!(seqs, sorted, "rows must arrive in seq order");

    // Pretty: 5 lines, each carries the kind + the per-row index.
    let pretty = render_replay(&rows, ReplayFormat::Pretty);
    let lines: Vec<&str> = pretty.lines().collect();
    assert_eq!(lines.len(), 5);
    for (i, line) in lines.iter().enumerate() {
        assert!(line.contains("assistant_message"), "line {i}: {line}");
        assert!(
            line.contains(&format!("step-{i}")),
            "line {i} should reference step-{i}: {line}"
        );
    }

    // Raw: 5 NDJSON lines, each parses to a JSON object with the
    // expected `seq`.
    let raw = render_replay(&rows, ReplayFormat::Raw);
    let raw_lines: Vec<&str> = raw.lines().collect();
    assert_eq!(raw_lines.len(), 5);
    for (i, line) in raw_lines.iter().enumerate() {
        let v: serde_json::Value = serde_json::from_str(line).unwrap();
        assert_eq!(v.get("seq").and_then(|s| s.as_i64()), Some(rows[i].seq));
        assert_eq!(
            v.get("payload")
                .and_then(|p| p.get("phase"))
                .and_then(|s| s.as_str()),
            Some(format!("step-{i}").as_str())
        );
    }
}

#[tokio::test]
async fn from_seq_filter_drops_earlier_rows() {
    let (store, session) = fresh().await;
    let sink = AuditSink::new(Arc::clone(&store));
    for i in 0..5 {
        sink.write(
            Some(session),
            AuditKind::AssistantMessage,
            serde_json::json!({"i": i}),
        )
        .await
        .unwrap();
    }
    let rows = rows_for_session(&store, session).await.unwrap();
    let cutoff = rows[2].seq;
    let filtered = filter_replay_rows(rows, Some(cutoff), None);
    assert_eq!(filtered.len(), 3);
    assert!(filtered.iter().all(|r| r.seq >= cutoff));
}
