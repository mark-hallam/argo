//! Public-surface contract tests for `argo-runtime`.
//!
//! These tests import through the same public module and re-export
//! paths used by downstream crates so runtime substrate refactors
//! cannot silently change planner gating, context-window handling,
//! planner output, or worker confinement contracts.

use argo_runtime::{
    context_compress::{
        self, CompressionConfig, CompressionError, Summarizer, Summary, TokenCounter,
    },
    context_truncate, DecomposeGate, HeuristicGate, Planner, PlannerContext, TemplatePlanner,
    WorkerResourceCage, WorkerResourceConfig, HEURISTIC_LENGTH_THRESHOLD,
};
use argo_types::{ContentBlock, Message, MessageId, MessageRole, SessionId};
use async_trait::async_trait;
use serde_json::json;

const EXPECTED_PUBLIC_MODULES: &[&str] = &[
    "action_throttle",
    "anomaly",
    "context_compress",
    "context_truncate",
    "dispatch_gate",
    "dreams_plan_templates",
    "oom_restart",
    "planner_bias",
    "pool_elasticity",
    "pool_elasticity_cron",
    "prompt_cache_affinity",
    "worker_resource_cage",
];

const EXPECTED_REEXPORTS: &[&str] = &[
    "AnomalyDecision",
    "AnomalyPolicy",
    "AnthropicCaller",
    "BiasConfig",
    "CageStatus",
    "DecomposeGate",
    "DecomposeResult",
    "ElasticityAuditRow",
    "ElasticityConfig",
    "ElasticityMode",
    "GateError",
    "HEURISTIC_LENGTH_THRESHOLD",
    "HEURISTIC_LIST_MARKERS",
    "HeuristicGate",
    "InMemoryTemplateSource",
    "LearnedPlanTemplateSource",
    "LearnedTemplateCandidate",
    "LlmPlanner",
    "PlanTemplate",
    "PlannedSubtask",
    "Planner",
    "PlannerBias",
    "PlannerContext",
    "PlannerError",
    "PoolController",
    "PoolMetricsSnapshot",
    "ScaleRecommendation",
    "SessionAnomalyState",
    "StepSignal",
    "SubtaskSink",
    "TaskSignature",
    "TemplatePlanner",
    "TickState",
    "WorkerResourceCage",
    "WorkerResourceConfig",
    "apply_recommendation",
    "emit_audit_row",
    "latest_tick",
    "recommend",
    "record_decomposition",
    "scale_down_total",
    "scale_up_total",
];

#[test]
fn lib_public_surface_inventory_is_intentional() {
    let lib = include_str!("../src/lib.rs");
    assert_eq!(public_modules(lib), EXPECTED_PUBLIC_MODULES);

    let mut expected_reexports: Vec<String> = EXPECTED_REEXPORTS
        .iter()
        .map(|name| (*name).to_string())
        .collect();
    expected_reexports.sort();
    assert_eq!(public_reexports(lib), expected_reexports);
}

#[tokio::test]
async fn dispatch_gate_contract_keeps_explicit_lists_and_long_goals_decomposable() {
    let gate = HeuristicGate::new();
    let ctx = PlannerContext::for_worker("wkr-runtime");

    assert!(!gate.should_decompose("  ", &ctx).await.unwrap());
    assert!(gate
        .should_decompose("research the market and then draft a summary", &ctx)
        .await
        .unwrap());

    let long_goal = "x".repeat(HEURISTIC_LENGTH_THRESHOLD + 1);
    assert!(gate.should_decompose(&long_goal, &ctx).await.unwrap());
}

#[tokio::test]
async fn compression_summarizes_older_turns_and_preserves_recent_context() {
    let session = SessionId::new();
    let messages = vec![
        msg(session, MessageRole::System, "system"),
        msg(session, MessageRole::User, "old user request"),
        msg(session, MessageRole::Assistant, "old assistant answer"),
        msg(session, MessageRole::User, "current task"),
    ];
    let cfg = CompressionConfig {
        keep_last_turns: 1,
        summary_max_tokens: 32,
        ..CompressionConfig::default()
    };

    assert!(context_compress::should_compress(80, 100, &cfg));
    let result = context_compress::compress(&messages, &cfg, &WordCounter, &FixedSummarizer)
        .await
        .unwrap();

    assert!(result.compressed());
    assert_eq!(result.turns_compressed, 2);
    assert_eq!(result.view.len(), 3);
    assert_eq!(result.view[0].role, MessageRole::System);
    assert_eq!(result.view[1].role, MessageRole::Assistant);
    assert_eq!(
        first_text(&result.view[1]),
        "<summary>old facts preserved</summary>"
    );
    assert_eq!(first_text(&result.view[2]), "current task");

    let payload = context_compress::build_audit_payload(&result);
    assert_eq!(payload["turns_compressed"], 2);
    assert_eq!(payload["summary_length_tokens"], 3);
}

