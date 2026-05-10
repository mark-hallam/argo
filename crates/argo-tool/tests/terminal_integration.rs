//! Integration coverage for `terminal`'s sandbox-backend wiring:
//! end-to-end through the registry's dispatch path (capability checks
//! plus tool invocation), confirming both the `local` (default,
//! requires `Spawn::Subprocess`) and `sandbox` (opt-in, requires
//! `Spawn::Container`) backends round-trip correctly.
//!
//! These tests run against the default [`LocalSandbox`] backend on
//! both routes — Docker / E2B / Firecracker are future swap targets
//! and not exercised here. They are gated `#[cfg(unix)]` to skip on
//! Windows (LocalSandbox shells through `bash`, same caveat as the
//! `terminal` tool's own tests).
#![cfg(unix)]

use argo_capability::CapabilityEngine;
use argo_tool::{Terminal, ToolContext, ToolRegistry};
use argo_types::{Capability, ContentBlock, Principal, SpawnKind};
use std::sync::Arc;

fn principal() -> Principal {
    Principal::Local {
        os_user: "terminal-it".into(),
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
async fn local_backend_round_trips_under_subprocess_capability() {
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
    let r = reg
        .dispatch(
            "terminal",
            // No `backend` arg → defaults to "local".
            serde_json::json!({"command": "echo terminal-local-ok"}),
            &ctx(ws.path()),
            &engine,
        )
        .await
        .unwrap();
    assert!(!r.is_error, "{}", body(&r.content[0]));
    let text = body(&r.content[0]);
    assert!(text.contains("exit_code: 0"), "{text}");
    assert!(text.contains("terminal-local-ok"), "{text}");
}

#[tokio::test]
async fn sandbox_backend_round_trips_under_container_capability() {
    let reg = registry_with_terminal();
    let engine = engine_for_principal(&[
        Capability::ToolDispatch {
            tool_name_glob: "terminal".into(),
        },
        Capability::Spawn {
            spawn_kind: SpawnKind::Container,
        },
    ]);
    let ws = tempfile::tempdir().unwrap();
    let r = reg
        .dispatch(
            "terminal",
            serde_json::json!({
                "command": "echo terminal-sandbox-ok",
                "backend": "sandbox",
            }),
            &ctx(ws.path()),
            &engine,
        )
        .await
        .unwrap();
    assert!(!r.is_error, "{}", body(&r.content[0]));
    let text = body(&r.content[0]);
    assert!(text.contains("exit_code: 0"), "{text}");
    assert!(text.contains("terminal-sandbox-ok"), "{text}");
}

#[tokio::test]
async fn sandbox_backend_blocked_without_container_capability() {
    let reg = registry_with_terminal();
    // Subprocess is granted but Container is not — the sandbox backend
    // must fail closed at the engine check, never reaching the sandbox.
    let engine = engine_for_principal(&[
        Capability::ToolDispatch {
            tool_name_glob: "terminal".into(),
        },
        Capability::Spawn {
            spawn_kind: SpawnKind::Subprocess,
        },
    ]);
    let ws = tempfile::tempdir().unwrap();
    let err = reg
        .dispatch(
            "terminal",
            serde_json::json!({
                "command": "echo should-not-run",
                "backend": "sandbox",
            }),
            &ctx(ws.path()),
            &engine,
        )
        .await
        .unwrap_err();
    assert!(matches!(err, argo_types::ArgoError::RbacDenied { .. }));
}

#[tokio::test]
async fn local_backend_blocked_without_subprocess_capability() {
    let reg = registry_with_terminal();
    // Container granted but Subprocess not — the default (local)
    // backend must still fail closed.
    let engine = engine_for_principal(&[
        Capability::ToolDispatch {
            tool_name_glob: "terminal".into(),
        },
        Capability::Spawn {
            spawn_kind: SpawnKind::Container,
        },
    ]);
    let ws = tempfile::tempdir().unwrap();
    let err = reg
        .dispatch(
            "terminal",
            serde_json::json!({"command": "echo should-not-run"}),
            &ctx(ws.path()),
            &engine,
        )
        .await
        .unwrap_err();
    assert!(matches!(err, argo_types::ArgoError::RbacDenied { .. }));
}
