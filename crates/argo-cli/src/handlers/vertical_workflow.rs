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
use argo_tool::{builtin_tools, ToolContext, ToolRegistry};
use argo_types::{ArgoError, ArgoResult, Principal, ToolCallId};
use argo_vertical::WorkflowStep;
use std::path::Path;
use std::time::Instant;

pub async fn run(vertical_id: &str, workflow_name: &str, json: bool) -> ArgoResult<()> {
    let dir = super::vertical::resolve_verticals_dir();
    let (registry, _errors) = argo_vertical::load_dir(&dir)?;
    let loaded = registry.get(vertical_id).ok_or_else(|| {
        ArgoError::Provider(format!(
            "vertical '{vertical_id}' not found in {} — run `argo vertical list` to see what's loaded",
            dir.display()
        ))
    })?;

    // Resolve the workflow by name across the manifest's
    // [workflows].files list.
    let workflow = find_workflow(loaded, workflow_name)?;

    let cwd = std::env::current_dir()
        .map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;

    // Build the dispatch context once; reused across tool_call
    // steps so the engine's role grants land predictably.
    let (registry_t, engine, principal) = build_local_dispatch_ctx(&cwd)?;

    // Vertical's deny list is consulted per step — same deny-wins
    // semantics as the agent runner's per-turn dispatch (#655).
    let denied: std::collections::HashSet<String> = loaded
        .manifest
        .tools
        .as_ref()
        .map(|t| t.denied.iter().cloned().collect())
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

    // Both `failed` and `denied_by_vertical` propagate as
    // workflow failure — the operator wants the runner to
    // surface "your manifest blocked this" the same way it
    // surfaces a tool error: non-zero exit + JSON ok=false.
    // `deferred_no_llm` is intentionally not a failure — it
    // describes work the runner *can't yet do*, not work that
    // went wrong.
    let any_failed = step_results
        .iter()
        .any(|s| s.status == "failed" || s.status == "denied_by_vertical");
    if json {
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "vertical_id": vertical_id,
                "workflow": workflow_name,
                "ok": !any_failed,
                "total_ms": total_ms,
                "steps": step_results.iter().map(|s| serde_json::json!({
                    "name": s.name,
                    "kind": s.kind,
                    "status": s.status,
                    "message": s.message,
                    "duration_ms": s.duration_ms,
                })).collect::<Vec<_>>(),
            }))
            .unwrap_or_default()
        );
    } else {
        print_text(&workflow.workflow.name, vertical_id, &step_results, total_ms);
    }
    if any_failed {
        return Err(ArgoError::Provider(format!(
            "workflow '{workflow_name}': one or more steps failed or were denied by the vertical's [tools].denied list"
        )));
    }
    Ok(())
}

#[derive(Clone)]
struct StepResult {
    name: String,
    kind: String,
    status: String,
    message: String,
    duration_ms: u64,
}

