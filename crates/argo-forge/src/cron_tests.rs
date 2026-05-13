use super::*;

async fn store() -> Arc<SessionStore> {
    let store = Arc::new(SessionStore::open(":memory:").await.expect("open store"));
    store.migrate().await.expect("migrate");
    store
}

async fn score(store: &SessionStore, skill: &str, score: i64, computed_at: i64) {
    sqlx::query(
        "INSERT INTO skill_quality_score (skill_id, score, inputs_json, computed_at) \
         VALUES (?, ?, '{}', ?)",
    )
    .bind(skill)
    .bind(score)
    .bind(computed_at)
    .execute(store.pool())
    .await
    .expect("insert score");
}

async fn queued_count(store: &SessionStore) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM forge_run WHERE status = 'queued'")
        .fetch_one(store.pool())
        .await
        .expect("count queued")
}

async fn queued_for(store: &SessionStore, skill: &str) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM forge_run WHERE status = 'queued' AND skill_name = ?")
        .bind(skill)
        .fetch_one(store.pool())
        .await
        .expect("count queued for skill")
}

#[tokio::test]
async fn tick_picks_n_lowest() {
    let store = store().await;
    for (skill, score_value) in [("a", 60), ("b", 65), ("c", 50), ("d", 80)] {
        score(&store, skill, score_value, score_value).await;
    }
    let cfg = ForgeCronConfig {
        max_skills_per_tick: 2,
        score_threshold: 70.0,
        ..Default::default()
    };
    assert_eq!(tick(Arc::clone(&store), &cfg).await.expect("tick"), 2);
    assert_eq!(queued_for(&store, "c").await, 1);
    assert_eq!(queued_for(&store, "a").await, 1);
    assert_eq!(queued_for(&store, "b").await, 0);
}

#[tokio::test]
async fn tick_skips_above_threshold() {
    let store = store().await;
    score(&store, "a", 70, 1).await;
    score(&store, "b", 71, 2).await;
    assert_eq!(
        tick(Arc::clone(&store), &ForgeCronConfig::default())
            .await
            .expect("tick"),
        0
    );
    assert_eq!(queued_count(&store).await, 0);
}

#[tokio::test]
async fn tick_skips_in_flight() {
    let store = store().await;
    score(&store, "a", 50, 1).await;
    store
        .enqueue_forge_run("a", "0.0.0", 0, "{}")
        .await
        .expect("enqueue existing");
    assert_eq!(
        tick(Arc::clone(&store), &ForgeCronConfig::default())
            .await
            .expect("tick"),
        0
    );
    assert_eq!(queued_for(&store, "a").await, 1);
}

#[tokio::test]
async fn tick_zero_candidates_spawns_nothing() {
    let store = store().await;
    assert_eq!(
        tick(Arc::clone(&store), &ForgeCronConfig::default())
            .await
            .expect("tick"),
        0
    );
    assert_eq!(queued_count(&store).await, 0);
}

#[tokio::test]
async fn dry_run_spawns_nothing() {
    let store = store().await;
    score(&store, "a", 50, 1).await;
    let cfg = ForgeCronConfig {
        dry_run: true,
        ..Default::default()
    };
    assert_eq!(tick(Arc::clone(&store), &cfg).await.expect("tick"), 0);
    assert_eq!(queued_count(&store).await, 0);
}

#[test]
fn interval_knob_respected() {
    let cfg = ForgeCronConfig::from_vars(|name| match name {
        "ARGO_FORGE_CRON_INTERVAL_SECS" => Some("12".into()),
        "ARGO_FORGE_CRON_MAX_SKILLS" => Some("7".into()),
        "ARGO_FORGE_CRON_THRESHOLD" => Some("42.5".into()),
        "ARGO_FORGE_CRON_DRY_RUN" => Some("true".into()),
        "ARGO_FORGE_CRON_BUDGET_USD" => Some("1.25".into()),
        _ => None,
    });
    assert_eq!(cfg.interval_secs, 12);
    assert_eq!(cfg.max_skills_per_tick, 7);
    assert_eq!(cfg.score_threshold, 42.5);
    assert!(cfg.dry_run);
    assert_eq!(cfg.budget_usd, 1.25);
}
