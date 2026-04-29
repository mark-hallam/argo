//! `web_search` and `web_extract` — declared as in-core tools per §9 so
//! the model always sees them in the schema list, but the actual fetch
//! is delegated to an MCP server (Phase 4 / §12). For Phase 3 the
//! dispatch path returns a clear "configure an MCP backend" error so
//! the agent knows to ask the user instead of silently failing.
use crate::tool::{error_text, Concurrency, Tool, ToolContext};
use argo_types::{ArgoResult, Capability, CapabilitySet, ToolCallResult};
use async_trait::async_trait;
use serde::Deserialize;
use std::time::Instant;

const NO_BACKEND: &str = "no MCP search/extract backend configured — Argo's in-core tools \
     declare `web_search` and `web_extract` so the model always sees \
     them, but the actual fetch is delegated to an MCP server. Configure \
     one in `argo.toml` (Phase 4 / §12).";

/// Web search — schema-only stub; backend lives in an MCP server (§12).
#[derive(Debug, Default)]
pub struct WebSearch;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct SearchArgs {
    query: String,
    #[serde(default)]
    limit: Option<u32>,
}

#[async_trait]
impl Tool for WebSearch {
    fn name(&self) -> &str {
        "web_search"
    }
    fn description(&self) -> &str {
        "Search the web for the given query. Returns up to `limit` \
         results (default 10) with title, snippet, and URL."
    }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "required": ["query"],
            "properties": {
                "query": { "type": "string" },
                "limit": { "type": "integer", "minimum": 1, "maximum": 50 },
            },
            "additionalProperties": false,
        })
    }
    fn concurrency(&self) -> Concurrency {
        Concurrency::Parallel
    }

    fn required_capabilities(&self, _args: &serde_json::Value) -> ArgoResult<CapabilitySet> {
        Ok(CapabilitySet(vec![Capability::NetEgress {
            host_glob: "**".into(),
            port: None,
        }]))
    }

    async fn dispatch(
        &self,
        args: serde_json::Value,
        ctx: &ToolContext,
    ) -> ArgoResult<ToolCallResult> {
        let started = Instant::now();
        let _: SearchArgs =
            serde_json::from_value(args).map_err(|e| argo_types::ArgoError::Tool {
                tool: "web_search".into(),
                msg: format!("invalid arguments: {e}"),
            })?;
        Ok(error_text(
            ctx.call_id,
            NO_BACKEND.into(),
            started.elapsed().as_millis() as u64,
        ))
    }
}

/// Web extract — schema-only stub; backend lives in an MCP server (§12).
#[derive(Debug, Default)]
pub struct WebExtract;

#[derive(Debug, Deserialize)]
struct ExtractArgs {
    url: String,
}

#[async_trait]
impl Tool for WebExtract {
    fn name(&self) -> &str {
        "web_extract"
    }
    fn description(&self) -> &str {
        "Fetch a URL and return its readable text content (article body, \
         no markup)."
    }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "required": ["url"],
            "properties": {
                "url": { "type": "string", "format": "uri" },
            },
            "additionalProperties": false,
        })
    }
    fn concurrency(&self) -> Concurrency {
        Concurrency::Parallel
    }

    fn required_capabilities(&self, args: &serde_json::Value) -> ArgoResult<CapabilitySet> {
        let parsed: ExtractArgs =
            serde_json::from_value(args.clone()).map_err(|e| argo_types::ArgoError::Tool {
                tool: "web_extract".into(),
                msg: format!("invalid arguments: {e}"),
            })?;
        let host = parsed
            .url
            .split("://")
            .nth(1)
            .and_then(|rest| rest.split('/').next())
            .unwrap_or("**")
            .to_string();
        Ok(CapabilitySet(vec![Capability::NetEgress {
            host_glob: host,
            port: None,
        }]))
    }

    async fn dispatch(
        &self,
        args: serde_json::Value,
        ctx: &ToolContext,
    ) -> ArgoResult<ToolCallResult> {
        let started = Instant::now();
        let _: ExtractArgs =
            serde_json::from_value(args).map_err(|e| argo_types::ArgoError::Tool {
                tool: "web_extract".into(),
                msg: format!("invalid arguments: {e}"),
            })?;
        Ok(error_text(
            ctx.call_id,
            NO_BACKEND.into(),
            started.elapsed().as_millis() as u64,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_types::Principal;

    #[tokio::test]
    async fn web_search_dispatch_returns_clear_no_backend_error() {
        let ctx = ToolContext::new(
            Principal::Local {
                os_user: "u".into(),
            },
            std::path::PathBuf::from("/tmp"),
        );
        let r = WebSearch
            .dispatch(serde_json::json!({"query": "argo"}), &ctx)
            .await
            .unwrap();
        assert!(r.is_error);
        let text = match &r.content[0] {
            argo_types::ContentBlock::Text { text } => text.clone(),
            _ => panic!("text"),
        };
        assert!(text.contains("MCP"));
    }

    #[test]
    fn web_extract_required_caps_narrow_to_host() {
        let caps = WebExtract
            .required_capabilities(&serde_json::json!({"url": "https://example.com/path"}))
            .unwrap();
        assert!(caps.covers(&Capability::NetEgress {
            host_glob: "example.com".into(),
            port: None
        }));
    }
}
