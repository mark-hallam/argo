//! End-to-end smoke for `mission-cu-native-macos-executor-dispatch`.
//!
//! Replaces the prior `argo --help` no-op with a real exercise of
//! the native-macOS computer-use dispatch path
//! (`crates/argo-cli/src/handlers/worker_actions_executor_cu_macos.rs`).
//! Authors a fixture vertical that pins
//! `executor.kind = "computer_use_macos"`, injects a pending
//! `worker_action`, then drives it through the running `argo`
//! binary's `worker actions execute` so the dispatch shim resolves
//! the macOS executor, runs preflight, and emits a real
//! [`ActionOutcome`].
//!
//! What this proves:
//!
//! - The `ExecutorKind::ComputerUseMacOS` arm is wired into
//!   `resolve_executor`/`invoke_executor_for` and runs end-to-end
//!   through the CLI surface (no stub, no `--help` short-circuit).
//! - On stock macOS without granted Accessibility / Screen
//!   Recording / Automation permissions, dispatch resolves to
//!   `ActionOutcome::HumanHandoff` and writes the per-action
//!   bundle under `.skycot/outputs/<job>/actions/<action>/`.
//! - When the operator's demo machine has the four permissions
//!   granted and `ARGO_NATIVE_MACOS_INTEGRATION=1`, the
//!   `dispatch_autonomous_loop_smoke` variant exercises the full
//!   Anthropic CU run loop end-to-end. That variant is gated on
//!   the env var so a stock CI run does not require keychain
//!   credentials.
//!
//! What this does NOT prove:
//!
//! - The autonomous Anthropic loop in the default variant — it is
//!   structurally short-circuited on CI by the macOS permission
//!   preflight (`InputMonitoring` is never probable from script
//!   land, so `PermissionReport::all_granted()` returns false on
//!   any host whose operator has not granted the four perms by
//!   hand).
//! - Non-macOS dispatch behavior — gated out by
//!   `#![cfg(target_os = "macos")]` at file top so the smoke does
//!   not false-pass through a skip path on Linux / Windows.
//!
//! Fixes F-10 in `docs/skycot/audit/2026-05-16-one-shot-architecture-audit.md`.

#![cfg(target_os = "macos")]

use std::path::{Path, PathBuf};
use std::process::Command;

const VERTICAL_ID: &str = "cu-macos-smoke";
const ACTION_TYPE: &str = "cu_macos_noop";
const JOB_ID: &str = "cu-macos-smoke-job";

fn fixture_manifest_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("cu_macos_smoke_manifest.toml")
}

fn write_vertical_manifest(verticals_dir: &Path) {
    let dir = verticals_dir.join(VERTICAL_ID);
    std::fs::create_dir_all(&dir).expect("verticals/<id> dir");
    let src = std::fs::read_to_string(fixture_manifest_path())
        .expect("read fixture manifest");
    std::fs::write(dir.join("vertical.toml"), src).expect("write vertical.toml");
}

fn assert_ok(label: &str, out: &std::process::Output) {
    assert!(
        out.status.success(),
        "{label} failed (exit {:?})\nstdout:\n{}\nstderr:\n{}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
}

fn append_pending_action(home: &Path, work: &Path, verticals_dir: &Path) -> String {
    let action_id = uuid::Uuid::now_v7().to_string();
    let body = serde_json::json!({
        "schema_key": "worker_action",
        "action_id": action_id,
        "job_id": JOB_ID,
        "action_type": ACTION_TYPE,
        "risk_level": "low",
        "requires_approval": false,
        "state": "pending",
        "payload": {"target": "Notes", "note": "cu-macos smoke probe"},
        "created_at": chrono::Utc::now().to_rfc3339(),
    });
    let body_str = serde_json::to_string(&body).expect("serialize action");
    let out = Command::new(env!("CARGO_BIN_EXE_argo"))
        .env("ARGO_HOME", home)
        .env("ARGO_VERTICALS_DIR", verticals_dir)
        .current_dir(work)
        .args([
            "memory",
            "append",
            "--vertical",
            VERTICAL_ID,
            "memory",
            &body_str,
        ])
        .output()
        .expect("spawn argo memory append");
    assert_ok("memory append", &out);
    action_id
}

fn workspace_setup() -> (tempfile::TempDir, tempfile::TempDir, PathBuf) {
    let home = tempfile::tempdir().expect("tempdir ARGO_HOME");
    let work = tempfile::tempdir().expect("tempdir workspace");
    let verticals_dir = work.path().join("verticals");
    write_vertical_manifest(&verticals_dir);
    let init = Command::new(env!("CARGO_BIN_EXE_argo"))
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", &verticals_dir)
        .current_dir(work.path())
        .args(["workspace", "init"])
        .output()
        .expect("spawn argo workspace init");
    assert_ok("workspace init", &init);
    (home, work, verticals_dir)
}

fn execute_action(home: &Path, work: &Path, verticals_dir: &Path, action_id: &str) -> serde_json::Value {
    let out = Command::new(env!("CARGO_BIN_EXE_argo"))
        .env("ARGO_HOME", home)
        .env("ARGO_VERTICALS_DIR", verticals_dir)
        .current_dir(work)
        .args(["worker", "actions", "execute", action_id, "--json"])
        .output()
        .expect("spawn argo worker actions execute");
    assert_ok("worker actions execute", &out);
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "execute stdout is not JSON: {e}\nstdout:\n{}",
            String::from_utf8_lossy(&out.stdout)
        )
    })
}

