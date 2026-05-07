use super::*;

fn input(parent_goal: &str, success: bool) -> PlanCohortInput {
    PlanCohortInput {
        parent_goal: parent_goal.into(),
        parent_session_id: None,
        workflow_yaml: "id: w\ntasks:\n  - id: t\n    worker: \"\"".into(),
        success,
    }
}

#[test]
fn goal_tokens_lowercases_and_splits() {
    assert_eq!(goal_tokens("Promote our launch!"), vec!["promote", "our", "launch"]);
    assert_eq!(goal_tokens(""), Vec::<String>::new());
    assert_eq!(goal_tokens("  hello-world\n"), vec!["hello", "world"]);
}

#[test]
fn cosine_similarity_recognises_overlap() {
    let a = vec!["promote".to_string(), "our".to_string(), "launch".to_string()];
    let b = vec!["promote".to_string(), "our".to_string(), "launch".to_string()];
    assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-9);
    let c = vec!["fly".to_string(), "kite".to_string(), "today".to_string()];
    assert!(cosine_similarity(&a, &c) < PLAN_COHORT_SIMILARITY_THRESHOLD);
}

#[test]
fn cosine_similarity_handles_empty_input() {
    assert_eq!(cosine_similarity(&[], &["x".to_string()]), 0.0);
    assert_eq!(cosine_similarity(&["x".to_string()], &[]), 0.0);
}

#[test]
fn group_into_plan_cohorts_drops_below_floor() {
    let only_two = vec![input("promote our launch", true), input("promote our launch", true)];
    assert!(group_into_plan_cohorts(only_two).is_empty());
}

#[test]
fn group_into_plan_cohorts_drops_failures() {
    let mostly_failures = vec![
        input("promote our launch", true),
        input("promote our launch", false),
        input("promote our launch", false),
        input("promote our launch", false),
    ];
    assert!(group_into_plan_cohorts(mostly_failures).is_empty());
}

#[test]
fn group_into_plan_cohorts_clusters_similar_goals() {
    // Anchor + two members chosen so all pairwise cosine
    // similarities clear PLAN_COHORT_SIMILARITY_THRESHOLD; the
    // outlier shares no tokens.
    let inputs = vec![
        input("promote our launch announcement", true),
        input("promote our launch announcement on blog", true),
        input("promote our launch event announcement", true),
        input("fly kites today", true),
    ];
    let cohorts = group_into_plan_cohorts(inputs);
    assert_eq!(cohorts.len(), 1);
    let c = &cohorts[0];
    assert_eq!(c.members.len(), 3);
    assert!(c.anchor_parent_goal.contains("promote"));
    assert!(c
        .members
        .iter()
        .all(|m| m.parent_goal.contains("promote")));
}

struct StubCaller {
    name: String,
    yaml: String,
    cost: i64,
}

#[async_trait]
impl PlanDistillationCaller for StubCaller {
    async fn distill(&self, _cohort: &PlanCohort) -> Result<PlanDistillationResult, PlannerError> {
        Ok(PlanDistillationResult {
            name: self.name.clone(),
            description: "stubbed".into(),
            keywords: vec!["promote".into(), "launch".into()],
            yaml: self.yaml.clone(),
            model: "claude-dreams-stub".into(),
            cost_micros: self.cost,
        })
    }
}

#[tokio::test]
async fn distill_plan_cohort_marks_body_source_and_records_size() {
    let cohort = PlanCohort {
        anchor_parent_goal: "promote our launch".into(),
        members: vec![
            input("promote our launch", true),
            input("promote launch announcement", true),
            input("promote our blog post launch", true),
        ],
    };
    let caller = StubCaller {
        name: "marketing-launch-distilled".into(),
        yaml: "id: marketing\ntasks:\n  - id: draft\n    worker: \"\"".into(),
        cost: 750_000,
    };
    let (template, result) = distill_plan_cohort(&cohort, &caller).await.unwrap();
    assert_eq!(template.body_source, BODY_SOURCE_DREAMS);
    assert_eq!(template.parent_goal, "promote our launch");
    assert_eq!(template.cohort_size, 3);
    assert_eq!(template.keywords, vec!["promote", "launch"]);
    assert_eq!(result.cost_micros, 750_000);
}

#[tokio::test]
async fn distill_plan_cohort_rejects_undersized_cohort() {
    let cohort = PlanCohort {
        anchor_parent_goal: "x".into(),
        members: vec![input("x", true)],
    };
    let caller = StubCaller {
        name: "n".into(),
        yaml: "y".into(),
        cost: 0,
    };
    let err = distill_plan_cohort(&cohort, &caller).await.unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("too small"));
}

struct FailingCaller;

#[async_trait]
impl PlanDistillationCaller for FailingCaller {
    async fn distill(&self, _cohort: &PlanCohort) -> Result<PlanDistillationResult, PlannerError> {
        Err(PlannerError::Llm("upstream timeout".into()))
    }
}

#[tokio::test]
async fn distill_plan_cohort_propagates_caller_errors() {
    let cohort = PlanCohort {
        anchor_parent_goal: "g".into(),
        members: vec![input("g", true), input("g", true), input("g", true)],
    };
    let err = distill_plan_cohort(&cohort, &FailingCaller).await.unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("upstream timeout"));
}
