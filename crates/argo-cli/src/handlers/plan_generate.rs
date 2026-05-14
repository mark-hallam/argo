//! `argo plan generate` handler — runs the offline TemplatePlanner
//! (default) or LlmPlanner (with `--llm`) and emits YAML.
//!
//! Template path: the planner is wrapped in
//! [`argo_runtime::PlannerBias`] (`mission-planner-bias-from-template`)
//! so promoted Dreams-distilled templates bias the result before the
//! shipped-default match fires. The decision is stamped on the JSON
//! envelope as `PlanProvenance` so operators can answer "why this
//! plan?" without re-running the planner.

use argo_runtime::{
    AnthropicCaller, BiasConfig, DecomposeResult, InMemoryTemplateSource, LearnedPlanTemplateSource,
    LearnedTemplateCandidate, LlmPlanner, Planner, PlannerBias, PlannerContext, PlannerError,
    TemplatePlanner,
};
use argo_store::learned_plan_template::{list_promoted, LearnedPlanTemplateRow};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, PlanProvenance};
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;

use crate::home::{db_path, resolve_home};

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
    let (result, provenance) = if llm {
        let caller = build_anthropic_caller()?;
        let planner = LlmPlanner::new(caller);
        let result = planner
            .decompose(goal, &ctx)
            .await
            .map_err(planner_to_argo)?;
        (result, PlanProvenance::ColdGenerated)
    } else {
        let planner = TemplatePlanner::shipped_defaults();
        let source = load_template_source().await;
        let config = load_bias_config();
        let bias = PlannerBias::new(planner, source, config);
        bias.decompose(goal, &ctx).await.map_err(planner_to_argo)?
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
            "provenance": provenance,
            "workflow_yaml": yaml,
            "tasks": result.workflow.tasks.len(),
            "output": output,
        });
        println!("{}", serde_json::to_string(&envelope).unwrap_or_default());
    } else if output.is_none() {
        println!(
            "# planner: {} ({}) [{}]",
            result.planner_kind,
            result.reasoning,
            provenance.kind()
        );
        print!("{yaml}");
    } else {
        println!(
            "wrote {} task workflow to {} [{}]",
            result.workflow.tasks.len(),
            output.unwrap_or("(stdout)"),
            provenance.kind()
        );
    }
    Ok(())
}

/// Load `[planner.bias]` from the layered argo.toml. Falls back to
/// `BiasConfig::default()` (bias enabled, 30-day recency) when no
/// argo.toml is reachable from cwd. Config failures degrade
/// silently — the planner stays functional even if the operator's
/// config file is malformed.
fn load_bias_config() -> BiasConfig {
    let cfg = argo_config::load().unwrap_or_default();
    BiasConfig {
        enabled: cfg.planner.bias.enabled,
        prefer_per_vertical: cfg.planner.bias.prefer_per_vertical,
        require_recent_within_days: cfg.planner.bias.require_recent_within_days,
    }
}

/// Build the learned-template source: SQL-backed when the workspace
/// store is reachable, in-memory empty otherwise. Errors degrade to
/// the empty source (the bias path treats no candidates the same as
/// no match — cold-generate fall-through).
async fn load_template_source() -> Arc<dyn LearnedPlanTemplateSource> {
    let Ok(cwd) = std::env::current_dir() else {
        return Arc::new(InMemoryTemplateSource::default());
    };
    let Ok(home) = resolve_home(&cwd) else {
        return Arc::new(InMemoryTemplateSource::default());
    };
    match SessionStore::open(&db_path(&home)).await {
        Ok(store) => Arc::new(StoreTemplateSource { store }),
        Err(_) => Arc::new(InMemoryTemplateSource::default()),
    }
}

/// SQL-backed [`LearnedPlanTemplateSource`] that reads promoted
/// rows from the workspace's `learned_plan_template` table. Lives
/// here (not in argo-runtime) to preserve runtime's zero-db
/// invariant.
struct StoreTemplateSource {
    store: SessionStore,
}

#[async_trait]
impl LearnedPlanTemplateSource for StoreTemplateSource {
    async fn list_promoted(&self) -> Result<Vec<LearnedTemplateCandidate>, PlannerError> {
        let rows = list_promoted(self.store.pool())
            .await
            .map_err(|e| PlannerError::Llm(format!("learned_plan_template list: {e}")))?;
        Ok(rows.into_iter().map(row_to_candidate).collect())
    }
}

fn row_to_candidate(row: LearnedPlanTemplateRow) -> LearnedTemplateCandidate {
    let recency_micros = row.promoted_at.unwrap_or(row.created_at);
    LearnedTemplateCandidate {
        id: row.id,
        name: row.name,
        anchor_goal: row.parent_goal,
        yaml: row.yaml,
        recency_micros,
    }
}

fn write_outputs(path: &str, yaml: &str, result: &DecomposeResult) -> ArgoResult<()> {
    let p = PathBuf::from(path);
    if let Some(parent) = p.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| ArgoError::Io(format!("mkdir {}: {e}", parent.display())))?;
        }
    }
    std::fs::write(&p, yaml).map_err(|e| ArgoError::Io(format!("write {path}: {e}")))?;
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
        .map_err(|e| ArgoError::Io(format!("write {}: {e}", sidecar.display())))?;
    Ok(())
}

fn planner_to_argo(e: PlannerError) -> ArgoError {
    ArgoError::Provider(format!("plan generate: {e}"))
}

/// Wire a real Anthropic caller. v1 reads `ANTHROPIC_API_KEY`
/// directly and posts to `https://api.anthropic.com/v1/messages`.
/// Errors during the call surface as `PlannerError::Llm`.
fn build_anthropic_caller() -> ArgoResult<Box<dyn AnthropicCaller>> {
    let key = argo_config::cred::get("ANTHROPIC_API_KEY").map_err(|_| {
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
