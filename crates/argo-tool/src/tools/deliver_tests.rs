//! Tests for the `enqueue_action` in-turn delivery tool. Loaded via
//! `#[path = "deliver_tests.rs"] mod tests;` from `deliver.rs`.
//!
//! The load-bearing proofs:
//! * the tool calls the injected backend and reports it TRUTH-BOUNDED
//!   ("submitted for approval", `delivered: false`) — never "done";
//! * the default (unwired) backend fails closed with a clear error.

use super::*;
use crate::tool::ToolContext;
use std::sync::Mutex;

#[derive(Debug, Default)]
struct CaptureDeliverDispatcher {
    seen: Mutex<Vec<(String, serde_json::Value)>>,
}

#[async_trait]
impl DeliverDispatcher for CaptureDeliverDispatcher {
    async fn enqueue(
        &self,
        action_type: &str,
        payload: serde_json::Value,
    ) -> Result<DeliverOutcome, String> {
        self.seen
            .lock()
            .unwrap()
            .push((action_type.to_string(), payload));
        Ok(DeliverOutcome {
            action_id: "act-123".into(),
            executor_kind: "document_csv".into(),
        })
    }
}

fn ctx() -> ToolContext {
    ToolContext::new(
        argo_types::Principal::Local {
            os_user: "u".into(),
        },
        std::env::temp_dir(),
    )
}

#[tokio::test]
async fn wired_dispatcher_reports_submitted_for_approval_not_delivered() {
    let backend = Arc::new(CaptureDeliverDispatcher::default());
    let tool = EnqueueAction::with_dispatcher(backend.clone() as Arc<dyn DeliverDispatcher>);
    assert_eq!(tool.name(), "enqueue_action");

    let r = tool
        .dispatch(
            serde_json::json!({
                "action_type": "export_report",
                "payload": { "rows": 3 },
            }),
            &ctx(),
        )
        .await
        .unwrap();

    assert!(!r.is_error, "wired enqueue should succeed");
    // The backend was actually called with the model's args.
    let seen = backend.seen.lock().unwrap();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].0, "export_report");
    assert_eq!(seen[0].1, serde_json::json!({ "rows": 3 }));

    // Truth-bounded result: "submitted for approval", NOT delivered/done.
    let text = match &r.content[0] {
        argo_types::ContentBlock::Text { text } => text.clone(),
        other => panic!("expected text content, got {other:?}"),
    };
    let body: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(body["status"], "submitted for approval");
    assert_eq!(body["delivered"], serde_json::json!(false));
    assert_eq!(body["action_id"], "act-123");
    assert_eq!(body["executor"], "document_csv");
    assert!(
        !text.to_lowercase().contains("\"done\"") && !text.to_lowercase().contains("delivered\": true"),
        "result must never claim done/delivered: {text}"
    );
}

#[tokio::test]
async fn default_backend_is_unwired_and_fails_closed() {
    let tool = EnqueueAction::default();
    let r = tool
        .dispatch(
            serde_json::json!({ "action_type": "export_report" }),
            &ctx(),
        )
        .await
        .unwrap();
    assert!(r.is_error, "the default backend must report 'not wired'");
    let text = match &r.content[0] {
        argo_types::ContentBlock::Text { text } => text.clone(),
        other => panic!("expected text content, got {other:?}"),
    };
    assert!(
        text.contains("not wired"),
        "unwired error should be explicit: {text}"
    );
}

#[tokio::test]
async fn empty_action_type_is_rejected() {
    let backend = Arc::new(CaptureDeliverDispatcher::default());
    let tool = EnqueueAction::with_dispatcher(backend.clone() as Arc<dyn DeliverDispatcher>);
    let r = tool
        .dispatch(serde_json::json!({ "action_type": "   " }), &ctx())
        .await
        .unwrap();
    assert!(r.is_error, "empty action_type must be rejected");
    assert!(
        backend.seen.lock().unwrap().is_empty(),
        "backend must NOT be called for an empty action_type"
    );
}

#[test]
fn required_capabilities_declares_no_extra_fine_grained_cap() {
    // The registry enforces ToolDispatch{"enqueue_action"} on its own; the
    // tool declares no EXTRA capability because the row only parks at the
    // approval gate (the gated executor run happens later, on approval).
    let tool = EnqueueAction::default();
    let caps = tool
        .required_capabilities(&serde_json::json!({ "action_type": "x" }))
        .unwrap();
    assert!(caps.0.is_empty());
}
