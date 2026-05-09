//! Pool-level tests for `worker_okr`. The handler layer drives the
//! full CLI round-trip; these focus on SQL invariants + the brief's
//! progress arithmetic so the math is asserted independently of stdout.

use super::*;
use crate::SessionStore;
use sqlx::SqlitePool;

async fn pool() -> SqlitePool {
    let store = SessionStore::open(":memory:").await.expect("open store");
    store.migrate().await.expect("migrate");
    crate::worker_kpi::ensure_schema(store.pool())
        .await
        .expect("apply kpi schema");
    ensure_schema(store.pool()).await.expect("apply okr schema");
    store.pool().clone()
}

fn kr(kpi: &str, op: Comparison, target: f64, weight: f64) -> KeyResultSpec {
    KeyResultSpec {
        kpi_name: kpi.into(),
        comparison: op,
        target_value: target,
        weight,
    }
}

#[tokio::test]
async fn set_then_list_round_trip() {
    let pool = pool().await;
    let worker = "11111111-1111-1111-1111-111111111111";
    let tenant = "22222222-2222-2222-2222-222222222222";
    let id = set_objective(
        &pool,
        worker,
        tenant,
        "2026Q2",
        "Lift CSAT",
        Some("Drive csat above 4.5 across the quarter"),
        &[
            kr("csat", Comparison::Gte, 4.5, 1.0),
            kr("reply_sla_hours", Comparison::Lte, 12.0, 0.5),
        ],
    )
    .await
    .expect("set");
    assert!(id > 0);

    let rows = list_objectives(&pool, worker, tenant, None).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].title, "Lift CSAT");
    assert_eq!(rows[0].period, "2026Q2");
    assert_eq!(rows[0].status, ObjectiveStatus::Active);

    let krs = key_results(&pool, id).await.unwrap();
    assert_eq!(krs.len(), 2);
    assert_eq!(krs[0].kpi_name, "csat");
    assert_eq!(krs[0].comparison, Comparison::Gte);
    assert_eq!(krs[1].comparison, Comparison::Lte);
}

#[tokio::test]
async fn set_rejects_empty_krs() {
    let pool = pool().await;
    let err = set_objective(&pool, "w", "t", "2026Q2", "x", None, &[])
        .await
        .expect_err("empty rejected");
    assert!(err.to_string().contains("--kr"));
}

#[tokio::test]
async fn set_rejects_blank_title() {
    let pool = pool().await;
    let err = set_objective(
        &pool,
        "w",
        "t",
        "2026Q2",
        "   ",
        None,
        &[kr("csat", Comparison::Gte, 4.0, 1.0)],
    )
    .await
    .expect_err("blank rejected");
    assert!(err.to_string().contains("title"));
}

#[tokio::test]
async fn list_filters_by_period() {
    let pool = pool().await;
    let worker = "33333333-3333-3333-3333-333333333333";
    let tenant = "44444444-4444-4444-4444-444444444444";
    set_objective(
        &pool,
        worker,
        tenant,
        "2026Q1",
        "old",
        None,
        &[kr("csat", Comparison::Gte, 4.0, 1.0)],
    )
    .await
    .unwrap();
    set_objective(
        &pool,
        worker,
        tenant,
        "2026Q2",
        "new",
        None,
        &[kr("csat", Comparison::Gte, 4.5, 1.0)],
    )
    .await
    .unwrap();

    let q2 = list_objectives(&pool, worker, tenant, Some("2026Q2"))
        .await
        .unwrap();
    assert_eq!(q2.len(), 1);
    assert_eq!(q2[0].title, "new");

    let empty = list_objectives(&pool, worker, tenant, Some("2027Q1"))
        .await
        .unwrap();
    assert!(empty.is_empty(), "missing period returns no rows");
}

#[tokio::test]
async fn list_isolates_by_tenant() {
    let pool = pool().await;
    let worker = "55555555-5555-5555-5555-555555555555";
    let tenant_a = "aaaaaaa1-aaaa-aaaa-aaaa-aaaaaaaaaaaa";
    let tenant_b = "aaaaaaa2-aaaa-aaaa-aaaa-aaaaaaaaaaaa";
    set_objective(
        &pool,
        worker,
        tenant_a,
        "2026Q2",
        "alpha",
        None,
        &[kr("csat", Comparison::Gte, 4.0, 1.0)],
    )
    .await
    .unwrap();
    set_objective(
        &pool,
        worker,
        tenant_b,
        "2026Q2",
        "beta",
        None,
        &[kr("csat", Comparison::Gte, 4.0, 1.0)],
    )
    .await
    .unwrap();
    let a = list_objectives(&pool, worker, tenant_a, None).await.unwrap();
    assert_eq!(a.len(), 1);
    assert_eq!(a[0].title, "alpha");
    let b = list_objectives(&pool, worker, tenant_b, None).await.unwrap();
    assert_eq!(b.len(), 1);
    assert_eq!(b[0].title, "beta");
}

#[test]
fn key_result_spec_parses_token() {
    let parsed = KeyResultSpec::parse("csat:gte:4.5:1.0").unwrap();
    assert_eq!(parsed.kpi_name, "csat");
    assert_eq!(parsed.comparison, Comparison::Gte);
    assert!((parsed.target_value - 4.5).abs() < f64::EPSILON);
    assert!((parsed.weight - 1.0).abs() < f64::EPSILON);
}

