//! End-to-end smoke for the read-only provider + workspace surface.
//!
//! Copies a bundled provider manifest into a temp workspace, then
//! drives the binary through:
//!   workspace init → provider validate-all → workspace verify
//!                  → provider validate-all (with a bad manifest)
//!
//! Verifies the bundled manifest set is valid, the verify command
//! greenlights a clean workspace, and validate-all surfaces a
//! non-zero exit when a manifest is malformed.
use std::path::Path;
use std::process::{Command, Output};

fn argo_bin() -> &'static str {
    env!("CARGO_BIN_EXE_argo")
}

fn run(workspace: &Path, home: &Path, args: &[&str]) -> Output {
    Command::new(argo_bin())
        .current_dir(workspace)
        .env("ARGO_HOME", home)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("spawn argo {args:?}: {e}"))
}

fn copy_bundled_manifest(workspace: &Path, name: &str) {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let src = repo_root.join("manifests").join("providers").join(name);
    let dst_dir = workspace.join("manifests").join("providers");
    std::fs::create_dir_all(&dst_dir).unwrap();
    let dst = dst_dir.join(name);
    std::fs::copy(&src, &dst)
        .unwrap_or_else(|e| panic!("copy {} → {}: {e}", src.display(), dst.display()));
}

#[test]
fn validate_all_then_workspace_verify_then_bad_manifest() {
    let workspace = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    let init = run(workspace.path(), home.path(), &["workspace", "init"]);
    assert!(
        init.status.success(),
        "workspace init failed: {}",
        String::from_utf8_lossy(&init.stderr)
    );

    copy_bundled_manifest(workspace.path(), "anthropic.toml");
    copy_bundled_manifest(workspace.path(), "openai-compatible.toml");

    // 1. Bundled set validates clean.
    let v = run(workspace.path(), home.path(), &["provider", "validate-all"]);
    assert!(
        v.status.success(),
        "validate-all failed: stdout={} stderr={}",
        String::from_utf8_lossy(&v.stdout),
        String::from_utf8_lossy(&v.stderr)
    );
    let stdout = String::from_utf8_lossy(&v.stdout);
    assert!(stdout.contains("anthropic.toml"));
    assert!(stdout.contains("openai-compatible.toml"));
    assert!(stdout.contains("manifest(s) validated"));

    // 2. Workspace verify greenlights a clean workspace.
    let w = run(workspace.path(), home.path(), &["workspace", "verify"]);
    assert!(
        w.status.success(),
        "workspace verify failed: stdout={} stderr={}",
        String::from_utf8_lossy(&w.stdout),
        String::from_utf8_lossy(&w.stderr)
    );
    let stdout = String::from_utf8_lossy(&w.stdout);
    assert!(stdout.contains("workspace ok"));

    // 3. Drop a malformed manifest in and confirm validate-all
    //    exits non-zero with a parsing error pointing at it.
    let bad = workspace
        .path()
        .join("manifests")
        .join("providers")
        .join("broken.toml");
    std::fs::write(&bad, "this is not = [valid toml = [[\n").unwrap();
    let v2 = run(workspace.path(), home.path(), &["provider", "validate-all"]);
    assert!(
        !v2.status.success(),
        "validate-all should have failed against the broken manifest"
    );
    let stderr = String::from_utf8_lossy(&v2.stderr);
    assert!(
        stderr.contains("INVALID") || stderr.contains("failed validation"),
        "expected validate-all to surface the failure; got: {stderr}"
    );
}
