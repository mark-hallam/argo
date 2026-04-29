//! `delegate_task` — declared in-core per §9. The actual subagent spawn
//! lives in `argo-delegate` (Phase 5 / §16); for Phase 3 dispatch
//! returns a clear error so the model knows not to call it yet.
use crate::tool::{error_text, Tool, ToolContext};
use argo_types::{ArgoResult, Capability, CapabilitySet, SpawnKind, ToolCallResult};
use async_trait::async_trait;
use serde::Deserialize;
use std::time::Instant;

/// Delegate-task — schema-only stub until `argo-delegate` (Phase 5,
/// Agent K / §16) lands.
#[derive(Debug, Default)]
pub struct DelegateTask;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Args {
    task: String,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    budget_tokens: Option<u32>,
}

#[async_trait]
impl Tool for DelegateTask {
    fn name(&self) -> &str {
        "delegate_task"
    }
    fn description(&self) -> &str {
        "Hand a sub-task to a child Argo agent with its own budget and \
         context. Returns the child's final answer. Inherits the \
         parent's fallback chain by default (Amendment 7)."
    }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "required": ["task"],
            "properties": {
                "task": { "type": "string", "description": "Self-contained task statement." },
                "model": { "type": "string", "description": "Override the parent's model." },
                "budget_tokens": { "type": "integer", "minimum": 1 },
            },
            "additionalProperties": false,
        })
    }

    fn required_capabilities(&self, _args: &serde_json::Value) -> ArgoResult<CapabilitySet> {
        Ok(CapabilitySet(vec![Capability::Spawn {
            spawn_kind: SpawnKind::Subagent,
        }]))
    }

    async fn dispatch(
        &self,
        args: serde_json::Value,
        ctx: &ToolContext,
    ) -> ArgoResult<ToolCallResult> {
        let started = Instant::now();
        let _: Args = serde_json::from_value(args).map_err(|e| argo_types::ArgoError::Tool {
            tool: "delegate_task".into(),
            msg: format!("invalid arguments: {e}"),
        })?;
        Ok(error_text(
            ctx.call_id,
            "subagent delegation not wired — argo-delegate ships with \
             Agent K (Phase 5 / §16)."
                .into(),
            started.elapsed().as_millis() as u64,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_types::Principal;

    #[tokio::test]
    async fn dispatch_returns_clear_not_wired_error() {
        let ctx = ToolContext::new(
            Principal::Local {
                os_user: "u".into(),
            },
            std::path::PathBuf::from("/tmp"),
        );
        let r = DelegateTask
            .dispatch(serde_json::json!({"task": "do something"}), &ctx)
            .await
            .unwrap();
        assert!(r.is_error);
    }

    #[test]
    fn requires_subagent_spawn_cap() {
        let caps = DelegateTask
            .required_capabilities(&serde_json::json!({"task": "x"}))
            .unwrap();
        assert!(caps.covers(&Capability::Spawn {
            spawn_kind: SpawnKind::Subagent,
        }));
    }
}
