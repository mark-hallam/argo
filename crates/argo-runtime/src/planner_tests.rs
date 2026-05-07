//! Unit tests for [`crate::planner`].

use super::planner::*;
use async_trait::async_trait;
use std::sync::{Arc, Mutex};

/// Canned-response Anthropic stub used by the LlmPlanner tests.
#[derive(Default)]
struct StubCaller {
    responses: Mutex<Vec<Result<String, PlannerError>>>,
    calls: Mutex<Vec<String>>,
}

impl StubCaller {
    fn with_responses(rs: Vec<Result<String, PlannerError>>) -> Arc<Self> {
        Arc::new(Self {
            responses: Mutex::new(rs),
            calls: Mutex::new(Vec::new()),
        })
    }

    fn call_count(&self) -> usize {
        self.calls.lock().unwrap().len()
    }

    fn last_prompt(&self) -> Option<String> {
        self.calls.lock().unwrap().last().cloned()
    }
}

#[async_trait]
impl AnthropicCaller for StubCaller {
    async fn call(&self, prompt: String) -> Result<String, PlannerError> {
        self.calls.lock().unwrap().push(prompt);
        match self.responses.lock().unwrap().pop() {
            Some(r) => r,
            None => Err(PlannerError::Llm("stub: out of responses".into())),
        }
    }
}

fn arc_caller(rs: Vec<Result<String, PlannerError>>) -> (Arc<StubCaller>, Box<dyn AnthropicCaller>) {
    let stub = StubCaller::with_responses(rs);
    let stub_clone = stub.clone();
    let boxed: Box<dyn AnthropicCaller> = Box::new(SharedCaller(stub_clone));
    (stub, boxed)
}

struct SharedCaller(Arc<StubCaller>);

#[async_trait]
impl AnthropicCaller for SharedCaller {
    async fn call(&self, prompt: String) -> Result<String, PlannerError> {
        self.0.call(prompt).await
    }
}

#[tokio::test]
async fn template_planner_matches_research_keyword() {
    let planner = TemplatePlanner::shipped_defaults();
    let ctx = PlannerContext::for_worker("wkr-1");
    let result = planner
        .decompose("research the competitive landscape", &ctx)
        .await
        .unwrap();
    assert_eq!(result.planner_kind, "template");
    assert_eq!(result.template_name.as_deref(), Some("research-then-summarise"));
    assert_eq!(result.workflow.tasks.len(), 2);
    // worker context applied
    assert_eq!(result.workflow.tasks[0].worker, "wkr-1");
}

#[tokio::test]
async fn template_planner_matches_marketing_keyword() {
    let planner = TemplatePlanner::shipped_defaults();
    let ctx = PlannerContext::for_worker("wkr-2");
    let result = planner
        .decompose("promote our new feature", &ctx)
        .await
        .unwrap();
    assert_eq!(result.template_name.as_deref(), Some("marketing-launch"));
    assert_eq!(result.workflow.tasks.len(), 3);
}

#[tokio::test]
async fn template_planner_returns_error_when_no_keyword_matches() {
    let planner = TemplatePlanner::shipped_defaults();
    let ctx = PlannerContext::for_worker("wkr-3");
    let err = planner
        .decompose("totally novel goal that nothing matches", &ctx)
        .await
        .unwrap_err();
    assert!(matches!(err, PlannerError::NoTemplateMatch { .. }));
}

#[tokio::test]
async fn template_planner_keyword_match_is_case_insensitive() {
    let planner = TemplatePlanner::shipped_defaults();
    let ctx = PlannerContext::for_worker("wkr-4");
    let result = planner
        .decompose("LAUNCH THE NEW SITE", &ctx)
        .await
        .unwrap();
    assert_eq!(result.template_name.as_deref(), Some("marketing-launch"));
}

#[tokio::test]
async fn template_planner_emitted_workflow_validates() {
    let planner = TemplatePlanner::shipped_defaults();
    let ctx = PlannerContext::for_worker("wkr-5");
    for goal in [
        "research the news",
        "promote our launch",
        "review the change",
    ] {
        let result = planner.decompose(goal, &ctx).await.unwrap();
        result.workflow.validate().unwrap();
    }
}

