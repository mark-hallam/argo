//! Context-compression / truncation helpers extracted from
//! `turn_helpers.rs` to keep that file under the 500-LOC ceiling
//! (HARD RULE 6).
//!
//! Loaded via `#[path = "turn_helpers_context.rs"] mod
//! turn_helpers_context;` from `turn.rs`, mirroring the sibling
//! module pattern used for `turn_helpers.rs` / `turn_memory.rs`.
//! The sole `pub(super)` entry point is
//! [`prepare_context_messages`], which `turn_helpers` calls before
//! sending the message list to the provider.
use super::ContextWindowPolicy;
use argo_audit::{AuditKind, AuditSink};
use argo_runtime::{
    context_compress::{self, TokenCounter},
    context_truncate,
};
use argo_types::{ArgoError, ArgoResult, ContentBlock, Message, MessageRole, SessionId};
use async_trait::async_trait;

const TRUNCATE_THRESHOLD_PCT: usize = 95;

pub(super) async fn prepare_context_messages(
    messages: Vec<Message>,
    policy: &ContextWindowPolicy,
    audit: &AuditSink,
    session_id: SessionId,
) -> ArgoResult<Vec<Message>> {
    if !policy.compression.enabled || policy.window_tokens == 0 {
        return Ok(messages);
    }
    let counter = ApproxTokenCounter;
    let summarizer = ExtractiveSummarizer;
    let mut view = messages;
    let token_count: usize = view.iter().map(|m| counter.count_message(m)).sum();

    if context_compress::should_compress(token_count, policy.window_tokens, &policy.compression) {
        let compressed =
            context_compress::compress(&view, &policy.compression, &counter, &summarizer)
                .await
                .map_err(|e| ArgoError::Provider(format!("context compression: {e}")))?;
        if compressed.compressed() {
            audit
                .write(
                    Some(session_id),
                    AuditKind::RuntimeContextCompressed,
                    context_compress::build_audit_payload(&compressed),
                    None,
                )
                .await?;
        }
        view = compressed.view;
    }

    let truncate_tokens = policy
        .window_tokens
        .saturating_mul(TRUNCATE_THRESHOLD_PCT)
        .checked_div(100)
        .unwrap_or(policy.window_tokens)
        .max(1);
    let token_count: usize = view.iter().map(|m| counter.count_message(m)).sum();
    if token_count >= truncate_tokens {
        let truncated = context_truncate::truncate_to_fit(&view, truncate_tokens, &counter)?;
        if truncated.truncated() {
            let payload = serde_json::json!({
                "event_type": context_truncate::AUDIT_KIND_LABEL,
                "details": context_truncate::build_audit_payload(&truncated),
            });
            audit
                .write(Some(session_id), AuditKind::HttpRequest, payload, None)
                .await?;
        }
        view = truncated.view;
    }

    Ok(view)
}

struct ApproxTokenCounter;

impl context_compress::TokenCounter for ApproxTokenCounter {
    fn count_message(&self, msg: &Message) -> usize {
        let role_overhead = 4usize;
        role_overhead
            + msg.content.iter().map(count_content_block).sum::<usize>()
            + msg.reasoning.as_deref().map_or(0, approximate_text_tokens)
    }

    fn count_text(&self, text: &str) -> usize {
        approximate_text_tokens(text)
    }
}

fn count_content_block(block: &ContentBlock) -> usize {
    match block {
        ContentBlock::Text { text } => approximate_text_tokens(text),
        ContentBlock::Image { url, alt } => {
            approximate_text_tokens(url) + alt.as_deref().map_or(0, approximate_text_tokens)
        }
        ContentBlock::Audio { url, mime } | ContentBlock::Document { url, mime } => {
            approximate_text_tokens(url) + approximate_text_tokens(mime)
        }
        ContentBlock::ToolUse(call) => {
            approximate_text_tokens(&call.name)
                + approximate_text_tokens(&call.arguments.to_string())
        }
        ContentBlock::ToolResult(result) => {
            approximate_text_tokens(&serde_json::to_string(result).unwrap_or_default())
        }
    }
}

fn approximate_text_tokens(text: &str) -> usize {
    text.len()
        .saturating_add(3)
        .checked_div(4)
        .unwrap_or(0)
        .max(1)
}

struct ExtractiveSummarizer;

#[async_trait]
impl context_compress::Summarizer for ExtractiveSummarizer {
    async fn summarize(
        &self,
        older_turns: &[Message],
        max_tokens: usize,
    ) -> Result<context_compress::Summary, context_compress::CompressionError> {
        let mut text = String::new();
        for msg in older_turns {
            let role = match msg.role {
                MessageRole::System => "system",
                MessageRole::User => "user",
                MessageRole::Assistant => "assistant",
                MessageRole::Tool => "tool",
            };
            let line = format!("{role}: {}\n", render_message_text(msg));
            if approximate_text_tokens(&(text.clone() + &line)) > max_tokens {
                break;
            }
            text.push_str(&line);
        }
        if text.trim().is_empty() {
            text = "Earlier conversation omitted during context compression.".to_string();
        }
        let token_count = approximate_text_tokens(&text);
        Ok(context_compress::Summary { text, token_count })
    }
}

fn render_message_text(msg: &Message) -> String {
    msg.content
        .iter()
        .map(|block| match block {
            ContentBlock::Text { text } => text.clone(),
            ContentBlock::Image { url, alt } => {
                format!("[image: {}{}]", url, alt.as_deref().map_or("", |a| a))
            }
            ContentBlock::Audio { url, mime } => format!("[audio: {mime} {url}]"),
            ContentBlock::Document { url, mime } => format!("[document: {mime} {url}]"),
            ContentBlock::ToolUse(call) => format!("[tool_use: {}]", call.name),
            ContentBlock::ToolResult(result) => format!("[tool_result: {}]", result.is_error),
        })
        .collect::<Vec<_>>()
        .join(" ")
}