/// Default smoke — exercises the dispatch arm end-to-end through
/// the CLI surface. On stock macOS CI runners the macOS permission
/// preflight fails (no Accessibility / Screen Recording granted to
/// the test binary), so the dispatch resolves to
/// `ActionOutcome::HumanHandoff`. The test asserts the CLI exits
/// cleanly, a bundle path lands in the JSON output, and the
/// per-action bundle dir on disk carries the executor-stamped
/// `instructions.md`. Either `HumanHandoff` (preflight short-
/// circuit) or `Completed` (operator demo machine with perms +
/// `ANTHROPIC_API_KEY`) is a valid proof — both leave a bundle
/// the test can inspect.
#[test]
fn dispatch_arm_resolves_for_computer_use_macos_kind() {
    let (home, work, verticals_dir) = workspace_setup();
    let action_id = append_pending_action(home.path(), work.path(), &verticals_dir);
    let v = execute_action(home.path(), work.path(), &verticals_dir, &action_id);

    let bundle_path = v
        .get("execution_bundle_path")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_else(|| {
            panic!(
                "execute --json missing /execution_bundle_path\nfull: {}",
                serde_json::to_string_pretty(&v).unwrap_or_default()
            )
        });
    let bundle = PathBuf::from(bundle_path);
    assert!(
        bundle.is_dir(),
        "executor did not create bundle dir at {}",
        bundle.display()
    );
    let instructions = std::fs::read_to_string(bundle.join("instructions.md"))
        .expect("bundle missing instructions.md");
    assert!(
        instructions.contains("computer-use-macos"),
        "expected `computer-use-macos` tag in instructions.md, got:\n{instructions}"
    );

    let state = v
        .get("state")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_else(|| panic!("execute --json missing /state: {v}"));
    assert!(
        matches!(state, "executing" | "completed"),
        "expected ActionOutcome HumanHandoff(executing) or Completed(completed) — got `{state}`"
    );
}

/// Operator-only smoke for the autonomous Anthropic CU loop.
/// Gated behind `ARGO_NATIVE_MACOS_INTEGRATION=1` so a default
/// `cargo test -p argo-cli` does not require macOS Accessibility /
/// Screen Recording grants + `ANTHROPIC_API_KEY`. When the env
/// var is set, the operator's demo machine should reach the
/// autonomous loop (`stop_success → ActionOutcome::Completed`); the
/// test still accepts `HumanHandoff` because preflight may fail
/// even on a demo machine if one of the four permissions was
/// revoked. The proof gate is "dispatch resolved cleanly", not
/// "Anthropic returned a specific summary".
#[test]
fn dispatch_autonomous_loop_smoke() {
    if std::env::var("ARGO_NATIVE_MACOS_INTEGRATION").ok().as_deref() != Some("1") {
        eprintln!(
            "WARN: skipping dispatch_autonomous_loop_smoke — \
             set ARGO_NATIVE_MACOS_INTEGRATION=1 on the demo machine to enable"
        );
        return;
    }
    let (home, work, verticals_dir) = workspace_setup();
    let action_id = append_pending_action(home.path(), work.path(), &verticals_dir);
    let v = execute_action(home.path(), work.path(), &verticals_dir, &action_id);
    let state = v
        .get("state")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_else(|| panic!("execute --json missing /state: {v}"));
    assert!(
        matches!(state, "executing" | "completed"),
        "autonomous loop produced unexpected state `{state}` — full: {v}"
    );
}
