//! `argo mcp serve` — MCP server bound to the workspace store.
//!
//! Reads JSON-RPC from stdin, writes responses to stdout. Three
//! tools advertised: \`sessions/list\`, \`sessions/get\`,
//! \`messages/count\`. The full §12.3 12-tool surface lands as the
//! daemon binding grows; what's here is enough for an MCP
//! inspector or Claude Desktop to browse stored sessions.
use crate::cli::McpCmd;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_mcp::{
    run_loop, CallToolParams, CallToolResult, McpContent, McpResult, McpServerService,
    McpToolSchema, ServerInfo,
};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId};
use async_trait::async_trait;
use std::io::{self, BufReader};
use std::sync::Arc;

pub async fn run(cmd: McpCmd) -> ArgoResult<()> {
    match cmd {
        McpCmd::Serve => serve().await,
        McpCmd::Tools {
            server_id,
            vertical,
            format,
            json,
            filter,
            schema_only,
        } => {
            super::mcp_tools::run(super::mcp_tools::ToolsArgs {
                server_id,
                vertical,
                format,
                json,
                filter,
                schema_only,
            })
            .await
        }
        McpCmd::Registry { cmd } => super::mcp_registry::run(cmd).await,
        McpCmd::ImageGen {
            config,
            require_confirm,
        } => {
            super::mcp_image_gen::run(super::mcp_image_gen::Args {
                config,
                require_confirm,
            })
            .await
        }
    }
}

async fn serve() -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let db = db_path(&home);
    let store = Arc::new(SessionStore::open(&db).await?);
    store.migrate().await?;
    let service = SessionsService { store };
    let stdin = io::stdin();
    let stdout = io::stdout();
    run_loop(&service, BufReader::new(stdin.lock()), stdout.lock())
        .await
        .map_err(|e| ArgoError::Provider(format!("mcp serve: {e}")))?;
    Ok(())
}

struct SessionsService {
    store: Arc<SessionStore>,
}

#[async_trait]
impl McpServerService for SessionsService {
    fn server_info(&self) -> ServerInfo {
        ServerInfo {
            name: "argo-sessions".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }

    fn list_tools(&self) -> Vec<McpToolSchema> {
        vec![
            McpToolSchema {
                name: "sessions/list".into(),
                description: "List recent sessions (across all tenants in this binary).".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "limit": {"type": "integer", "minimum": 1, "maximum": 1000}
                    }
                }),
            },
            McpToolSchema {
                name: "sessions/get".into(),
                description: "Fetch one session header by id.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {"id": {"type": "string"}},
                    "required": ["id"]
                }),
            },
            McpToolSchema {
                name: "messages/count".into(),
                description: "Count messages in a session.".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {"session_id": {"type": "string"}},
                    "required": ["session_id"]
                }),
            },
        ]
    }

    async fn call_tool(&self, params: CallToolParams) -> McpResult<CallToolResult> {
        let body = match params.name.as_str() {
            "sessions/list" => self.tool_list(&params.arguments).await,
            "sessions/get" => self.tool_get(&params.arguments).await,
            "messages/count" => self.tool_count(&params.arguments).await,
            other => Err(format!("unknown tool: {other}")),
        };
        Ok(match body {
            Ok(value) => CallToolResult {
                content: vec![McpContent::Text {
                    text: value.to_string(),
                }],
                is_error: false,
            },
            Err(msg) => CallToolResult {
                content: vec![McpContent::Text { text: msg }],
                is_error: true,
            },
        })
    }
}

impl SessionsService {
    async fn tool_list(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        let limit = args
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|n| n.min(1000) as u32)
            .unwrap_or(25);
        let sessions = self
            .store
            .list_all_sessions(limit)
            .await
            .map_err(|e| e.to_string())?;
        let rows: Vec<_> = sessions
            .into_iter()
            .map(|s| {
                serde_json::json!({
                    "id": s.id.0.to_string(),
                    "tenant_id": s.tenant_id.0.to_string(),
                    "model": s.model,
                    "provider": s.provider,
                    "title": s.title,
                    "started_at": s.started_at.timestamp_micros(),
                })
            })
            .collect();
        Ok(serde_json::json!({"sessions": rows}))
    }

    async fn tool_get(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        let raw = args
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "missing 'id'".to_string())?;
        let uuid = uuid::Uuid::parse_str(raw).map_err(|e| format!("bad id: {e}"))?;
        let header = self
            .store
            .get_session(SessionId(uuid))
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("no session {raw}"))?;
        Ok(serde_json::json!({
            "id": header.id.0.to_string(),
            "model": header.model,
            "provider": header.provider,
            "title": header.title,
        }))
    }

    async fn tool_count(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        let raw = args
            .get("session_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "missing 'session_id'".to_string())?;
        let uuid = uuid::Uuid::parse_str(raw).map_err(|e| format!("bad id: {e}"))?;
        let n = self
            .store
            .message_count(SessionId(uuid))
            .await
            .map_err(|e| e.to_string())?;
        Ok(serde_json::json!({"count": n}))
    }
}
