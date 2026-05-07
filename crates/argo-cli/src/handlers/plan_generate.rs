//! `argo plan generate` handler — runs the offline TemplatePlanner
//! (default) or LlmPlanner (with `--llm`) and emits YAML.

use argo_runtime::{
    AnthropicCaller, DecomposeResult, LlmPlanner, Planner, PlannerContext, PlannerError,
    TemplatePlanner,
};
use argo_types::{ArgoError, ArgoResult};
use async_trait::async_trait;
use std::path::PathBuf;

pub async fn run(
    goal: &str,
    worker: &str,
    llm: bool,
    vertical: Option<&str>,
    output: Option<&str>,
    json: bool,
) -> ArgoResult<()> {
    let mut ctx = PlannerContext::for_worker(worker);
    if let Some(v) = vertical {
        ctx = ctx.with_vertical(v);
    }
    let result = if llm {
        let caller = build_anthropic_caller()?;
        let planner = LlmPlanner::new(caller);
        planner
            .decompose(goal, &ctx)
            .await
            .map_err(planner_to_argo)?
    } else {
        let planner = TemplatePlanner::shipped_defaults();
        planner
            .decompose(goal, &ctx)
            .await
            .map_err(planner_to_argo)?
    };
    let yaml = serde_yaml::to_string(&result.workflow)
        .map_err(|e| ArgoError::Provider(format!("serialize workflow: {e}")))?;

    if let Some(path) = output {
        write_outputs(path, &yaml, &result)?;
    }

    if json {
        let envelope = serde_json::json!({
            "ok": true,
            "planner_kind": result.planner_kind,
            "template_name": result.template_name,
            "reasoning": result.reasoning,
            "workflow_yaml": yaml,
            "tasks": result.workflow.tasks.len(),
            "output": output,
        });
        println!("{}", serde_json::to_string(&envelope).unwrap_or_default());
    } else if output.is_none() {
        println!("# planner: {} ({})", result.planner_kind, result.reasoning);
        print!("{yaml}");
    } else {
        println!(
            "wrote {} task workflow to {}",
            result.workflow.tasks.len(),
            output.unwrap_or("(stdout)")
        );
    }
    Ok(())
}

fn write_outputs(path: &str, yaml: &str, result: &DecomposeResult) -> ArgoResult<()> {
    let p = PathBuf::from(path);
    if let Some(parent) = p.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| ArgoError::Provider(format!("mkdir {}: {e}", parent.display())))?;
        }
    }
    std::fs::write(&p, yaml)
        .map_err(|e| ArgoError::Provider(format!("write {path}: {e}")))?;
    let mut sidecar = p.clone();
    let new_ext = match p.extension().and_then(|e| e.to_str()) {
        Some(ext) => format!("{ext}.reasoning"),
        None => "reasoning".to_string(),
    };
    sidecar.set_extension(new_ext);
    let body = format!(
        "planner_kind: {}\ntemplate: {}\n\n{}\n",
        result.planner_kind,
        result.template_name.as_deref().unwrap_or("-"),
        result.reasoning
    );
    std::fs::write(&sidecar, body)
        .map_err(|e| ArgoError::Provider(format!("write {}: {e}", sidecar.display())))?;
    Ok(())
}

fn planner_to_argo(e: PlannerError) -> ArgoError {
    ArgoError::Provider(format!("plan generate: {e}"))
}

/// Wire a real Anthropic caller. v1 reads `ANTHROPIC_API_KEY`
/// directly and posts to `https://api.anthropic.com/v1/messages`.
/// Errors during the call surface as `PlannerError::Llm`.
fn build_anthropic_caller() -> ArgoResult<Box<dyn AnthropicCaller>> {
    let key = std::env::var("ANTHROPIC_API_KEY").map_err(|_| {
        ArgoError::Provider(
            "ANTHROPIC_API_KEY unset; pass without --llm to use the offline planner".into(),
        )
    })?;
    Ok(Box::new(EnvAnthropicCaller { api_key: key }))
}

struct EnvAnthropicCaller {
    api_key: String,
}

#[async_trait]
impl AnthropicCaller for EnvAnthropicCaller {
    async fn call(&self, prompt: String) -> Result<String, PlannerError> {
        // Minimal Anthropic /v1/messages caller. We use reqwest
        // (already a workspace dep) with a single user-message
        // payload and parse `content[0].text` as the body.
        let body = serde_json::json!({
            "model": "claude-sonnet-4-6",
            "max_tokens": 2048,
            "messages": [{"role": "user", "content": prompt}],
        });
        let client = reqwest::Client::new();
        let resp = client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .await
            .map_err(|e| PlannerError::Llm(format!("send: {e}")))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let txt = resp.text().await.unwrap_or_default();
            return Err(PlannerError::Llm(format!("http {status}: {txt}")));
        }
        let parsed: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| PlannerError::Llm(format!("decode: {e}")))?;
        let text = parsed
            .pointer("/content/0/text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| PlannerError::Llm("response missing /content/0/text".into()))?
            .to_string();
        Ok(text)
    }
}
