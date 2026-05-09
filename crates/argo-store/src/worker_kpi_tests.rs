//! Pool-level tests for `worker_kpi`. The handler layer drives the
//! full CLI round-trip; these focus on SQL invariants: insert,
//! tenant isolation, KPI-name filter, since filter, and bucket math.

use super::*;
use crate::SessionStore;
use sqlx::SqlitePool;

async fn pool() -> SqlitePool {
    let store = SessionStore::open(":memory:").await.expect("open store");
    store.migrate().await.expect("migrate");
    ensure_schema(store.pool()).await.expect("apply kpi schema");
    store.pool().clone()
}

#[tokio::test]
async fn record_then_list_round_trip() {
    let pool = pool().await;
    let worker = "11111111-1111-1111-1111-111111111111";
    let tenant = "22222222-2222-2222-2222-222222222222";

    let id = record_value(&pool, worker, tenant, "csat", 4.6, Some("alpha"))
        .await
        .expect("record");
    assert!(id > 0);

    let rows = list_values(&pool, worker, tenant, None, None)
        .await
        .expect("list");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].kpi_name, "csat");
    assert!((rows[0].value - 4.6).abs() < f64::EPSILON);
    assert_eq!(rows[0].note.as_deref(), Some("alpha"));
}

#[tokio::test]
async fn list_isolates_by_tenant() {
    let pool = pool().await;
    let worker = "33333333-3333-3333-3333-333333333333";
    let tenant_a = "aaaaaaa1-aaaa-aaaa-aaaa-aaaaaaaaaaaa";
    let tenant_b = "aaaaaaa2-aaaa-aaaa-aaaa-aaaaaaaaaaaa";

    record_value(&pool, worker, tenant_a, "csat", 1.0, None)
        .await
        .unwrap();
    record_value(&pool, worker, tenant_b, "csat", 2.0, None)
        .await
        .unwrap();

    let only_a = list_values(&pool, worker, tenant_a, None, None).await.unwrap();
    assert_eq!(only_a.len(), 1);
    assert!((only_a[0].value - 1.0).abs() < f64::EPSILON);

    let only_b = list_values(&pool, worker, tenant_b, None, None).await.unwrap();
    assert_eq!(only_b.len(), 1);
    assert!((only_b[0].value - 2.0).abs() < f64::EPSILON);
}

#[tokio::test]
async fn list_filters_by_kpi_name() {
    let pool = pool().await;
    let worker = "44444444-4444-4444-4444-444444444444";
    let tenant = "55555555-5555-5555-5555-555555555555";

    record_value(&pool, worker, tenant, "csat", 4.0, None).await.unwrap();
    record_value(&pool, worker, tenant, "reply_sla_hours", 12.0, None)
        .await
        .unwrap();
    record_value(&pool, worker, tenant, "csat", 4.5, None).await.unwrap();

    let csat = list_values(&pool, worker, tenant, Some("csat"), None)
        .await
        .unwrap();
    assert_eq!(csat.len(), 2);
    assert!(csat.iter().all(|r| r.kpi_name == "csat"));

    let sla = list_values(&pool, worker, tenant, Some("reply_sla_hours"), None)
        .await
        .unwrap();
    assert_eq!(sla.len(), 1);
    assert!((sla[0].value - 12.0).abs() < f64::EPSILON);
}

#[tokio::test]
async fn list_filters_by_since_micros() {
    let pool = pool().await;
    let worker = "66666666-6666-6666-6666-666666666666";
    let tenant = "77777777-7777-7777-7777-777777777777";

    record_value(&pool, worker, tenant, "csat", 1.0, None).await.unwrap();
    record_value(&pool, worker, tenant, "csat", 2.0, None).await.unwrap();
    record_value(&pool, worker, tenant, "csat", 3.0, None).await.unwrap();

    // Bump observed_at deterministically so the filter is meaningful
    // without depending on test wall-clock spacing.
    sqlx::query("UPDATE worker_kpi_value SET observed_at = id * 1000 WHERE worker_id = ?")
        .bind(worker)
        .execute(&pool)
        .await
        .unwrap();

    let recent = list_values(&pool, worker, tenant, None, Some(2_500))
        .await
        .unwrap();
    assert_eq!(recent.len(), 1);
    assert!((recent[0].value - 3.0).abs() < f64::EPSILON);

    let everything = list_values(&pool, worker, tenant, None, Some(0))
        .await
        .unwrap();
    assert_eq!(everything.len(), 3);
}

#[tokio::test]
async fn record_rejects_non_finite_values() {
    let pool = pool().await;
    let err = record_value(
        &pool,
        "88888888-8888-8888-8888-888888888888",
        "99999999-9999-9999-9999-999999999999",
        "csat",
        f64::NAN,
        None,
    )
    .await
    .expect_err("nan rejected");
    assert!(err.to_string().contains("finite"));
}

