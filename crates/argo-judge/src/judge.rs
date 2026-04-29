//! `Judge` trait + LLM-backed implementation.
//!
//! Spec: `docs/plan/argo-build-plan.md` §14.3 + §25.
use crate::evidence::JudgeEvidence;
use crate::rubric::Rubric;
use argo_provider::{ProviderRequest, Sampling, Transport};
use argo_types::{
    ArgoError, ArgoResult, ContentBlock, JudgeRecord, JudgeRecordId, JudgeSubject, JudgeVerdict,
    Message, MessageId, MessageRole, ProviderModel, SessionId,
};
use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;

/// The judge seam.
///
/// Every reflective skill creation / refinement (and optionally memory
/// writes + delegate outcomes) flows through `adjudicate` before the
/// downstream write commits. Three-state verdict per §14.3.
///
/// ```
/// # use argo_judge::Judge;
/// fn assert_object_safe(_j: &dyn Judge) {}
/// ```
#[async_trait]
pub trait Judge: Send + Sync {
    /// Render the rubric against the evidence, call the judge model,
    /// and return a persisted-shaped [`JudgeRecord`].
    async fn adjudicate(
        &self,
        session_id: SessionId,
        subject: JudgeSubject,
        evidence: JudgeEvidence,
    ) -> ArgoResult<JudgeRecord>;
}

/// LLM-backed judge.
pub struct LlmJudge {
    transport: Arc<dyn Transport>,
    judge_model: ProviderModel,
    rubric: Rubric,
}

impl std::fmt::Debug for LlmJudge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LlmJudge")
            .field("judge_model", &self.judge_model)
            .field("rubric_id", &self.rubric.id)
            .field("rubric_version", &self.rubric.version)
            .finish()
    }
}

impl LlmJudge {
    /// New judge bound to a transport, judge model, and rubric.
    #[must_use]
    pub fn new(transport: Arc<dyn Transport>, judge_model: ProviderModel, rubric: Rubric) -> Self {
        Self {
            transport,
            judge_model,
            rubric,
        }
    }

    /// New judge using the §25.2 default rubric.
    #[must_use]
    pub fn with_default_rubric(transport: Arc<dyn Transport>, judge_model: ProviderModel) -> Self {
        Self::new(transport, judge_model, Rubric::default_rubric())
    }

    /// Borrowed rubric.
    #[must_use]
    pub fn rubric(&self) -> &Rubric {
        &self.rubric
    }
}

#[async_trait]
impl Judge for LlmJudge {
    async fn adjudicate(
        &self,
        session_id: SessionId,
        subject: JudgeSubject,
        evidence: JudgeEvidence,
    ) -> ArgoResult<JudgeRecord> {
        let prompt = render_prompt(&self.rubric, &subject, &evidence)?;
        let req = build_request(&self.judge_model, session_id, &prompt);
        let resp = self.transport.complete(&req).await?;
        let raw = resp.text.unwrap_or_default();
        let scores = parse_scores(&raw)?;
        let (_score, verdict) = self.rubric.aggregate(&scores)?;
        Ok(JudgeRecord {
            id: JudgeRecordId::new(),
            session_id,
            subject,
            verdict,
            judge_model: self.judge_model.clone(),
            rubric_id: self.rubric.id.clone(),
            rubric_version: self.rubric.version.clone(),
            evidence_hashes: evidence.evidence_hashes(),
            created_at: Utc::now(),
        })
    }
}

/// Build the system + user prompt the judge model sees.
fn render_prompt(
    rubric: &Rubric,
    subject: &JudgeSubject,
    evidence: &JudgeEvidence,
) -> ArgoResult<String> {
    let evidence_json = serde_json::to_string_pretty(&serde_json::json!({
        "subject": subject,
        "task_statement": evidence.task_statement,
        "agent_output": evidence.agent_output,
        "tool_calls": evidence.tool_calls,
        "tool_results": evidence.tool_results,
        "proposed_artifact": evidence.proposed_artifact,
        "ground_truth": evidence.ground_truth,
    }))
    .map_err(|e| ArgoError::Schema(format!("evidence serialize: {e}")))?;
    let mut checks_block = String::new();
    for c in &rubric.checks {
        checks_block.push_str(&format!(
            "- id: {}\n  weight: {}\n  prompt: |\n    {}\n",
            c.id, c.weight, c.prompt
        ));
    }
    let response_keys: Vec<&str> = rubric.checks.iter().map(|c| c.id.as_str()).collect();
    let example_keys = response_keys
        .iter()
        .map(|k| format!("\"{k}\": {{\"score\": 0.0, \"rationale\": \"...\"}}"))
        .collect::<Vec<_>>()
        .join(", ");
    Ok(format!(
        "You are an external judge for an AI agent. Evaluate the evidence \
         below against the rubric. For every check return a score in \
         [0, 1] and a one-line rationale. Reply with ONE JSON object of \
         the form {{\"scores\": {{{example_keys}}}}} — no extra prose.\n\n\
         RUBRIC\n{}\nEVIDENCE\n{}",
        checks_block, evidence_json
    ))
}

