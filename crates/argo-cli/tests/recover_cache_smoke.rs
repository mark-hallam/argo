mod common;

use common::argo_bin;
use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};

/// `argo recover-cache list --json` on a fresh ARGO_HOME returns an
/// empty envelope. `clear --all` requires `--confirm`. End-to-end
/// exercises the new `Command::RecoverCache` dispatch + the
/// argo-store `recovery_cache` helpers.
#[test]
fn recover_cache_list_empty_then_clear_all_requires_confirm() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();

    let list = run(home.path(), work.path(), &["recover-cache", "list", "--json"]);
    assert_success(&list);
    let body: Value = serde_json::from_slice(&list.stdout).unwrap();
    assert_eq!(body["ok"], true);
    assert_eq!(body["kind"], "recover_cache_list");
    assert_eq!(body["count"], 0);
    assert!(body["entries"].as_array().unwrap().is_empty());

    let bad = run(home.path(), work.path(), &["recover-cache", "clear", "--all"]);
    assert!(
        !bad.status.success(),
        "clear --all without --confirm should fail; stderr={}",
        String::from_utf8_lossy(&bad.stderr)
    );
    let stderr = String::from_utf8_lossy(&bad.stderr);
    assert!(
        stderr.contains("yes-i-mean-it"),
        "expected confirm-token hint, got: {stderr}"
    );

    let good = run(
        home.path(),
        work.path(),
        &[
            "recover-cache",
            "clear",
            "--all",
            "--confirm",
            "yes-i-mean-it",
            "--json",
        ],
    );
    assert_success(&good);
    let body: Value = serde_json::from_slice(&good.stdout).unwrap();
    assert_eq!(body["ok"], true);
    assert_eq!(body["kind"], "recover_cache_clear");
    assert_eq!(body["target"], "all");
    assert_eq!(body["removed"], 0);

    let show = run(
        home.path(),
        work.path(),
        &["recover-cache", "show", "missing-session", "--json"],
    );
    assert!(
        !show.status.success(),
        "show on missing session should fail; stderr={}",
        String::from_utf8_lossy(&show.stderr)
    );
    let stderr = String::from_utf8_lossy(&show.stderr);
    assert!(
        stderr.contains("no cached suggestions"),
        "expected missing-session error, got: {stderr}"
    );
}

fn run(home: &Path, cwd: &Path, args: &[&str]) -> Output {
    Command::new(argo_bin())
        .env("ARGO_HOME", home)
        .env_remove("ANTHROPIC_API_KEY")
        .current_dir(cwd)
        .args(args)
        .output()
        .unwrap()
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "status: {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
