//! Bedrock `ConverseStream` event-loop machinery.
//!
//! Decoupled from [`super::BedrockTransport`] so the pure parser
//! ([`StreamCore`] + [`handle_stream_event`]) can be unit-tested without
//! the SDK transport layer. The runner [`StreamRunner`] glues the SDK's
//! `EventReceiver` onto the parser inside the `complete_stream` future.

use crate::transport::{ProviderResponseDelta, StopReason};
use argo_types::{ArgoError, ArgoResult};
use aws_sdk_bedrockruntime::types as bt;

pub(super) struct StreamRunner {
    stream: aws_sdk_bedrockruntime::primitives::event_stream::EventReceiver<
        bt::ConverseStreamOutput,
        aws_sdk_bedrockruntime::types::error::ConverseStreamOutputError,
    >,
    core: StreamCore,
}

/// Pure mutable state for the Bedrock stream parser. Decoupled from
/// `EventReceiver` so unit tests can drive constructed events without the
/// SDK's transport layer.
#[derive(Debug, Default)]
pub(super) struct StreamCore {
    pub(super) pending: std::collections::VecDeque<ArgoResult<ProviderResponseDelta>>,
    pub(super) blocks: std::collections::HashMap<i32, BlockKind>,
    pub(super) done: bool,
}

impl StreamCore {
    pub(super) fn new() -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone)]
pub(super) enum BlockKind {
    Text,
    ToolUse { id: String, name: String },
}

impl StreamRunner {
    pub(super) fn new(
        stream: aws_sdk_bedrockruntime::primitives::event_stream::EventReceiver<
            bt::ConverseStreamOutput,
            aws_sdk_bedrockruntime::types::error::ConverseStreamOutputError,
        >,
    ) -> Self {
        Self {
            stream,
            core: StreamCore::new(),
        }
    }
}

pub(super) async fn step_stream(
    mut s: StreamRunner,
) -> Option<(ArgoResult<ProviderResponseDelta>, StreamRunner)> {
    if let Some(item) = s.core.pending.pop_front() {
        return Some((item, s));
    }
    if s.core.done {
        return None;
    }
    loop {
        match s.stream.recv().await {
            Ok(Some(event)) => {
                handle_stream_event(event, &mut s.core);
                if let Some(item) = s.core.pending.pop_front() {
                    return Some((item, s));
                }
                if s.core.done {
                    return None;
                }
            }
            Ok(None) => {
                s.core.done = true;
                return None;
            }
            Err(e) => {
                s.core.done = true;
                return Some((Err(ArgoError::Provider(format!("bedrock stream: {e}"))), s));
            }
        }
    }
}

pub(super) fn handle_stream_event(event: bt::ConverseStreamOutput, s: &mut StreamCore) {
    use bt::ConverseStreamOutput as Ev;
    match event {
        Ev::ContentBlockStart(start) => track_block_start(s, &start),
        Ev::ContentBlockDelta(delta) => emit_block_delta(s, &delta),
        Ev::MessageStop(stop) => emit_message_stop(s, &stop),
        Ev::Metadata(meta) => emit_metadata(s, &meta),
        _ => {} // MessageStart, ContentBlockStop, etc.
    }
}

fn track_block_start(s: &mut StreamCore, start: &bt::ContentBlockStartEvent) {
    let index = start.content_block_index();
    let kind = match start.start() {
        Some(bt::ContentBlockStart::ToolUse(tu)) => {
            let id = tu.tool_use_id().to_string();
            let name = tu.name().to_string();
            s.pending
                .push_back(Ok(ProviderResponseDelta::ToolCallDelta {
                    id: id.clone(),
                    name: name.clone(),
                    args_partial: String::new(),
                }));
            BlockKind::ToolUse { id, name }
        }
        _ => BlockKind::Text,
    };
    s.blocks.insert(index, kind);
}

fn emit_block_delta(s: &mut StreamCore, delta: &bt::ContentBlockDeltaEvent) {
    let index = delta.content_block_index();
    let kind = s.blocks.get(&index).cloned();
    match (kind, delta.delta()) {
        (Some(BlockKind::Text), Some(bt::ContentBlockDelta::Text(t))) if !t.is_empty() => {
            s.pending
                .push_back(Ok(ProviderResponseDelta::TextChunk(t.clone())));
        }
        (Some(BlockKind::ToolUse { id, name }), Some(bt::ContentBlockDelta::ToolUse(tu))) => {
            s.pending
                .push_back(Ok(ProviderResponseDelta::ToolCallDelta {
                    id,
                    name,
                    args_partial: tu.input().to_string(),
                }));
        }
        _ => {}
    }
}

fn emit_message_stop(s: &mut StreamCore, stop: &bt::MessageStopEvent) {
    let reason = match stop.stop_reason() {
        bt::StopReason::EndTurn => StopReason::EndTurn,
        bt::StopReason::ToolUse => StopReason::ToolUse,
        bt::StopReason::MaxTokens => StopReason::MaxTokens,
        bt::StopReason::ContentFiltered => StopReason::ContentFilter,
        other => StopReason::Other(other.as_str().to_string()),
    };
    s.pending
        .push_back(Ok(ProviderResponseDelta::FinishReason(reason)));
    s.done = true;
}

fn emit_metadata(s: &mut StreamCore, meta: &bt::ConverseStreamMetadataEvent) {
    if let Some(usage) = meta.usage() {
        s.pending.push_back(Ok(ProviderResponseDelta::TokenUsage {
            tokens_in: usage.input_tokens() as u32,
            tokens_out: usage.output_tokens() as u32,
        }));
    }
}
