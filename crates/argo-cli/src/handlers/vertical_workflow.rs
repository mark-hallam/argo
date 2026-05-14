//! `argo vertical workflow run <vertical-id> <workflow-name>`
//! — minimum-viable workflow runner (#659, Phase 5 first slice).
//!
//! Walks a workflow's steps in declaration order. Each step is
//! one of three kinds:
//!
//! - **`tool_call`** — runs end-to-end today. The runner builds
//!   a fresh dispatch context (same shape as the agent runner)
//!   and calls the tool with the literal args from the step.
//!   Capability + deny-list checks both apply.
//! - **`skill_invoke`** — *deferred*. Requires injecting the
//!   skill into a one-shot agent turn, which needs an LLM
//!   transport. Step records as `status="deferred_no_llm"`.
//! - **`prompt`** — *deferred*. Same reason — needs an LLM
//!   transport to consume the prompt. Status: `deferred_no_llm`.
//!
//! Why ship the partial runner now? Tool-only workflows are
//! genuinely useful (e.g., "read these three files and tee
//! them"), and the deferred steps emit clear status so future
//! PRs can light them up incrementally without re-architecting.
use argo_capability::CapabilityEngine;
use argo_tool::{builtin_tools, ToolRegistry};
use argo_types::{ArgoError, ArgoResult, Principal};
use std::path::Path;
use std::time::Instant;

use step::{open_audit_ctx, run_one_step};

pub async fn run(
    vertical_id: &str,
    workflow_name: &str,
    session_id_str: Option<&str>,
    json: bool,
) -> ArgoResult<()> {
    let report = run_collect(vertical_id, workflow_name, session_id_str).await?;
    if json {
        println!("{}", report.to_json());
    } else {
        report.print_text();
    }
    if !report.ok {
        return Err(ArgoError::Provider(format!(
            "workflow '{workflow_name}': one or more steps failed or were denied by the vertical's [tools].denied list"
        )));
    }
    Ok(())
}

/// Structured workflow run report — used by both the standalone
/// CLI command and by the higher-level composite handlers (e.g.
/// `argo vertical run`, #669) so they can combine the workflow
/// outcome with their own metadata in a single JSON document.
#[derive(Clone)]
pub struct WorkflowReport {
    pub vertical_id: String,
    pub workflow_name: String,
    pub ok: bool,
    pub total_ms: u64,
    pub steps: Vec<StepResult>,
}

impl WorkflowReport {
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "vertical_id": self.vertical_id,
            "workflow": self.workflow_name,
            "ok": self.ok,
            "total_ms": self.total_ms,
            "steps": self.steps.iter().map(|s| serde_json::json!({
                "name": s.name,
                "kind": s.kind,
                "status": s.status,
                "message": s.message,
                "duration_ms": s.duration_ms,
            })).collect::<Vec<_>>(),
        })
    }

    pub fn print_text(&self) {
        print_text(
            &self.workflow_name,
            &self.vertical_id,
            &self.steps,
            self.total_ms,
        );
    }
}

