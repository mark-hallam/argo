//! Integration tests for `argo setup --check` (#761).
//!
//! Verification mode for the launch checklist: must exit 0 when
//! at least one provider key is loadable from env, and exit non-
//! zero with a clear FAIL message when none are.

mod common;

use common::argo_bin;
use std::process::Command;

/// Strip every provider env-var so the parent shell's keys don't
/// bleed into the child. Used by every test in this file —
/// otherwise an operator running `cargo test` with their real
/// `ANTHROPIC_API_KEY` exported would see the no-key tests turn
/// flaky.
fn drop_provider_env(cmd: &mut Command) {
    for k in [
        "ANTHROPIC_API_KEY",
        "OPENAI_API_KEY",
        "GEMINI_API_KEY",
        "AWS_ACCESS_KEY_ID",
        "ELEVENLABS_API_KEY",
    ] {
        cmd.env_remove(k);
    }
}

#[test]
fn cli_setup_check_exits_zero_with_anthropic_key_set() {
    let home = tempfile::tempdir().unwrap();
    let mut cmd = Command::new(argo_bin());
    cmd.env("ARGO_HOME", home.path());
    drop_provider_env(&mut cmd);
    cmd.env("ANTHROPIC_API_KEY", "sk-ant-test-only-not-real");
    cmd.args(["setup", "--check"]);
    let out = cmd.output().expect("spawn");
    assert!(
        out.status.success(),
        "setup --check should pass when a key is set; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        stdout.contains("[ok]") && stdout.contains("ANTHROPIC_API_KEY"),
        "stdout should confirm the loadable key; got:\n{stdout}"
    );
}

#[test]
fn cli_setup_check_exits_non_zero_when_no_keys_are_set() {
    let home = tempfile::tempdir().unwrap();
    let mut cmd = Command::new(argo_bin());
    cmd.env("ARGO_HOME", home.path());
    drop_provider_env(&mut cmd);
    cmd.args(["setup", "--check"]);
    let out = cmd.output().expect("spawn");
    assert!(
        !out.status.success(),
        "setup --check must fail when no keys are set; stdout={}",
        String::from_utf8_lossy(&out.stdout)
    );
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.contains("[FAIL]"),
        "stderr should carry the FAIL marker; got:\n{stderr}"
    );
    assert!(
        stderr.contains("ANTHROPIC_API_KEY"),
        "stderr should suggest at least one canonical key; got:\n{stderr}"
    );
    assert!(
        stderr.contains("provider_keys.md"),
        "stderr should point at the canonical doc; got:\n{stderr}"
    );
}

#[test]
fn cli_setup_check_treats_empty_string_key_as_unset() {
    // An empty `ANTHROPIC_API_KEY=""` (common copy-paste-from-
    // env-file mistake) must NOT count as a loadable key.
    // Locks the trim-empty branch of `detect_provider_keys`.
    let home = tempfile::tempdir().unwrap();
    let mut cmd = Command::new(argo_bin());
    cmd.env("ARGO_HOME", home.path());
    drop_provider_env(&mut cmd);
    cmd.env("ANTHROPIC_API_KEY", "");
    cmd.env("OPENAI_API_KEY", "   ");
    cmd.args(["setup", "--check"]);
    let out = cmd.output().expect("spawn");
    assert!(
        !out.status.success(),
        "empty / whitespace-only keys must not count as set; stdout={}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn cli_setup_check_accepts_either_anthropic_or_openai() {
    // Locks the inclusive-OR resolution: a setup with the
    // OpenAI key but no Anthropic key passes (and vice versa).
    let home = tempfile::tempdir().unwrap();
    let mut cmd = Command::new(argo_bin());
    cmd.env("ARGO_HOME", home.path());
    drop_provider_env(&mut cmd);
    cmd.env("OPENAI_API_KEY", "sk-test-only-not-real");
    cmd.args(["setup", "--check"]);
    let out = cmd.output().expect("spawn");
    assert!(
        out.status.success(),
        "openai-only setup must pass; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        stdout.contains("OPENAI_API_KEY"),
        "stdout should mention OPENAI_API_KEY; got:\n{stdout}"
    );
}
