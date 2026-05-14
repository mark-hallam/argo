//! Unit tests for the Ghostwriter pipeline (Track L brief: L6).
//!
//! Covers:
//! - Empty-prompt rejection.
//! - Five-role sample prompts each produce ≥4 node graphs.
//! - Schema-validation failure → self-correction retry succeeds.
//! - Invalid JSON → self-correction retry succeeds.
//! - Validation failure on BOTH attempts surfaces a typed error.
//! - Decode failure on BOTH attempts surfaces a typed error.
//! - `validate_graph` rejects unknown edge endpoints.
//! - Offline default produces a valid 5-node graph for any prompt.

use super::{
    generate_from_prompt, offline_template, validate_graph, GhostwriterClient, GhostwriterError,
    OfflineGhostwriterClient, GHOSTWRITER_SOURCE,
};
use crate::ir::types::{Edge, EdgeCond, EdgeId, Node, NodeAnnotations, NodeId, NodeKind, SkillGraph};
use argo_types::{ArgoError, ArgoResult};
use async_trait::async_trait;
use std::sync::Mutex;

struct ScriptedClient {
    responses: Mutex<Vec<String>>,
}

impl ScriptedClient {
    fn new(responses: Vec<&str>) -> Self {
        Self {
            responses: Mutex::new(responses.into_iter().map(String::from).collect()),
        }
    }
}

#[async_trait]
impl GhostwriterClient for ScriptedClient {
    async fn generate(&self, _prompt: &str) -> ArgoResult<String> {
        let mut guard = self.responses.lock().map_err(|e| {
            ArgoError::Provider(format!("scripted client mutex: {e}"))
        })?;
        if guard.is_empty() {
            return Err(ArgoError::Provider(
                "scripted client exhausted".to_string(),
            ));
        }
        Ok(guard.remove(0))
    }
}

const FIVE_NODE_JSON: &str = r#"{
  "entry": "a",
  "nodes": [
    {"id":"a","label":"Fetch","kind":{"type":"fetch","target":"https://x","method":"GET"},"annotations":{}},
    {"id":"b","label":"Transform","kind":{"type":"transform","template":"x"},"annotations":{}},
    {"id":"c","label":"Decide","kind":{"type":"condition","expression":"ok"},"annotations":{}},
    {"id":"d","label":"Approve","kind":{"type":"await-approval","approverRole":"cfo"},"annotations":{}},
    {"id":"e","label":"Act","kind":{"type":"action","instruction":"do it"},"annotations":{}}
  ],
  "edges": [
    {"id":"e1","from":"a","to":"b","cond":{"kind":"on-success"}},
    {"id":"e2","from":"b","to":"c","cond":{"kind":"on-success"}},
    {"id":"e3","from":"c","to":"d","cond":{"kind":"on-success"}},
    {"id":"e4","from":"d","to":"e","cond":{"kind":"on-success"}}
  ],
  "variables": [],
  "metadata": {"name":"Sample","version":"1.0.0"}
}"#;

/// Same shape as FIVE_NODE_JSON but with an entry that does not match
/// any node — the structural validator must reject it.
const BROKEN_ENTRY_JSON: &str = r#"{
  "entry": "nope",
  "nodes": [
    {"id":"a","label":"Fetch","kind":{"type":"fetch","target":"https://x","method":"GET"},"annotations":{}},
    {"id":"b","label":"Transform","kind":{"type":"transform","template":"x"},"annotations":{}},
    {"id":"c","label":"Decide","kind":{"type":"condition","expression":"ok"},"annotations":{}},
    {"id":"d","label":"Action","kind":{"type":"action","instruction":"go"},"annotations":{}}
  ],
  "edges": [],
  "variables": [],
  "metadata": {}
}"#;

const RECRUITER_PROMPT: &str = "Build a skill that screens inbound CVs: parse \
the resume, score against the job spec, flag mismatches over 30% for the \
hiring manager.";
const AP_SPECIALIST_PROMPT: &str = "Build a skill that processes vendor invoices \
weekly: fetch the inbox, match line items against the PO, flag deltas over $250 \
for AP approval.";
const VENDOR_MANAGER_PROMPT: &str = "Build a skill that reviews vendor renewals \
monthly: fetch the contract, score risk against the supplier scorecard, route \
non-trivial changes to procurement.";
const SUPPORT_PROMPT: &str = "Build a skill that triages support tickets hourly: \
fetch the new tickets, classify by sentiment and product, escalate P1s to \
on-call.";
const DEAL_DESK_PROMPT: &str = "Build a skill that reviews deal-desk requests \
daily: fetch the quote, compare against pricing guardrails, route discounts \
over 15% to the deal-desk analyst.";

