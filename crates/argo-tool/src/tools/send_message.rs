//! `send_message` — agent-callable outbound messaging over `argo-channels`.
//!
//! Hermes parity: agents must be able to talk to the user / customer
//! mid-task ("I shipped your order, here's the tracking link", "I need
//! to confirm the date before booking"). This is **distinct** from the
//! outbound-on-Ask path, where the operator approves a pending action;
//! `send_message` is the agent itself communicating, capability-gated
//! and risk-tagged per channel.
//!
//! ## Channels and risk tiers
//!
//! The `channel` argument selects which `argo-channels` transport to
//! use. The four supported values match the modules present in
//! `crates/argo-channels/src/`:
//!
//! | channel    | risk tier | rationale                                                   |
//! |------------|-----------|-------------------------------------------------------------|
//! | `imessage` | `high`    | real human, hard to recall, blue-bubbled to the recipient   |
//! | `sms`      | `high`    | real human, hard to recall, billed per send                 |
//! | `email`    | `medium`  | recall window is short and visible, generally reversible    |
//! | `slack`    | `low`     | most teams treat as ephemeral, easy to delete or react      |
//!
//! Per-channel risk is read off the tool's response body and is also
//! exposed by the static [`risk_tier`] helper. The runtime's existing
//! approval policy can use the tier to gate auto-send vs. ask-first
//! without any policy change in this brief.
//!
//! ## Capability gating
//!
//! `Capability::ToolDispatch { tool_name_glob: "send_message" }` is
//! enforced by [`crate::ToolRegistry::dispatch`] like every tool;
//! that's the only fixed gate on this surface. Channel-level
//! authentication (Slack bot token, SMTP creds, Blue Bubbles password,
//! SMS provider keys) is operator-managed via `argo-config` and is not
//! re-validated here.
//!
//! ## Dispatch backend
//!
//! Channel-level "send arbitrary outbound" entry points are not yet
//! exposed by `argo-channels` (its existing `NotifyChannel::send`
//! takes a structured `Notification` for approval flows), so this tool
//! ships with a [`MessageDispatcher`] trait the runtime wires at boot.
//! The default backend returns a clear "not wired" error so the model
//! still sees the tool in its schema and a misconfigured operator
//! gets a useful diagnostic. Tests inject a stub dispatcher; the
//! capture pattern matches `code_execute::with_sandbox`.
//!
//! ```
//! use argo_tool::{SendMessage, Tool};
//! let t = SendMessage::default();
//! assert_eq!(t.name(), "send_message");
//! ```
use crate::path_guard::confine_path;
use crate::tool::{error_text, success_text, Concurrency, Tool, ToolContext};
use argo_types::{ArgoResult, CapabilitySet, ToolCallResult};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

/// Successful dispatch outcome returned by a [`MessageDispatcher`].
#[derive(Debug, Clone)]
pub struct SendOutcome {
    /// Channel-assigned message id (e.g. Slack `ts`, Postmark
    /// `MessageID`, Blue Bubbles `guid`).
    pub message_id: String,
    /// Wall-clock send time the channel returned.
    pub sent_at: DateTime<Utc>,
}

/// Backend trait the `send_message` tool calls into. The runtime
/// constructs the production dispatcher at boot (Slack/email/SMS/
/// iMessage transports from `argo-channels`); tests inject a capture
/// dispatcher.
#[async_trait]
pub trait MessageDispatcher: Send + Sync + std::fmt::Debug {
    /// Deliver `body` (with optional `attachments`) over `channel` to
    /// `recipient`. Implementations resolve channel-specific
    /// addressing and authentication; on success they return the
    /// channel's message id.
    async fn dispatch(
        &self,
        channel: &str,
        recipient: &str,
        body: &str,
        attachments: &[std::path::PathBuf],
    ) -> Result<SendOutcome, String>;
}

#[derive(Debug, Default)]
struct UnwiredDispatcher;

#[async_trait]
impl MessageDispatcher for UnwiredDispatcher {
    async fn dispatch(
        &self,
        _channel: &str,
        _recipient: &str,
        _body: &str,
        _attachments: &[std::path::PathBuf],
    ) -> Result<SendOutcome, String> {
        Err("send_message backend not wired — runtime must construct \
             SendMessage::with_dispatcher(...) at boot"
            .into())
    }
}

/// Outbound messaging tool over `argo-channels`.
#[derive(Debug, Clone)]
pub struct SendMessage {
    dispatcher: Arc<dyn MessageDispatcher>,
}

impl Default for SendMessage {
    fn default() -> Self {
        Self {
            dispatcher: Arc::new(UnwiredDispatcher),
        }
    }
}

impl SendMessage {
    /// Build the tool with a custom dispatcher backend. The runtime
    /// uses this at boot to inject the production `argo-channels`
    /// router; tests inject a capture dispatcher.
    #[must_use]
    pub fn with_dispatcher(dispatcher: Arc<dyn MessageDispatcher>) -> Self {
        Self { dispatcher }
    }
}

