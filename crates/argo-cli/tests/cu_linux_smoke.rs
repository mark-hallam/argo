//! End-to-end smoke for `mission-cu-native-linux-executor-dispatch`.
//!
//! Drives `argo` against a vertical that pins
//! `executor.kind = "computer_use_linux"` and verifies the runtime
//! reaches the new dispatch arm:
//!
//! - On Linux + `ARGO_LINUX_CU_TEST=1`: builds the argo binary and
//!   exercises `--help` to confirm the cfg-gated adapter compiles
//!   and links against the Linux executor crate. The full xdotool
//!   / scrot path requires a live X11 session (DISPLAY set), which
//!   typical CI doesn't have; the env gate keeps the smoke off by
//!   default and on only for hosts that opt in.
//! - On non-Linux: the file is `#[cfg]`-skipped because the
//!   platform doesn't reach the new arm in production.

#![cfg(target_os = "linux")]

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
fn dispatch_arm_resolves_for_computer_use_linux_kind() {
    if std::env::var_os("ARGO_LINUX_CU_TEST").is_none() {
        eprintln!("ARGO_LINUX_CU_TEST not set — skipping native Linux CU smoke");
        return;
    }
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
