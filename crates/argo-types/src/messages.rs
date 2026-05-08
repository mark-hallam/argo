//! Conversation message types.
use crate::ids::{MessageId, SessionId, ToolCallId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Role in a conversation.
///
/// ```
/// use argo_types::MessageRole;
/// let json = serde_json::to_string(&MessageRole::Assistant).unwrap();
/// assert_eq!(json, "\"assistant\"");
/// assert_eq!(serde_json::from_str::<MessageRole>(&json).unwrap(), MessageRole::Assistant);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    /// System / instruction prompt.
    System,
    /// End-user input.
    User,
    /// Model output.
    Assistant,
    /// Tool result.
    Tool,
}

/// A single message in a conversation.
///
/// ```
/// use argo_types::{ContentBlock, Message, MessageId, MessageRole, SessionId};
/// let msg = Message {
///     id: MessageId::new(), session_id: SessionId::new(), role: MessageRole::User,
///     content: vec![ContentBlock::Text { text: "hello".into() }], tool_calls: vec![],
///     tool_call_id: None, reasoning: None, created_at: chrono::Utc::now(),
///     provider_meta: serde_json::Value::Null,
/// };
/// assert!(serde_json::to_string(&msg).unwrap().contains("hello"));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Stable identifier.
    pub id: MessageId,
    /// Owning session.
    pub session_id: SessionId,
    /// Role.
    pub role: MessageRole,
    /// One or more content blocks (text, image, tool result, …).
    pub content: Vec<ContentBlock>,
    /// Tool calls emitted by an Assistant message (empty otherwise).
    #[serde(default)]
    pub tool_calls: Vec<ToolCallRequest>,
    /// When `role == Tool`, the call this message answers.
    #[serde(default)]
    pub tool_call_id: Option<ToolCallId>,
    /// Reasoning content (for models that emit it; e.g., Anthropic thinking).
    #[serde(default)]
    pub reasoning: Option<String>,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Opaque provider-specific artifacts.
    #[serde(default)]
    pub provider_meta: serde_json::Value,
}

/// A piece of message content.
///
/// ```
/// use argo_types::ContentBlock;
/// let block = ContentBlock::Text { text: "done".into() };
/// assert!(serde_json::to_string(&block).unwrap().contains("text"));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    /// Plain text.
    Text {
        /// The text body.
        text: String,
    },
    /// An image.
    Image {
        /// URL or data URI.
        url: String,
        /// Optional alt text.
        #[serde(default)]
        alt: Option<String>,
    },
    /// An audio attachment.
    Audio {
        /// URL.
        url: String,
        /// MIME type.
        mime: String,
    },
    /// A document attachment.
    Document {
        /// URL.
        url: String,
        /// MIME type.
        mime: String,
    },
    /// A tool-use request (assistant authored).
    ToolUse(ToolCallRequest),
    /// A tool-use result (tool authored).
    ToolResult(ToolCallResult),
}

/// A request from an Assistant turn to invoke a tool.
///
/// ```
/// use argo_types::{ToolCallId, ToolCallRequest};
/// let call = ToolCallRequest {
///     id: ToolCallId::new(), provider_call_id: Some("call_123".into()),
///     name: "search".into(), arguments: serde_json::json!({"q": "argo"}),
/// };
/// assert_eq!(call.arguments["q"], "argo");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRequest {
    /// Stable internal identifier (UUIDv7). Always set; uniquely
    /// identifies the call within Argo's audit + replay.
    pub id: ToolCallId,
    /// Provider-issued opaque identifier echoed back when sending tool
    /// results so the provider can correlate. Set when extracted from a
    /// provider response that carried one (OpenAI `call_FAxa…`,
    /// Anthropic `toolu_…`, Bedrock `toolUseId`, OpenAI Responses
    /// `call_id`). The wire layer prefers this over `id.to_string()`
    /// when present.
    ///
    /// Issue #4 (F-9): without this round-trip, OpenAI rejects tool
    /// results in multi-turn sequences because the `tool_call_id` we
    /// echo doesn't match what they originally issued.
    #[serde(default)]
    pub provider_call_id: Option<String>,
    /// Tool name.
    pub name: String,
    /// JSON arguments per the tool's schema.
    pub arguments: serde_json::Value,
}

/// The outcome of a tool invocation.
///
/// ```
/// use argo_types::{ContentBlock, ToolCallId, ToolCallResult};
/// let result = ToolCallResult {
///     id: ToolCallId::new(), is_error: false,
///     content: vec![ContentBlock::Text { text: "ok".into() }],
///     duration_ms: 42, tokens_in: Some(3), tokens_out: Some(1),
/// };
/// assert!(!result.is_error);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResult {
    /// Correlates with the request.
    pub id: ToolCallId,
    /// Was the call an error?
    pub is_error: bool,
    /// Result content.
    pub content: Vec<ContentBlock>,
    /// Wallclock duration.
    pub duration_ms: u64,
    /// Provider-reported input tokens for the call (if known).
    pub tokens_in: Option<u32>,
    /// Provider-reported output tokens for the call (if known).
    pub tokens_out: Option<u32>,
}
