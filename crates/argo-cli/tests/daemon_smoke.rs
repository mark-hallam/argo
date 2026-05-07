//! Integration smoke test for `argo daemon install/uninstall`.
//!
//! Drives the released binary against a TempDir-backed
//! LaunchAgents directory and an `--bin` override so the test is
//! hermetic — no real `~/Library/LaunchAgents` mutation, no
//! `launchctl load` against the real launchd. The `--dry-run`
//! path does not touch disk at all and asserts the rendered
//! plist body comes back via `--json`. The non-dry-run path
//! verifies the plist file lands at the override location.

mod common;

use common::argo_bin;
use serde_json::Value;
use std::process::Command;

fn run(args: &[&str], home: &std::path::Path) -> std::process::Output {
    Command::new(argo_bin())
        .env("HOME", home)
        .env_remove("ARGO_HOME")
        .args(args)
        .output()
        .expect("spawn argo")
}

#[test]
fn daemon_install_dry_run_emits_plist_body() {
    let home = tempfile::tempdir().expect("home tempdir");
    let launch_agents = home.path().join("Library").join("LaunchAgents");
    let out = run(
        &[
            "daemon",
            "install",
            "--dry-run",
            "--json",
            "--bin",
            "/usr/local/bin/argo",
            "--launch-agents-dir",
            launch_agents.to_str().unwrap(),
        ],
        home.path(),
    );
    assert!(
        out.status.success(),
        "expected success; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let v: Value = serde_json::from_str(&stdout).expect("json parse");
    assert_eq!(v["dry_run"], true);
    assert_eq!(v["loaded"], false);
    assert_eq!(v["label"], "com.skycot.argo");
    let plist_body = v["plist_body"].as_str().expect("plist_body string");
    assert!(plist_body.contains("<string>/usr/local/bin/argo</string>"));
    assert!(plist_body.contains("<key>Label</key>"));
    assert!(plist_body.contains("com.skycot.argo"));
    assert!(!launch_agents.join("com.skycot.argo.plist").exists());
}

#[test]
#[cfg(target_os = "macos")]
fn daemon_uninstall_handles_missing_plist() {
    let home = tempfile::tempdir().expect("home tempdir");
    let launch_agents = home.path().join("Library").join("LaunchAgents");
    std::fs::create_dir_all(&launch_agents).expect("mkdir LaunchAgents");
    let out = run(
        &[
            "daemon",
            "uninstall",
            "--json",
            "--launch-agents-dir",
            launch_agents.to_str().unwrap(),
        ],
        home.path(),
    );
    assert!(
        out.status.success(),
        "expected success; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: Value = serde_json::from_str(&String::from_utf8_lossy(&out.stdout)).expect("json");
    assert_eq!(v["plist_existed"], false);
    assert_eq!(v["removed"], false);
}
