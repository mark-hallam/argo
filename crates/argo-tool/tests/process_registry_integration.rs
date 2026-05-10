//! Integration test: spawn → list → tail → kill round-trip
//! exercising the `terminal { background: true }` and `process`
//! tools through their public dispatch surface.
//!
//! Unix-only — the test commands use Unix shell built-ins.
#![cfg(unix)]

use argo_tool::{ProcessRegistry, Terminal, ToolContext};
use argo_types::{ContentBlock, Principal};
use std::sync::Arc;
use std::time::Duration;

fn ctx_with_registry(reg: Arc<ProcessRegistry>, dir: &std::path::Path) -> ToolContext {
    ToolContext::new(
        Principal::Local {
            os_user: "u".into(),
        },
        dir.to_path_buf(),
    )
    .with_background_processes(reg)
}

fn text(r: &argo_types::ToolCallResult) -> String {
    match &r.content[0] {
        ContentBlock::Text { text } => text.clone(),
        _ => panic!("expected text"),
    }
}

#[tokio::test]
async fn terminal_background_then_process_list_tail_kill_round_trip() {
    use argo_tool::{Process, Tool};

    let dir = tempfile::tempdir().unwrap();
    let registry = Arc::new(ProcessRegistry::new());
    let ctx = ctx_with_registry(registry.clone(), dir.path());

    // 1. Spawn via `terminal { background: true }`.
    let r = Terminal::default()
        .dispatch(
            serde_json::json!({
                "command": "for i in 1 2 3 4 5; do echo line-$i; sleep 0.1; done; sleep 30",
                "background": true,
            }),
            &ctx,
        )
        .await
        .unwrap();
    assert!(!r.is_error, "terminal background dispatch failed: {r:?}");
    let body = text(&r);
    let pid = body
        .strip_prefix("process_id: ")
        .expect("expected process_id prefix")
        .trim()
        .to_string();

    // 2. `process { action: "list" }` should see one entry.
    let r = Process
        .dispatch(serde_json::json!({"action": "list"}), &ctx)
        .await
        .unwrap();
    assert!(!r.is_error);
    assert!(text(&r).contains(&pid), "list missing pid: {}", text(&r));

    // Give the inner command time to print + tail to drain.
    tokio::time::sleep(Duration::from_millis(800)).await;

    // 3. `process { action: "tail" }` returns the early stdout.
    let r = Process
        .dispatch(
            serde_json::json!({"action": "tail", "pid": pid.clone()}),
            &ctx,
        )
        .await
        .unwrap();
    assert!(!r.is_error);
    let body = text(&r);
    assert!(body.contains("line-1"), "tail output: {body:?}");
    assert!(body.contains("line-5"), "tail output: {body:?}");

    // 4. `process { action: "kill" }` SIGTERMs and reaps.
    let r = Process
        .dispatch(
            serde_json::json!({"action": "kill", "pid": pid.clone()}),
            &ctx,
        )
        .await
        .unwrap();
    assert!(!r.is_error);

    // 5. Registry is empty; subsequent `list` shows no entries.
    let r = Process
        .dispatch(serde_json::json!({"action": "list"}), &ctx)
        .await
        .unwrap();
    assert!(!r.is_error);
    assert!(!text(&r).contains(&pid));
}
