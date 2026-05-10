//! Unit tests for `crate::quality`.

use super::*;
use crate::audit_reader::{AuditRow, AuditStoreReader};
use async_trait::async_trait;
use chrono::TimeZone;

fn at(hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 5, 7, hour, 0, 0).unwrap()
}

#[test]
fn perfect_inputs_score_100() {
    let inputs = QualityInputs::default();
    assert_eq!(compute_score(&inputs), 100);
}

#[test]
fn zero_success_rate_floors_score() {
    let inputs = QualityInputs {
        success_rate: 0.0,
        ..Default::default()
    };
    assert_eq!(compute_score(&inputs), 0);
}

#[test]
fn half_success_rate_yields_50_baseline() {
    let inputs = QualityInputs {
        success_rate: 0.5,
        ..Default::default()
    };
    assert_eq!(compute_score(&inputs), 50);
}

#[test]
fn retries_penalize_score_2_per_retry_capped_at_50() {
    let with_3_retries = QualityInputs {
        retry_count: 3,
        ..Default::default()
    };
    assert_eq!(compute_score(&with_3_retries), 100 - 6);
    let lots = QualityInputs {
        retry_count: 100,
        ..Default::default()
    };
    // Cap at 25 retries → 50 penalty.
    assert_eq!(compute_score(&lots), 100 - 50);
}

#[test]
fn slow_avg_latency_drops_score() {
    let medium = QualityInputs {
        avg_latency_ms: 10_000.0,
        ..Default::default()
    };
    assert_eq!(compute_score(&medium), 90);
    let slow = QualityInputs {
        avg_latency_ms: 60_000.0,
        ..Default::default()
    };
    assert_eq!(compute_score(&slow), 80);
}

#[test]
fn slow_p99_drops_score() {
    let medium = QualityInputs {
        p99_latency_ms: 60_000.0,
        ..Default::default()
    };
    assert_eq!(compute_score(&medium), 90);
    let extreme = QualityInputs {
        p99_latency_ms: 200_000.0,
        ..Default::default()
    };
    assert_eq!(compute_score(&extreme), 80);
}

#[test]
fn combined_penalties_clamp_to_zero() {
    let bad = QualityInputs {
        success_rate: 0.1,
        avg_latency_ms: 60_000.0,
        retry_count: 100,
        p99_latency_ms: 200_000.0,
    };
    // base=10, retry=50, avg=20, p99=20 → -80, clamped to 0.
    assert_eq!(compute_score(&bad), 0);
}

#[test]
fn clamp_handles_out_of_range_success_rate() {
    let weird = QualityInputs {
        success_rate: 1.5,
        ..Default::default()
    };
    assert_eq!(compute_score(&weird), 100);
    let neg = QualityInputs {
        success_rate: -0.5,
        ..Default::default()
    };
    assert_eq!(compute_score(&neg), 0);
}

async fn open_store() -> SkillQualityStore {
    let inner = Arc::new(SessionStore::open(":memory:").await.unwrap());
    inner.migrate().await.unwrap();
    let store = SkillQualityStore::new(inner);
    store.ensure_schema().await.unwrap();
    store
}

#[tokio::test]
async fn store_record_then_latest_round_trip() {
    let store = open_store().await;
    let skill = SkillId::new();
    let inputs = QualityInputs {
        success_rate: 0.8,
        avg_latency_ms: 1_000.0,
        retry_count: 2,
        p99_latency_ms: 5_000.0,
    };
    let recorded = store.record(skill, &inputs, at(12)).await.unwrap();
    // base = 0.8 * 100 = 80; retry=2 → 4 penalty → 76
    assert_eq!(recorded.score, 76);
    let loaded = store.latest(skill).await.unwrap().unwrap();
    assert_eq!(loaded.score, recorded.score);
    assert!((loaded.inputs.success_rate - 0.8).abs() < 1e-9);
}

