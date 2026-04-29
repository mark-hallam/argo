//! Integration tests: drive `LlmJudge` against a mock judge transport
//! and assert deliberate-success / deliberate-failure scenarios produce
//! the right verdict (§25.3 acceptance).
use super::*;
use argo_provider::{
    ProviderRequest, ProviderResponse, ProviderResponseDelta, StopReason, Transport,
};
use argo_store::SessionStore;
use argo_types::{
    ArgoError, ArgoResult, CapabilitySet, JudgeSubject, JudgeVerdict, ProviderModel, SessionId,
    TenantId, WorkspaceId,
};
use async_trait::async_trait;
use futures::stream::BoxStream;
use parking_lot::Mutex;
use std::sync::Arc;

/// Scripted transport: each `complete()` call returns the next entry.
struct MockJudgeTransport {
    script: Mutex<std::collections::VecDeque<String>>,
}

impl MockJudgeTransport {
    fn new(replies: Vec<&'static str>) -> Arc<Self> {
        Arc::new(Self {
            script: Mutex::new(replies.into_iter().map(String::from).collect()),
        })
    }
}

#[async_trait]
impl Transport for MockJudgeTransport {
    fn id(&self) -> &str {
        "mock-judge"
    }
    fn capabilities(&self) -> &CapabilitySet {
        static EMPTY: std::sync::OnceLock<CapabilitySet> = std::sync::OnceLock::new();
        EMPTY.get_or_init(CapabilitySet::empty)
    }
    async fn complete(&self, _req: &ProviderRequest) -> ArgoResult<ProviderResponse> {
        let next = self
            .script
            .lock()
            .pop_front()
            .ok_or_else(|| ArgoError::Provider("script exhausted".into()))?;
        Ok(ProviderResponse {
            text: Some(next),
            reasoning: None,
            tool_calls: vec![],
            stop_reason: StopReason::EndTurn,
            tokens_in: None,
            tokens_out: None,
            provider_meta: serde_json::Value::Null,
        })
    }
    async fn complete_stream(
        &self,
        _req: &ProviderRequest,
    ) -> ArgoResult<BoxStream<'static, ArgoResult<ProviderResponseDelta>>> {
        Err(ArgoError::Provider("not used".into()))
    }
}

fn judge_model() -> ProviderModel {
    ProviderModel {
        provider: "mock".into(),
        model: "claude-haiku-4-5".into(),
    }
}

fn evidence(task: &str, output: &str) -> JudgeEvidence {
    JudgeEvidence {
        task_statement: task.into(),
        agent_output: output.into(),
        tool_calls: vec![],
        tool_results: vec![],
        proposed_artifact: ProposedArtifact::SkillCreate {
            name: "alpha".into(),
            body: "skill body".into(),
        },
        ground_truth: vec![],
    }
}

#[tokio::test]
async fn deliberate_success_yields_success_verdict() {
    let transport = MockJudgeTransport::new(vec![
        r#"{
            "scores": {
                "task_completed": {"score": 1.0, "rationale": "did the task"},
                "tool_calls_grounded": {"score": 1.0, "rationale": "no claims"},
                "skill_quality": {"score": 1.0, "rationale": "tight skill"},
                "side_effects": {"score": 1.0, "rationale": "none"}
            }
        }"#,
    ]);
    let judge = LlmJudge::with_default_rubric(transport, judge_model());
    let record = judge
        .adjudicate(
            SessionId::new(),
            JudgeSubject::SkillCreate {
                name: "alpha".into(),
                hash: [0u8; 32],
            },
            evidence("do x", "did x"),
        )
        .await
        .unwrap();
    assert!(matches!(record.verdict, JudgeVerdict::Success));
    assert_eq!(record.rubric_id, DEFAULT_RUBRIC_ID);
    assert_eq!(record.rubric_version, DEFAULT_RUBRIC_VERSION);
    assert_eq!(record.evidence_hashes.len(), 6);
}

