//! Tests for the `argo skill distill-now` audit-emission leg (DEC-042 p1c).
//!
//! Criterion (b): a distill-now run with a mock caller emits a
//! `skill.distilled` audit row carrying a non-placeholder `tenant_id` plus
//! the `model` + cost fields — so the real Dreams provider spend on this
//! CLI path is tamper-evident, not just stdout.

use super::run;
use argo_skill::dreams_distillation::{Cohort, DistillationResult, SkillDistillationCaller};
use argo_skill::{SkillStore, PLACEHOLDER_TENANT_ID};
use argo_store::SessionStore;
use argo_types::ArgoResult;
use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

/// Deterministic mock — returns a fixed body/model/cost without a network.
struct MockCaller;

#[async_trait]
impl SkillDistillationCaller for MockCaller {
    async fn distill(&self, cohort: &Cohort) -> ArgoResult<DistillationResult> {
        Ok(DistillationResult {
            body: format!("# {}\n\ndistilled body\n", cohort.action_type),
            model: "mock-dreams-1".to_string(),
            cost_micros: 123_456,
        })
    }
}

/// Three identical trajectories under one `(vertical, action)` → one cohort
/// (cosine similarity 1.0 ≥ the 0.7 membership floor).
fn one_cohort_json() -> String {
    let traj = serde_json::json!([
        {"session_id": "sess-aaaa1111", "action_type": "refund",
         "trajectory": ["lookup", "verify", "refund"], "final_text": "done",
         "success_metric": 0.9, "vertical_id": "support"},
        {"session_id": "sess-bbbb2222", "action_type": "refund",
         "trajectory": ["lookup", "verify", "refund"], "final_text": "done",
         "success_metric": 0.9, "vertical_id": "support"},
        {"session_id": "sess-cccc3333", "action_type": "refund",
         "trajectory": ["lookup", "verify", "refund"], "final_text": "done",
         "success_metric": 0.9, "vertical_id": "support"},
    ]);
    traj.to_string()
}

#[tokio::test]
async fn distill_now_emits_skill_distilled_with_tenant_model_and_cost() {
    let store = Arc::new(SessionStore::open(":memory:").await.expect("store"));
    store.migrate().await.expect("migrate");
    let skills = SkillStore::new(Arc::clone(&store));
    let tenant = Uuid::now_v7().to_string();

    let state_root = std::env::temp_dir().join(format!("distill-now-test-{}", Uuid::now_v7()));
    let mut reader = std::io::Cursor::new(one_cohort_json().into_bytes());

    run(
        "skill-distillation",
        &state_root,
        &MockCaller,
        &mut reader,
        true,
        &skills,
        Some(&tenant),
    )
    .await
    .expect("distill-now run");

    // Exactly one tamper-evident `skill.distilled` row, bound to the real
    // tenant (never the migration placeholder), with model + cost populated.
    let rows: Vec<(String, Option<String>)> = sqlx::query_as(
        "SELECT payload_json, tenant_id FROM audit_record WHERE kind = 'skill.distilled'",
    )
    .fetch_all(store.pool())
    .await
    .expect("distilled rows");
    assert_eq!(rows.len(), 1, "one distilled audit row");
    let (payload_json, tenant_col) = &rows[0];
    assert_eq!(tenant_col.as_deref(), Some(tenant.as_str()), "tenant column stamped");
    assert_ne!(
        tenant_col.as_deref(),
        Some(PLACEHOLDER_TENANT_ID),
        "must not be the migration placeholder"
    );
    let payload: serde_json::Value = serde_json::from_str(payload_json).expect("payload json");
    assert_eq!(payload["tenant_id"], tenant);
    assert_eq!(payload["model"], "mock-dreams-1");
    assert_eq!(payload["cost_usd_micros"], 123_456);
    assert_eq!(payload["vertical"], "support");
    assert_eq!(payload["action"], "refund");
    assert_eq!(payload["size"], 3);
    assert!(
        payload["skill_name"].as_str().is_some_and(|s| !s.is_empty()),
        "candidate skill name carried"
    );
}

/// `tenant = None` → no audit row (no tenant to attribute the spend to);
/// the distillation itself still runs (the summary still reports it).
#[tokio::test]
async fn distill_now_without_tenant_emits_no_audit_row() {
    let store = Arc::new(SessionStore::open(":memory:").await.expect("store"));
    store.migrate().await.expect("migrate");
    let skills = SkillStore::new(Arc::clone(&store));

    let state_root = std::env::temp_dir().join(format!("distill-now-test-{}", Uuid::now_v7()));
    let mut reader = std::io::Cursor::new(one_cohort_json().into_bytes());

    run(
        "skill-distillation",
        &state_root,
        &MockCaller,
        &mut reader,
        true,
        &skills,
        None,
    )
    .await
    .expect("distill-now run");

    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM audit_record WHERE kind = 'skill.distilled'")
            .fetch_one(store.pool())
            .await
            .expect("count");
    assert_eq!(count, 0, "no tenant → no distilled audit row");
}