fn build_request(model: &ProviderModel, session_id: SessionId, prompt: &str) -> ProviderRequest {
    let user = Message {
        id: MessageId::new(),
        session_id,
        role: MessageRole::User,
        content: vec![ContentBlock::Text {
            text: prompt.to_string(),
        }],
        tool_calls: vec![],
        tool_call_id: None,
        provider_meta: serde_json::Value::Null,
        reasoning: None,
        created_at: Utc::now(),
    };
    ProviderRequest {
        model: model.model.clone(),
        messages: vec![user],
        tools: vec![],
        system: None,
        sampling: Sampling {
            temperature: Some(0.0),
            max_tokens: Some(512),
            ..Default::default()
        },
    }
}

/// Parse the judge model's reply into a per-check score map.
///
/// Robust to common LLM hedges: extracts the first balanced JSON object,
/// expects either `{ "scores": { id: {score, rationale}, ... } }` or
/// `{ id: {score, rationale}, ... }` or `{ id: 0.7, ... }`.
pub(crate) fn parse_scores(raw: &str) -> ArgoResult<HashMap<String, f64>> {
    let json_text = extract_first_object(raw)
        .ok_or_else(|| ArgoError::Provider("judge response: no JSON object".into()))?;
    let value: serde_json::Value = serde_json::from_str(json_text)
        .map_err(|e| ArgoError::Provider(format!("judge response JSON: {e}")))?;
    let scores_obj = value
        .get("scores")
        .and_then(|v| v.as_object())
        .or_else(|| value.as_object());
    let map = scores_obj.ok_or_else(|| {
        ArgoError::Provider("judge response: expected object or {scores: {...}}".into())
    })?;
    let mut out = HashMap::new();
    for (k, v) in map {
        let score = match v {
            serde_json::Value::Number(n) => n.as_f64(),
            serde_json::Value::Object(inner) => inner.get("score").and_then(|s| s.as_f64()),
            _ => None,
        };
        if let Some(s) = score {
            out.insert(k.clone(), s.clamp(0.0, 1.0));
        }
    }
    Ok(out)
}

fn extract_first_object(s: &str) -> Option<&str> {
    let start = s.find('{')?;
    let bytes = s.as_bytes();
    let mut depth: i32 = 0;
    let mut in_str = false;
    let mut escape = false;
    for (i, &b) in bytes.iter().enumerate().skip(start) {
        if in_str {
            if escape {
                escape = false;
            } else if b == b'\\' {
                escape = true;
            } else if b == b'"' {
                in_str = false;
            }
            continue;
        }
        match b {
            b'"' => in_str = true,
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&s[start..=i]);
                }
            }
            _ => {}
        }
    }
    None
}

/// Convenience: aggregate via rubric and produce just a verdict (test
/// helper exposed at crate root for the integration cases).
#[must_use]
pub fn verdict_from_scores(rubric: &Rubric, scores: &HashMap<String, f64>) -> JudgeVerdict {
    rubric
        .aggregate(scores)
        .map(|(_, v)| v)
        .unwrap_or(JudgeVerdict::Failure {
            reason: "rubric aggregate failed".into(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_first_object_finds_balanced_braces_inside_prose() {
        let s = "before {\"scores\": {\"a\": 1}} after";
        assert_eq!(extract_first_object(s), Some("{\"scores\": {\"a\": 1}}"));
    }

    #[test]
    fn extract_first_object_handles_nested_braces_in_strings() {
        // Tricky: the } inside the quoted string must not close the object.
        let s = "{\"k\": \"v}\"}";
        assert_eq!(extract_first_object(s), Some(s));
    }

    #[test]
    fn parse_scores_accepts_nested_score_objects() {
        let raw = "{\"scores\": {\"task_completed\": {\"score\": 0.9, \"rationale\": \"ok\"}}}";
        let m = parse_scores(raw).unwrap();
        assert!((m["task_completed"] - 0.9).abs() < 1e-9);
    }

    #[test]
    fn parse_scores_accepts_flat_numbers() {
        let raw = "{\"task_completed\": 0.7, \"side_effects\": 1.0}";
        let m = parse_scores(raw).unwrap();
        assert!((m["task_completed"] - 0.7).abs() < 1e-9);
        assert!((m["side_effects"] - 1.0).abs() < 1e-9);
    }

    #[test]
    fn parse_scores_clamps_out_of_range() {
        let raw = "{\"a\": -0.5, \"b\": 1.5}";
        let m = parse_scores(raw).unwrap();
        assert_eq!(m["a"], 0.0);
        assert_eq!(m["b"], 1.0);
    }

    #[test]
    fn parse_scores_rejects_non_json() {
        let err = parse_scores("not json").unwrap_err();
        assert!(matches!(err, ArgoError::Provider(_)));
    }

    #[test]
    fn render_prompt_includes_each_check_id() {
        use argo_types::ToolCallId;
        let rubric = Rubric::default_rubric();
        let evidence = JudgeEvidence {
            task_statement: "do x".into(),
            agent_output: "did x".into(),
            tool_calls: vec![argo_types::ToolCallRequest {
                id: ToolCallId::new(),
                provider_call_id: None,
                name: "ls".into(),
                arguments: serde_json::json!({}),
            }],
            tool_results: vec![],
            proposed_artifact: crate::evidence::ProposedArtifact::None,
            ground_truth: vec![],
        };
        let p = render_prompt(
            &rubric,
            &JudgeSubject::DelegateOutcome {
                delegate_session: SessionId::new(),
            },
            &evidence,
        )
        .unwrap();
        for c in &rubric.checks {
            assert!(p.contains(&c.id), "prompt missing check id `{}`", c.id);
        }
        assert!(p.contains("\"task_statement\": \"do x\""));
    }
}
