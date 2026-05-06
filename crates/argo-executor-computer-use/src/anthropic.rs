//! Anthropic computer-use loop driver. Pinned to the
//! `computer_20241022` tool variant; the loop sends a chat
//! message containing the system prompt + the latest screenshot
//! and parses the response. The model emits either a
//! `tool_use` block with one of the CU primitives or a
//! `stop_reason` signal that ends the loop.
//!
//! Error handling: an HTTP failure surfaces as
//! [`crate::CuError::Http`]; an unparseable model response
//! surfaces as [`crate::CuError::Protocol`]. The caller (the
//! `executor` module) maps both to either `Completed` or
//! `HumanHandoff` outcomes per
//! [`CuError::is_recoverable_to_handoff`].

use crate::e2b::CuAction;
use crate::CuError;

/// Thin client over Anthropic's `/v1/messages`.
#[derive(Debug, Clone)]
pub struct AnthropicClient {
    base_url: String,
    api_key: String,
    model: String,
    http: reqwest::Client,
}

impl AnthropicClient {
    pub fn new(
        base_url: String,
        api_key: String,
        model: String,
        per_request_timeout_seconds: u32,
    ) -> Result<Self, CuError> {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(u64::from(
                per_request_timeout_seconds,
            )))
            .build()
            .map_err(|e| CuError::Http {
                stage: "anthropic/client.build",
                source: e,
            })?;
        Ok(Self {
            base_url,
            api_key,
            model,
            http,
        })
    }

    /// Send one CU loop step. Returns the parsed model response:
    /// either a tool call (next CU action) or a stop signal.
    pub async fn step(&self, req: CuStepRequest<'_>) -> Result<CuStepResponse, CuError> {
        let url = format!("{}/v1/messages", self.base_url);
        let body = build_messages_body(req, &self.model);
        let resp = self
            .http
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("anthropic-beta", "computer-use-2024-10-22")
            .json(&body)
            .send()
            .await
            .map_err(|e| CuError::Http {
                stage: "anthropic/messages",
                source: e,
            })?;
        let status = resp.status();
        if !status.is_success() {
            return Err(CuError::HttpStatus {
                stage: "anthropic/messages",
                status: status.as_u16(),
                body: read_body_truncated(resp).await,
            });
        }
        let raw: serde_json::Value = resp.json().await.map_err(|e| CuError::Http {
            stage: "anthropic/messages.parse",
            source: e,
        })?;
        parse_response(raw)
    }
}

/// One loop input. The system prompt fully describes the task;
/// `prior_actions` is the audit-log-friendly trace appended to
/// the assistant message so the model has continuity.
#[derive(Debug, Clone)]
pub struct CuStepRequest<'a> {
    /// Full system prompt — task description, URL whitelist,
    /// any photo paths mounted in the sandbox.
    pub system_prompt: &'a str,
    /// Latest screenshot bytes (PNG). The driver base64-encodes
    /// it inline.
    pub screenshot_png: &'a [u8],
    /// Trace of CU actions already taken this dispatch. The
    /// model uses these for continuity (so it doesn't repeat).
    pub prior_actions: &'a [String],
    /// Sandbox screen dimensions — wired into the tool spec so
    /// the model emits coordinates in the right space.
    pub screen_width: u32,
    pub screen_height: u32,
}

/// Possible model outputs after one step.
#[derive(Debug, Clone, PartialEq)]
pub enum CuStepResponse {
    /// Model emitted a `tool_use` for one CU primitive.
    ToolCall {
        action: CuAction,
        /// Total tokens (input+output) consumed by this step.
        /// The budget tracker charges this.
        tokens_used: u32,
    },
    /// Model emitted a `stop_reason` signal. The loop ends and
    /// the executor maps `success` → `Completed`,
    /// `failure` → `HumanHandoff`.
    Stop {
        reason: StopReason,
        /// Optional final text the model wants to surface.
        text: Option<String>,
        tokens_used: u32,
    },
}

/// The two stop signals the CU loop respects. See the brief's
/// "Notes for the implementer" section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    Success,
    Failure,
}

fn build_messages_body(req: CuStepRequest<'_>, model: &str) -> serde_json::Value {
    let screenshot_b64 = encode_b64(req.screenshot_png);
    let prior_actions_text = if req.prior_actions.is_empty() {
        "(none yet)".to_string()
    } else {
        req.prior_actions
            .iter()
            .enumerate()
            .map(|(i, a)| format!("{}. {a}", i + 1))
            .collect::<Vec<_>>()
            .join("\n")
    };
    serde_json::json!({
        "model": model,
        "max_tokens": 4096,
        "system": req.system_prompt,
        "tools": [{
            "type": "computer_20241022",
            "name": "computer",
            "display_width_px": req.screen_width,
            "display_height_px": req.screen_height,
            "display_number": 1
        }],
        "messages": [{
            "role": "user",
            "content": [
                {
                    "type": "text",
                    "text": format!(
                        "Prior actions:\n{prior_actions_text}\n\nLatest screenshot below. \
                         Emit the next CU primitive, or call `stop_success` / `stop_failure`."
                    )
                },
                {
                    "type": "image",
                    "source": {
                        "type": "base64",
                        "media_type": "image/png",
                        "data": screenshot_b64,
                    }
                }
            ]
        }]
    })
}

fn parse_response(raw: serde_json::Value) -> Result<CuStepResponse, CuError> {
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
    // Walk the content blocks; the first tool_use wins.
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
            return parse_tool_use(name, &input, tokens_used);
        }
        // The model is also allowed to emit `text` with a
        // stop signal in plain prose — we recognise specific
        // sentinels.
        if kind == Some("text") {
            let text = block
                .get("text")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            if let Some(stop) = recognise_text_stop(text) {
                return Ok(CuStepResponse::Stop {
                    reason: stop,
                    text: Some(text.to_string()),
                    tokens_used,
                });
            }
        }
    }
    Err(CuError::Protocol(
        "anthropic response carried no tool_use or recognised stop signal".into(),
    ))
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

/// Minimal base64 encoder. We don't pull in the `base64` crate
/// just for one call — the CU loop body is the only consumer.
fn encode_b64(bytes: &[u8]) -> String {
    const CHARS: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    let mut i = 0;
    while i + 3 <= bytes.len() {
        let b0 = bytes[i];
        let b1 = bytes[i + 1];
        let b2 = bytes[i + 2];
        out.push(CHARS[(b0 >> 2) as usize] as char);
        out.push(CHARS[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
        out.push(CHARS[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize] as char);
        out.push(CHARS[(b2 & 0x3f) as usize] as char);
        i += 3;
    }
    if i < bytes.len() {
        let rem = bytes.len() - i;
        let b0 = bytes[i];
        let b1 = if rem > 1 { bytes[i + 1] } else { 0 };
        out.push(CHARS[(b0 >> 2) as usize] as char);
        out.push(CHARS[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
        if rem == 2 {
            out.push(CHARS[((b1 & 0x0f) << 2) as usize] as char);
            out.push('=');
        } else {
            out.push('=');
            out.push('=');
        }
    }
    out
}

async fn read_body_truncated(resp: reqwest::Response) -> String {
    match resp.text().await {
        Ok(t) => {
            let mut s = t;
            if s.len() > 512 {
                s.truncate(512);
                s.push_str("…(truncated)");
            }
            s
        }
        Err(_) => "(body read failed)".into(),
    }
}

#[cfg(test)]
#[path = "anthropic_tests.rs"]
mod tests;