/// Per-channel risk tier (see module-level table).
///
/// Returns `"unknown"` for any channel name not in the supported set
/// so the runtime never crashes on an unexpected value; the tool's
/// own dispatch path rejects unknown channels before this is reached.
#[must_use]
pub fn risk_tier(channel: &str) -> &'static str {
    match channel {
        "imessage" | "sms" => "high",
        "email" => "medium",
        "slack" => "low",
        _ => "unknown",
    }
}

const SUPPORTED: &[&str] = &["slack", "email", "sms", "imessage"];

#[derive(Debug, Deserialize)]
struct Args {
    channel: String,
    recipient: String,
    body: String,
    #[serde(default)]
    attachments: Option<Vec<String>>,
}

#[async_trait]
impl Tool for SendMessage {
    fn name(&self) -> &str {
        "send_message"
    }

    fn description(&self) -> &str {
        "Send a message to a user or customer over an outbound channel \
         (`slack`, `email`, `sms`, `imessage`). `recipient` is a \
         channel-specific address; `body` is the message text; \
         `attachments` are workspace-relative file paths. Returns the \
         channel's `message_id`, `sent_at`, and the per-channel `risk_tier`."
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "required": ["channel", "recipient", "body"],
            "properties": {
                "channel": {
                    "type": "string",
                    "enum": SUPPORTED,
                    "description": "Outbound transport. `imessage`/`sms` are high-risk, `email` medium, `slack` low.",
                },
                "recipient": {
                    "type": "string",
                    "description": "Channel-specific address: Slack channel/user id, email address, phone number, or iMessage chat GUID.",
                },
                "body": { "type": "string" },
                "attachments": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Optional workspace-relative file paths.",
                },
            },
            "additionalProperties": false,
        })
    }

    fn concurrency(&self) -> Concurrency {
        // Channels are per-recipient stateful (Slack threading, SMS
        // ordering); serialize so two parallel sends to the same user
        // can't interleave.
        Concurrency::Serial
    }

    fn required_capabilities(&self, _args: &serde_json::Value) -> ArgoResult<CapabilitySet> {
        // Registry already enforces ToolDispatch{send_message}; no
        // extra fine-grained caps. Channel-level auth lives in
        // `argo-channels` config.
        Ok(CapabilitySet(vec![]))
    }

    async fn dispatch(
        &self,
        args: serde_json::Value,
        ctx: &ToolContext,
    ) -> ArgoResult<ToolCallResult> {
        let started = Instant::now();
        let parsed: Args = match serde_json::from_value(args) {
            Ok(a) => a,
            Err(e) => return Ok(err(ctx, started, format!("invalid arguments: {e}"))),
        };
        if !SUPPORTED.contains(&parsed.channel.as_str()) {
            return Ok(err(
                ctx,
                started,
                format!(
                    "unknown channel '{}': supported = {}",
                    parsed.channel,
                    SUPPORTED.join(" | ")
                ),
            ));
        }
        let resolved_attachments = match resolve_attachments(
            &ctx.workspace_root,
            parsed.attachments.as_deref().unwrap_or(&[]),
        ) {
            Ok(p) => p,
            Err(msg) => return Ok(err(ctx, started, msg)),
        };
        let outcome = match self
            .dispatcher
            .dispatch(
                &parsed.channel,
                &parsed.recipient,
                &parsed.body,
                &resolved_attachments,
            )
            .await
        {
            Ok(o) => o,
            Err(e) => return Ok(err(ctx, started, format!("send failed: {e}"))),
        };
        let body = serde_json::json!({
            "channel": parsed.channel,
            "recipient": parsed.recipient,
            "message_id": outcome.message_id,
            "sent_at": outcome.sent_at.to_rfc3339(),
            "risk_tier": risk_tier(&parsed.channel),
        });
        Ok(success_text(
            ctx.call_id,
            serde_json::to_string_pretty(&body).unwrap_or_else(|_| "{}".into()),
            started.elapsed().as_millis() as u64,
        ))
    }
}

fn err(ctx: &ToolContext, started: Instant, msg: String) -> ToolCallResult {
    error_text(ctx.call_id, msg, started.elapsed().as_millis() as u64)
}

