use argo_skill::{
    per_worker_learning_tick_with_config, LifecycleState, PerWorkerLearningConfig, SkillStore,
};
use argo_store::skill_rating::{record_rating, SkillVote};
use argo_store::{SessionStore, WorkerId};
use argo_types::TenantId;
use std::sync::Arc;

fn skill_md(name: &str) -> String {
    format!(
        "---\n\
         name: {name}\n\
         version: 1.0.0\n\
         description: worker overlay visibility regression\n\
         provenance:\n  source: published:verified:vendor\n  hash: sha256-test\n\
         ---\n\
         Run {name} safely.\n"
    )
}

#[tokio::test]
async fn cron_promoted_worker_overlay_loads_for_worker_session() {
    let session = Arc::new(SessionStore::open(":memory:").await.expect("store"));
    session.migrate().await.expect("migrate");
    let skills = SkillStore::new(Arc::clone(&session));
    let tenant = TenantId::new();
    let worker = WorkerId::new();
    let source = skills
        .install(&skill_md("visibility_overlay"), None)
        .await
        .expect("install source skill");

    record_rating(
        session.pool(),
        source.id,
        &source.frontmatter.version,
        "vendor",
        tenant,
        SkillVote::Up,
        None,
    )
    .await
    .expect("rating");

    let report = per_worker_learning_tick_with_config(
        worker,
        &skills,
        PerWorkerLearningConfig {
            since_micros: 0,
            tenant_id: Some(tenant),
            min_ratings: 1,
            min_score: 0.75,
            vertical_id: "worker-overlay-visibility-regression".to_string(),
        },
        None,
    )
    .await
    .expect("learning tick");

    assert_eq!(report.proposed.len(), 1);
    let pending = skills.list_pending_review().await.expect("pending review");
    assert_eq!(pending.len(), 1);
    skills
        .transition_lifecycle(pending[0].id, LifecycleState::Active)
        .await
        .expect("operator approves overlay candidate");

    let overlays = skills
        .list_for_worker(worker)
        .await
        .expect("worker overlays");
    assert_eq!(overlays.len(), 1);
    assert!(overlays[0]
        .frontmatter
        .name
        .starts_with("visibility_overlay__worker_overlay_"));
    assert_eq!(overlays[0].body, "Run visibility_overlay safely.\n");

    let other_worker = skills
        .list_for_worker(WorkerId::new())
        .await
        .expect("other worker overlays");
    assert!(other_worker.is_empty());
}
