//! Integration smoke for `QualityInputs::from_audit_history` over a
//! real `SessionStore`-backed audit log (Mission Claim 4 §3 #77).
//!
//! Seeds 10 `tool_result` rows for a synthetic skill name and asserts
//! that the aggregator produces the expected success rate, retry
//! count, and latency aggregates.

use argo_skill::{
    audit_reader::{AuditStoreReader, SessionStoreAuditReader},
    QualityInputs,
};
use argo_store::SessionStore;
use argo_types::SkillId;
use chrono::{TimeZone, Utc};
use std::sync::Arc;
use uuid::Uuid;

async fn write_tool_result(
    store: &SessionStore,
    tool: &str,
    is_error: bool,
    duration_ms: u64,
    created_at_micros: i64,
) {
    let id = Uuid::now_v7();
    let payload = serde_json::json!({
        "tool": tool,
        "call_id": Uuid::now_v7().to_string(),
        "is_error": is_error,
        "duration_ms": duration_ms,
    });
    let payload_json = serde_json::to_string(&payload).unwrap();
    let parent_hash = vec![0u8; 32];
    let self_hash = vec![0u8; 32];
    sqlx::query(
        "INSERT INTO audit_record \
         (id, session_id, kind, payload_json, parent_hash, self_hash, created_at) \
         VALUES (?, NULL, 'tool_result', ?, ?, ?, ?)",
    )
    .bind(id.as_bytes().as_slice())
    .bind(payload_json)
    .bind(parent_hash)
    .bind(self_hash)
    .bind(created_at_micros)
    .execute(store.pool())
    .await
    .unwrap();
}

#[tokio::test]
async fn from_audit_history_aggregates_from_real_store() {
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();

    let skill_name = "calculator";
    let base = Utc.with_ymd_and_hms(2026, 5, 7, 12, 0, 0).unwrap();
    // 8 succeed, 2 fail. Durations 50..=500ms.
    for i in 0..10_i64 {
        let is_error = i >= 8;
        let duration_ms = u64::try_from((i + 1) * 50).unwrap();
        let created_at = (base + chrono::Duration::seconds(i)).timestamp_micros();
        write_tool_result(&store, skill_name, is_error, duration_ms, created_at).await;
    }
    // Noise: a different tool's rows should not be counted.
    write_tool_result(&store, "other_tool", true, 9_999, base.timestamp_micros()).await;

    let reader = SessionStoreAuditReader::new(Arc::clone(&store), skill_name.to_string());
    let inputs = QualityInputs::from_audit_history(&SkillId::new(), &reader, None)
        .await
        .unwrap();

    // 8/10 succeeded.
    assert!((inputs.success_rate - 0.8).abs() < 1e-9);
    // 2 failures contribute to retry_count.
    assert_eq!(inputs.retry_count, 2);
    // Mean of (50..=500 step 50) = 275.
    assert!((inputs.avg_latency_ms - 275.0).abs() < 1e-9);
    // p99 (nearest-rank on 10 values) is the max → 500.
    assert!((inputs.p99_latency_ms - 500.0).abs() < 1e-9);
}

#[tokio::test]
async fn from_audit_history_with_since_filters_old_rows() {
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();

    let skill_name = "calculator";
    let base = Utc.with_ymd_and_hms(2026, 5, 7, 12, 0, 0).unwrap();
    // 5 old (all failures) + 5 new (all successes).
    for i in 0..5_i64 {
        let created_at = (base + chrono::Duration::seconds(i)).timestamp_micros();
        write_tool_result(&store, skill_name, true, 100, created_at).await;
    }
    for i in 5..10_i64 {
        let created_at = (base + chrono::Duration::seconds(i + 60)).timestamp_micros();
        write_tool_result(&store, skill_name, false, 200, created_at).await;
    }

    let cutoff = base + chrono::Duration::seconds(30);
    let reader = SessionStoreAuditReader::new(Arc::clone(&store), skill_name.to_string());
    let inputs = QualityInputs::from_audit_history(&SkillId::new(), &reader, Some(cutoff))
        .await
        .unwrap();
    // Only the 5 newer rows should count; all succeeded.
    assert!((inputs.success_rate - 1.0).abs() < 1e-9);
    assert_eq!(inputs.retry_count, 0);
    assert!((inputs.avg_latency_ms - 200.0).abs() < 1e-9);
}

#[tokio::test]
async fn from_audit_history_empty_log_yields_default() {
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let reader = SessionStoreAuditReader::new(Arc::clone(&store), "nobody".to_string());
    let inputs = QualityInputs::from_audit_history(&SkillId::new(), &reader, None)
        .await
        .unwrap();
    assert_eq!(inputs, QualityInputs::default());
}

#[tokio::test]
async fn session_store_reader_filters_by_skill_name() {
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let base = Utc.with_ymd_and_hms(2026, 5, 7, 12, 0, 0).unwrap();
    write_tool_result(&store, "skill_a", false, 100, base.timestamp_micros()).await;
    write_tool_result(&store, "skill_b", true, 200, base.timestamp_micros()).await;

    let reader = SessionStoreAuditReader::new(Arc::clone(&store), "skill_a".to_string());
    let rows = reader
        .rows_for_skill(&SkillId::new(), None)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0].payload.get("tool").and_then(|v| v.as_str()),
        Some("skill_a")
    );
}
