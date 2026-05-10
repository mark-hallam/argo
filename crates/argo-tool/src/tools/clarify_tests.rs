//! Unit tests for the `clarify` tool. Split out per the
//! `memory_tests.rs` / `vision_tests.rs` convention.

use super::*;
use argo_types::{ContentBlock, Principal};
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug, Default)]
struct StubDispatcher {
    last: Mutex<Option<(String, Option<Vec<String>>)>>,
    reply: ClarifyAnswer,
}

#[async_trait]
impl ClarifyDispatcher for StubDispatcher {
    fn surface_id(&self) -> &str {
        "stub"
    }
    async fn ask(
        &self,
        question: &str,
        options: Option<&[String]>,
    ) -> Result<ClarifyAnswer, String> {
        *self.last.lock().unwrap() = Some((question.to_string(), options.map(|o| o.to_vec())));
        Ok(self.reply.clone())
    }
}

fn ctx() -> ToolContext {
    ToolContext::new(
        Principal::Local {
            os_user: "u".into(),
        },
        PathBuf::from("/tmp"),
    )
}

fn body(r: &ToolCallResult) -> String {
    r.content
        .iter()
        .find_map(|b| match b {
            ContentBlock::Text { text } => Some(text.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

#[tokio::test]
async fn default_form_returns_not_wired_error_with_hint() {
    let r = Clarify::default()
        .dispatch(
            serde_json::json!({"question": "ship now?"}),
            &ctx(),
        )
        .await
        .unwrap();
    assert!(r.is_error);
    let text = body(&r);
    assert!(
        text.contains("not wired") && text.contains("Clarify::new"),
        "expected not-wired hint pointing at Clarify::new; got: {text}"
    );
}

#[tokio::test]
async fn missing_question_is_argument_error() {
    let err = Clarify::default()
        .dispatch(serde_json::json!({}), &ctx())
        .await
        .expect_err("must reject");
    assert!(err.to_string().contains("question"), "got: {err}");
}

#[tokio::test]
async fn options_round_trip_through_dispatcher() {
    let stub = Arc::new(StubDispatcher {
        reply: ClarifyAnswer {
            answer: "yes".into(),
            option_index: Some(0),
        },
        ..Default::default()
    });
    let tool = Clarify::new(stub.clone());
    let r = tool
        .dispatch(
            serde_json::json!({
                "question": "approve send?",
                "options": ["yes", "no"],
            }),
            &ctx(),
        )
        .await
        .unwrap();
    assert!(!r.is_error);
    let payload: serde_json::Value = serde_json::from_str(&body(&r)).unwrap();
    assert_eq!(payload["answer"], "yes");
    assert_eq!(payload["option_index"], 0);
    assert_eq!(payload["surface"], "stub");
    let last = stub.last.lock().unwrap();
    let (q, opts) = last.as_ref().unwrap();
    assert_eq!(q, "approve send?");
    assert_eq!(
        opts.as_deref(),
        Some(&["yes".to_string(), "no".to_string()][..])
    );
}

#[tokio::test]
async fn free_form_path_omits_options() {
    let stub = Arc::new(StubDispatcher {
        reply: ClarifyAnswer {
            answer: "let's wait until tomorrow".into(),
            option_index: None,
        },
        ..Default::default()
    });
    let tool = Clarify::new(stub.clone());
    let r = tool
        .dispatch(
            serde_json::json!({"question": "when should we ship?"}),
            &ctx(),
        )
        .await
        .unwrap();
    assert!(!r.is_error);
    let payload: serde_json::Value = serde_json::from_str(&body(&r)).unwrap();
    assert_eq!(payload["answer"], "let's wait until tomorrow");
    assert!(payload["option_index"].is_null());
    let last = stub.last.lock().unwrap();
    let (_, opts) = last.as_ref().unwrap();
    assert!(opts.is_none(), "expected no options forwarded");
}

#[test]
fn dispatch_only_requires_tool_dispatch_capability() {
    let caps = Clarify::default()
        .required_capabilities(&serde_json::json!({"question": "x"}))
        .unwrap();
    assert!(caps.covers(&Capability::ToolDispatch {
        tool_name_glob: "clarify".into(),
    }));
}