#[tokio::test]
async fn template_planner_with_custom_templates_uses_them() {
    let custom = vec![PlanTemplate {
        name: "custom-1".into(),
        description: "for tests".into(),
        keywords: vec!["chickens".into()],
        yaml: "id: c\ntasks:\n  - id: t1\n    worker: \"\"\n".into(),
    }];
    let planner = TemplatePlanner::new(custom);
    let ctx = PlannerContext::for_worker("wkr-6");
    let result = planner
        .decompose("count chickens this evening", &ctx)
        .await
        .unwrap();
    assert_eq!(result.template_name.as_deref(), Some("custom-1"));
    assert_eq!(result.workflow.tasks[0].worker, "wkr-6");
}

#[tokio::test]
async fn llm_planner_parses_well_formed_response() {
    let body = serde_json::json!({
        "reasoning": "Two-stage plan",
        "workflow_yaml": "id: gen\ntasks:\n  - id: a\n    worker: \"\"\n  - id: b\n    worker: \"\"\n    depends_on:\n      - a\n",
    })
    .to_string();
    let (stub, boxed) = arc_caller(vec![Ok(body)]);
    let planner = LlmPlanner::new(boxed);
    let ctx = PlannerContext::for_worker("wkr-7");
    let result = planner
        .decompose("any goal", &ctx)
        .await
        .unwrap();
    assert_eq!(result.planner_kind, "llm");
    assert_eq!(result.reasoning, "Two-stage plan");
    assert_eq!(result.workflow.tasks.len(), 2);
    assert_eq!(stub.call_count(), 1);
    let prompt = stub.last_prompt().unwrap();
    assert!(prompt.contains("any goal"), "prompt should embed goal: {prompt}");
    assert!(prompt.contains("wkr-7"));
}

#[tokio::test]
async fn llm_planner_propagates_caller_error() {
    let (_, boxed) = arc_caller(vec![Err(PlannerError::Llm("network down".into()))]);
    let planner = LlmPlanner::new(boxed);
    let ctx = PlannerContext::for_worker("wkr-8");
    let err = planner.decompose("goal", &ctx).await.unwrap_err();
    assert!(matches!(err, PlannerError::Llm(_)));
}

#[tokio::test]
async fn llm_planner_rejects_invalid_json() {
    let (_, boxed) = arc_caller(vec![Ok("this is not json @@@".into())]);
    let planner = LlmPlanner::new(boxed);
    let ctx = PlannerContext::for_worker("wkr-9");
    let err = planner.decompose("goal", &ctx).await.unwrap_err();
    assert!(matches!(err, PlannerError::InvalidResponse(_)));
}

#[tokio::test]
async fn llm_planner_rejects_invalid_workflow_yaml() {
    let body = serde_json::json!({
        "reasoning": "bad plan",
        "workflow_yaml": "this is not workflow yaml",
    })
    .to_string();
    let (_, boxed) = arc_caller(vec![Ok(body)]);
    let planner = LlmPlanner::new(boxed);
    let ctx = PlannerContext::for_worker("wkr-10");
    let err = planner.decompose("goal", &ctx).await.unwrap_err();
    assert!(matches!(err, PlannerError::InvalidResponse(_)));
}

#[tokio::test]
async fn build_prompt_includes_vertical_and_hints() {
    let (_, boxed) = arc_caller(vec![]);
    let planner = LlmPlanner::new(boxed);
    let mut ctx = PlannerContext::for_worker("wkr-11").with_vertical("marketplace-seller");
    ctx.add_hint("skill", "outreach");
    let prompt = planner.build_prompt("a goal", &ctx);
    assert!(prompt.contains("marketplace-seller"), "vertical missing: {prompt}");
    assert!(prompt.contains("skill: outreach"), "hint missing: {prompt}");
    assert!(prompt.contains("a goal"));
}

#[tokio::test]
async fn workflow_yaml_round_trips() {
    let planner = TemplatePlanner::shipped_defaults();
    let ctx = PlannerContext::for_worker("wkr-12");
    let result = planner.decompose("research X", &ctx).await.unwrap();
    let yaml = serde_yaml::to_string(&result.workflow).unwrap();
    let reparsed = argo_workflow::parse_yaml_str(&yaml).unwrap();
    assert_eq!(reparsed.tasks.len(), result.workflow.tasks.len());
}
