//! End-to-end coverage for `send_message`: drives the registry's
//! dispatch path (capability check + tool invocation) with a stub
//! channel dispatcher so the test can assert the message reaches the
//! channel layer with the correct shape and risk tier without
//! actually contacting Slack/Postmark/Blue Bubbles/SMS providers.

use argo_capability::CapabilityEngine;
use argo_tool::{MessageDispatcher, SendMessage, SendOutcome, ToolContext, ToolRegistry};
use argo_types::{Capability, ContentBlock, Principal};
use async_trait::async_trait;
use chrono::Utc;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

type Call = (String, String, String, Vec<PathBuf>);

#[derive(Debug, Default)]
struct StubDispatcher {
    calls: Mutex<Vec<Call>>,
}

#[async_trait]
impl MessageDispatcher for StubDispatcher {
    async fn dispatch(
        &self,
        channel: &str,
        recipient: &str,
        body: &str,
        attachments: &[PathBuf],
    ) -> Result<SendOutcome, String> {
        self.calls.lock().unwrap().push((
            channel.into(),
            recipient.into(),
            body.into(),
            attachments.to_vec(),
        ));
        Ok(SendOutcome {
            message_id: format!("{channel}-stub-1"),
            sent_at: Utc::now(),
        })
    }
}

fn principal() -> Principal {
    Principal::Local {
        os_user: "send-message-it".into(),
    }
}

fn registry_with(stub: Arc<StubDispatcher>) -> ToolRegistry {
    let mut reg = ToolRegistry::new();
    reg.register(Arc::new(SendMessage::with_dispatcher(stub)));
    reg
}

fn engine_for_principal(grants: &[Capability]) -> CapabilityEngine {
    let engine = CapabilityEngine::with_defaults();
    engine.bind(principal(), "guest".into()).unwrap();
    for grant in grants {
        engine.grant_dynamic(principal(), grant.clone());
    }
    engine
}

fn ctx(workspace: &std::path::Path) -> ToolContext {
    ToolContext::new(principal(), workspace.to_path_buf())
}

fn body(text: &ContentBlock) -> &str {
    match text {
        ContentBlock::Text { text } => text.as_str(),
        _ => panic!("expected text content"),
    }
}

#[tokio::test]
async fn dispatch_blocks_principal_without_send_message_tooldispatch() {
    let stub = Arc::new(StubDispatcher::default());
    let reg = registry_with(stub.clone());
    // Grant ToolDispatch for a *different* tool name — engine must
    // refuse before the dispatcher ever sees the call.
    let engine = engine_for_principal(&[Capability::ToolDispatch {
        tool_name_glob: "file_read".into(),
    }]);
    let ws = tempfile::tempdir().unwrap();
    let err = reg
        .dispatch(
            "send_message",
            serde_json::json!({
                "channel": "slack",
                "recipient": "C123",
                "body": "hi",
            }),
            &ctx(ws.path()),
            &engine,
        )
        .await
        .unwrap_err();
    assert!(matches!(err, argo_types::ArgoError::RbacDenied { .. }));
    assert!(stub.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn end_to_end_slack_send_carries_low_risk_tier() {
    let stub = Arc::new(StubDispatcher::default());
    let reg = registry_with(stub.clone());
    let engine = engine_for_principal(&[Capability::ToolDispatch {
        tool_name_glob: "send_message".into(),
    }]);
    let ws = tempfile::tempdir().unwrap();
    let result = reg
        .dispatch(
            "send_message",
            serde_json::json!({
                "channel": "slack",
                "recipient": "C123",
                "body": "your order shipped",
            }),
            &ctx(ws.path()),
            &engine,
        )
        .await
        .unwrap();
    assert!(!result.is_error, "{}", body(&result.content[0]));
    let text = body(&result.content[0]);
    assert!(text.contains("\"channel\": \"slack\""), "{text}");
    assert!(text.contains("\"risk_tier\": \"low\""), "{text}");
    assert!(text.contains("slack-stub-1"), "{text}");

    let calls = stub.calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "slack");
    assert_eq!(calls[0].1, "C123");
    assert_eq!(calls[0].2, "your order shipped");
    assert!(calls[0].3.is_empty());
}

#[tokio::test]
async fn end_to_end_imessage_marks_high_risk_and_resolves_attachment() {
    let stub = Arc::new(StubDispatcher::default());
    let reg = registry_with(stub.clone());
    let engine = engine_for_principal(&[Capability::ToolDispatch {
        tool_name_glob: "send_message".into(),
    }]);
    let ws = tempfile::tempdir().unwrap();
    let attach_rel = "receipt.pdf";
    std::fs::write(ws.path().join(attach_rel), b"%PDF-stub").unwrap();
    let result = reg
        .dispatch(
            "send_message",
            serde_json::json!({
                "channel": "imessage",
                "recipient": "iMessage;-;chat-guid-1",
                "body": "your receipt is attached",
                "attachments": [attach_rel],
            }),
            &ctx(ws.path()),
            &engine,
        )
        .await
        .unwrap();
    assert!(!result.is_error, "{}", body(&result.content[0]));
    let text = body(&result.content[0]);
    assert!(text.contains("\"risk_tier\": \"high\""), "{text}");

    let calls = stub.calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    let (channel, recipient, _body, attachments) = &calls[0];
    assert_eq!(channel, "imessage");
    assert_eq!(recipient, "iMessage;-;chat-guid-1");
    assert_eq!(attachments.len(), 1);
    assert_eq!(attachments[0], ws.path().join(attach_rel));
}

#[tokio::test]
async fn end_to_end_unknown_channel_does_not_call_dispatcher() {
    let stub = Arc::new(StubDispatcher::default());
    let reg = registry_with(stub.clone());
    let engine = engine_for_principal(&[Capability::ToolDispatch {
        tool_name_glob: "send_message".into(),
    }]);
    let ws = tempfile::tempdir().unwrap();
    let result = reg
        .dispatch(
            "send_message",
            serde_json::json!({
                "channel": "carrier-pigeon",
                "recipient": "x",
                "body": "y",
            }),
            &ctx(ws.path()),
            &engine,
        )
        .await
        .unwrap();
    assert!(result.is_error);
    assert!(body(&result.content[0]).contains("unknown channel"));
    assert!(stub.calls.lock().unwrap().is_empty());
}
