//! Pure helpers used by the agent loop in `turn.rs`. Public so
//! external drivers (e.g. eval harnesses) can reuse the same
//! lifting + spend math without re-implementing it.
use argo_provider::{ProviderManifest, ProviderRequest, ProviderResponseDelta, Transport};
use argo_store::SessionStore;
use argo_types::{
    ArgoError, ArgoResult, ContentBlock, Message, MessageId, MessageRole, SessionId, TenantId,
    ToolCallId, ToolCallRequest,
};
use chrono::Utc;
use futures::StreamExt;
use std::collections::HashMap;
use std::io::Write;
use std::sync::Arc;

/// One per-turn event emitted by the agent loop. Used by the
/// gateway's `POST /sessions/<id>/runs/stream` endpoint to push
/// chunks to the HTTP client in real time without polling. The
/// CLI handler ignores the sink (passes `None`).
///
/// Each variant maps 1:1 to an `AuditKind`, which keeps the wire
/// shape stable: clients can parse `RunEvent` JSON the same way
/// they parse `ExportedRow.payload`.
pub type RunEventSink = tokio::sync::mpsc::UnboundedSender<serde_json::Value>;

/// Send a JSON event to the optional sink. Drops the message if
/// the receiver has been closed (the gateway streaming task may
/// have torn down e.g. on client disconnect).
pub fn emit_event(sink: Option<&RunEventSink>, kind: &str, payload: serde_json::Value) {
    if let Some(tx) = sink {
        let env = serde_json::json!({"event": kind, "payload": payload});
        let _ = tx.send(env);
    }
}

/// Per-round response shape used by both the streaming and non-
/// streaming paths in the agent loop. Lets the loop body stay
/// one shape regardless of which transport call produced it.
pub struct RoundResp {
    /// Assistant text accumulated across this round.
    pub text: String,
    /// Tool calls the assistant requested (in wire order).
    pub tool_calls: Vec<ToolCallRequest>,
    /// Provider-reported input tokens, if any.
    pub tokens_in: Option<u32>,
    /// Provider-reported output tokens, if any.
    pub tokens_out: Option<u32>,
}

/// Stream one provider round, printing text chunks to stdout as
/// they arrive and accumulating tool-call deltas into proper
/// `ToolCallRequest`s. The agent loop dispatches any returned
/// tool calls just as it does for the non-streaming path.
pub async fn stream_one_round(
    transport: &dyn Transport,
    req: &ProviderRequest,
) -> ArgoResult<RoundResp> {
    let mut stream = transport.complete_stream(req).await?;
    let mut text = String::new();
    let mut tokens_in: Option<u32> = None;
    let mut tokens_out: Option<u32> = None;
    // Accumulator: provider call_id → (tool_name, args fragment).
    let mut acc: HashMap<String, (String, String)> = HashMap::new();
    // Order in which call ids first appear, so the dispatch order
    // matches the wire order rather than HashMap iteration.
    let mut order: Vec<String> = Vec::new();
    let mut stdout = std::io::stdout();
    while let Some(delta) = stream.next().await {
        match delta? {
            ProviderResponseDelta::TextChunk(s) => {
                text.push_str(&s);
                let _ = stdout.write_all(s.as_bytes());
                let _ = stdout.flush();
            }
            ProviderResponseDelta::TokenUsage {
                tokens_in: ti,
                tokens_out: to,
            } => {
                tokens_in = Some(ti);
                tokens_out = Some(to);
            }
            ProviderResponseDelta::ToolCallDelta {
                id,
                name,
                args_partial,
            } => {
                let entry = acc.entry(id.clone()).or_insert_with(|| {
                    order.push(id.clone());
                    (String::new(), String::new())
                });
                if !name.is_empty() {
                    entry.0 = name;
                }
                entry.1.push_str(&args_partial);
            }
            ProviderResponseDelta::FinishReason(_) | ProviderResponseDelta::ReasoningChunk(_) => {
                // Ignored for now — reasoning streams will land with
                // the dedicated thinking-block surface.
            }
        }
    }
    let mut tool_calls = Vec::with_capacity(order.len());
    for provider_id in order {
        let (name, args_str) = acc.remove(&provider_id).unwrap_or_default();
        let arguments = if args_str.is_empty() {
            serde_json::json!({})
        } else {
            serde_json::from_str(&args_str).unwrap_or(serde_json::Value::String(args_str.clone()))
        };
        tool_calls.push(ToolCallRequest {
            id: ToolCallId::new(),
            provider_call_id: Some(provider_id),
            name,
            arguments,
        });
    }
    Ok(RoundResp {
        text,
        tool_calls,
        tokens_in,
        tokens_out,
    })
}

