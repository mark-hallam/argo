use super::*;
use uuid::Uuid;

async fn store_with_rows(rows: &[(i64, serde_json::Value)]) -> SessionStore {
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    for (index, (created_at, payload)) in rows.iter().enumerate() {
        sqlx::query(
            "INSERT INTO audit_record \
             (id, session_id, kind, payload_json, parent_hash, self_hash, created_at) \
             VALUES (?, NULL, 'http.request', ?, x'00', x'01', ?)",
        )
        .bind(Uuid::from_u128(index as u128 + 1).as_bytes().as_slice())
        .bind(payload.to_string())
        .bind(created_at)
        .execute(store.pool())
        .await
        .unwrap();
    }
    store
}

#[tokio::test]
async fn error_rate_counts_total_errors_and_histogram() {
    let store = store_with_rows(&[
        (10, serde_json::json!({"status": 200, "path": "/ok"})),
        (20, serde_json::json!({"status": 503, "path": "/bad"})),
        (30, serde_json::json!({"status": 500, "path": "/bad"})),
    ])
    .await;
    let got = error_rate_in_window(&store, 0, 40).await.unwrap();
    assert_eq!((got.total, got.errors), (3, 2));
    assert_eq!(got.per_status_histogram.get(&200), Some(&1));
    assert_eq!(got.per_status_histogram.get(&500), Some(&1));
    assert_eq!(got.per_status_histogram.get(&503), Some(&1));
}

#[tokio::test]
async fn window_bounds_clip_rows() {
    let store = store_with_rows(&[
        (10, serde_json::json!({"status": 500})),
        (20, serde_json::json!({"status": 200})),
        (30, serde_json::json!({"status": 500})),
    ])
    .await;
    let got = error_rate_in_window(&store, 15, 25).await.unwrap();
    assert_eq!((got.total, got.errors), (1, 0));
    assert_eq!(got.per_status_histogram.get(&200), Some(&1));
}

#[tokio::test]
async fn tenant_filter_matches_payload_tenant() {
    let store = store_with_rows(&[
        (10, serde_json::json!({"status": 500, "tenant_id": "a"})),
        (20, serde_json::json!({"status": 500, "tenant_id": "b"})),
    ])
    .await;
    let filter = ErrorRateFilter {
        tenant_id: Some("a".into()),
        route: None,
    };
    let got = error_rate_in_window_filtered(&store, 0, 30, &filter)
        .await
        .unwrap();
    assert_eq!((got.total, got.errors), (1, 1));
}

#[tokio::test]
async fn route_filter_accepts_exact_and_trailing_star_prefix() {
    let store = store_with_rows(&[
        (
            10,
            serde_json::json!({"status": 500, "path": "/api/v1/workers/a"}),
        ),
        (
            20,
            serde_json::json!({"status": 200, "path": "/api/v1/sessions"}),
        ),
    ])
    .await;
    let prefix = ErrorRateFilter {
        tenant_id: None,
        route: Some("/api/v1/workers/*".into()),
    };
    let exact = ErrorRateFilter {
        tenant_id: None,
        route: Some("/api/v1/sessions".into()),
    };
    assert_eq!(
        error_rate_in_window_filtered(&store, 0, 30, &prefix)
            .await
            .unwrap()
            .total,
        1
    );
    assert_eq!(
        error_rate_in_window_filtered(&store, 0, 30, &exact)
            .await
            .unwrap()
            .errors,
        0
    );
}

#[tokio::test]
async fn availability_counts_success_and_failure_codes() {
    let store = store_with_rows(&[
        (10, serde_json::json!({"status": 200, "duration_ms": 5})),
        (20, serde_json::json!({"status": 404, "duration_ms": 6})),
        (30, serde_json::json!({"status": 500, "duration_ms": 7})),
        (40, serde_json::json!({"status": 502, "duration_ms": 8})),
    ])
    .await;
    let snap = availability_in_window(&store, 0, 50).await.unwrap();
    assert_eq!(snap.total, 4);
    assert_eq!(snap.success_count, 2);
    assert!((snap.success_pct() - 50.0).abs() < f64::EPSILON);
}

#[tokio::test]
async fn availability_tenant_filter_scopes_window() {
    let store = store_with_rows(&[
        (10, serde_json::json!({"status": 200, "tenant_id": "acme"})),
        (20, serde_json::json!({"status": 500, "tenant_id": "acme"})),
        (
            30,
            serde_json::json!({"status": 500, "tenant_id": "globex"}),
        ),
    ])
    .await;
    let filter = ErrorRateFilter {
        tenant_id: Some("acme".into()),
        route: None,
    };
    let snap = availability_in_window_filtered(&store, 0, 40, &filter)
        .await
        .unwrap();
    assert_eq!((snap.total, snap.success_count), (2, 1));
}

#[tokio::test]
async fn latency_percentiles_match_seeded_distribution() {
    // 100 samples: 1..=100ms. p50≈50ms, p95≈95ms, p99≈99ms.
    let mut rows = Vec::with_capacity(100);
    for i in 1u64..=100 {
        rows.push((
            i as i64,
            serde_json::json!({"status": 200, "duration_ms": i}),
        ));
    }
    let store = store_with_rows(&rows).await;
    let pcts = latency_percentiles_in_window(&store, 0, 200).await.unwrap();
    assert_eq!(pcts.total, 100);
    assert_eq!(pcts.p50_us, 50_000);
    assert_eq!(pcts.p95_us, 95_000);
    assert_eq!(pcts.p99_us, 99_000);
    assert!(pcts.p50_us <= pcts.p95_us);
    assert!(pcts.p95_us <= pcts.p99_us);
}

#[tokio::test]
async fn latency_percentiles_handle_empty_and_single_sample() {
    let empty = SessionStore::open(":memory:").await.unwrap();
    empty.migrate().await.unwrap();
    let pcts = latency_percentiles_in_window(&empty, 0, 100).await.unwrap();
    assert_eq!(
        (pcts.total, pcts.p50_us, pcts.p95_us, pcts.p99_us),
        (0, 0, 0, 0)
    );

    let one = store_with_rows(&[(10, serde_json::json!({"status": 200, "duration_ms": 42}))]).await;
    let pcts = latency_percentiles_in_window(&one, 0, 100).await.unwrap();
    assert_eq!((pcts.total, pcts.p50_us, pcts.p99_us), (1, 42_000, 42_000));
}

#[tokio::test]
async fn latency_percentiles_skip_rows_without_duration() {
    let store = store_with_rows(&[
        // "Timeout" rows lacking a duration — treated as outside the sample set.
        (10, serde_json::json!({"status": 504})),
        (20, serde_json::json!({"status": 200, "duration_ms": 1})),
        (30, serde_json::json!({"status": 500, "duration_ms": 9})),
    ])
    .await;
    let pcts = latency_percentiles_in_window(&store, 0, 40).await.unwrap();
    assert_eq!(pcts.total, 2);
    assert_eq!(pcts.p50_us, 1_000);
    assert_eq!(pcts.p99_us, 9_000);
}

#[tokio::test]
async fn status_code_fallback_accepts_legacy_key() {
    let store = store_with_rows(&[
        (10, serde_json::json!({"status_code": 502})),
        (20, serde_json::json!({"path": "/missing-status"})),
    ])
    .await;
    let got = error_rate_in_window(&store, 0, 30).await.unwrap();
    assert_eq!((got.total, got.errors), (1, 1));
    assert_eq!(got.per_status_histogram.get(&502), Some(&1));
}
