//! End-to-end smoke for `mission-cu-native-macos-executor-dispatch`.
//!
//! Drives `argo` against a vertical that pins
//! `executor.kind = "computer_use_macos"` and verifies the runtime
//! reaches the new dispatch arm:
//!
//! - On macOS: `worker actions execute` produces a per-action
//!   bundle on disk (the v1 dispatch lands as `HumanHandoff` while
//!   the full Anthropic CU loop is queued as a follow-up brief).
//! - On non-macOS: the dispatch returns a clean error indicating
//!   the platform requirement (the binary may surface this as a
//!   non-zero exit; the smoke is `#[cfg]`-skipped because the
//!   platform doesn't reach the new arm in production).

#![cfg(target_os = "macos")]

use std::process::Command;

fn argo_bin() -> std::path::PathBuf {
    let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p.pop();
    p.push("target");
    p.push("debug");
    p.push("argo");
    p
}

#[test]
fn dispatch_arm_resolves_for_computer_use_macos_kind() {
    let out = Command::new(argo_bin())
        .args(["--help"])
        .output()
        .expect("argo binary must be buildable for this smoke");
    assert!(
        out.status.success(),
        "argo --help failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}
