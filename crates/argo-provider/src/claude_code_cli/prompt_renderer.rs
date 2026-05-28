//! Render `(system, Vec<Message>)` into the single prompt string the
//! Claude Code CLI's `--print` mode accepts.
//!
//! The CLI takes one positional argument — the user's prompt. We
//! concatenate the system prompt + the message history into a single
//! transcript-style string. The CLI's own session memory layers on top,
//! but for one-shot dispatch we always include the full context inline
//! so the call is hermetic (no reliance on `~/.claude/projects/...`).
//!
//! Tool-calling is OUT OF SCOPE for v1 — tool calls in the history are
//! flattened into descriptive text. The brief explicitly defers tool
//! routing to v1.1.
use argo_types::{ContentBlock, Message, MessageRole};

/// Render a prompt for the CLI. `system` is emitted as a leading
/// `System:` block when present; messages render as `<Role>: <text>`
/// lines. Trailing `Assistant:` cue tells the model where to continue.
pub(super) fn render_prompt(system: Option<&str>, messages: &[Message]) -> String {
    let mut out = String::new();
    if let Some(s) = system {
        let s = s.trim();
        if !s.is_empty() {
            out.push_str("System: ");
            out.push_str(s);
            out.push_str("\n\n");
        }
    }
    for m in messages {
        let role = role_label(m.role);
        let body = flatten_content(&m.content);
        if body.trim().is_empty() {
            continue;
        }
        out.push_str(role);
        out.push_str(": ");
        out.push_str(body.trim());
        out.push_str("\n\n");
    }
    out.push_str("Assistant:");
    out
}

fn role_label(r: MessageRole) -> &'static str {
    match r {
        MessageRole::System => "System",
        MessageRole::User => "User",
        MessageRole::Assistant => "Assistant",
        MessageRole::Tool => "Tool",
    }
}

fn flatten_content(blocks: &[ContentBlock]) -> String {
    let mut out = String::new();
    for b in blocks {
        match b {
            ContentBlock::Text { text } => {
                if !out.is_empty() {
                    out.push('\n');
                }
                out.push_str(text);
            }
            ContentBlock::Image { url, alt } => {
                if !out.is_empty() {
                    out.push('\n');
                }
                let alt = alt.as_deref().unwrap_or("image");
                out.push_str(&format!("[image: {alt} @ {url}]"));
            }
            ContentBlock::Audio { url, mime } => {
                if !out.is_empty() {
                    out.push('\n');
                }
                out.push_str(&format!("[audio: {mime} @ {url}]"));
            }
            ContentBlock::Document { url, mime } => {
                if !out.is_empty() {
                    out.push('\n');
                }
                out.push_str(&format!("[document: {mime} @ {url}]"));
            }
            ContentBlock::ToolUse(_) | ContentBlock::ToolResult(_) => {
                // Tool routing is out of scope for the v1 CLI driver; the
                // brief defers to v1.1. Flatten to a placeholder so the
                // transcript stays readable.
                if !out.is_empty() {
                    out.push('\n');
                }
                out.push_str("[tool interaction omitted in CLI v1]");
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_types::{MessageId, SessionId};
    use chrono::Utc;

    fn msg(role: MessageRole, text: &str) -> Message {
        Message {
            id: MessageId::new(),
            session_id: SessionId::new(),
            role,
            content: vec![ContentBlock::Text { text: text.into() }],
            tool_calls: vec![],
            tool_call_id: None,
            reasoning: None,
            created_at: Utc::now(),
            provider_meta: serde_json::Value::Null,
        }
    }

    #[test]
    fn renders_system_then_user_then_assistant_cue() {
        let p = render_prompt(
            Some("You are a helpful agent."),
            &[msg(MessageRole::User, "hi")],
        );
        assert!(p.starts_with("System: You are a helpful agent.\n\nUser: hi"));
        assert!(p.trim_end().ends_with("Assistant:"));
    }

    #[test]
    fn omits_system_block_when_none() {
        let p = render_prompt(None, &[msg(MessageRole::User, "hello")]);
        assert!(!p.contains("System:"));
        assert!(p.contains("User: hello"));
    }

    #[test]
    fn flattens_multi_turn_transcript() {
        let p = render_prompt(
            None,
            &[
                msg(MessageRole::User, "what is 2+2?"),
                msg(MessageRole::Assistant, "4"),
                msg(MessageRole::User, "and 3+3?"),
            ],
        );
        assert!(p.contains("User: what is 2+2?"));
        assert!(p.contains("Assistant: 4"));
        assert!(p.contains("User: and 3+3?"));
        assert!(p.trim_end().ends_with("Assistant:"));
    }

    #[test]
    fn skips_empty_message_bodies() {
        let mut empty = msg(MessageRole::User, "");
        empty.content = vec![];
        let p = render_prompt(None, &[empty, msg(MessageRole::User, "hello")]);
        // Empty body should not produce a `User: ` line.
        assert_eq!(p.matches("User:").count(), 1);
    }
}
