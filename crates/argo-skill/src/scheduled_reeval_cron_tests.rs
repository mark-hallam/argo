use super::*;
use argo_store::{SessionStore, WorkerId};
use serde_json::Value;
use std::sync::{Arc, Mutex};

async fn store() -> (Arc<SessionStore>, SkillStore) {
    let session = Arc::new(SessionStore::open(":memory:").await.expect("store"));
    session.migrate().await.expect("migrate");
    let skills = SkillStore::new(Arc::clone(&session));
    (session, skills)
}

fn skill_doc(name: &str, evals: &str) -> String {
    format!(
        "---\n\
         name: {name}\n\
         version: \"1.0.0\"\n\
         description: scheduled reeval test\n\
         provenance:\n  source: builtin\n  hash: sha256-test\n\
         {evals}---\n\
         Use {name} safely.\n"
    )
}

fn two_contains_evals() -> &'static str {
    "evals:\n  - input: first\n    expected:\n      kind: contains\n      text: ok\n  - input: second\n    expected:\n      kind: contains\n      text: ok\n"
}

struct SequenceExecutor {
    outputs: Mutex<Vec<Value>>,
}

impl SequenceExecutor {
    fn new(outputs: Vec<Value>) -> Self {
        Self {
            outputs: Mutex::new(outputs),
        }
    }
}

#[async_trait::async_trait]
impl EvalExecutor for SequenceExecutor {
    async fn run_one(&self, _skill: &Skill, _case: &crate::EvalCase) -> ArgoResult<ActualOutcome> {
        let mut outputs = self.outputs.lock().unwrap();
        assert!(!outputs.is_empty(), "SequenceExecutor exhausted");
        Ok(ActualOutcome::new(outputs.remove(0)))
    }
}

async fn seed_previous(skills: &SkillStore, skill: &Skill, pass_rate: f64) {
    ensure_schema(skills).await.unwrap();
    insert_history(
        skills,
        skill,
        "previous-run",
        1,
        Some(pass_rate),
        None,
        false,
    )
    .await
    .unwrap();
}

fn cfg() -> ScheduledReevalConfig {
    ScheduledReevalConfig {
        regression_threshold: 0.10,
    }
}

#[tokio::test]
async fn flagging_fires_when_pass_rate_drops_beyond_threshold() {
    let (_session, skills) = store().await;
    let skill = skills
        .install(&skill_doc("reeval-regress", two_contains_evals()), None)
        .await
        .unwrap();
    seed_previous(&skills, &skill, 1.0).await;
    let exec = SequenceExecutor::new(vec![Value::String("no".into()), Value::String("ok".into())]);

    let report = scheduled_reeval_skill_with_config(&skills, skill.id, &exec, cfg())
        .await
        .unwrap();

    assert_eq!(report.evaluated, 1);
    assert_eq!(report.regressed, 1);
    assert_eq!(report.flagged, 1);
    let state: String = sqlx::query_scalar("SELECT lifecycle_state FROM skill WHERE id = ?")
        .bind(skill.id.0.as_bytes().as_slice())
        .fetch_one(skills.store_arc().pool())
        .await
        .unwrap();
    assert_eq!(state, "flagged");
}

#[tokio::test]
async fn no_flag_when_drop_is_within_threshold_and_history_is_recorded() {
    let (_session, skills) = store().await;
    let skill = skills
        .install(&skill_doc("reeval-stable", two_contains_evals()), None)
        .await
        .unwrap();
    seed_previous(&skills, &skill, 1.0).await;
    let exec = SequenceExecutor::new(vec![Value::String("ok".into()), Value::String("ok".into())]);

    let report = scheduled_reeval_skill_with_config(&skills, skill.id, &exec, cfg())
        .await
        .unwrap();

    assert_eq!(report.evaluated, 1);
    assert_eq!(report.regressed, 0);
    let rows: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM skill_reeval_history WHERE skill_id = ?")
            .bind(skill.id.to_string())
            .fetch_one(skills.store_arc().pool())
            .await
            .unwrap();
    assert_eq!(rows, 2);
}

#[tokio::test]
async fn skills_without_eval_suites_are_skipped() {
    let (_session, skills) = store().await;
    skills
        .install(&skill_doc("reeval-no-evals", ""), None)
        .await
        .unwrap();
    let exec = SequenceExecutor::new(Vec::new());

    let report = scheduled_reeval_tick_with_config(&skills, &exec, cfg())
        .await
        .unwrap();

    assert_eq!(report.evaluated, 0);
    assert_eq!(report.skipped_no_eval, 1);
}

#[tokio::test]
async fn audit_row_count_matches_regressed_count() {
    let (session, skills) = store().await;
    let skill = skills
        .install(&skill_doc("reeval-audit", two_contains_evals()), None)
        .await
        .unwrap();
    seed_previous(&skills, &skill, 1.0).await;
    let exec = SequenceExecutor::new(vec![
        Value::String("bad".into()),
        Value::String("bad".into()),
    ]);

    let report = scheduled_reeval_skill_with_config(&skills, skill.id, &exec, cfg())
        .await
        .unwrap();

    let audit_rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM audit_record WHERE kind = ?")
        .bind(AUDIT_KIND)
        .fetch_one(session.pool())
        .await
        .unwrap();
    assert_eq!(audit_rows, i64::try_from(report.regressed).unwrap());
}

#[tokio::test]
async fn flagged_overlay_does_not_surface_for_worker() {
    let (_session, skills) = store().await;
    let worker = WorkerId::new();
    let skill = skills
        .install(&skill_doc("reeval-overlay", two_contains_evals()), None)
        .await
        .unwrap();
    sqlx::query("INSERT INTO worker_skill (worker_id, skill_id, proficiency) VALUES (?, ?, 5)")
        .bind(worker.to_string())
        .bind(skill.id.to_string())
        .execute(skills.store_arc().pool())
        .await
        .unwrap();
    seed_previous(&skills, &skill, 1.0).await;
    let exec = SequenceExecutor::new(vec![
        Value::String("bad".into()),
        Value::String("bad".into()),
    ]);

    scheduled_reeval_skill_with_config(&skills, skill.id, &exec, cfg())
        .await
        .unwrap();

    let overlays = skills.list_for_worker(worker).await.unwrap();
    assert!(overlays.is_empty(), "flagged overlays must be hidden");
}
