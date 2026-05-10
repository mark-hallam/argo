//! End-to-end integration tests for the `todo` and `clarify` tools
//! (brief `tool-todo-and-clarify`).
//!
//! - **`todo`**: drives a full add → list → complete sequence across
//!   two distinct `ToolContext` invocations sharing the same workspace
//!   root, proving the on-disk JSON state persists between calls.
//! - **`clarify`**: drives the dispatcher seam end-to-end with a
//!   stub [`ClarifyDispatcher`] (the agent runner will wire a live
//!   surface in a follow-up — see the module docs in
//!   `crates/argo-tool/src/tools/clarify.rs`).

use argo_tool::{Clarify, ClarifyAnswer, ClarifyDispatcher, Todo, TodoItem, Tool, ToolContext};
use argo_types::{ContentBlock, Principal, ToolCallResult};
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

fn ctx(root: PathBuf) -> ToolContext {
    ToolContext::new(
        Principal::Local {
            os_user: "u".into(),
        },
        root,
    )
}

fn first_text(r: &ToolCallResult) -> String {
    r.content
        .iter()
        .find_map(|b| match b {
            ContentBlock::Text { text } => Some(text.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

#[tokio::test]
async fn todo_state_persists_across_separate_tool_contexts() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();

    // Context A: add two items.
    let r = Todo
        .dispatch(
            serde_json::json!({"action": "add", "item": "draft PRD"}),
            &ctx(root.clone()),
        )
        .await
        .unwrap();
    assert!(!r.is_error, "{}", first_text(&r));
    let r = Todo
        .dispatch(
            serde_json::json!({"action": "add", "item": "schedule review"}),
            &ctx(root.clone()),
        )
        .await
        .unwrap();
    assert!(!r.is_error);

    // The state file must exist on disk.
    let state_path = root.join(".argo").join("todo.json");
    assert!(state_path.exists(), "expected {} to exist", state_path.display());

    // Context B: list reflects both writes from A.
    let r = Todo
        .dispatch(
            serde_json::json!({"action": "list"}),
            &ctx(root.clone()),
        )
        .await
        .unwrap();
    let items: Vec<TodoItem> = serde_json::from_str(&first_text(&r)).unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].text, "draft PRD");
    assert_eq!(items[1].text, "schedule review");

    // Context C: complete the first, list shows partial done state.
    let r = Todo
        .dispatch(
            serde_json::json!({"action": "complete", "id": 1}),
            &ctx(root.clone()),
        )
        .await
        .unwrap();
    assert!(!r.is_error);
    let r = Todo
        .dispatch(
            serde_json::json!({"action": "list"}),
            &ctx(root.clone()),
        )
        .await
        .unwrap();
    let items: Vec<TodoItem> = serde_json::from_str(&first_text(&r)).unwrap();
    assert!(items[0].done, "id=1 should be done");
    assert!(!items[1].done, "id=2 should still be open");
}

#[derive(Debug)]
struct CountingDispatcher {
    calls: Mutex<Vec<(String, Option<Vec<String>>)>>,
}

#[async_trait]
impl ClarifyDispatcher for CountingDispatcher {
    fn surface_id(&self) -> &str {
        "test-surface"
    }
    async fn ask(
        &self,
        question: &str,
        options: Option<&[String]>,
    ) -> Result<ClarifyAnswer, String> {
        self.calls
            .lock()
            .unwrap()
            .push((question.to_string(), options.map(|o| o.to_vec())));
        Ok(ClarifyAnswer::free_form("operator-said-yes"))
    }
}

#[tokio::test]
async fn clarify_routes_through_wired_dispatcher_end_to_end() {
    let stub = Arc::new(CountingDispatcher {
        calls: Mutex::new(Vec::new()),
    });
    let tool = Clarify::new(stub.clone());
    let r = tool
        .dispatch(
            serde_json::json!({
                "question": "ship 1.0?",
                "options": ["yes", "no"],
            }),
            &ctx(PathBuf::from("/tmp")),
        )
        .await
        .unwrap();
    assert!(!r.is_error);
    let payload: serde_json::Value = serde_json::from_str(&first_text(&r)).unwrap();
    assert_eq!(payload["answer"], "operator-said-yes");
    assert_eq!(payload["surface"], "test-surface");

    let calls = stub.calls.lock().unwrap();
    assert_eq!(calls.len(), 1, "dispatcher must be invoked exactly once");
    let (q, opts) = &calls[0];
    assert_eq!(q, "ship 1.0?");
    assert_eq!(
        opts.as_deref(),
        Some(&["yes".to_string(), "no".to_string()][..])
    );
}

#[tokio::test]
async fn clarify_default_form_errors_with_actionable_hint() {
    let r = Clarify::default()
        .dispatch(
            serde_json::json!({"question": "anything?"}),
            &ctx(PathBuf::from("/tmp")),
        )
        .await
        .unwrap();
    assert!(r.is_error);
    let text = first_text(&r);
    assert!(
        text.contains("not wired") && text.contains("Clarify::new"),
        "expected actionable hint; got: {text}"
    );
}
