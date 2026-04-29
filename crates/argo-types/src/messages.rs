//! Conversation message types.
use crate::ids::{MessageId, SessionId, ToolCallId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Role in a conversation.
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRequest {
    /// Stable identifier.
    pub id: ToolCallId,
    /// Tool name.
    pub name: String,
    /// JSON arguments per the tool's schema.
    pub arguments: serde_json::Value,
}

/// The outcome of a tool invocation.
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