#[tokio::test]
async fn deliberate_failure_yields_failure_verdict() {
    let transport = MockJudgeTransport::new(vec![
        r#"{
            "scores": {
                "task_completed": {"score": 0.0, "rationale": "did nothing"},
                "tool_calls_grounded": {"score": 0.0, "rationale": "hallucinated"},
                "skill_quality": {"score": 0.0, "rationale": "boilerplate"},
                "side_effects": {"score": 0.0, "rationale": "deleted things"}
            }
        }"#,
    ]);
    let judge = LlmJudge::with_default_rubric(transport, judge_model());
    let record = judge
        .adjudicate(
            SessionId::new(),
            JudgeSubject::SkillCreate {
                name: "alpha".into(),
                hash: [0u8; 32],
            },
            evidence("do x", "did nothing"),
        )
        .await
        .unwrap();
    assert!(matches!(record.verdict, JudgeVerdict::Failure { .. }));
}

#[tokio::test]
async fn middle_score_yields_partial_verdict() {
    let transport = MockJudgeTransport::new(vec![
        r#"{
            "scores": {
                "task_completed": {"score": 1.0, "rationale": "ok"},
                "tool_calls_grounded": {"score": 1.0, "rationale": "ok"},
                "skill_quality": {"score": 0.0, "rationale": "thin"},
                "side_effects": {"score": 0.0, "rationale": "none"}
            }
        }"#,
    ]);
    let judge = LlmJudge::with_default_rubric(transport, judge_model());
    let record = judge
        .adjudicate(
            SessionId::new(),
            JudgeSubject::SkillCreate {
                name: "alpha".into(),
                hash: [0u8; 32],
            },
            evidence("do x", "did most of x"),
        )
        .await
        .unwrap();
    assert!(matches!(record.verdict, JudgeVerdict::Partial { .. }));
}

#[tokio::test]
async fn malformed_judge_response_propagates_provider_error() {
    let transport = MockJudgeTransport::new(vec!["not even close to JSON"]);
    let judge = LlmJudge::with_default_rubric(transport, judge_model());
    let err = judge
        .adjudicate(
            SessionId::new(),
            JudgeSubject::DelegateOutcome {
                delegate_session: SessionId::new(),
            },
            evidence("x", "y"),
        )
        .await
        .unwrap_err();
    assert!(matches!(err, ArgoError::Provider(_)));
}

#[tokio::test]
async fn persist_judge_record_writes_to_table() {
    // Set up a fresh store + workspace + session so the FK on judge_record
    // resolves.
    let store = SessionStore::open(":memory:").await.unwrap();
    store.migrate().await.unwrap();
    let tenant = TenantId::new();
    let ws = WorkspaceId::new();
    sqlx::query(
        "INSERT INTO workspace (id, tenant_id, name, root_path, created_at)
         VALUES (?, ?, 'test', '.', strftime('%s','now'))",
    )
    .bind(ws.0.as_bytes().as_slice())
    .bind(tenant.0.as_bytes().as_slice())
    .execute(store.pool())
    .await
    .unwrap();
    let session_id = store
        .create_session(tenant, ws, "model", "provider", Some("t"))
        .await
        .unwrap();

    let transport = MockJudgeTransport::new(vec![
        r#"{"scores":{"task_completed":1.0,"tool_calls_grounded":1.0,"skill_quality":1.0,"side_effects":1.0}}"#,
    ]);
    let judge = LlmJudge::with_default_rubric(transport, judge_model());
    let record = judge
        .adjudicate(
            session_id,
            JudgeSubject::SkillCreate {
                name: "alpha".into(),
                hash: [0u8; 32],
            },
            evidence("x", "y"),
        )
        .await
        .unwrap();
    persist_judge_record(&store, &record).await.unwrap();
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM judge_record WHERE session_id = ?")
        .bind(session_id.0.as_bytes().as_slice())
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(n, 1);
}

#[tokio::test]
async fn evidence_hashes_are_stable_for_identical_input() {
    let e1 = evidence("task", "out");
    let e2 = evidence("task", "out");
    assert_eq!(e1.evidence_hashes(), e2.evidence_hashes());
}

#[tokio::test]
async fn evidence_hashes_change_when_any_field_changes() {
    let baseline = evidence("task", "out").evidence_hashes();
    let other = evidence("task", "different").evidence_hashes();
    assert_ne!(baseline, other);
}
