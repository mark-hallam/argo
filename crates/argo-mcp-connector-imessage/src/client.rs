//! MCP-facing Blue Bubbles client adapter.

use crate::auth;
use argo_bluebubbles_client::{
    BlueBubblesClient, ListQuery, SendMessageRequest, SendReactionRequest,
};
use argo_mcp::{CallToolResult, McpContent};
use serde::Deserialize;
use serde_json::Value;

/// iMessage MCP client.
#[derive(Clone)]
pub struct IMessageMcpClient {
    client: BlueBubblesClient,
}

impl IMessageMcpClient {
    /// Build from `BLUE_BUBBLES_URL` and `BLUE_BUBBLES_PASSWORD`.
    pub fn from_env() -> Result<Self, String> {
        Self::new(auth::url_from_env()?, auth::password_from_env()?)
    }

    /// Build with explicit connection settings.
    pub fn new(url: String, password: String) -> Result<Self, String> {
        let client = BlueBubblesClient::new(url, password).map_err(|e| e.to_string())?;
        Ok(Self { client })
    }

    /// Send a text iMessage.
    pub async fn send_message(&self, args: &Value) -> Result<Value, String> {
        let args: SendMessageArgs = parse(args)?;
        self.client
            .send_message(SendMessageRequest {
                chat_guid: args.chat_guid,
                message: args.message,
                temp_guid: args.temp_guid,
            })
            .await
            .map_err(|e| e.to_string())
    }

    /// List chats.
    pub async fn list_chats(&self, args: &Value) -> Result<Value, String> {
        let args: ListArgs = parse(args)?;
        self.client
            .list_chats(ListQuery {
                limit: args.limit,
                offset: args.offset,
                with_fields: args.with,
                sort: args.sort,
            })
            .await
            .map_err(|e| e.to_string())
    }

    /// List messages.
    pub async fn list_messages(&self, args: &Value) -> Result<Value, String> {
        self.client
            .list_messages(args.clone())
            .await
            .map_err(|e| e.to_string())
    }

    /// Fetch one message by GUID.
    pub async fn get_message(&self, args: &Value) -> Result<Value, String> {
        let args: GetMessageArgs = parse(args)?;
        self.client
            .get_message(&args.message_guid)
            .await
            .map_err(|e| e.to_string())
    }

    /// Send a tapback reaction.
    pub async fn send_reaction(&self, args: &Value) -> Result<Value, String> {
        let args: SendReactionArgs = parse(args)?;
        self.client
            .send_reaction(SendReactionRequest {
                chat_guid: args.chat_guid,
                selected_message_guid: args.selected_message_guid,
                reaction: args.reaction,
                part_index: args.part_index,
            })
            .await
            .map_err(|e| e.to_string())
    }
}

/// Convert a connector result into MCP text content.
pub fn to_call_result(result: Result<Value, String>) -> CallToolResult {
    match result {
        Ok(value) => CallToolResult {
            content: vec![McpContent::Text {
                text: value.to_string(),
            }],
            is_error: false,
        },
        Err(text) => CallToolResult {
            content: vec![McpContent::Text { text }],
            is_error: true,
        },
    }
}

#[derive(Deserialize)]
struct SendMessageArgs {
    #[serde(rename = "chat_guid", alias = "chatGuid")]
    chat_guid: String,
    message: String,
    #[serde(rename = "temp_guid", alias = "tempGuid", default)]
    temp_guid: Option<String>,
}

#[derive(Deserialize)]
struct ListArgs {
    #[serde(default)]
    limit: Option<u32>,
    #[serde(default)]
    offset: Option<u32>,
    #[serde(default, rename = "with")]
    with: Option<Vec<String>>,
    #[serde(default)]
    sort: Option<String>,
}

#[derive(Deserialize)]
struct GetMessageArgs {
    #[serde(rename = "message_guid", alias = "messageGuid")]
    message_guid: String,
}

#[derive(Deserialize)]
struct SendReactionArgs {
    #[serde(rename = "chat_guid", alias = "chatGuid")]
    chat_guid: String,
    #[serde(rename = "selected_message_guid", alias = "selectedMessageGuid")]
    selected_message_guid: String,
    reaction: String,
    #[serde(rename = "part_index", alias = "partIndex", default)]
    part_index: Option<u32>,
}

fn parse<T: for<'de> Deserialize<'de>>(args: &Value) -> Result<T, String> {
    serde_json::from_value(args.clone()).map_err(|e| format!("invalid imessage args: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    fn serve_once(body: &'static str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = [0; 4096];
            let n = stream.read(&mut buf).unwrap();
            let req = String::from_utf8_lossy(&buf[..n]);
            assert!(req.contains("password=pw"));
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).unwrap();
        });
        format!("http://{addr}")
    }

    #[tokio::test]
    async fn send_message_accepts_snake_case_args() {
        let client =
            IMessageMcpClient::new(serve_once(r#"{"data":{"guid":"msg-1"}}"#), "pw".into())
                .unwrap();
        let out = client
            .send_message(&serde_json::json!({
                "chat_guid": "iMessage;+;+1555",
                "message": "hello"
            }))
            .await
            .unwrap();
        assert_eq!(out["data"]["guid"], "msg-1");
    }

    #[test]
    fn bad_reaction_args_return_error() {
        let value = serde_json::json!({"chat_guid":"chat"});
        assert!(parse::<SendReactionArgs>(&value).is_err());
    }
}
