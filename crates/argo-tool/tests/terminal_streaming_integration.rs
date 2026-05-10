//! End-to-end integration coverage for the `terminal` streaming-output
//! path: subscribes a `mpsc::Receiver<ToolProgressEvent>`, drives the
//! tool through the registry's dispatch path under the appropriate
//! capability grant, and asserts the per-line events arrive in order
//! and the trailing `Exited` event lands before the dispatch future
//! resolves.
//!
//! Gated `#[cfg(unix)]` because the local backend shells through
//! `bash`, same as the rest of the `terminal` test suite.
#![cfg(unix)]

use argo_capability::CapabilityEngine;
use argo_tool::{Terminal, ToolContext, ToolProgressEvent, ToolRegistry};
use argo_types::{Capability, ContentBlock, Principal, SpawnKind};
use std::sync::Arc;
use tokio::sync::mpsc;

fn principal() -> Principal {
    Principal::Local {
        os_user: "terminal-stream-it".into(),
    }
}

fn registry_with_terminal() -> ToolRegistry {
    let mut reg = ToolRegistry::new();
    reg.register(Arc::new(Terminal::default()));
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

fn body(text: &ContentBlock) -> &str {
    match text {
        ContentBlock::Text { text } => text.as_str(),
        _ => panic!("expected text content"),
    }
}

#[tokio::test]
async fn streaming_round_trip_emits_lines_then_exit() {
    let reg = registry_with_terminal();
    let engine = engine_for_principal(&[
        Capability::ToolDispatch {
            tool_name_glob: "terminal".into(),
        },
        Capability::Spawn {
            spawn_kind: SpawnKind::Subprocess,
        },
    ]);
    let ws = tempfile::tempdir().unwrap();
    let (tx, mut rx) = mpsc::channel::<ToolProgressEvent>(64);
    let r = {
        // Scoped so `ctx` (and its tx clone) drops before we drain
        // `rx`; otherwise the receiver would never see `None`.
        let ctx = ToolContext::new(principal(), ws.path().to_path_buf()).with_progress_tx(tx);
        reg.dispatch(
            "terminal",
            serde_json::json!({"command": "printf 'one\\ntwo\\nthree\\n'"}),
            &ctx,
            &engine,
        )
        .await
        .unwrap()
    };
    assert!(!r.is_error, "{}", body(&r.content[0]));

    let mut events = Vec::new();
    while let Some(ev) = rx.recv().await {
        events.push(ev);
    }
    let stdouts: Vec<&str> = events
        .iter()
        .filter_map(|e| match e {
            ToolProgressEvent::Stdout(s) => Some(s.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(stdouts, vec!["one", "two", "three"]);
    match events.last() {
        Some(ToolProgressEvent::Exited { exit_code: 0 }) => {}
        other => panic!("expected trailing Exited(0), got {other:?}"),
    }

    let text = body(&r.content[0]);
    assert!(text.contains("one"), "{text}");
    assert!(text.contains("three"), "{text}");
}

#[tokio::test]
async fn streaming_separates_stdout_and_stderr() {
    let reg = registry_with_terminal();
    let engine = engine_for_principal(&[
        Capability::ToolDispatch {
            tool_name_glob: "terminal".into(),
        },
        Capability::Spawn {
            spawn_kind: SpawnKind::Subprocess,
        },
    ]);
    let ws = tempfile::tempdir().unwrap();
    let (tx, mut rx) = mpsc::channel::<ToolProgressEvent>(16);
    {
        let ctx = ToolContext::new(principal(), ws.path().to_path_buf()).with_progress_tx(tx);
        let _ = reg
            .dispatch(
                "terminal",
                serde_json::json!({"command": "echo out; echo err 1>&2"}),
                &ctx,
                &engine,
            )
            .await
            .unwrap();
    }

    let mut saw_out = false;
    let mut saw_err = false;
    while let Some(ev) = rx.recv().await {
        match ev {
            ToolProgressEvent::Stdout(s) if s == "out" => saw_out = true,
            ToolProgressEvent::Stderr(s) if s == "err" => saw_err = true,
            _ => {}
        }
    }
    assert!(saw_out, "expected stdout event");
    assert!(saw_err, "expected stderr event");
}

#[tokio::test]
async fn no_callback_path_is_unchanged_through_registry() {
    // Belt-and-braces: re-runs the existing buffered behavior via
    // the public registry surface to confirm the `progress_tx = None`
    // path is byte-for-byte unchanged.
    let reg = registry_with_terminal();
    let engine = engine_for_principal(&[
        Capability::ToolDispatch {
            tool_name_glob: "terminal".into(),
        },
        Capability::Spawn {
            spawn_kind: SpawnKind::Subprocess,
        },
    ]);
    let ws = tempfile::tempdir().unwrap();
    let ctx = ToolContext::new(principal(), ws.path().to_path_buf());
    assert!(ctx.progress_tx.is_none());

    let r = reg
        .dispatch(
            "terminal",
            serde_json::json!({"command": "echo hi"}),
            &ctx,
            &engine,
        )
        .await
        .unwrap();
    let text = body(&r.content[0]);
    assert!(text.contains("--- stdout ---\nhi"), "{text}");
}
