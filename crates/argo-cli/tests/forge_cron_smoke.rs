use argo_forge::{spawn_forge_cron, ForgeCronConfig};
use argo_store::SessionStore;
use std::sync::Arc;
use std::time::Duration;

#[tokio::test]
async fn forge_cron_enqueues_two_lowest_quality_skills() {
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    for (idx, (skill, score)) in [
        ("alpha", 60_i64),
        ("beta", 65),
        ("gamma", 75),
        ("delta", 80),
        ("epsilon", 85),
    ]
    .into_iter()
    .enumerate()
    {
        sqlx::query(
            "INSERT INTO skill_quality_score (skill_id, score, inputs_json, computed_at) \
             VALUES (?, ?, '{}', ?)",
        )
        .bind(skill)
        .bind(score)
        .bind(i64::try_from(idx + 1).unwrap())
        .execute(store.pool())
        .await
        .unwrap();
    }
    let cfg = ForgeCronConfig {
        interval_secs: 1,
        max_skills_per_tick: 2,
        score_threshold: 70.0,
        ..ForgeCronConfig::default()
    };

    let handle = spawn_forge_cron(Arc::clone(&store), cfg);
    tokio::time::sleep(Duration::from_millis(1_200)).await;
    handle.abort();

    let rows: Vec<(String,)> =
        sqlx::query_as("SELECT skill_name FROM forge_run ORDER BY skill_name")
            .fetch_all(store.pool())
            .await
            .unwrap();
    assert_eq!(rows, vec![("alpha".to_string(),), ("beta".to_string(),)]);
}