#[tokio::test]
async fn record_rejects_empty_kpi_name() {
    let pool = pool().await;
    let err = record_value(
        &pool,
        "88888888-8888-8888-8888-888888888888",
        "99999999-9999-9999-9999-999999999999",
        "",
        1.0,
        None,
    )
    .await
    .expect_err("empty rejected");
    assert!(err.to_string().contains("kpi_name"));
}

#[test]
fn bucket_rows_groups_by_day_and_aggregates() {
    use chrono::TimeZone;
    let day_a = Utc.with_ymd_and_hms(2026, 5, 1, 9, 0, 0).unwrap().timestamp_micros();
    let day_a_late = Utc
        .with_ymd_and_hms(2026, 5, 1, 21, 0, 0)
        .unwrap()
        .timestamp_micros();
    let day_b = Utc.with_ymd_and_hms(2026, 5, 2, 9, 0, 0).unwrap().timestamp_micros();
    let rows = vec![
        row(day_a, 4.0),
        row(day_a_late, 6.0),
        row(day_b, 2.0),
    ];
    let buckets = bucket_rows(&rows, Bucket::Day);
    assert_eq!(buckets.len(), 2);

    let day_a_bucket = buckets.iter().find(|b| b.bucket == "2026-05-01").unwrap();
    assert_eq!(day_a_bucket.count, 2);
    assert!((day_a_bucket.avg - 5.0).abs() < f64::EPSILON);
    assert!((day_a_bucket.min - 4.0).abs() < f64::EPSILON);
    assert!((day_a_bucket.max - 6.0).abs() < f64::EPSILON);

    let day_b_bucket = buckets.iter().find(|b| b.bucket == "2026-05-02").unwrap();
    assert_eq!(day_b_bucket.count, 1);
    assert!((day_b_bucket.avg - 2.0).abs() < f64::EPSILON);
}

#[test]
fn bucket_rows_handles_week_and_month() {
    use chrono::TimeZone;
    // Friday + Monday of the same ISO week → both land in the
    // Monday-stamped bucket.
    let fri = Utc.with_ymd_and_hms(2026, 5, 1, 12, 0, 0).unwrap().timestamp_micros();
    let mon = Utc.with_ymd_and_hms(2026, 4, 27, 9, 0, 0).unwrap().timestamp_micros();
    // Different month / week.
    let next_month = Utc
        .with_ymd_and_hms(2026, 6, 3, 12, 0, 0)
        .unwrap()
        .timestamp_micros();

    let rows = vec![row(fri, 4.0), row(mon, 6.0), row(next_month, 1.0)];
    let week = bucket_rows(&rows, Bucket::Week);
    assert_eq!(week.len(), 2);
    let same_week = week.iter().find(|b| b.bucket == "2026-04-27").unwrap();
    assert_eq!(same_week.count, 2);
    assert!((same_week.avg - 5.0).abs() < f64::EPSILON);

    let month = bucket_rows(&rows, Bucket::Month);
    // May 1 → 2026-05, Apr 27 → 2026-04, Jun 3 → 2026-06 — three
    // distinct calendar months even though the first two share an
    // ISO week.
    assert_eq!(month.len(), 3);
    let may = month.iter().find(|b| b.bucket == "2026-05").unwrap();
    assert_eq!(may.count, 1);
    let apr = month.iter().find(|b| b.bucket == "2026-04").unwrap();
    assert_eq!(apr.count, 1);
}

#[tokio::test]
async fn series_round_trips_through_pool() {
    let pool = pool().await;
    let worker = "abababab-abab-abab-abab-abababababab";
    let tenant = "cdcdcdcd-cdcd-cdcd-cdcd-cdcdcdcdcdcd";
    record_value(&pool, worker, tenant, "csat", 4.0, None).await.unwrap();
    record_value(&pool, worker, tenant, "csat", 5.0, None).await.unwrap();
    // Different KPI — must not bleed into the series.
    record_value(&pool, worker, tenant, "reply_sla_hours", 99.0, None)
        .await
        .unwrap();

    let buckets = series(&pool, worker, tenant, "csat", Bucket::Day)
        .await
        .unwrap();
    let total: i64 = buckets.iter().map(|b| b.count).sum();
    assert_eq!(total, 2, "only csat rows counted");
}

#[test]
fn bucket_parse_round_trip() {
    assert_eq!(Bucket::parse("day").unwrap(), Bucket::Day);
    assert_eq!(Bucket::parse("week").unwrap(), Bucket::Week);
    assert_eq!(Bucket::parse("month").unwrap(), Bucket::Month);
    assert!(Bucket::parse("hour").is_err());
}

fn row(observed_at: i64, value: f64) -> KpiValueRow {
    KpiValueRow {
        id: 0,
        worker_id: "w".into(),
        tenant_id: "t".into(),
        kpi_name: "csat".into(),
        value,
        observed_at,
        note: None,
    }
}
