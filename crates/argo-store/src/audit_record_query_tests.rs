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
