//! End-to-end smoke test for `argo mcp registry add → list → remove`.
//!
//! Drives the real `argo` binary against a tempdir-rooted workspace to
//! exercise the full clap → handler → store path. Stream
//! `codex-mcp-server-registry-cli`.

use std::process::Command;

fn argo_bin() -> &'static str {
    env!("CARGO_BIN_EXE_argo")
}

fn run(dir: &std::path::Path, args: &[&str]) -> String {
    let out = Command::new(argo_bin())
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("spawn argo {args:?}: {e}"));
    assert!(
        out.status.success(),
        "argo {args:?} failed\nstdout={}\nstderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("stdout utf8")
}

#[test]
fn mcp_registry_add_list_remove_smoke() {
    let dir = tempfile::tempdir().unwrap();
    let tenant = "tenant-smoke";
    let add = run(
        dir.path(),
        &[
            "mcp",
            "registry",
            "add",
            "jira",
            "--kind",
            "http",
            "--url",
            "http://127.0.0.1:9",
            "--tenant",
            tenant,
            "--json",
        ],
    );
    assert!(add.contains("\"server_id\": \"jira\""), "{add}");

    let list = run(
        dir.path(),
        &[
            "mcp", "registry", "list", "--tenant", tenant, "--format", "json",
        ],
    );
    assert!(list
        .lines()
        .any(|line| line.contains("\"server_id\":\"jira\"")));

    let remove = run(
        dir.path(),
        &[
            "mcp", "registry", "remove", "jira", "--tenant", tenant, "--force", "--json",
        ],
    );
    assert!(remove.contains("\"removed\""), "{remove}");

    let list = run(
        dir.path(),
        &[
            "mcp", "registry", "list", "--tenant", tenant, "--format", "json",
        ],
    );
    assert!(list.trim().is_empty(), "{list}");
}
