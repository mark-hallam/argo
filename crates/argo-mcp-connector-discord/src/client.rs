//! Thin Discord REST API client.

use crate::auth;
use argo_mcp::{CallToolResult, McpContent};

/// Discord REST client authenticated with a bot token.
#[derive(Clone)]
pub struct DiscordClient {
    http: reqwest::Client,
    base_url: String,
    bot_token: String,
}

impl DiscordClient {
    /// Build from `DISCORD_BOT_TOKEN`.
    pub fn from_env() -> Result<Self, String> {
        Self::new(auth::bot_token_from_env()?, auth::base_url_from_env())
    }

    /// Build with an explicit bot token and base URL.
    pub fn new(bot_token: String, base_url: String) -> Result<Self, String> {
        if bot_token.trim().is_empty() {
            return Err(format!("missing {}", auth::DISCORD_BOT_TOKEN_ENV));
        }
        let http = reqwest::Client::builder()
            .user_agent(concat!(
                "argo-mcp-connector-discord/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .map_err(|e| format!("discord http client: {e}"))?;
        Ok(Self {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
            bot_token,
        })
    }

    /// GET `/users/@me/guilds`.
    pub async fn list_guilds(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        let mut query = Vec::new();
        push_limit(args, &mut query, 100);
        push_snowflake(args, &mut query, "before")?;
        push_snowflake(args, &mut query, "after")?;
        self.get("/users/@me/guilds", query).await
    }

    /// GET `/guilds/{guild_id}/channels`.
    pub async fn list_channels(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let guild_id = clean_snowflake(&required_str(args, "guild_id")?)?;
        self.get(&format!("/guilds/{guild_id}/channels"), Vec::new())
            .await
    }

    /// GET `/channels/{channel_id}/messages`.
    pub async fn list_messages(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let channel_id = clean_snowflake(&required_str(args, "channel_id")?)?;
        let mut query = Vec::new();
        push_limit(args, &mut query, 50);
        push_snowflake(args, &mut query, "before")?;
        push_snowflake(args, &mut query, "after")?;
        self.get(&format!("/channels/{channel_id}/messages"), query)
            .await
    }

    /// GET `/channels/{channel_id}/messages/{message_id}`.
    pub async fn get_message(&self, args: &serde_json::Value) -> Result<serde_json::Value, String> {
        let channel_id = clean_snowflake(&required_str(args, "channel_id")?)?;
        let message_id = clean_snowflake(&required_str(args, "message_id")?)?;
        self.get(
            &format!("/channels/{channel_id}/messages/{message_id}"),
            Vec::new(),
        )
        .await
    }

    /// POST `/channels/{channel_id}/messages`.
    pub async fn send_message(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let channel_id = clean_snowflake(&required_str(args, "channel_id")?)?;
        let content = required_str(args, "content")?;
        let mut body = serde_json::json!({ "content": content });
        copy_value(args, &mut body, "allowed_mentions");
        copy_value(args, &mut body, "message_reference");
        self.post_json(&format!("/channels/{channel_id}/messages"), body)
            .await
    }

    /// PUT `/channels/{channel_id}/messages/{message_id}/reactions/{emoji}/@me`.
    pub async fn add_reaction(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let channel_id = clean_snowflake(&required_str(args, "channel_id")?)?;
        let message_id = clean_snowflake(&required_str(args, "message_id")?)?;
        let emoji = encode_emoji(&required_str(args, "emoji")?)?;
        self.put_empty(&format!(
            "/channels/{channel_id}/messages/{message_id}/reactions/{emoji}/@me"
        ))
        .await
    }

    /// GET `/guilds/{guild_id}/members`.
    pub async fn list_members(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let guild_id = clean_snowflake(&required_str(args, "guild_id")?)?;
        let mut query = Vec::new();
        push_limit(args, &mut query, 1000);
        push_snowflake(args, &mut query, "after")?;
        self.get(&format!("/guilds/{guild_id}/members"), query)
            .await
    }

    async fn get(
        &self,
        path: &str,
        query: Vec<(String, String)>,
    ) -> Result<serde_json::Value, String> {
        let resp = self
            .http
            .get(format!("{}{}", self.base_url, path))
            .header("Authorization", format!("Bot {}", self.bot_token))
            .query(&query)
            .send()
            .await
            .map_err(|e| format!("discord GET {path}: {e}"))?;
        decode_response("GET", path, resp).await
    }

    async fn post_json(
        &self,
        path: &str,
        body: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let resp = self
            .http
            .post(format!("{}{}", self.base_url, path))
            .header("Authorization", format!("Bot {}", self.bot_token))
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("discord POST {path}: {e}"))?;
        decode_response("POST", path, resp).await
    }

    async fn put_empty(&self, path: &str) -> Result<serde_json::Value, String> {
        let resp = self
            .http
            .put(format!("{}{}", self.base_url, path))
            .header("Authorization", format!("Bot {}", self.bot_token))
            .send()
            .await
            .map_err(|e| format!("discord PUT {path}: {e}"))?;
        decode_response("PUT", path, resp).await
    }
}

/// Convert a Discord tool result into MCP text content.
pub fn to_call_result(result: Result<serde_json::Value, String>) -> CallToolResult {
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

async fn decode_response(
    method: &str,
    path: &str,
    resp: reqwest::Response,
) -> Result<serde_json::Value, String> {
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| format!("discord {method} {path}: read response: {e}"))?;
    let parsed = if text.trim().is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str(&text)
            .map_err(|e| format!("discord {method} {path}: decode response: {e}: {text}"))?
    };
    if status.is_success() {
        Ok(parsed)
    } else {
        Err(format!("discord {method} {path}: HTTP {status}: {parsed}"))
    }
}

