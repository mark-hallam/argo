//! Response parsing for the Anthropic CU loop driver. Extracted
//! from `anthropic.rs` to keep that file under the per-file
//! 500-LOC budget. The entry point is [`parse_response`], which
//! walks the model's content blocks and yields one or more
//! [`CuStepResponse`] entries — a [`CuStepResponse::ToolCall`] per
//! `tool_use` block in document order, optionally followed by a
//! [`CuStepResponse::Stop`] when the model emits a stop signal in
//! the same turn.

use crate::anthropic::{CuStepResponse, StopReason};
use crate::e2b::CuAction;
use crate::CuError;

/// Parse the model's reply into the ordered sequence of step
/// responses it represents. A turn may carry multiple `tool_use`
/// blocks (e.g. `mouse_move` then `left_click`); the executor
/// dispatches each in order before re-screenshotting.
///
/// The returned `Vec` is guaranteed non-empty: every `Ok` carries
/// at least one entry. If the response carries neither a
/// `tool_use` block nor a recognised stop-signal text block, the
/// canonical `"anthropic response carried no tool_use or
/// recognised stop signal"` error surfaces verbatim.
pub(crate) fn parse_response(
    raw: serde_json::Value,
) -> Result<Vec<CuStepResponse>, CuError> {
    let usage = raw.get("usage").and_then(|u| u.as_object());
    let tokens_used = usage
        .and_then(|u| {
            let i = u.get("input_tokens").and_then(serde_json::Value::as_u64)?;
            let o = u.get("output_tokens").and_then(serde_json::Value::as_u64)?;
            Some(u32::try_from(i.saturating_add(o)).unwrap_or(u32::MAX))
        })
        .unwrap_or(0);

    let content = raw
        .get("content")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| {
            CuError::Protocol("anthropic response missing `content` array".into())
        })?;

    let mut out: Vec<CuStepResponse> = Vec::new();
    for block in content {
        let kind = block.get("type").and_then(serde_json::Value::as_str);
        if kind == Some("tool_use") {
            let name = block
                .get("name")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            let input = block
                .get("input")
                .cloned()
                .unwrap_or(serde_json::json!({}));
            // Token charge is per-turn, not per-action; only the
            // first parsed entry carries the model's token usage
            // so the budget tracker doesn't double-charge a
            // multi-action turn.
            let charge = if out.is_empty() { tokens_used } else { 0 };
            out.push(parse_tool_use(name, &input, charge)?);
        } else if kind == Some("text") {
            let text = block
                .get("text")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            if let Some(stop) = recognise_text_stop(text) {
                let charge = if out.is_empty() { tokens_used } else { 0 };
                out.push(CuStepResponse::Stop {
                    reason: stop,
                    text: Some(text.to_string()),
                    tokens_used: charge,
                });
            }
        }
    }

    if out.is_empty() {
        return Err(CuError::Protocol(
            "anthropic response carried no tool_use or recognised stop signal".into(),
        ));
    }
    Ok(out)
}

fn parse_tool_use(
    name: &str,
    input: &serde_json::Value,
    tokens_used: u32,
) -> Result<CuStepResponse, CuError> {
    // The CU tool's `action` field is the primitive name.
    let action_name = input
        .get("action")
        .and_then(serde_json::Value::as_str)
        .unwrap_or(name);
    let coord = input
        .get("coordinate")
        .and_then(serde_json::Value::as_array);
    let xy = coord.and_then(|arr| {
        let x = arr.first().and_then(serde_json::Value::as_u64)? as u32;
        let y = arr.get(1).and_then(serde_json::Value::as_u64)? as u32;
        Some((x, y))
    });
    match action_name {
        "left_click" | "click" => {
            let (x, y) = xy.ok_or_else(|| {
                CuError::Protocol("tool_use click missing coordinate".into())
            })?;
            Ok(CuStepResponse::ToolCall {
                action: CuAction::Click { x, y },
                tokens_used,
            })
        }
        "double_click" => {
            let (x, y) = xy.ok_or_else(|| {
                CuError::Protocol("tool_use double_click missing coordinate".into())
            })?;
            Ok(CuStepResponse::ToolCall {
                action: CuAction::DoubleClick { x, y },
                tokens_used,
            })
        }
        "mouse_move" => {
            let (x, y) = xy.ok_or_else(|| {
                CuError::Protocol("tool_use mouse_move missing coordinate".into())
            })?;
            Ok(CuStepResponse::ToolCall {
                action: CuAction::MouseMove { x, y },
                tokens_used,
            })
        }
        "type" => {
            let text = input
                .get("text")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| CuError::Protocol("tool_use type missing text".into()))?;
            Ok(CuStepResponse::ToolCall {
                action: CuAction::Type {
                    text: text.to_string(),
                },
                tokens_used,
            })
        }
        "key" => {
            let text = input
                .get("text")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| CuError::Protocol("tool_use key missing text".into()))?;
            // Anthropic encodes key chords as space-separated
            // tokens (e.g. "ctrl+a"), per the CU spec.
            let keys = text
                .split('+')
                .map(|s| s.trim().to_string())
                .collect::<Vec<_>>();
            Ok(CuStepResponse::ToolCall {
                action: CuAction::Key { keys },
                tokens_used,
            })
        }
        "scroll" => {
            let (x, y) = xy.unwrap_or((0, 0));
            let dy = input
                .get("scroll_amount")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(0) as i32;
            Ok(CuStepResponse::ToolCall {
                action: CuAction::Scroll { x, y, dy },
                tokens_used,
            })
        }
        "navigate" => {
            let url = input
                .get("url")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| CuError::Protocol("tool_use navigate missing url".into()))?;
            Ok(CuStepResponse::ToolCall {
                action: CuAction::Navigate {
                    url: url.to_string(),
                },
                tokens_used,
            })
        }
        "stop_success" => Ok(CuStepResponse::Stop {
            reason: StopReason::Success,
            text: input
                .get("text")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string),
            tokens_used,
        }),
        "stop_failure" => Ok(CuStepResponse::Stop {
            reason: StopReason::Failure,
            text: input
                .get("text")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string),
            tokens_used,
        }),
        other => Err(CuError::Protocol(format!(
            "tool_use has unknown action `{other}`"
        ))),
    }
}

/// Some prompts induce the model to emit a stop signal as plain
/// text rather than a tool call. We recognise the canonical
/// markers `stop_success` / `stop_failure` (case-insensitive,
/// anywhere in the text).
fn recognise_text_stop(text: &str) -> Option<StopReason> {
    let lower = text.to_lowercase();
    if lower.contains("stop_success") {
        Some(StopReason::Success)
    } else if lower.contains("stop_failure") {
        Some(StopReason::Failure)
    } else {
        None
    }
}
