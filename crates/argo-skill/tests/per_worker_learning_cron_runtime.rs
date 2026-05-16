use argo_skill::{spawn_per_worker_learning_cron, PerWorkerLearningConfig, SkillStore};
use argo_store::skill_rating::{record_rating, SkillVote};
use argo_store::{SessionStore, WorkerId};
use argo_types::TenantId;
use std::sync::Arc;
use std::time::Duration;

fn skill_md(name: &str) -> String {
    format!(
        "---\n\
         name: {name}\n\
         version: 1.0.0\n\
         description: runtime cron test skill\n\
         provenance:\n  source: published:verified:vendor\n  hash: sha256-test\n\
         ---\n\
         Run {name} safely.\n"
    )
}

#[tokio::test]
async fn spawned_cron_proposes_overlay_without_direct_tick() {
    let session = Arc::new(SessionStore::open(":memory:").await.expect("store"));
    session.migrate().await.expect("migrate");
    let skills = Arc::new(SkillStore::new(Arc::clone(&session)));
    let tenant = TenantId::new();
    let worker = WorkerId::new();
    let source = skills
        .install(&skill_md("runtime_overlay"), None)
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

    let cfg = PerWorkerLearningConfig {
        since_micros: 0,
        tenant_id: Some(tenant),
        min_ratings: 1,
        min_score: 0.75,
        vertical_id: "runtime-cron-test".to_string(),
    };
    let handle =
        spawn_per_worker_learning_cron(worker, Arc::clone(&skills), cfg, Duration::from_millis(20));

    let proposed = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let pending = skills.list_pending_review().await.expect("pending");
            if pending.len() == 1 {
                break pending;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("cron should propose before timeout");
    handle.abort();

    assert!(proposed[0]
        .frontmatter
        .name
        .starts_with("runtime_overlay__worker_overlay_"));
}