#[test]
fn truncation_drops_unprotected_old_turns_before_touching_must_keep_context() {
    let session = SessionId::new();
    let messages = vec![
        msg(session, MessageRole::System, "system"),
        msg(session, MessageRole::User, "old old old old"),
        msg(session, MessageRole::Assistant, "last answer"),
        msg(session, MessageRole::User, "current task"),
    ];

    let result = context_truncate::truncate_to_fit(&messages, 5, &WordCounter).unwrap();

    assert!(result.truncated());
    assert_eq!(result.turns_dropped, 1);
    assert_eq!(result.intra_turn_truncations, 0);
    assert_eq!(result.post_token_count, 5);
    assert_eq!(
        result.view.iter().map(first_text).collect::<Vec<_>>(),
        vec!["system", "last answer", "current task"]
    );

    let payload = context_truncate::build_audit_payload(&result);
    assert_eq!(payload["turns_dropped"], 1);
    assert_eq!(payload["intra_turn_truncations"], 0);
}

#[tokio::test]
async fn template_planner_public_contract_emits_worker_bound_marketing_dag() {
    let planner = TemplatePlanner::shipped_defaults();
    let ctx = PlannerContext::for_worker("wkr-marketing");
    let result = planner
        .decompose("promote the launch announcement", &ctx)
        .await
        .unwrap();

    assert_eq!(result.planner_kind, "template");
    assert_eq!(result.template_name.as_deref(), Some("marketing-launch"));
    assert_eq!(result.workflow.id, "shipped-marketing-launch");
    assert_eq!(result.workflow.tasks.len(), 3);
    assert!(result
        .workflow
        .tasks
        .iter()
        .all(|task| task.worker == "wkr-marketing"));
}

#[test]
fn worker_resource_cage_disabled_config_is_non_destructive_not_supported() {
    let cage = WorkerResourceCage::with_root("/tmp/argo-runtime-surface-cgroup");
    assert!(cage.slice_path("wkr-123").ends_with("argo-worker-wkr-123"));

    let status = cage.install_on(
        "wkr-123",
        1234,
        &WorkerResourceConfig {
            enabled: false,
            ..WorkerResourceConfig::default()
        },
    );
    let argo_runtime::CageStatus::NotSupported { reason } = status else {
        panic!("disabled cage should not attempt filesystem writes");
    };
    assert_eq!(reason, "disabled by config");
}

struct WordCounter;

impl TokenCounter for WordCounter {
    fn count_message(&self, msg: &Message) -> usize {
        msg.content.iter().map(count_block).sum()
    }

    fn count_text(&self, text: &str) -> usize {
        text.split_whitespace().count()
    }
}

struct FixedSummarizer;

#[async_trait]
impl Summarizer for FixedSummarizer {
    async fn summarize(
        &self,
        older_turns: &[Message],
        max_tokens: usize,
    ) -> Result<Summary, CompressionError> {
        assert_eq!(older_turns.len(), 2);
        assert_eq!(max_tokens, 32);
        Ok(Summary {
            text: "old facts preserved".into(),
            token_count: 3,
        })
    }
}

fn msg(session_id: SessionId, role: MessageRole, text: &str) -> Message {
    Message {
        id: MessageId::new(),
        session_id,
        role,
        content: vec![ContentBlock::Text { text: text.into() }],
        tool_calls: vec![],
        tool_call_id: None,
        reasoning: None,
        created_at: chrono::Utc::now(),
        provider_meta: json!({}),
    }
}

fn count_block(block: &ContentBlock) -> usize {
    match block {
        ContentBlock::Text { text } => text.split_whitespace().count(),
        _ => 0,
    }
}

fn first_text(message: &Message) -> &str {
    match message.content.first() {
        Some(ContentBlock::Text { text }) => text,
        _ => "",
    }
}

fn public_modules(lib: &str) -> Vec<&str> {
    lib.lines()
        .filter_map(|line| line.trim().strip_prefix("pub mod "))
        .map(|rest| rest.trim_end_matches(';'))
        .collect()
}

fn public_reexports(lib: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut block = String::new();
    for line in lib.lines().map(str::trim) {
        if line.starts_with("pub use ") || !block.is_empty() {
            block.push_str(line);
            block.push(' ');
            if line.ends_with(';') {
                names.extend(reexport_names(&block));
                block.clear();
            }
        }
    }
    names.sort();
    names
}

fn reexport_names(block: &str) -> Vec<String> {
    let rest = block
        .trim()
        .strip_prefix("pub use ")
        .expect("called only for pub use blocks")
        .trim_end_matches(';')
        .trim();
    let inner = rest
        .split_once('{')
        .and_then(|(_, tail)| tail.rsplit_once('}').map(|(names, _)| names))
        .unwrap_or(rest);
    inner
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(|name| name.rsplit("::").next().unwrap_or(name).to_string())
        .collect()
}