#[tokio::test]
async fn store_history_returns_newest_first() {
    let store = open_store().await;
    let skill = SkillId::new();
    let inputs_a = QualityInputs {
        success_rate: 0.9,
        ..Default::default()
    };
    let inputs_b = QualityInputs {
        success_rate: 0.5,
        ..Default::default()
    };
    store.record(skill, &inputs_a, at(10)).await.unwrap();
    store.record(skill, &inputs_b, at(11)).await.unwrap();
    let history = store.history(skill).await.unwrap();
    assert_eq!(history.len(), 2);
    assert!(history[0].computed_at >= history[1].computed_at);
}

#[tokio::test]
async fn store_latest_all_orders_worst_first() {
    let store = open_store().await;
    let s1 = SkillId::new();
    let s2 = SkillId::new();
    let s3 = SkillId::new();
    store
        .record(
            s1,
            &QualityInputs {
                success_rate: 0.9,
                ..Default::default()
            },
            at(10),
        )
        .await
        .unwrap();
    store
        .record(
            s2,
            &QualityInputs {
                success_rate: 0.3,
                ..Default::default()
            },
            at(10),
        )
        .await
        .unwrap();
    store
        .record(
            s3,
            &QualityInputs {
                success_rate: 0.6,
                ..Default::default()
            },
            at(10),
        )
        .await
        .unwrap();
    let rows = store.latest_all().await.unwrap();
    assert_eq!(rows.len(), 3);
    assert!(rows[0].score <= rows[1].score);
    assert!(rows[1].score <= rows[2].score);
}

#[tokio::test]
async fn latest_by_skill_map_includes_every_scored_skill() {
    let store = open_store().await;
    let s1 = SkillId::new();
    let s2 = SkillId::new();
    store
        .record(s1, &QualityInputs::default(), at(10))
        .await
        .unwrap();
    store
        .record(
            s2,
            &QualityInputs {
                success_rate: 0.5,
                ..Default::default()
            },
            at(10),
        )
        .await
        .unwrap();
    let map = store.latest_by_skill().await.unwrap();
    assert_eq!(map.len(), 2);
    assert_eq!(map.get(&s1.to_string()).copied(), Some(100));
    assert_eq!(map.get(&s2.to_string()).copied(), Some(50));
}

#[tokio::test]
async fn latest_returns_none_for_unscored_skill() {
    let store = open_store().await;
    let skill = SkillId::new();
    assert!(store.latest(skill).await.unwrap().is_none());
}

/// In-memory [`AuditStoreReader`] for unit tests. Rows are
/// returned verbatim; the caller is responsible for excluding
/// rows older than `since` (matching the trait contract).
struct FakeAuditReader {
    rows: Vec<AuditRow>,
}

#[async_trait]
impl AuditStoreReader for FakeAuditReader {
    async fn rows_for_skill(
        &self,
        _skill_id: &SkillId,
        since: Option<DateTime<Utc>>,
    ) -> ArgoResult<Vec<AuditRow>> {
        Ok(self
            .rows
            .iter()
            .filter(|r| match since {
                Some(cutoff) => r.created_at >= cutoff,
                None => true,
            })
            .cloned()
            .collect())
    }
}

fn tool_result(is_error: bool, duration_ms: f64, at: DateTime<Utc>) -> AuditRow {
    AuditRow {
        kind: "tool_result".to_string(),
        payload: serde_json::json!({
            "tool": "demo",
            "call_id": "abc",
            "is_error": is_error,
            "duration_ms": duration_ms,
        }),
        created_at: at,
    }
}

#[tokio::test]
async fn from_audit_empty_history_yields_default() {
    let reader = FakeAuditReader { rows: Vec::new() };
    let inputs = QualityInputs::from_audit_history(&SkillId::new(), &reader, None)
        .await
        .unwrap();
    assert_eq!(inputs, QualityInputs::default());
}

#[tokio::test]
async fn from_audit_counts_success_and_failure() {
    let rows = vec![
        tool_result(false, 100.0, at(10)),
        tool_result(false, 200.0, at(11)),
        tool_result(true, 500.0, at(12)),
        tool_result(false, 300.0, at(13)),
    ];
    let reader = FakeAuditReader { rows };
    let inputs = QualityInputs::from_audit_history(&SkillId::new(), &reader, None)
        .await
        .unwrap();
    // 3 of 4 succeeded → 0.75
    assert!((inputs.success_rate - 0.75).abs() < 1e-9);
    // retry_count = number of failures = 1
    assert_eq!(inputs.retry_count, 1);
    // avg = (100 + 200 + 500 + 300) / 4 = 275
    assert!((inputs.avg_latency_ms - 275.0).abs() < 1e-9);
}

