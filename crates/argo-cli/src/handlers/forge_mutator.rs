//! LLM-driven skill-body mutator used by `argo forge optimize`.
//! Lives in its own file so `forge_optimize.rs` stays under
//! the 500-LOC budget. Pure helper — no orchestration logic.
use argo_agent_runner::{build_transport, load_manifest};
use argo_eval::{EvalRecord, EvalTask};
use argo_provider::{ProviderRequest, Sampling, ToolSchema};
use argo_types::ArgoResult;

/// LLM-driven mutator: ask the configured provider/model to
/// propose a refined skill body, given the baseline body + the
/// records from the previous bench iteration (so the prompt
/// shows which examples failed and what the wrong output was).
///
/// Returns `Ok(None)` when there's nothing to learn from (no
/// failed records yet, e.g. the very first iteration). The
/// caller falls back to the mechanical template in that case.
pub(super) async fn mutate_via_llm(
    cwd: &std::path::Path,
    provider_id: &str,
    model: &str,
    baseline_body: &str,
    tasks: &[EvalTask],
    prev_records: &[EvalRecord],
) -> ArgoResult<Option<String>> {
    if prev_records.is_empty() {
        return Ok(None);
    }
    let failures: Vec<&EvalRecord> = prev_records.iter().filter(|r| !r.passed).collect();
    if failures.is_empty() {
        return Ok(None);
    }
    let mut failure_block = String::new();
    for r in failures.iter().take(8) {
        let task = tasks.iter().find(|t| t.name == r.task);
        let input = task.map(|t| t.input.as_str()).unwrap_or("?");
        let expected = task.map(|t| t.expected.as_str()).unwrap_or("?");
        let actual: String = r.output.chars().take(280).collect();
        failure_block.push_str(&format!(
            "- task: {name}\n  input: {input}\n  expected: {expected}\n  actual: {actual}\n  score: {score:.3}\n\n",
            name = r.task,
            score = r.score,
        ));
    }
    let user_prompt = format!(
        "You are tuning a workspace skill. The current skill body is:\n\n\
         ```markdown\n{baseline}\n```\n\n\
         The skill was just benched against an eval-task suite. \
         Below are the tasks that *failed*:\n\n{failures}\n\
         Propose a refined skill body that addresses these failures \
         while preserving correct behaviour on passing tasks. Reply \
         with ONLY the refined Markdown body — no preamble, no \
         frontmatter, no code fences.",
        baseline = baseline_body.trim(),
        failures = failure_block,
    );

    let manifest = load_manifest(cwd, provider_id)?;
    let transport = build_transport(manifest).await?;
    let req = ProviderRequest {
        model: model.to_string(),
        messages: vec![argo_types::Message {
            id: argo_types::MessageId(uuid::Uuid::now_v7()),
            session_id: argo_types::SessionId(uuid::Uuid::nil()),
            role: argo_types::MessageRole::User,
            content: vec![argo_types::ContentBlock::Text { text: user_prompt }],
            tool_calls: Vec::new(),
            tool_call_id: None,
            reasoning: None,
            created_at: chrono::Utc::now(),
            provider_meta: serde_json::Value::Null,
        }],
        tools: Vec::<ToolSchema>::new(),
        system: Some(
            "You refine workspace skills based on bench-result feedback. \
             Output only the refined skill body."
                .to_string(),
        ),
        sampling: Sampling::default(),
    };
    let resp = transport.complete(&req).await?;
    let text = resp.text.unwrap_or_default();
    if text.trim().is_empty() {
        return Ok(None);
    }
    Ok(Some(strip_code_fence(&text)))
}

/// If the LLM wrapped the body in a Markdown code fence
/// (`\`\`\`markdown … \`\`\``), strip it. Idempotent on
/// fence-free inputs.
pub(super) fn strip_code_fence(s: &str) -> String {
    let t = s.trim();
    let stripped = t.strip_prefix("```markdown").or_else(|| t.strip_prefix("```"));
    if let Some(after_open) = stripped {
        let body = after_open.trim_start_matches('\n');
        if let Some(idx) = body.rfind("```") {
            return body[..idx].trim_end().to_string();
        }
    }
    t.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_code_fence_drops_markdown_wrapper() {
        let wrapped = "```markdown\nbody line\nmore\n```\n";
        assert_eq!(strip_code_fence(wrapped), "body line\nmore");
    }

    #[test]
    fn strip_code_fence_drops_plain_fence() {
        let wrapped = "```\nbody\n```";
        assert_eq!(strip_code_fence(wrapped), "body");
    }

    #[test]
    fn strip_code_fence_idempotent_on_unfenced_text() {
        assert_eq!(strip_code_fence("just text"), "just text");
    }
}