/// Run the workflow and return the structured report without
/// printing or propagating an Err on step failures. Callers
/// decide how to surface the outcome.
pub async fn run_collect(
    vertical_id: &str,
    workflow_name: &str,
    session_id_str: Option<&str>,
) -> ArgoResult<WorkflowReport> {
    let dir = super::vertical::resolve_verticals_dir();
    let (registry, _errors) = argo_vertical::load_dir(&dir)?;
    let loaded = registry.get(vertical_id).ok_or_else(|| {
        ArgoError::Provider(format!(
            "vertical '{vertical_id}' not found in {} — run `argo vertical list` to see what's loaded",
            dir.display()
        ))
    })?;

    let audit_ctx = match session_id_str {
        Some(s) => Some(open_audit_ctx(s).await?),
        None => None,
    };

    let workflow = find_workflow(loaded, workflow_name)?;

    let cwd = std::env::current_dir()
        .map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;

    let (registry_t, engine, principal) = build_local_dispatch_ctx(&cwd)?;

    // Deny-wins (#655) and approval-gate (#660) sets are
    // resolved once per workflow run — same precedence as the
    // agent runner's per-turn dispatch.
    let denied: std::collections::HashSet<String> = loaded
        .manifest
        .tools
        .as_ref()
        .map(|t| t.denied.iter().cloned().collect())
        .unwrap_or_default();
    let needs_approval: std::collections::HashSet<String> = loaded
        .manifest
        .escalation
        .as_ref()
        .map(|e| e.human_approval_required.iter().cloned().collect())
        .unwrap_or_default();

    let mut step_results: Vec<StepResult> = Vec::with_capacity(workflow.steps.len());
    let started = Instant::now();
    for step in &workflow.steps {
        let s_started = Instant::now();
        let outcome = run_one_step(
            step,
            &registry_t,
            &engine,
            &principal,
            &cwd,
            &denied,
            &needs_approval,
            audit_ctx.as_ref(),
        )
        .await;
        step_results.push(StepResult {
            name: step.name.clone(),
            kind: step.kind.clone(),
            status: outcome.status,
            message: outcome.message,
            duration_ms: u64::try_from(s_started.elapsed().as_millis()).unwrap_or(u64::MAX),
        });
    }
    let total_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    let any_failed = step_results.iter().any(|s| {
        s.status == "failed"
            || s.status == "denied_by_vertical"
            || s.status == "escalation_required"
    });
    Ok(WorkflowReport {
        vertical_id: vertical_id.to_string(),
        workflow_name: workflow.workflow.name.clone(),
        ok: !any_failed,
        total_ms,
        steps: step_results,
    })
}

#[derive(Clone)]
pub struct StepResult {
    pub name: String,
    pub kind: String,
    pub status: String,
    pub message: String,
    pub duration_ms: u64,
}

fn find_workflow(
    loaded: &argo_vertical::LoadedVertical,
    name: &str,
) -> ArgoResult<argo_vertical::Workflow> {
    let files = loaded
        .manifest
        .workflows
        .as_ref()
        .map(|w| w.files.clone())
        .unwrap_or_default();
    if files.is_empty() {
        return Err(ArgoError::Provider(format!(
            "vertical '{}' declares no [workflows].files",
            loaded.manifest.vertical.id
        )));
    }
    for rel in &files {
        let path = loaded.root.join(rel);
        let src = std::fs::read_to_string(&path)
            .map_err(|e| ArgoError::Io(format!("read {}: {e}", path.display())))?;
        let wf = argo_vertical::parse_workflow(&src)?;
        if wf.workflow.name == name {
            return Ok(wf);
        }
    }
    Err(ArgoError::Provider(format!(
        "no workflow named '{name}' in vertical '{}'",
        loaded.manifest.vertical.id
    )))
}

/// Build a dispatch context that grants the local OS user the
/// `owner` role — same shape `argo-agent-runner::dispatch::build_dispatch_ctx`
/// uses. Duplicated rather than re-exported to keep this PR
/// independently revertable (no public API change to
/// argo-agent-runner).
fn build_local_dispatch_ctx(
    cwd: &Path,
) -> ArgoResult<(ToolRegistry, CapabilityEngine, Principal)> {
    let mut reg = ToolRegistry::new();
    for t in builtin_tools(cwd.to_path_buf()) {
        reg.register(t);
    }
    let engine = CapabilityEngine::with_defaults();
    let os_user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "argo".into());
    let principal = Principal::Local { os_user };
    engine.bind(principal.clone(), "owner".into())?;
    Ok((reg, engine, principal))
}

fn print_text(name: &str, vertical_id: &str, results: &[StepResult], total_ms: u64) {
    println!(
        "vertical '{vertical_id}' workflow '{name}' — {} step(s), {total_ms} ms total",
        results.len()
    );
    for s in results {
        let marker = match s.status.as_str() {
            "ok" => "ok",
            "deferred_no_llm" => "deferred",
            "denied_by_vertical" => "DENIED",
            "escalation_required" => "APPROVE?",
            _ => "FAIL",
        };
        println!(
            "  [{marker}] {} ({}, {} ms): {}",
            s.name, s.kind, s.duration_ms, s.message
        );
    }
}

#[path = "vertical_workflow_step.rs"]
mod step;

#[cfg(test)]
#[path = "vertical_workflow_tests.rs"]
mod tests;
