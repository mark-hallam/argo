use super::*;
use argo_identity::{MemoryNs, MemoryScope, WorkerId};
use argo_store::SessionStore;
use argo_types::{TenantId, WorkspaceId};
use chrono::{TimeZone, Utc};
use proptest::prelude::*;
use std::sync::Arc;

async fn fresh() -> (MemoryStore, WorkspaceId, TenantId) {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let tenant = TenantId::new();
    let ws = WorkspaceId::new();
    sqlx::query(
        "INSERT INTO workspace (id, tenant_id, name, root_path, created_at)
         VALUES (?, ?, 'layer-test', '.', strftime('%s','now'))",
    )
    .bind(ws.0.as_bytes().as_slice())
    .bind(tenant.0.as_bytes().as_slice())
    .execute(store.pool())
    .await
    .unwrap();
    (MemoryStore::new(Arc::new(store)), ws, tenant)
}

#[tokio::test]
async fn append_defaults_rows_to_recent_layer() {
    let (memory, ws, _) = fresh().await;
    let row = memory
        .append(ws, MemoryFile::Memory, "fresh note".into(), None)
        .await
        .unwrap();
    let layer: String = sqlx::query_scalar("SELECT layer FROM memory_entry WHERE id = ?")
        .bind(row.id.as_bytes().as_slice())
        .fetch_one(memory.pool())
        .await
        .unwrap();
    assert_eq!(layer, "recent");
}

#[tokio::test]
async fn promote_recent_rows_makes_them_index_queryable() {
    let (memory, ws, tenant) = fresh().await;
    let ns = MemoryNs::new(tenant, WorkerId::new(), MemoryScope::WorkerSelf);
    let row = memory
        .append_in_namespace(
            &ns,
            ws,
            MemoryFile::Memory,
            "refund escalation playbook".into(),
            None,
        )
        .await
        .unwrap();
    let old = Utc::now() - chrono::Duration::days(45);
    sqlx::query("UPDATE memory_entry SET learned_at = ? WHERE id = ?")
        .bind(old.timestamp_micros())
        .bind(row.id.as_bytes().as_slice())
        .execute(memory.pool())
        .await
        .unwrap();

    let changed = memory.promote_recent_older_than(30).await.unwrap();
    assert_eq!(changed, 1);
    let hits = memory.query(&ns, "refund", 3).await.unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].id, row.id);
}

#[tokio::test]
async fn compose_context_layers_summary_recent_and_indexed_rows() {
    let (memory, ws, tenant) = fresh().await;
    let ns = MemoryNs::new(tenant, WorkerId::new(), MemoryScope::WorkerSelf);
    memory
        .append_in_namespace_with_layer(
            &ns,
            ws,
            MemoryFile::Memory,
            "last month: customer asked for SOC2 evidence".into(),
            None,
            MemoryLayer::Summary,
        )
        .await
        .unwrap();
    memory
        .append_in_namespace(
            &ns,
            ws,
            MemoryFile::Memory,
            "today: prioritize enterprise refund escalation".into(),
            None,
        )
        .await
        .unwrap();
    memory
        .append_in_namespace_with_layer(
            &ns,
            ws,
            MemoryFile::Memory,
            "older refund policy context".into(),
            None,
            MemoryLayer::Indexed,
        )
        .await
        .unwrap();

    let pack = memory
        .compose_context(&ns, "refund_escalation")
        .await
        .unwrap();
    assert!(pack.text.contains("[summary]"));
    assert!(pack.text.contains("[recent]"));
    assert!(pack.text.contains("[indexed]"));
    assert!(pack.token_estimate < 8_000);
}

#[tokio::test]
async fn nightly_summary_writes_one_summary_row_per_worker_month() {
    let (memory, ws, tenant) = fresh().await;
    let ns = MemoryNs::new(tenant, WorkerId::new(), MemoryScope::WorkerSelf);
    let row = memory
        .append_in_namespace(
            &ns,
            ws,
            MemoryFile::Memory,
            "april customer handoff detail".into(),
            None,
        )
        .await
        .unwrap();
    let learned_at = Utc.with_ymd_and_hms(2026, 4, 10, 8, 0, 0).unwrap();
    sqlx::query("UPDATE memory_entry SET learned_at = ? WHERE id = ?")
        .bind(learned_at.timestamp_micros())
        .bind(row.id.as_bytes().as_slice())
        .execute(memory.pool())
        .await
        .unwrap();

    let now = Utc.with_ymd_and_hms(2026, 5, 6, 2, 0, 0).unwrap();
    let report = memory
        .run_nightly_summary_job(ws, now, &HaikuSummaryProvider)
        .await
        .unwrap();
    assert_eq!(report.month.label(), "2026-04");
    assert_eq!(report.workers_seen, 1);
    assert_eq!(report.summaries_written, 1);

    let summaries = memory
        .layered_entries(&ns, MemoryLayer::Summary)
        .await
        .unwrap();
    assert_eq!(summaries.len(), 1);
    assert!(summaries[0].entry.body.contains("april customer handoff"));

    let report_again = memory
        .run_nightly_summary_job(ws, now, &HaikuSummaryProvider)
        .await
        .unwrap();
    assert_eq!(report_again.summaries_written, 0);
}

proptest! {
    #[test]
    fn cross_tenant_index_query_returns_zero_rows(
        body in "[a-z ]{3,48}",
        query in "[a-z ]{3,24}",
    ) {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async move {
            let (memory, ws, tenant_a) = fresh().await;
            let tenant_b = TenantId::new();
            let ns_a = MemoryNs::new(tenant_a, WorkerId::new(), MemoryScope::WorkerSelf);
            let ns_b = MemoryNs::new(tenant_b, ns_a.worker_id, MemoryScope::WorkerSelf);
            memory
                .append_in_namespace_with_layer(
                    &ns_a,
                    ws,
                    MemoryFile::Memory,
                    format!("{body} tenant alpha"),
                    None,
                    MemoryLayer::Indexed,
                )
                .await
                .unwrap();

            let hits = memory.query(&ns_b, &query, 10).await.unwrap();
            prop_assert!(hits.is_empty());
            Ok(())
        })?;
    }
}