fn required_str(args: &serde_json::Value, field: &str) -> Result<String, String> {
    args.get(field)
        .and_then(|v| v.as_str())
        .filter(|v| !v.trim().is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("missing string field '{field}'"))
}

fn copy_value(args: &serde_json::Value, body: &mut serde_json::Value, field: &str) {
    if let Some(value) = args.get(field) {
        body[field] = value.clone();
    }
}

fn push_limit(args: &serde_json::Value, out: &mut Vec<(String, String)>, max: u64) {
    if let Some(value) = args.get("limit").and_then(|v| v.as_u64()) {
        out.push(("limit".into(), value.clamp(1, max).to_string()));
    }
}

fn push_snowflake(
    args: &serde_json::Value,
    out: &mut Vec<(String, String)>,
    field: &str,
) -> Result<(), String> {
    if let Some(value) = args.get(field).and_then(|v| v.as_str()) {
        out.push((field.into(), clean_snowflake(value)?));
    }
    Ok(())
}

fn clean_snowflake(raw: &str) -> Result<String, String> {
    if !raw.is_empty() && raw.bytes().all(|b| b.is_ascii_digit()) {
        Ok(raw.to_string())
    } else {
        Err(format!("unsafe Discord snowflake: {raw}"))
    }
}

fn encode_emoji(raw: &str) -> Result<String, String> {
    if raw.trim().is_empty() || raw.contains('/') {
        return Err(format!("unsafe Discord emoji: {raw}"));
    }
    let mut out = String::new();
    for b in raw.as_bytes() {
        if b.is_ascii_alphanumeric() || matches!(*b, b'-' | b'_' | b'.' | b'~') {
            out.push(char::from(*b));
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    #[tokio::test]
    async fn list_messages_sends_bot_auth_and_query() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = [0; 4096];
            let n = stream.read(&mut buf).unwrap();
            let req = String::from_utf8_lossy(&buf[..n]);
            assert!(req.contains("GET /channels/123/messages?limit=25&after=456"));
            assert!(req.to_ascii_lowercase().contains("authorization: bot tok"));
            let body = r#"[{"id":"789","content":"hello"}]"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).unwrap();
        });
        let client = DiscordClient::new("tok".into(), format!("http://{addr}")).unwrap();
        let value = client
            .list_messages(&serde_json::json!({
                "channel_id": "123",
                "limit": 25,
                "after": "456"
            }))
            .await
            .unwrap();
        assert_eq!(value[0]["id"], "789");
    }

    #[test]
    fn emoji_encoding_escapes_custom_reaction() {
        assert_eq!(encode_emoji("party:123").unwrap(), "party%3A123");
    }
}
