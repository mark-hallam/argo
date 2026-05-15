use super::*;
use argo_store::skill_rating::{record_rating, SkillVote};
use argo_store::SessionStore;
use argo_types::TenantId;
use std::sync::Arc;

async fn store() -> (Arc<SessionStore>, SkillStore) {
    let session = Arc::new(SessionStore::open(":memory:").await.expect("store"));
    session.migrate().await.expect("migrate");
    let skills = SkillStore::new(Arc::clone(&session));
    (session, skills)
}

fn skill_md(name: &str) -> String {
    format!(
        "---\n\
         name: {name}\n\
         version: 1.0.0\n\
         description: test skill\n\
         provenance:\n  source: published:verified:vendor\n  hash: sha256-test\n\
         ---\n\
         Run {name} safely.\n"
    )
}

fn config(min_ratings: u32, min_score: f32) -> PerWorkerLearningConfig {
    PerWorkerLearningConfig {
        since_micros: 0,
        tenant_id: None,
        min_ratings,
        min_score,
        vertical_id: "test-vertical".to_string(),
    }
}

async fn rate(session: &SessionStore, skill: &Skill, tenant: TenantId, vote: SkillVote) {
    record_rating(
        session.pool(),
        skill.id,
        &skill.frontmatter.version,
        "vendor",
        tenant,
        vote,
        None,
    )
    .await
    .expect("rating");
}

#[tokio::test]
async fn threshold_blocks_until_minimum_rating_count() {
    let (session, skills) = store().await;
    let source = skills
        .install(&skill_md("alpha"), None)
        .await
        .expect("skill");
    rate(&session, &source, TenantId::new(), SkillVote::Up).await;

    let report =
        per_worker_learning_tick_with_config(WorkerId::new(), &skills, config(2, 0.75), None)
            .await
            .expect("tick");

    assert_eq!(report.evaluated, 0);
    assert!(report.proposed.is_empty());
}

#[tokio::test]
async fn positive_trend_proposes_for_review_and_worker_overlay() {
    let (session, skills) = store().await;
    let worker = WorkerId::new();
    let source = skills
        .install(&skill_md("bravo"), None)
        .await
        .expect("skill");
    rate(&session, &source, TenantId::new(), SkillVote::Up).await;
    rate(&session, &source, TenantId::new(), SkillVote::Up).await;

    let report = per_worker_learning_tick_with_config(worker, &skills, config(2, 0.75), None)
        .await
        .expect("tick");

    assert_eq!(report.evaluated, 1);
    assert_eq!(report.proposed.len(), 1);
    assert_eq!(
        skills.list_pending_review().await.expect("pending").len(),
        1
    );
    let overlay_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM worker_skill WHERE worker_id = ?")
            .bind(worker.to_string())
            .fetch_one(session.pool())
            .await
            .expect("overlay count");
    assert_eq!(overlay_count, 1);
}

#[tokio::test]
async fn mixed_feedback_below_score_does_not_fire() {
    let (session, skills) = store().await;
    let source = skills
        .install(&skill_md("charlie"), None)
        .await
        .expect("skill");
    rate(&session, &source, TenantId::new(), SkillVote::Up).await;
    rate(&session, &source, TenantId::new(), SkillVote::Down).await;

    let report =
        per_worker_learning_tick_with_config(WorkerId::new(), &skills, config(2, 0.75), None)
            .await
            .expect("tick");

    assert_eq!(report.evaluated, 0);
    assert!(skills
        .list_pending_review()
        .await
        .expect("pending")
        .is_empty());
}

#[tokio::test]
async fn tenant_filter_prevents_cross_tenant_learning() {
    let (session, skills) = store().await;
    let worker_tenant = TenantId::new();
    let other_tenant = TenantId::new();
    let source = skills
        .install(&skill_md("delta"), None)
        .await
        .expect("skill");
    rate(&session, &source, other_tenant, SkillVote::Up).await;

    let mut cfg = config(1, 0.75);
    cfg.tenant_id = Some(worker_tenant);
    let report = per_worker_learning_tick_with_config(WorkerId::new(), &skills, cfg, None)
        .await
        .expect("tick");

    assert_eq!(report.evaluated, 0);
    assert!(report.proposed.is_empty());
}

#[tokio::test]
async fn tick_is_idempotent_and_writes_learning_audit() {
    let (session, skills) = store().await;
    let worker = WorkerId::new();
    let source = skills
        .install(&skill_md("echo"), None)
        .await
        .expect("skill");
    rate(&session, &source, TenantId::new(), SkillVote::Up).await;

    let first = per_worker_learning_tick_with_config(worker, &skills, config(1, 0.75), None)
        .await
        .expect("first");
    let second = per_worker_learning_tick_with_config(worker, &skills, config(1, 0.75), None)
        .await
        .expect("second");

    assert_eq!(first.proposed.len(), 1);
    assert!(second.proposed.is_empty());
    assert_eq!(second.skipped_existing, 1);
    let audit: String =
        sqlx::query_scalar("SELECT payload_json FROM audit_record WHERE action_type = ? LIMIT 1")
            .bind(LEARNING_ACTION)
            .fetch_one(session.pool())
            .await
            .expect("audit");
    assert!(audit.contains(LEARNING_ACTION));
    assert!(audit.contains(&worker.to_string()));
}
