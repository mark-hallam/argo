//! Integration coverage for `code_execute`: end-to-end run through
//! the registry's dispatch path (capability checks + tool invocation),
//! proving the `created_files` round-trip the brief calls out.
//!
//! These tests run against the default [`LocalSandbox`] backend —
//! Docker is unavailable on CI runners and Firecracker / E2B are
//! future backends. They are gated `#[cfg(unix)]` to skip on Windows
//! (the LocalSandbox shells through `bash`, same caveat as the
//! `terminal` tool's tests).
#![cfg(unix)]

use argo_capability::CapabilityEngine;
use argo_tool::{CodeExecute, ToolContext, ToolRegistry};
use argo_types::{Capability, ContentBlock, Principal, SpawnKind};
use std::sync::Arc;

fn principal() -> Principal {
    Principal::Local {
        os_user: "code-execute-it".into(),
    }
}

fn registry_with_code_execute() -> ToolRegistry {
    let mut reg = ToolRegistry::new();
    reg.register(Arc::new(CodeExecute::default()));
    reg
}

fn engine_for_principal(grants: &[Capability]) -> CapabilityEngine {
    // Bind to "guest" (no useful caps) and layer the tested grants on
    // top via dynamic per-principal capabilities, so each test can
    // hand-pick exactly the bag it needs without inheriting the
    // owner-role power grant.
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

fn python3_available() -> bool {
    std::process::Command::new("python3")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[tokio::test]
async fn dispatch_blocks_principal_without_spawn_container_capability() {
    let reg = registry_with_code_execute();
    // Grant ToolDispatch but NOT Spawn::Container — call must fail
    // closed at the engine check, never reach the sandbox.
    let engine = engine_for_principal(&[Capability::ToolDispatch {
        tool_name_glob: "code_execute".into(),
    }]);
    let ws = tempfile::tempdir().unwrap();
    let err = reg
        .dispatch(
            "code_execute",
            serde_json::json!({"language": "bash", "source": "echo hi"}),
            &ctx(ws.path()),
            &engine,
        )
        .await
        .unwrap_err();
    assert!(matches!(err, argo_types::ArgoError::RbacDenied { .. }));
}

#[tokio::test]
async fn dispatch_succeeds_with_both_capabilities_granted() {
    let reg = registry_with_code_execute();
    let engine = engine_for_principal(&[
        Capability::ToolDispatch {
            tool_name_glob: "code_execute".into(),
        },
        Capability::Spawn {
            spawn_kind: SpawnKind::Container,
        },
    ]);
    let ws = tempfile::tempdir().unwrap();
    let r = reg
        .dispatch(
            "code_execute",
            serde_json::json!({"language": "bash", "source": "echo round-trip-ok"}),
            &ctx(ws.path()),
            &engine,
        )
        .await
        .unwrap();
    assert!(!r.is_error, "{}", body(&r.content[0]));
    let text = body(&r.content[0]);
    assert!(text.contains("round-trip-ok"), "{text}");
    assert!(text.contains("\"exit\": 0"), "{text}");
}

#[tokio::test]
async fn python_script_creates_file_and_snapshot_round_trips() {
    if !python3_available() {
        // Python3 is not on PATH (unusual on CI; skip rather than fail).
        return;
    }
    let reg = registry_with_code_execute();
    let engine = engine_for_principal(&[
        Capability::ToolDispatch {
            tool_name_glob: "code_execute".into(),
        },
        Capability::Spawn {
            spawn_kind: SpawnKind::Container,
        },
    ]);
    let ws = tempfile::tempdir().unwrap();
    let source = r#"
with open("report.csv", "w") as f:
    f.write("col_a,col_b\n1,2\n3,4\n")
print("rows:2")
"#;
    let r = reg
        .dispatch(
            "code_execute",
            serde_json::json!({
                "language": "python",
                "source": source,
            }),
            &ctx(ws.path()),
            &engine,
        )
        .await
        .unwrap();
    assert!(!r.is_error, "{}", body(&r.content[0]));
    let text = body(&r.content[0]);
    let parsed: serde_json::Value = serde_json::from_str(text).expect("body must be JSON");
    assert_eq!(parsed["exit"], 0);
    assert!(
        parsed["stdout"].as_str().unwrap().contains("rows:2"),
        "stdout missing marker: {text}"
    );
    let created = parsed["created_files"].as_array().unwrap();
    assert_eq!(created.len(), 1, "expected exactly one created file: {text}");
    assert_eq!(created[0]["path"], "report.csv");
    let bytes = created[0]["bytes"].as_u64().unwrap();
    assert!(bytes >= 18, "created file should have non-trivial size: {bytes}");
    // No __pycache__ leakage from python3 -B.
    assert!(
        !created.iter().any(|c| c["path"]
            .as_str()
            .map(|p| p.contains("__pycache__"))
            .unwrap_or(false)),
        "snapshot must filter __pycache__: {text}"
    );
}

#[tokio::test]
async fn side_car_files_are_visible_to_the_script_and_not_in_created_set() {
    if !python3_available() {
        return;
    }
    let reg = registry_with_code_execute();
    let engine = engine_for_principal(&[
        Capability::ToolDispatch {
            tool_name_glob: "code_execute".into(),
        },
        Capability::Spawn {
            spawn_kind: SpawnKind::Container,
        },
    ]);
    let ws = tempfile::tempdir().unwrap();
    let r = reg
        .dispatch(
            "code_execute",
            serde_json::json!({
                "language": "python",
                "source": "print(open('input.txt').read().strip())",
                "files": { "input.txt": "from-side-car" },
            }),
            &ctx(ws.path()),
            &engine,
        )
        .await
        .unwrap();
    assert!(!r.is_error, "{}", body(&r.content[0]));
    let text = body(&r.content[0]);
    let parsed: serde_json::Value = serde_json::from_str(text).expect("body must be JSON");
    assert!(
        parsed["stdout"]
            .as_str()
            .unwrap()
            .contains("from-side-car"),
        "{text}"
    );
    let created = parsed["created_files"].as_array().unwrap();
    assert!(
        !created.iter().any(|c| c["path"] == "input.txt"),
        "side-car must not appear in created_files: {text}"
    );
}