#[tokio::test]
async fn empty_prompt_is_rejected_before_calling_llm() {
    let llm = ScriptedClient::new(vec![]);
    let err = generate_from_prompt("   ", &llm).await.unwrap_err();
    assert!(matches!(err, GhostwriterError::EmptyPrompt), "{err:?}");
}

#[tokio::test]
async fn five_sample_prompts_each_yield_at_least_four_nodes() {
    for prompt in [
        RECRUITER_PROMPT,
        AP_SPECIALIST_PROMPT,
        VENDOR_MANAGER_PROMPT,
        SUPPORT_PROMPT,
        DEAL_DESK_PROMPT,
    ] {
        let llm = OfflineGhostwriterClient;
        let graph = generate_from_prompt(prompt, &llm).await.unwrap();
        validate_graph(&graph).expect("offline graph passes structural validation");
        assert!(
            graph.nodes.len() >= 4,
            "expected >=4 nodes for prompt `{}`, got {}",
            prompt,
            graph.nodes.len()
        );
        assert_eq!(graph.metadata.source.as_deref(), Some(GHOSTWRITER_SOURCE));
    }
}

#[tokio::test]
async fn validation_failure_retries_with_self_correction() {
    let llm = ScriptedClient::new(vec![BROKEN_ENTRY_JSON, FIVE_NODE_JSON]);
    let graph = generate_from_prompt("Build a thing", &llm).await.unwrap();
    assert_eq!(graph.nodes.len(), 5);
    assert_eq!(graph.metadata.source.as_deref(), Some(GHOSTWRITER_SOURCE));
    // Both responses were consumed → exactly one retry.
    assert!(
        llm.responses.lock().unwrap().is_empty(),
        "scripted client should have used both responses"
    );
}

#[tokio::test]
async fn malformed_json_retries_with_self_correction() {
    let llm = ScriptedClient::new(vec!["{ not really json", FIVE_NODE_JSON]);
    let graph = generate_from_prompt("Build a thing", &llm).await.unwrap();
    assert_eq!(graph.nodes.len(), 5);
}

#[tokio::test]
async fn validation_failure_on_both_attempts_surfaces_typed_error() {
    let llm = ScriptedClient::new(vec![BROKEN_ENTRY_JSON, BROKEN_ENTRY_JSON]);
    let err = generate_from_prompt("Build a thing", &llm).await.unwrap_err();
    match err {
        GhostwriterError::Validation(msg) => {
            assert!(msg.contains("entry"), "expected entry-mismatch msg, got: {msg}");
        }
        other => panic!("expected Validation, got {other:?}"),
    }
}

#[tokio::test]
async fn decode_failure_on_both_attempts_surfaces_typed_error() {
    let llm = ScriptedClient::new(vec!["not json", "still not json"]);
    let err = generate_from_prompt("Build a thing", &llm).await.unwrap_err();
    assert!(matches!(err, GhostwriterError::Decode(_)), "{err:?}");
}

#[test]
fn validate_graph_rejects_unknown_edge_endpoint() {
    let graph = SkillGraph {
        entry: NodeId::from("a"),
        nodes: vec![Node {
            id: NodeId::from("a"),
            label: "only".into(),
            kind: NodeKind::Action {
                instruction: "x".into(),
            },
            annotations: NodeAnnotations::default(),
        }],
        edges: vec![Edge {
            id: EdgeId::from("e1"),
            from: NodeId::from("a"),
            to: NodeId::from("ghost"),
            cond: EdgeCond::Always,
        }],
        variables: vec![],
        metadata: Default::default(),
    };
    let err = validate_graph(&graph).unwrap_err();
    assert!(err.contains("ghost"), "got: {err}");
}

#[test]
fn offline_template_is_structurally_valid_and_has_five_nodes() {
    let graph = offline_template("reconcile bank statements weekly");
    assert_eq!(graph.nodes.len(), 5);
    assert_eq!(graph.edges.len(), 5);
    validate_graph(&graph).unwrap();
    assert!(graph.metadata.description.is_some());
    assert_eq!(graph.metadata.source.as_deref(), Some(GHOSTWRITER_SOURCE));
}

#[tokio::test]
async fn code_fenced_response_is_decoded() {
    let fenced = format!("```json\n{FIVE_NODE_JSON}\n```");
    let llm = ScriptedClient::new(vec![fenced.as_str()]);
    let graph = generate_from_prompt("Build a thing", &llm).await.unwrap();
    assert_eq!(graph.nodes.len(), 5);
}
