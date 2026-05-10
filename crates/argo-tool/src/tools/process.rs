//! `process` — lifecycle tool for `terminal { background: true }`
//! children.
//!
//! Four actions:
//!
//! - `list` — every entry in the [`crate::process_registry::ProcessRegistry`]
//!   the [`crate::ToolContext`] points at; reaps already-finished
//!   entries on the way through.
//! - `status` — single-process status (`pid`, `state`, `age_seconds`).
//! - `tail` — last ~32 KB of stdout + stderr.
//! - `kill` — SIGTERM, then SIGKILL after `KILL_GRACE`.
//!
//! Required capability: `Capability::Spawn { spawn_kind:
//! SpawnKind::BackgroundProcess }`. Strictly stronger than
//! `Spawn::Subprocess`, so verticals that already hand out
//! `terminal` access don't accidentally pick this up.
//!
//! ```
//! # use argo_tool::Tool;
//! # let tool = argo_tool::Process;
//! assert_eq!(tool.name(), "process");
//! ```
use crate::process_registry::ProcessId;
use crate::tool::{error_text, success_text, Concurrency, Tool, ToolContext};
use argo_types::{ArgoResult, Capability, CapabilitySet, SpawnKind, ToolCallResult};
use async_trait::async_trait;
use serde::Deserialize;
use std::time::{Duration, Instant};

/// SIGTERM grace before falling back to SIGKILL. Matches Hermes.
const KILL_GRACE: Duration = Duration::from_secs(2);

/// Lifecycle tool for background processes spawned via `terminal`
/// with `background: true`.
#[derive(Debug, Default)]
pub struct Process;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Action {
    List,
    Status,
    Tail,
    Kill,
}

#[derive(Debug, Deserialize)]
struct Args {
    action: Action,
    #[serde(default)]
    pid: Option<String>,
}

#[async_trait]
impl Tool for Process {
    fn name(&self) -> &str {
        "process"
    }
    fn description(&self) -> &str {
        "Manage background processes spawned by `terminal { background: true }`. \
         Actions: `list`, `status`, `tail`, `kill`. `pid` is the process_id \
         returned by the original terminal call (required for everything except \
         `list`)."
    }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "required": ["action"],
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["list", "status", "tail", "kill"],
                },
                "pid": {
                    "type": "string",
                    "description": "process_id; required for status/tail/kill.",
                },
            },
            "additionalProperties": false,
        })
    }
    fn concurrency(&self) -> Concurrency {
        Concurrency::Parallel
    }

    fn required_capabilities(&self, _args: &serde_json::Value) -> ArgoResult<CapabilitySet> {
        Ok(CapabilitySet(vec![Capability::Spawn {
            spawn_kind: SpawnKind::BackgroundProcess,
        }]))
    }

    async fn dispatch(
        &self,
        args: serde_json::Value,
        ctx: &ToolContext,
    ) -> ArgoResult<ToolCallResult> {
        let started = Instant::now();
        let parsed: Args =
            serde_json::from_value(args).map_err(|e| argo_types::ArgoError::Tool {
                tool: "process".into(),
                msg: format!("invalid arguments: {e}"),
            })?;
        let Some(registry) = ctx.background_processes.clone() else {
            return Ok(error_text(
                ctx.call_id,
                "background unsupported in this context: no ProcessRegistry attached".into(),
                started.elapsed().as_millis() as u64,
            ));
        };
        let elapsed = || started.elapsed().as_millis() as u64;
        match parsed.action {
            Action::List => action_list(&registry, ctx, elapsed()).await,
            Action::Status => match resolve_pid(&parsed.pid, ctx, elapsed())? {
                Ok(id) => action_status(&registry, ctx, &id, elapsed()).await,
                Err(r) => Ok(r),
            },
            Action::Tail => match resolve_pid(&parsed.pid, ctx, elapsed())? {
                Ok(id) => action_tail(&registry, ctx, &id, elapsed()).await,
                Err(r) => Ok(r),
            },
            Action::Kill => match resolve_pid(&parsed.pid, ctx, elapsed())? {
                Ok(id) => action_kill(&registry, ctx, &id, elapsed()).await,
                Err(r) => Ok(r),
            },
        }
    }
}

fn resolve_pid(
    pid: &Option<String>,
    ctx: &ToolContext,
    elapsed_ms: u64,
) -> ArgoResult<Result<ProcessId, ToolCallResult>> {
    match pid {
        Some(s) if !s.is_empty() => Ok(Ok(ProcessId(s.clone()))),
        _ => Ok(Err(error_text(
            ctx.call_id,
            "this action requires a `pid`".into(),
            elapsed_ms,
        ))),
    }
}

async fn action_list(
    registry: &crate::process_registry::ProcessRegistry,
    ctx: &ToolContext,
    elapsed_ms: u64,
) -> ArgoResult<ToolCallResult> {
    registry.reap_finished().await;
    let live = registry.list().await;
    let body =
        serde_json::to_string_pretty(&live).map_err(|e| argo_types::ArgoError::Tool {
            tool: "process".into(),
            msg: format!("serialize list failed: {e}"),
        })?;
    Ok(success_text(ctx.call_id, body, elapsed_ms))
}

