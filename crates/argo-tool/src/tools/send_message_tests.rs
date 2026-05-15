//! Unit tests for the `send_message` tool. Split out per the
//! `clarify_tests.rs` / `vision_tests.rs` convention.

use super::*;
use argo_types::{Capability, ContentBlock, Principal};
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

#[test]
fn required_capabilities_scope_to_channel_and_recipient() {
    let caps = SendMessage::default()
        .required_capabilities(&serde_json::json!({
            "channel": "email",
            "recipient": "ops@example.com",
            "body": "hi",
        }))
        .unwrap();
    assert!(caps.covers(&Capability::MessageSend {
        channel: "email".into(),
        recipient_scope: "ops@example.com".into(),
    }));
    assert!(!caps.covers(&Capability::MessageSend {
        channel: "sms".into(),
        recipient_scope: "ops@example.com".into(),
    }));
}

#[test]
fn required_capabilities_reject_invalid_args_cleanly() {
    let err = SendMessage::default()
        .required_capabilities(&serde_json::json!({
            "channel": "slack",
            "body": "missing recipient",
        }))
        .unwrap_err();
    assert!(err.to_string().contains("invalid arguments"));
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
