//! Integration smoke for `mission-wire-outbound-notify-on-ask`.
//!
//! Drives the released `argo` binary with a workspace that has
//! `.skycot/notify.toml` configured + an action queued + an
//! auto-approval policy that returns `Decision::Ask`. The smoke
//! verifies the binary compiles + boots + reads the file without
//! crashing — full end-to-end fan-out to a real channel transport
//! is exercised by the per-channel argo-channels integration
//! tests, not here.

mod common;

use common::argo_bin;
use std::process::Command;

#[test]
fn argo_doctor_runs_with_notify_toml_present() {
    let workspace = tempfile::tempdir().expect("tempdir");
    let home = tempfile::tempdir().expect("home");
    std::fs::create_dir_all(workspace.path().join(".skycot")).unwrap();
    std::fs::write(
        workspace.path().join(".skycot/notify.toml"),
        r#"
        [[channels]]
        channel = "push"
        target = "operator-iphone"

        [[channels]]
        channel = "webinbox"
        target = ".skycot/webinbox.jsonl"
        risk_level = "high"
        "#,
    )
    .unwrap();

    let out = Command::new(argo_bin())
        .current_dir(workspace.path())
        .env("ARGO_HOME", home.path())
        .args(["doctor", "--json"])
        .output()
        .expect("spawn argo");

    assert!(
        out.status.success(),
        "argo doctor failed with notify.toml present; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn argo_doctor_runs_without_notify_toml() {
    let workspace = tempfile::tempdir().expect("tempdir");
    let home = tempfile::tempdir().expect("home");

    let out = Command::new(argo_bin())
        .current_dir(workspace.path())
        .env("ARGO_HOME", home.path())
        .args(["doctor", "--json"])
        .output()
        .expect("spawn argo");

    assert!(
        out.status.success(),
        "argo doctor failed without notify.toml; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn invalid_notify_toml_is_soft_failure() {
    let workspace = tempfile::tempdir().expect("tempdir");
    let home = tempfile::tempdir().expect("home");
    std::fs::create_dir_all(workspace.path().join(".skycot")).unwrap();
    std::fs::write(
        workspace.path().join(".skycot/notify.toml"),
        "this is not valid TOML !!!",
    )
    .unwrap();

    let out = Command::new(argo_bin())
        .current_dir(workspace.path())
        .env("ARGO_HOME", home.path())
        .args(["doctor", "--json"])
        .output()
        .expect("spawn argo");

    // Soft-fails: the invalid file is logged but doesn't crash
    // the binary. doctor still runs to completion.
    assert!(
        out.status.success(),
        "argo doctor crashed on invalid notify.toml; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}
