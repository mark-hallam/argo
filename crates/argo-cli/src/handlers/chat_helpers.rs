//! Pure helpers used by the agent loop in `chat.rs`. Extracted
//! to keep that file under the 500-LOC budget.
use argo_provider::ProviderManifest;
use argo_store::SessionStore;
use argo_types::{
    ArgoError, ArgoResult, ContentBlock, Message, MessageId, MessageRole, SessionId, TenantId,
    ToolCallId, ToolCallRequest,
};
use chrono::Utc;
use std::sync::Arc;

/// Static-for-the-turn data the agent loop needs to record spend
/// (tenant + provider + model + price-per-1k tuple).
pub(crate) struct SpendCtx<'a> {
    pub(crate) tenant_id: TenantId,
    pub(crate) provider: &'a str,
    pub(crate) model: &'a str,
    pub(crate) pricing: Option<(f64, f64)>,
}

/// Look up `(price_per_1k_in, price_per_1k_out)` for a given model
/// id within a manifest. Returns `None` if the model isn't listed
/// or if either price field is missing — chat skips spend recording
/// in that case (no silent zero-cost rows).
pub(crate) fn price_for(manifest: &ProviderManifest, model: &str) -> Option<(f64, f64)> {
    let spec = manifest.models.iter().find(|m| m.id == model)?;
    Some((spec.price_per_1k_in?, spec.price_per_1k_out?))
}

/// Convert `(tokens_in, tokens_out)` × pricing into USD-micros and
/// record one spend row per turn. No-op when pricing or token
/// counts are missing.
pub(crate) async fn record_turn_spend(
    store: &Arc<SessionStore>,
    session_id: SessionId,
    spend: &SpendCtx<'_>,
    tokens_in: Option<u32>,
    tokens_out: Option<u32>,
) -> ArgoResult<()> {
    let (Some((p_in, p_out)), Some(ti), Some(to)) = (spend.pricing, tokens_in, tokens_out) else {
        return Ok(());
    };
    let usd = (f64::from(ti) * p_in + f64::from(to) * p_out) / 1000.0;
    let micros = (usd * 1_000_000.0).round().max(0.0) as u64;
    if micros == 0 {
        return Ok(());
    }
    store
        .record_spend(
            spend.tenant_id,
            session_id,
            Utc::now().timestamp_micros(),
            micros,
            spend.provider,
            spend.model,
        )
        .await?;
    Ok(())
}

/// Convert a `MessageRow` from the store into the canonical
/// `Message` the wire layer expects. Tool-call / tool-result
/// content blocks are lifted onto `tool_calls` / `tool_call_id`
/// (see [`lift_tool_blocks`]).
pub(crate) fn message_from_row(
    row: &argo_store::MessageRow,
    session_id: SessionId,
) -> ArgoResult<Message> {
    let id_bytes: [u8; 16] = row
        .id
        .as_slice()
        .try_into()
        .map_err(|_| ArgoError::Provider("message id is not 16 bytes".into()))?;
    let role = match row.role.as_str() {
        "system" => MessageRole::System,
        "user" => MessageRole::User,
        "assistant" => MessageRole::Assistant,
        "tool" => MessageRole::Tool,
        other => {
            return Err(ArgoError::Provider(format!("unknown role '{other}'")));
        }
    };
    let raw_content: Vec<ContentBlock> = serde_json::from_str(&row.content_json).map_err(|e| {
        ArgoError::Provider(format!("parse content_json: {e}: {}", row.content_json))
    })?;
    let (content, tool_calls, tool_call_id) = lift_tool_blocks(raw_content);
    let created_at = chrono::DateTime::from_timestamp_micros(row.created_at)
        .ok_or_else(|| ArgoError::Provider("created_at out of range".into()))?
        .with_timezone(&Utc);
    Ok(Message {
        id: MessageId(uuid::Uuid::from_bytes(id_bytes)),
        session_id,
        role,
        content,
        tool_calls,
        tool_call_id,
        reasoning: None,
        created_at,
        provider_meta: serde_json::Value::Null,
    })
}

/// Split a `Vec<ContentBlock>` (the on-disk form) into the shape
/// the wire layer expects: plain content blocks stay on
/// `Message.content`, `ToolUse` blocks get hoisted to
/// `Message.tool_calls`, and the first `ToolResult`'s id is
/// hoisted to `Message.tool_call_id`. Tool-result blocks remain
/// in `content` so wire layers (Anthropic `push_tool_result`)
/// can read the body.
pub(crate) fn lift_tool_blocks(
    blocks: Vec<ContentBlock>,
) -> (Vec<ContentBlock>, Vec<ToolCallRequest>, Option<ToolCallId>) {
    let mut content = Vec::with_capacity(blocks.len());
    let mut tool_calls = Vec::new();
    let mut tool_call_id: Option<ToolCallId> = None;
    for b in blocks {
        match b {
            ContentBlock::ToolUse(tc) => tool_calls.push(tc),
            ContentBlock::ToolResult(r) => {
                if tool_call_id.is_none() {
                    tool_call_id = Some(r.id);
                }
                content.push(ContentBlock::ToolResult(r));
            }
            other => content.push(other),
        }
    }
    (content, tool_calls, tool_call_id)
}

/// Build the `content_json` payload for an assistant message
/// that may also carry tool calls. Tool calls are serialized as
/// `ContentBlock::ToolUse` blocks alongside any leading text.
pub(crate) fn build_assistant_content(
    text: &str,
    tool_calls: &[ToolCallRequest],
) -> serde_json::Value {
    let mut blocks: Vec<ContentBlock> = Vec::new();
    if !text.is_empty() {
        blocks.push(ContentBlock::Text {
            text: text.to_string(),
        });
    }
    for tc in tool_calls {
        blocks.push(ContentBlock::ToolUse(tc.clone()));
    }
    serde_json::json!(blocks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_types::ToolCallId;

    #[test]
    fn lift_extracts_tool_uses_and_first_tool_result_id() {
        let cid = ToolCallId::new();
        let blocks = vec![
            ContentBlock::Text {
                text: "ok".into(),
            },
            ContentBlock::ToolUse(ToolCallRequest {
                id: ToolCallId::new(),
                provider_call_id: None,
                name: "x".into(),
                arguments: serde_json::json!({}),
            }),
            ContentBlock::ToolResult(argo_types::ToolCallResult {
                id: cid,
                is_error: false,
                content: vec![],
                duration_ms: 0,
                tokens_in: None,
                tokens_out: None,
            }),
        ];
        let (content, tool_calls, tool_call_id) = lift_tool_blocks(blocks);
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_call_id, Some(cid));
        assert_eq!(content.len(), 2); // text + ToolResult
    }

    #[test]
    fn build_assistant_content_skips_empty_text() {
        let v = build_assistant_content("", &[]);
        assert_eq!(v.as_array().map(Vec::len), Some(0));
    }
}