#[test]
fn key_result_spec_rejects_zero_weight() {
    let err = KeyResultSpec::parse("csat:gte:4.5:0").expect_err("zero rejected");
    assert!(err.to_string().contains("weight"));
}

#[test]
fn key_result_spec_rejects_bad_op() {
    let err = KeyResultSpec::parse("csat:wat:4.5:1.0").expect_err("bad op rejected");
    assert!(err.to_string().contains("comparison"));
}

#[test]
fn key_result_spec_rejects_wrong_arity() {
    let err = KeyResultSpec::parse("csat:gte:4.5").expect_err("arity rejected");
    assert!(err.to_string().contains("kpi:op:target:weight"));
}

#[test]
fn achievement_passes_when_gte_meets_target() {
    let (ratio, ok) = achievement(Some(4.7), 4.5, Comparison::Gte);
    assert!(ok);
    assert!((ratio - 1.0).abs() < f64::EPSILON);
}

#[test]
fn achievement_partial_when_gte_below_target() {
    let (ratio, ok) = achievement(Some(3.0), 4.0, Comparison::Gte);
    assert!(!ok);
    assert!((ratio - 0.75).abs() < f64::EPSILON);
}

#[test]
fn achievement_partial_when_lte_above_target() {
    // Brief: lte ratio is target/actual when comparison fails.
    let (ratio, ok) = achievement(Some(20.0), 10.0, Comparison::Lte);
    assert!(!ok);
    assert!((ratio - 0.5).abs() < f64::EPSILON);
}

#[test]
fn achievement_zero_when_no_observation() {
    let (ratio, ok) = achievement(None, 4.5, Comparison::Gte);
    assert!(!ok);
    assert_eq!(ratio, 0.0);
}

#[test]
fn weighted_score_combines_per_kr() {
    let progress = vec![
        KeyResultProgress {
            key_result: KeyResultRow {
                id: 1,
                objective_id: 1,
                kpi_name: "csat".into(),
                target_value: 4.5,
                comparison: Comparison::Gte,
                weight: 2.0,
            },
            actual_value: Some(4.7),
            achievement: 1.0,
            achieved: true,
        },
        KeyResultProgress {
            key_result: KeyResultRow {
                id: 2,
                objective_id: 1,
                kpi_name: "reply_sla_hours".into(),
                target_value: 12.0,
                comparison: Comparison::Lte,
                weight: 1.0,
            },
            actual_value: Some(24.0),
            achievement: 0.5,
            achieved: false,
        },
    ];
    let score = weighted_score(&progress);
    // (2*1 + 1*0.5) / 3 = 2.5/3 = 0.8333...
    assert!((score - (2.5 / 3.0)).abs() < 1e-9, "got {score}");
}

#[tokio::test]
async fn progress_joins_latest_kpi_value() {
    let pool = pool().await;
    let worker = "66666666-6666-6666-6666-666666666666";
    let tenant = "77777777-7777-7777-7777-777777777777";
    let _id = set_objective(
        &pool,
        worker,
        tenant,
        "2026Q2",
        "Lift CSAT",
        None,
        &[
            kr("csat", Comparison::Gte, 4.5, 2.0),
            kr("reply_sla_hours", Comparison::Lte, 12.0, 1.0),
        ],
    )
    .await
    .unwrap();

    // csat passes (4.7 >= 4.5), reply_sla_hours fails (24 > 12 → 12/24 = 0.5).
    crate::worker_kpi::record_value(&pool, worker, tenant, "csat", 4.7, None)
        .await
        .unwrap();
    crate::worker_kpi::record_value(&pool, worker, tenant, "reply_sla_hours", 24.0, None)
        .await
        .unwrap();

    let reports = progress(&pool, worker, tenant, Some("2026Q2"))
        .await
        .unwrap();
    assert_eq!(reports.len(), 1);
    let report = &reports[0];
    assert_eq!(report.key_results.len(), 2);
    assert!(report.key_results[0].achieved, "csat KR achieved");
    assert!(!report.key_results[1].achieved, "sla KR not achieved");
    assert!(
        (report.weighted_score - (2.5 / 3.0)).abs() < 1e-9,
        "weighted score: {}",
        report.weighted_score
    );
}

#[tokio::test]
async fn progress_uses_latest_value_when_multiple_observations() {
    let pool = pool().await;
    let worker = "88888888-8888-8888-8888-888888888888";
    let tenant = "99999999-9999-9999-9999-999999999999";
    set_objective(
        &pool,
        worker,
        tenant,
        "2026Q2",
        "Lift CSAT",
        None,
        &[kr("csat", Comparison::Gte, 4.5, 1.0)],
    )
    .await
    .unwrap();
    crate::worker_kpi::record_value(&pool, worker, tenant, "csat", 3.0, None)
        .await
        .unwrap();
    crate::worker_kpi::record_value(&pool, worker, tenant, "csat", 4.8, None)
        .await
        .unwrap();
    let reports = progress(&pool, worker, tenant, None).await.unwrap();
    assert_eq!(reports.len(), 1);
    assert!(reports[0].key_results[0].achieved);
    assert!((reports[0].weighted_score - 1.0).abs() < f64::EPSILON);
}

#[test]
fn comparison_round_trip() {
    for op in [Comparison::Gte, Comparison::Lte, Comparison::Eq] {
        let parsed = Comparison::parse(op.as_str()).unwrap();
        assert_eq!(parsed, op);
    }
    assert!(Comparison::parse("ne").is_err());
}
