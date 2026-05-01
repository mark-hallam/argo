//! End-to-end smoke for the offline CLI surface.
//!
//! Spins up a fresh workspace + ARGO_HOME under a `TempDir`, then
//! drives the binary through:
//!   workspace init → memory append → memory verify → memory edit
//!
//! Each step shells out to `argo` so a regression in dispatch,
//! argument parsing, or argo.toml resolution surfaces as a real
//! failure rather than a unit test passing while the binary is
//! broken.
use std::path::Path;
use std::process::{Command, Output};

fn argo_bin() -> &'static str {
    env!("CARGO_BIN_EXE_argo")
}

fn run(workspace: &Path, home: &Path, args: &[&str], extra_env: &[(&str, &str)]) -> Output {
    let mut cmd = Command::new(argo_bin());
    cmd.current_dir(workspace).env("ARGO_HOME", home).args(args);
    for (k, v) in extra_env {
        cmd.env(k, v);
    }
    cmd.output()
        .unwrap_or_else(|e| panic!("spawn argo {args:?}: {e}"))
}

fn parse_uuid_after(prefix: &str, stdout: &str) -> String {
    let line = stdout
        .lines()
        .find(|l| l.starts_with(prefix))
        .unwrap_or_else(|| panic!("no '{prefix}' line in:\n{stdout}"));
    let rest = line.trim_start_matches(prefix).trim_start();
    let token = rest
        .split_whitespace()
        .next()
        .unwrap_or_else(|| panic!("no uuid token after '{prefix}' in: {line}"));
    token.to_string()
}

// The fake editor uses `#!/bin/sh` and `chmod 0o755`; both are
// Unix-specific. Skip the test on Windows — the underlying
// memory-edit handler is platform-agnostic and is exercised by
// `cargo test -p argo-cli` unit tests.
#[cfg(unix)]
#[test]
fn workspace_init_then_memory_append_verify_edit() {
    let workspace = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    // 1. workspace init
    let out = run(workspace.path(), home.path(), &["workspace", "init"], &[]);
    assert!(
        out.status.success(),
        "workspace init failed: stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("workspace initialized"));
    assert!(stdout.contains("db row: workspace "));

    // 2. memory append
    let out = run(
        workspace.path(),
        home.path(),
        &["memory", "append", "memory", "smoke entry one"],
        &[],
    );
    assert!(
        out.status.success(),
        "memory append failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let entry_id = parse_uuid_after("appended", &stdout);

    // 3. memory verify (hash ok)
    let out = run(
        workspace.path(),
        home.path(),
        &["memory", "verify", &entry_id],
        &[],
    );
    assert!(
        out.status.success(),
        "memory verify failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains("hash ok"));

    // 4. memory edit via fake editor (writes new body)
    let editor_path = home.path().join("fake-editor.sh");
    std::fs::write(
        &editor_path,
        "#!/bin/sh\nprintf 'edited body\\n' > \"$1\"\n",
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&editor_path).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&editor_path, perms).unwrap();
    }
    let editor_str = editor_path.to_str().unwrap();
    let out = run(
        workspace.path(),
        home.path(),
        &["memory", "edit", &entry_id],
        &[("EDITOR", editor_str)],
    );
    assert!(
        out.status.success(),
        "memory edit failed: stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("edited:"));
    let new_id = parse_uuid_after("edited: new active entry", &stdout);
    assert_ne!(new_id, entry_id, "edit must produce a fresh uuid");

    // 5. memory list shows the new active entry, not the original.
    let out = run(workspace.path(), home.path(), &["memory", "list"], &[]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains(&new_id), "list should mention new id");
    assert!(
        !stdout.contains(&entry_id),
        "list should NOT mention the superseded id"
    );
}

#[test]
fn agents_create_then_list_then_delete_round_trips() {
    let workspace = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    let out = run(workspace.path(), home.path(), &["workspace", "init"], &[]);
    assert!(
        out.status.success(),
        "workspace init failed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );

    let out = run(
        workspace.path(),
        home.path(),
        &[
            "agents",
            "create",
            "alice",
            "--model",
            "claude-test",
            "--toolset",
            "default",
        ],
        &[],
    );
    assert!(
        out.status.success(),
        "agents create failed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains("[agents.alice]"));

    let out = run(workspace.path(), home.path(), &["agents", "list"], &[]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("alice"));
    assert!(stdout.contains("claude-test"));

    let toml_path = workspace.path().join("argo.toml");
    let body = std::fs::read_to_string(&toml_path).unwrap();
    assert!(body.contains("[agents.alice]"));

    // Re-creating fails (already exists).
    let out = run(
        workspace.path(),
        home.path(),
        &["agents", "create", "alice"],
        &[],
    );
    assert!(!out.status.success(), "duplicate create must fail");

    // Delete + verify the block is gone.
    let out = run(
        workspace.path(),
        home.path(),
        &["agents", "delete", "alice"],
        &[],
    );
    assert!(
        out.status.success(),
        "agents delete failed: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body = std::fs::read_to_string(&toml_path).unwrap();
    assert!(!body.contains("[agents.alice]"));

    // Deleting a missing agent fails cleanly.
    let out = run(
        workspace.path(),
        home.path(),
        &["agents", "delete", "ghost"],
        &[],
    );
    assert!(!out.status.success(), "delete-missing must fail");
}

/// End-to-end smoke for `argo session title` (set / clear).
/// Boots a fresh workspace, creates a session via `argo new`,
/// renames it, confirms via `argo session show --json`, then
/// clears the title and confirms again.
#[cfg(unix)]
#[test]
fn session_title_set_then_clear_round_trips() {
    let workspace = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    // Bootstrap workspace + ensure the store exists.
    let out = run(workspace.path(), home.path(), &["workspace", "init"], &[]);
    assert!(out.status.success());

    // Create a session row. argo new just inserts a row — no
    // provider call — so we don't need an API key.
    let out = run(
        workspace.path(),
        home.path(),
        &[
            "new",
            "--provider",
            "anthropic",
            "--model",
            "claude-haiku-4-5",
            "--title",
            "initial",
        ],
        &[],
    );
    assert!(
        out.status.success(),
        "argo new failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let session_id = parse_uuid_after("created session", &stdout);

    // Rename to "renamed via test".
    let out = run(
        workspace.path(),
        home.path(),
        &["session", "title", &session_id, "renamed via test"],
        &[],
    );
    assert!(
        out.status.success(),
        "session title set failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Confirm via JSON show.
    let out = run(
        workspace.path(),
        home.path(),
        &["session", "show", &session_id, "--json"],
        &[],
    );
    let body = String::from_utf8_lossy(&out.stdout);
    assert!(
        body.contains("\"title\":\"renamed via test\""),
        "title not set in show: {body}"
    );

    // Now clear it.
    let out = run(
        workspace.path(),
        home.path(),
        &["session", "title", &session_id, "--clear"],
        &[],
    );
    assert!(out.status.success());

    let out = run(
        workspace.path(),
        home.path(),
        &["session", "show", &session_id, "--json"],
        &[],
    );
    let body = String::from_utf8_lossy(&out.stdout);
    assert!(
        body.contains("\"title\":null"),
        "title not cleared in show: {body}"
    );
}
