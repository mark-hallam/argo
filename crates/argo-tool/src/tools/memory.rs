//! `memory` — declared as in-core per §9 so the schema is always
//! present, but actual reads/writes go through `argo-memory` (Agent H).
//! Until that crate lands, dispatch returns a clear "memory store not
//! wired" error.
use crate::tool::{error_text, Tool, ToolContext};
use argo_types::{ArgoResult, Capability, CapabilitySet, ToolCallResult};
use async_trait::async_trait;
use serde::Deserialize;
use std::time::Instant;

/// Memory — schema-only stub until `argo-memory` (Agent H) lands.
#[derive(Debug, Default)]
pub struct Memory;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Args {
    action: String,
    #[serde(default)]
    entry: Option<String>,
    #[serde(default)]
    content: Option<String>,
}

#[async_trait]
impl Tool for Memory {
    fn name(&self) -> &str {
        "memory"
    }
    fn description(&self) -> &str {
        "List, read, or write entries in Argo's memory store. \
         `action` is one of `list`, `read`, or `write`."
    }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "required": ["action"],
            "properties": {
                "action": { "type": "string", "enum": ["list", "read", "write"] },
                "entry": { "type": "string", "description": "Entry name (`read`, `write`)." },
                "content": { "type": "string", "description": "Body to write (`write`)." },
            },
            "additionalProperties": false,
        })
    }

    fn required_capabilities(&self, args: &serde_json::Value) -> ArgoResult<CapabilitySet> {
        let parsed: Args =
            serde_json::from_value(args.clone()).map_err(|e| argo_types::ArgoError::Tool {
                tool: "memory".into(),
                msg: format!("invalid arguments: {e}"),
            })?;
        let mut caps = Vec::new();
        if parsed.action == "write" {
            caps.push(Capability::MemoryWrite);
        }
        Ok(CapabilitySet(caps))
    }

    async fn dispatch(
        &self,
        args: serde_json::Value,
        ctx: &ToolContext,
    ) -> ArgoResult<ToolCallResult> {
        let started = Instant::now();
        let _: Args = serde_json::from_value(args).map_err(|e| argo_types::ArgoError::Tool {
            tool: "memory".into(),
            msg: format!("invalid arguments: {e}"),
        })?;
        Ok(error_text(
            ctx.call_id,
            "memory store not wired — argo-memory ships with Agent H \
             (Phase 3 / §14)."
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
        let r = Memory
            .dispatch(serde_json::json!({"action": "list"}), &ctx)
            .await
            .unwrap();
        assert!(r.is_error);
    }

    #[test]
    fn write_action_requires_memory_write_cap() {
        let caps = Memory
            .required_capabilities(
                &serde_json::json!({"action": "write", "entry": "n", "content": "c"}),
            )
            .unwrap();
        assert!(caps.covers(&Capability::MemoryWrite));
    }

    #[test]
    fn read_action_does_not_require_memory_write_cap() {
        let caps = Memory
            .required_capabilities(&serde_json::json!({"action": "read", "entry": "n"}))
            .unwrap();
        assert!(!caps.covers(&Capability::MemoryWrite));
    }
}