async fn action_status(
    registry: &crate::process_registry::ProcessRegistry,
    ctx: &ToolContext,
    id: &ProcessId,
    elapsed_ms: u64,
) -> ArgoResult<ToolCallResult> {
    match registry.status(id).await {
        Some(info) => {
            let body = serde_json::to_string_pretty(&info).map_err(|e| {
                argo_types::ArgoError::Tool {
                    tool: "process".into(),
                    msg: format!("serialize status failed: {e}"),
                }
            })?;
            Ok(success_text(ctx.call_id, body, elapsed_ms))
        }
        None => Ok(error_text(
            ctx.call_id,
            format!("no such process: {id}"),
            elapsed_ms,
        )),
    }
}

async fn action_tail(
    registry: &crate::process_registry::ProcessRegistry,
    ctx: &ToolContext,
    id: &ProcessId,
    elapsed_ms: u64,
) -> ArgoResult<ToolCallResult> {
    match registry.tail(id).await {
        Some(out) => Ok(success_text(
            ctx.call_id,
            format!(
                "--- stdout ---\n{}\n--- stderr ---\n{}",
                out.stdout, out.stderr
            ),
            elapsed_ms,
        )),
        None => Ok(error_text(
            ctx.call_id,
            format!("no such process: {id}"),
            elapsed_ms,
        )),
    }
}

async fn action_kill(
    registry: &crate::process_registry::ProcessRegistry,
    ctx: &ToolContext,
    id: &ProcessId,
    elapsed_ms: u64,
) -> ArgoResult<ToolCallResult> {
    match registry.kill(id, KILL_GRACE).await {
        Ok(()) => Ok(success_text(
            ctx.call_id,
            format!("killed: {id}"),
            elapsed_ms,
        )),
        Err(e) => Ok(error_text(
            ctx.call_id,
            format!("kill failed: {e}"),
            elapsed_ms,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process_registry::{ProcessRegistry, SpawnRequest};
    use argo_types::Principal;
    use std::sync::Arc;

    fn ctx_with_registry(reg: Arc<ProcessRegistry>) -> ToolContext {
        ToolContext::new(
            Principal::Local {
                os_user: "u".into(),
            },
            std::env::temp_dir(),
        )
        .with_background_processes(reg)
    }

    #[tokio::test]
    async fn list_without_registry_errors() {
        let ctx = ToolContext::new(
            Principal::Local {
                os_user: "u".into(),
            },
            std::env::temp_dir(),
        );
        let r = Process
            .dispatch(serde_json::json!({"action": "list"}), &ctx)
            .await
            .unwrap();
        assert!(r.is_error);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn list_returns_live_processes() {
        let reg = Arc::new(ProcessRegistry::new());
        let id = reg
            .spawn(SpawnRequest {
                command: "sleep 30".into(),
                working_dir: std::env::temp_dir(),
            })
            .await
            .unwrap();
        let ctx = ctx_with_registry(reg.clone());
        let r = Process
            .dispatch(serde_json::json!({"action": "list"}), &ctx)
            .await
            .unwrap();
        assert!(!r.is_error);
        let body = match &r.content[0] {
            argo_types::ContentBlock::Text { text } => text.clone(),
            _ => panic!("text"),
        };
        assert!(body.contains(&id.0));
        reg.kill(&id, Duration::from_secs(2)).await.unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn tail_returns_recent_output() {
        let reg = Arc::new(ProcessRegistry::new());
        let id = reg
            .spawn(SpawnRequest {
                command: "echo hello-process; sleep 0.2".into(),
                working_dir: std::env::temp_dir(),
            })
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(400)).await;
        let ctx = ctx_with_registry(reg.clone());
        let r = Process
            .dispatch(
                serde_json::json!({"action": "tail", "pid": id.0}),
                &ctx,
            )
            .await
            .unwrap();
        assert!(!r.is_error);
        let body = match &r.content[0] {
            argo_types::ContentBlock::Text { text } => text.clone(),
            _ => panic!("text"),
        };
        assert!(body.contains("hello-process"), "got: {body:?}");
        reg.reap_finished().await;
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn kill_terminates_child() {
        let reg = Arc::new(ProcessRegistry::new());
        let id = reg
            .spawn(SpawnRequest {
                command: "sleep 60".into(),
                working_dir: std::env::temp_dir(),
            })
            .await
            .unwrap();
        let ctx = ctx_with_registry(reg.clone());
        let r = Process
            .dispatch(
                serde_json::json!({"action": "kill", "pid": id.0}),
                &ctx,
            )
            .await
            .unwrap();
        assert!(!r.is_error);
        assert_eq!(reg.list().await.len(), 0);
    }

    #[tokio::test]
    async fn status_without_pid_errors() {
        let reg = Arc::new(ProcessRegistry::new());
        let ctx = ctx_with_registry(reg);
        let r = Process
            .dispatch(serde_json::json!({"action": "status"}), &ctx)
            .await
            .unwrap();
        assert!(r.is_error);
    }

    #[test]
    fn requires_background_process_capability() {
        let caps = Process
            .required_capabilities(&serde_json::json!({"action": "list"}))
            .unwrap();
        assert!(caps.covers(&Capability::Spawn {
            spawn_kind: SpawnKind::BackgroundProcess,
        }));
        // Strictly stronger: doesn't cover Subprocess.
        assert!(!caps.covers(&Capability::Spawn {
            spawn_kind: SpawnKind::Subprocess,
        }));
    }
}
