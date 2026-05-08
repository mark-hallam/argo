//! Google Meet MCP service implementation.

use crate::client::{to_call_result, GoogleMeetClient};
use argo_mcp::{
    CallToolParams, CallToolResult, McpContent, McpResult, McpServerService, McpToolSchema,
    ServerInfo,
};
use async_trait::async_trait;

/// Google Meet MCP service.
#[derive(Default)]
pub struct GoogleMeetService;

#[async_trait]
impl McpServerService for GoogleMeetService {
    fn server_info(&self) -> ServerInfo {
        ServerInfo {
            name: "argo-mcp-connector-google-meet".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }

    fn list_tools(&self) -> Vec<McpToolSchema> {
        crate::schemas::all()
    }

    async fn call_tool(&self, params: CallToolParams) -> McpResult<CallToolResult> {
        let client = match GoogleMeetClient::from_env() {
            Ok(c) => c,
            Err(text) => {
                return Ok(CallToolResult {
                    content: vec![McpContent::Text { text }],
                    is_error: true,
                });
            }
        };
        let out = match params.name.as_str() {
            "google_meet.list_meetings" => client.list_meetings(&params.arguments).await,
            "google_meet.schedule_meeting" => client.schedule_meeting(&params.arguments).await,
            "google_meet.get_meeting" => client.get_meeting(&params.arguments).await,
            "google_meet.list_participants" => client.list_participants(&params.arguments).await,
            "google_meet.get_recording" => client.get_recording(&params.arguments).await,
            other => Err(format!("unknown tool: {other}")),
        };
        Ok(to_call_result(out))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[tokio::test]
    async fn list_advertises_five_google_meet_tools() {
        let tools = GoogleMeetService.list_tools();
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "google_meet.list_meetings",
                "google_meet.schedule_meeting",
                "google_meet.get_meeting",
                "google_meet.list_participants",
                "google_meet.get_recording",
            ]
        );
    }

    #[tokio::test]
    async fn stdio_wire_lists_tools() {
        let input = Cursor::new(
            br#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}
"#
            .to_vec(),
        );
        let mut out = Vec::new();
        argo_mcp::run_loop(&GoogleMeetService, input, &mut out)
            .await
            .unwrap();
        let line = String::from_utf8(out).unwrap();
        let value: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
        let tools = value["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 5);
        assert_eq!(tools[0]["inputSchema"]["approval_required"], false);
        assert_eq!(tools[1]["inputSchema"]["approval_required"], true);
    }
}
