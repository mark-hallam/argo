//! Anthropic computer-use loop driver. The variant string sent
//! as `tools[0].type` is configurable via
//! [`crate::CuConfig::tool_variant`]; the loop sends a chat
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

use crate::anthropic_parse::parse_response;
use crate::e2b::CuAction;
use crate::CuError;

/// Thin client over Anthropic's `/v1/messages`.
#[derive(Debug, Clone)]
pub struct AnthropicClient {
    base_url: String,
    api_key: String,
    model: String,
    tool_variant: String,
    http: reqwest::Client,
}

impl AnthropicClient {
    pub fn new(
        base_url: String,
        api_key: String,
        model: String,
        tool_variant: String,
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
            tool_variant,
            http,
        })
    }

    /// Send one CU loop step. Returns the parsed model response:
    /// either a tool call (next CU action) or a stop signal.
    pub async fn step(&self, req: CuStepRequest<'_>) -> Result<CuStepResponse, CuError> {
        let url = format!("{}/v1/messages", self.base_url);
        let body = build_messages_body(req, &self.model, &self.tool_variant);
        let resp = self
            .http
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("anthropic-beta", beta_header_for(&self.tool_variant))
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

fn build_messages_body(
    req: CuStepRequest<'_>,
    model: &str,
    tool_variant: &str,
) -> serde_json::Value {
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
            "type": tool_variant,
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

/// Map a computer-use tool variant to its required
/// `anthropic-beta` header value. Unknown variants fall through
/// to the current default header, which lets a config-only bump
/// to a same-family variant work without a code change.
pub(crate) fn beta_header_for(tool_variant: &str) -> &'static str {
    match tool_variant {
        "computer_20250124" => "computer-use-2025-01-24",
        // Default: current Anthropic-published header (paired
        // with `DEFAULT_TOOL_VARIANT` in `config.rs`).
        _ => "computer-use-2025-11-24",
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