struct StepOutcome {
    status: String,
    message: String,
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

async fn run_one_step(
    step: &WorkflowStep,
    registry: &ToolRegistry,
    engine: &CapabilityEngine,
    principal: &Principal,
    cwd: &Path,
    denied: &std::collections::HashSet<String>,
) -> StepOutcome {
    match step.kind.as_str() {
        "tool_call" => run_tool_call_step(step, registry, engine, principal, cwd, denied).await,
        "skill_invoke" => StepOutcome {
            status: "deferred_no_llm".into(),
            message: format!(
                "skill_invoke '{}' deferred — needs the agent runner's LLM transport (PR #659 ships tool_call only)",
                step.skill.as_deref().unwrap_or("")
            ),
        },
        "prompt" => StepOutcome {
            status: "deferred_no_llm".into(),
            message: "prompt step deferred — needs the agent runner's LLM transport (PR #659 ships tool_call only)".into(),
        },
        other => StepOutcome {
            status: "failed".into(),
            message: format!("unknown step kind '{other}'"),
        },
    }
}

async fn run_tool_call_step(
    step: &WorkflowStep,
    registry: &ToolRegistry,
    engine: &CapabilityEngine,
    principal: &Principal,
    cwd: &Path,
    denied: &std::collections::HashSet<String>,
) -> StepOutcome {
    let Some(tool_name) = step.tool.as_deref() else {
        return StepOutcome {
            status: "failed".into(),
            message: "tool_call step missing `tool` field".into(),
        };
    };
    if denied.contains(tool_name) {
        return StepOutcome {
            status: "denied_by_vertical".into(),
            message: format!(
                "tool '{tool_name}' is in the bound vertical's [tools].denied list"
            ),
        };
    }
    let args = step.args.clone().unwrap_or(serde_json::Value::Null);
    let mut ctx = ToolContext::new(principal.clone(), cwd.to_path_buf());
    ctx.call_id = ToolCallId::new();
    let result = registry.dispatch(tool_name, args, &ctx, engine).await;
    match result {
        Ok(r) if r.is_error => StepOutcome {
            status: "failed".into(),
            message: tool_result_message(&r),
        },
        Ok(r) => StepOutcome {
            status: "ok".into(),
            message: tool_result_message(&r),
        },
        Err(e) => StepOutcome {
            status: "failed".into(),
            message: format!("dispatch '{tool_name}' errored: {e}"),
        },
    }
}

fn tool_result_message(r: &argo_types::ToolCallResult) -> String {
    // Take a small preview from the first text content block —
    // enough for the operator to confirm the tool returned what
    // they expected without dumping the full output.
    use argo_types::ContentBlock;
    for block in &r.content {
        if let ContentBlock::Text { text } = block {
            let preview: String = text.chars().take(120).collect();
            return preview.replace('\n', " ");
        }
    }
    String::new()
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
            _ => "FAIL",
        };
        println!(
            "  [{marker}] {} ({}, {} ms): {}",
            s.name, s.kind, s.duration_ms, s.message
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_vertical::{Workflow, WorkflowHeader};

    fn empty_denied() -> std::collections::HashSet<String> {
        std::collections::HashSet::new()
    }

    #[tokio::test]
    async fn step_outcome_for_unknown_kind_fails() {
        let step = WorkflowStep {
            name: "weird".into(),
            kind: "telepathy".into(),
            depends_on: vec![],
            skill: None,
            prompt: None,
            tool: None,
            args: None,
        };
        // The outer parse_workflow rejects unknown kinds, but
        // run_one_step is defensive in case future schema
        // additions land mid-flight.
        let dir = std::env::current_dir().unwrap();
        let registry = ToolRegistry::new();
        let engine = CapabilityEngine::with_defaults();
        let principal = Principal::Local { os_user: "test".into() };
        let outcome = run_one_step(
            &step, &registry, &engine, &principal, &dir, &empty_denied(),
        )
        .await;
        assert_eq!(outcome.status, "failed");
        assert!(outcome.message.contains("telepathy"));
    }

    #[tokio::test]
    async fn skill_invoke_step_is_deferred() {
        let step = WorkflowStep {
            name: "scan".into(),
            kind: "skill_invoke".into(),
            depends_on: vec![],
            skill: Some("identify_invariant_violations".into()),
            prompt: None,
            tool: None,
            args: None,
        };
        let dir = std::env::current_dir().unwrap();
        let registry = ToolRegistry::new();
        let engine = CapabilityEngine::with_defaults();
        let principal = Principal::Local { os_user: "test".into() };
        let outcome = run_one_step(
            &step, &registry, &engine, &principal, &dir, &empty_denied(),
        )
        .await;
        assert_eq!(outcome.status, "deferred_no_llm");
        assert!(outcome.message.contains("identify_invariant_violations"));
    }

    #[tokio::test]
    async fn prompt_step_is_deferred() {
        let step = WorkflowStep {
            name: "ask".into(),
            kind: "prompt".into(),
            depends_on: vec![],
            skill: None,
            prompt: Some("say hi".into()),
            tool: None,
            args: None,
        };
        let dir = std::env::current_dir().unwrap();
        let registry = ToolRegistry::new();
        let engine = CapabilityEngine::with_defaults();
        let principal = Principal::Local { os_user: "test".into() };
        let outcome = run_one_step(
            &step, &registry, &engine, &principal, &dir, &empty_denied(),
        )
        .await;
        assert_eq!(outcome.status, "deferred_no_llm");
    }

    #[tokio::test]
    async fn tool_call_step_in_deny_list_is_blocked_before_dispatch() {
        // Even though the registry has no tools, the deny check
        // fires first — so we get `denied_by_vertical`, not
        // "no such tool". This locks in the deny-wins ordering.
        let step = WorkflowStep {
            name: "boom".into(),
            kind: "tool_call".into(),
            depends_on: vec![],
            skill: None,
            prompt: None,
            tool: Some("file_write".into()),
            args: None,
        };
        let dir = std::env::current_dir().unwrap();
        let registry = ToolRegistry::new();
        let engine = CapabilityEngine::with_defaults();
        let principal = Principal::Local { os_user: "test".into() };
        let mut denied = empty_denied();
        denied.insert("file_write".into());
        let outcome = run_one_step(
            &step, &registry, &engine, &principal, &dir, &denied,
        )
        .await;
        assert_eq!(outcome.status, "denied_by_vertical");
    }

    #[tokio::test]
    async fn tool_call_step_missing_tool_field_fails() {
        let step = WorkflowStep {
            name: "noop".into(),
            kind: "tool_call".into(),
            depends_on: vec![],
            skill: None,
            prompt: None,
            tool: None,
            args: None,
        };
        let dir = std::env::current_dir().unwrap();
        let registry = ToolRegistry::new();
        let engine = CapabilityEngine::with_defaults();
        let principal = Principal::Local { os_user: "test".into() };
        let outcome = run_one_step(
            &step, &registry, &engine, &principal, &dir, &empty_denied(),
        )
        .await;
        assert_eq!(outcome.status, "failed");
        assert!(outcome.message.contains("missing `tool`"));
    }

    #[test]
    fn workflow_types_are_publicly_constructible() {
        // Lock the re-exports so the runner can construct headers
        // + workflows from outside the argo-vertical crate.
        let _wh = WorkflowHeader {
            name: "x".into(),
            description: String::new(),
        };
        let _wf = Workflow {
            schema_version: "1".into(),
            workflow: _wh,
            steps: vec![],
        };
    }
}