/// Static-for-the-turn data the agent loop needs to record spend
/// (tenant + provider + model + price-per-1k tuple).
pub struct SpendCtx<'a> {
    /// Acting tenant for the spend row.
    pub tenant_id: TenantId,
    /// Provider id (matches manifest `provider.id`).
    pub provider: &'a str,
    /// Model id (matches manifest `models[*].id`).
    pub model: &'a str,
    /// `(price_per_1k_in, price_per_1k_out)` if the manifest carries it.
    pub pricing: Option<(f64, f64)>,
}

/// Look up `(price_per_1k_in, price_per_1k_out)` for a given model
/// id within a manifest. Returns `None` if the model isn't listed
/// or if either price field is missing — chat skips spend recording
/// in that case (no silent zero-cost rows).
pub fn price_for(manifest: &ProviderManifest, model: &str) -> Option<(f64, f64)> {
    let spec = manifest.models.iter().find(|m| m.id == model)?;
    Some((spec.price_per_1k_in?, spec.price_per_1k_out?))
}

/// Convert `(tokens_in, tokens_out)` × pricing into USD-micros and
/// record one spend row per turn. No-op when pricing or token
/// counts are missing.
pub async fn record_turn_spend(
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
pub fn message_from_row(
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
pub fn lift_tool_blocks(
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
pub fn build_assistant_content(
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

    #[tokio::test]
    async fn stream_one_round_accumulates_tool_calls_in_wire_order() {
        use argo_provider::{
            ProviderResponse, ProviderResponseDelta, Sampling, StopReason,
            ToolSchema as ProviderToolSchema,
        };
        use argo_types::CapabilitySet;
        use async_trait::async_trait;
        use futures::stream::{self, BoxStream};

        struct MockTransport;
        #[async_trait]
        impl Transport for MockTransport {
            fn id(&self) -> &str {
                "mock"
            }
            fn capabilities(&self) -> &CapabilitySet {
                static CAPS: std::sync::OnceLock<CapabilitySet> = std::sync::OnceLock::new();
                CAPS.get_or_init(CapabilitySet::empty)
            }
            async fn complete(&self, _: &ProviderRequest) -> ArgoResult<ProviderResponse> {
                unreachable!("test only uses streaming path")
            }
            async fn complete_stream(
                &self,
                _: &ProviderRequest,
            ) -> ArgoResult<BoxStream<'static, ArgoResult<ProviderResponseDelta>>> {
                let deltas = vec![
                    Ok(ProviderResponseDelta::TextChunk("hi ".into())),
                    Ok(ProviderResponseDelta::ToolCallDelta {
                        id: "call_a".into(),
                        name: "ls".into(),
                        args_partial: "{\"path\":\"/\"".into(),
                    }),
                    Ok(ProviderResponseDelta::ToolCallDelta {
                        id: "call_a".into(),
                        name: String::new(),
                        args_partial: "}".into(),
                    }),
                    Ok(ProviderResponseDelta::ToolCallDelta {
                        id: "call_b".into(),
                        name: "echo".into(),
                        args_partial: "{}".into(),
                    }),
                    Ok(ProviderResponseDelta::TextChunk("there".into())),
                    Ok(ProviderResponseDelta::TokenUsage {
                        tokens_in: 12,
                        tokens_out: 4,
                    }),
                    Ok(ProviderResponseDelta::FinishReason(StopReason::ToolUse)),
                ];
                Ok(Box::pin(stream::iter(deltas)))
            }
        }

        let req = ProviderRequest {
            model: "m".into(),
            messages: vec![],
            tools: Vec::<ProviderToolSchema>::new(),
            system: None,
            sampling: Sampling::default(),
        };
        let resp = stream_one_round(&MockTransport, &req).await.unwrap();
        assert_eq!(resp.text, "hi there");
        assert_eq!(resp.tokens_in, Some(12));
        assert_eq!(resp.tokens_out, Some(4));
        assert_eq!(resp.tool_calls.len(), 2);
        assert_eq!(resp.tool_calls[0].name, "ls");
        assert_eq!(
            resp.tool_calls[0].provider_call_id.as_deref(),
            Some("call_a")
        );
        assert_eq!(resp.tool_calls[0].arguments["path"], "/");
        assert_eq!(resp.tool_calls[1].name, "echo");
        assert_eq!(
            resp.tool_calls[1].provider_call_id.as_deref(),
            Some("call_b")
        );
    }
}