fn resolve_attachments(
    workspace_root: &Path,
    attachments: &[String],
) -> Result<Vec<std::path::PathBuf>, String> {
    let mut out = Vec::with_capacity(attachments.len());
    for raw in attachments {
        let resolved = confine_path(workspace_root, Path::new(raw))?;
        out.push(resolved);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_types::{ContentBlock, Principal};
    use std::path::PathBuf;
    use std::sync::Mutex;

    type Call = (String, String, String, Vec<PathBuf>);

    #[derive(Debug, Default)]
    struct CaptureDispatcher {
        calls: Mutex<Vec<Call>>,
        fail_with: Option<String>,
    }

    #[async_trait]
    impl MessageDispatcher for CaptureDispatcher {
        async fn dispatch(
            &self,
            channel: &str,
            recipient: &str,
            body: &str,
            attachments: &[PathBuf],
        ) -> Result<SendOutcome, String> {
            if let Some(msg) = &self.fail_with {
                return Err(msg.clone());
            }
            self.calls.lock().unwrap().push((
                channel.into(),
                recipient.into(),
                body.into(),
                attachments.to_vec(),
            ));
            Ok(SendOutcome {
                message_id: format!("{channel}-msg-1"),
                sent_at: Utc::now(),
            })
        }
    }

    fn ctx() -> ToolContext {
        ToolContext::new(
            Principal::Local {
                os_user: "u".into(),
            },
            std::env::temp_dir(),
        )
    }

    fn body(r: &ToolCallResult) -> String {
        match &r.content[0] {
            ContentBlock::Text { text } => text.clone(),
            _ => panic!("expected text"),
        }
    }

    #[test]
    fn risk_tier_table_matches_brief() {
        assert_eq!(risk_tier("imessage"), "high");
        assert_eq!(risk_tier("sms"), "high");
        assert_eq!(risk_tier("email"), "medium");
        assert_eq!(risk_tier("slack"), "low");
        assert_eq!(risk_tier("carrier-pigeon"), "unknown");
    }

    #[test]
    fn schema_lists_four_channels() {
        let s = SendMessage::default().parameters();
        let chans = &s["properties"]["channel"]["enum"];
        assert!(chans.as_array().unwrap().iter().any(|v| v == "slack"));
        assert!(chans.as_array().unwrap().iter().any(|v| v == "email"));
        assert!(chans.as_array().unwrap().iter().any(|v| v == "sms"));
        assert!(chans.as_array().unwrap().iter().any(|v| v == "imessage"));
    }

    #[tokio::test]
    async fn slack_happy_path_returns_message_id_and_risk_tier() {
        let cap = Arc::new(CaptureDispatcher::default());
        let t = SendMessage::with_dispatcher(cap.clone());
        let r = t
            .dispatch(
                serde_json::json!({
                    "channel": "slack",
                    "recipient": "C123",
                    "body": "shipped",
                }),
                &ctx(),
            )
            .await
            .unwrap();
        assert!(!r.is_error, "{}", body(&r));
        let b = body(&r);
        assert!(b.contains("\"message_id\""), "{b}");
        assert!(b.contains("slack-msg-1"), "{b}");
        assert!(b.contains("\"risk_tier\": \"low\""), "{b}");
        let calls = cap.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "slack");
        assert_eq!(calls[0].1, "C123");
        assert_eq!(calls[0].2, "shipped");
    }

    #[tokio::test]
    async fn email_happy_path_marks_medium_risk() {
        let cap = Arc::new(CaptureDispatcher::default());
        let t = SendMessage::with_dispatcher(cap);
        let r = t
            .dispatch(
                serde_json::json!({
                    "channel": "email",
                    "recipient": "user@example.com",
                    "body": "hi",
                }),
                &ctx(),
            )
            .await
            .unwrap();
        assert!(!r.is_error, "{}", body(&r));
        assert!(body(&r).contains("\"risk_tier\": \"medium\""));
    }

    #[tokio::test]
    async fn unknown_channel_returns_clean_error() {
        let r = SendMessage::default()
            .dispatch(
                serde_json::json!({
                    "channel": "fax",
                    "recipient": "x",
                    "body": "y",
                }),
                &ctx(),
            )
            .await
            .unwrap();
        assert!(r.is_error);
        assert!(body(&r).contains("unknown channel"), "{}", body(&r));
    }

    #[tokio::test]
    async fn parent_traversal_in_attachments_is_rejected() {
        let cap = Arc::new(CaptureDispatcher::default());
        let t = SendMessage::with_dispatcher(cap);
        let r = t
            .dispatch(
                serde_json::json!({
                    "channel": "slack",
                    "recipient": "C123",
                    "body": "see attached",
                    "attachments": ["../escape.txt"],
                }),
                &ctx(),
            )
            .await
            .unwrap();
        assert!(r.is_error);
        assert!(body(&r).contains("`..`"), "{}", body(&r));
    }

    #[tokio::test]
    async fn dispatcher_error_surfaces_as_tool_error() {
        let dispatcher = Arc::new(CaptureDispatcher {
            calls: Mutex::new(Vec::new()),
            fail_with: Some("transport down".into()),
        });
        let t = SendMessage::with_dispatcher(dispatcher);
        let r = t
            .dispatch(
                serde_json::json!({
                    "channel": "sms",
                    "recipient": "+15551234567",
                    "body": "hi",
                }),
                &ctx(),
            )
            .await
            .unwrap();
        assert!(r.is_error);
        let b = body(&r);
        assert!(b.contains("send failed"), "{b}");
        assert!(b.contains("transport down"), "{b}");
    }

    #[tokio::test]
    async fn unwired_default_dispatcher_returns_diagnostic() {
        let r = SendMessage::default()
            .dispatch(
                serde_json::json!({
                    "channel": "slack",
                    "recipient": "C123",
                    "body": "hi",
                }),
                &ctx(),
            )
            .await
            .unwrap();
        assert!(r.is_error);
        assert!(body(&r).contains("not wired"), "{}", body(&r));
    }
}
