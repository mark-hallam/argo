//! Integration smoke tests for the `argo cred backend` /
//! `argo cred migrate-to-keyring` CLI surface.
//!
//! These tests focus on the CLI behaviour that does not depend on a
//! live OS keyring service: the `--dry-run` migration enumeration and
//! the `argo cred backend` writer. Library-level round-trip semantics
//! through the keyring (mock or real) are covered by the unit tests
//! in `argo-config::cred_keyring`. The brief's `cred migrate-to-keyring`
//! → `cred get` CLI round-trip relies on a `cred get` subcommand that
//! does not exist; verifying argo.toml + dry-run output through the
//! real binary plus the in-process unit tests is the equivalent
//! coverage at the CLI boundary.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn argo_bin() -> &'static str {
    env!("CARGO_BIN_EXE_argo")
}

fn run(workspace: &Path, args: &[&str]) -> Output {
    Command::new(argo_bin())
        .current_dir(workspace)
        .env("ARGO_HOME", workspace.join(".argo-home"))
        .args(args)
        .output()
        .expect("spawn argo binary")
}

fn assert_success(out: &Output) {
    assert!(
        out.status.success(),
        "argo cli failed: status={:?} stdout={} stderr={}",
        out.status,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
}

#[test]
fn dry_run_lists_entries_without_modifying_state() {
    let workspace = tempfile::tempdir().unwrap();
    let argo_dir = workspace.path().join(".argo");
    fs::create_dir_all(&argo_dir).unwrap();
    let creds_path = argo_dir.join("credentials.toml");
    fs::write(
        &creds_path,
        "[credentials]\nOPENAI_API_KEY = \"sk-one\"\nANTHROPIC_API_KEY = \"sk-two\"\n",
    )
    .unwrap();

    let before = fs::read_to_string(&creds_path).unwrap();
    let out = run(workspace.path(), &["cred", "migrate-to-keyring", "--dry-run"]);
    assert_success(&out);

    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("dry-run"), "stdout missing dry-run banner: {stdout}");
    assert!(stdout.contains("OPENAI_API_KEY"), "stdout missing key: {stdout}");
    assert!(stdout.contains("ANTHROPIC_API_KEY"), "stdout missing key: {stdout}");
    // File untouched.
    let after = fs::read_to_string(&creds_path).unwrap();
    assert_eq!(before, after, "credentials.toml mutated by --dry-run");
}

#[test]
fn backend_command_writes_argo_toml_credentials_section() {
    let workspace = tempfile::tempdir().unwrap();

    let out = run(workspace.path(), &["cred", "backend", "keyring"]);
    assert_success(&out);
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("backend set to keyring"), "stdout: {stdout}");

    let argo_toml = workspace.path().join("argo.toml");
    let body = fs::read_to_string(&argo_toml).unwrap();
    assert!(
        body.contains("[credentials]") && body.contains("backend = \"keyring\""),
        "argo.toml missing credentials backend: {body}"
    );

    // Flipping back to `auto` must preserve any unrelated TOML
    // contents that operators may have added by hand.
    fs::write(
        &argo_toml,
        format!("[core]\nprofile = \"smoke\"\n\n{body}"),
    )
    .unwrap();
    let out = run(workspace.path(), &["cred", "backend", "auto"]);
    assert_success(&out);
    let updated = fs::read_to_string(&argo_toml).unwrap();
    assert!(updated.contains("backend = \"auto\""), "auto not written: {updated}");
    assert!(
        updated.contains("profile = \"smoke\""),
        "unrelated [core] section dropped: {updated}"
    );
}

#[test]
fn backend_command_rejects_invalid_kind() {
    let workspace = tempfile::tempdir().unwrap();
    let out = run(workspace.path(), &["cred", "backend", "vault"]);
    assert!(!out.status.success(), "invalid backend should fail");
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.contains("invalid credential backend"),
        "stderr missing reason: {stderr}"
    );
}
