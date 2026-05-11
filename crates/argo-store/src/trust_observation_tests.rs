use super::*;
use crate::SessionStore;
use chrono::Duration;

async fn fresh_store() -> SessionStore {
    let store = SessionStore::open(":memory:").await.expect("open");
    store.migrate().await.expect("migrate");
    store
}

#[tokio::test]
async fn record_and_list_round_trips_one_row() {
    let store = fresh_store().await;
    let source = TenantId::new();
    let consumer = TenantId::new();
    let skill = SkillId::new();
    let observed_at = Utc::now();

    let id = record_observation(
        &store,
        &NewTrustObservation {
            source_tenant: source,
            consumer_tenant: consumer,
            skill_id: skill,
            quality_score: 0.75,
            observed_at,
        },
    )
    .await
    .expect("record");

    let rows = list_observations_in_window(&store, consumer, observed_at - Duration::days(1))
        .await
        .expect("list");
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert_eq!(row.id, id);
    assert_eq!(row.source_tenant, source);
    assert_eq!(row.consumer_tenant, consumer);
    assert_eq!(row.skill_id, skill);
    assert!((row.quality_score - 0.75).abs() < 1e-5);
}

#[tokio::test]
async fn list_observations_filters_by_consumer() {
    let store = fresh_store().await;
    let source = TenantId::new();
    let consumer_a = TenantId::new();
    let consumer_b = TenantId::new();
    let skill = SkillId::new();
    let now = Utc::now();

    record_observation(
        &store,
        &NewTrustObservation {
            source_tenant: source,
            consumer_tenant: consumer_a,
            skill_id: skill,
            quality_score: 0.9,
            observed_at: now,
        },
    )
    .await
    .unwrap();
    record_observation(
        &store,
        &NewTrustObservation {
            source_tenant: source,
            consumer_tenant: consumer_b,
            skill_id: skill,
            quality_score: 0.3,
            observed_at: now,
        },
    )
    .await
    .unwrap();

    let rows_a = list_observations_in_window(&store, consumer_a, now - Duration::days(1))
        .await
        .unwrap();
    assert_eq!(rows_a.len(), 1);
    assert_eq!(rows_a[0].consumer_tenant, consumer_a);
}

#[tokio::test]
async fn list_observations_filters_by_window() {
    let store = fresh_store().await;
    let source = TenantId::new();
    let consumer = TenantId::new();
    let skill = SkillId::new();
    let now = Utc::now();
    let old = now - Duration::days(120);

    record_observation(
        &store,
        &NewTrustObservation {
            source_tenant: source,
            consumer_tenant: consumer,
            skill_id: skill,
            quality_score: 0.5,
            observed_at: old,
        },
    )
    .await
    .unwrap();
    record_observation(
        &store,
        &NewTrustObservation {
            source_tenant: source,
            consumer_tenant: consumer,
            skill_id: skill,
            quality_score: 0.5,
            observed_at: now,
        },
    )
    .await
    .unwrap();

    let in_window =
        list_observations_in_window(&store, consumer, now - Duration::days(90))
            .await
            .unwrap();
    assert_eq!(in_window.len(), 1, "row outside 90d window should not return");
}

#[tokio::test]
async fn list_all_observations_returns_every_consumer() {
    let store = fresh_store().await;
    let source = TenantId::new();
    let consumer_a = TenantId::new();
    let consumer_b = TenantId::new();
    let skill = SkillId::new();
    let now = Utc::now();

    for consumer in [consumer_a, consumer_b] {
        record_observation(
            &store,
            &NewTrustObservation {
                source_tenant: source,
                consumer_tenant: consumer,
                skill_id: skill,
                quality_score: 0.6,
                observed_at: now,
            },
        )
        .await
        .unwrap();
    }

    let all = list_all_observations_since(&store, now - Duration::days(1))
        .await
        .unwrap();
    assert_eq!(all.len(), 2);
}