#[tokio::test]
async fn from_audit_since_excludes_old_rows() {
    let rows = vec![
        tool_result(true, 100.0, at(8)), // older than cutoff
        tool_result(false, 200.0, at(10)),
        tool_result(false, 400.0, at(11)),
    ];
    let reader = FakeAuditReader { rows };
    let cutoff = at(9);
    let inputs = QualityInputs::from_audit_history(&SkillId::new(), &reader, Some(cutoff))
        .await
        .unwrap();
    // Only the two newer rows count; both succeeded.
    assert!((inputs.success_rate - 1.0).abs() < 1e-9);
    assert_eq!(inputs.retry_count, 0);
    assert!((inputs.avg_latency_ms - 300.0).abs() < 1e-9);
}

#[tokio::test]
async fn from_audit_p99_uses_nearest_rank() {
    // 10 rows: 9 fast + 1 slow. Nearest-rank p99 over 10
    // samples lands on the top value (index 9, the slow one),
    // so the constructor must surface the tail latency.
    let mut rows = Vec::new();
    for i in 0..9 {
        rows.push(tool_result(false, 50.0, at(10) + chrono::Duration::seconds(i)));
    }
    rows.push(tool_result(false, 10_000.0, at(10) + chrono::Duration::seconds(20)));
    let reader = FakeAuditReader { rows };
    let inputs = QualityInputs::from_audit_history(&SkillId::new(), &reader, None)
        .await
        .unwrap();
    // 1.0 success rate; p99 picks the slow tail value.
    assert!((inputs.success_rate - 1.0).abs() < 1e-9);
    assert!(
        (inputs.p99_latency_ms - 10_000.0).abs() < 1e-9,
        "p99={} expected 10000",
        inputs.p99_latency_ms
    );
}

#[tokio::test]
async fn from_audit_skips_malformed_rows_without_error() {
    let rows = vec![
        AuditRow {
            kind: "tool_result".to_string(),
            // missing is_error + duration_ms entirely
            payload: serde_json::json!({"tool": "demo"}),
            created_at: at(10),
        },
        tool_result(false, 100.0, at(11)),
        AuditRow {
            kind: "tool_result".to_string(),
            // is_error present but duration_ms missing
            payload: serde_json::json!({"tool": "demo", "is_error": false}),
            created_at: at(12),
        },
        tool_result(true, 200.0, at(13)),
    ];
    let reader = FakeAuditReader { rows };
    let inputs = QualityInputs::from_audit_history(&SkillId::new(), &reader, None)
        .await
        .unwrap();
    // Two well-formed rows: 1 success + 1 failure → 0.5 success
    assert!((inputs.success_rate - 0.5).abs() < 1e-9);
    assert_eq!(inputs.retry_count, 1);
    assert!((inputs.avg_latency_ms - 150.0).abs() < 1e-9);
}

#[tokio::test]
async fn from_audit_ignores_non_tool_result_rows() {
    let rows = vec![
        AuditRow {
            kind: "tool_call".to_string(),
            payload: serde_json::json!({"tool": "demo", "call_id": "x"}),
            created_at: at(10),
        },
        tool_result(false, 200.0, at(11)),
        AuditRow {
            kind: "turn_start".to_string(),
            payload: serde_json::json!({}),
            created_at: at(12),
        },
        tool_result(true, 400.0, at(13)),
    ];
    let reader = FakeAuditReader { rows };
    let inputs = QualityInputs::from_audit_history(&SkillId::new(), &reader, None)
        .await
        .unwrap();
    // Only the two tool_result rows count: 1/2 success, 1 retry.
    assert!((inputs.success_rate - 0.5).abs() < 1e-9);
    assert_eq!(inputs.retry_count, 1);
    assert!((inputs.avg_latency_ms - 300.0).abs() < 1e-9);
}
