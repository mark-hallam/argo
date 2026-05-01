//! End-to-end smoke for `argo session` lifecycle subcommands —
//! title, messages, export. Lives in its own file so
//! `end_to_end.rs` (the original workspace+memory smoke) stays
//! under the 500-LOC budget as more session tests land.
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

fn boot_session(workspace: &Path, home: &Path, title: &str) -> String {
    assert!(run(workspace, home, &["workspace", "init"]).status.success());
    let out = run(
        workspace,
        home,
        &[
            "new",
            "--provider",
            "anthropic",
            "--model",
            "claude-haiku-4-5",
            "--title",
            title,
        ],
    );
    assert!(
        out.status.success(),
        "argo new failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    parse_uuid_after("created session", &stdout)
}

/// `argo session title` set then clear (#494).
#[cfg(unix)]
#[test]
fn session_title_set_then_clear_round_trips() {
    let workspace = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let session_id = boot_session(workspace.path(), home.path(), "initial");

    let out = run(
        workspace.path(),
        home.path(),
        &["session", "title", &session_id, "renamed via test"],
    );
    assert!(out.status.success());

    let out = run(
        workspace.path(),
        home.path(),
        &["session", "show", &session_id, "--json"],
    );
    let body = String::from_utf8_lossy(&out.stdout);
    assert!(
        body.contains("\"title\":\"renamed via test\""),
        "title not set: {body}"
    );

    let out = run(
        workspace.path(),
        home.path(),
        &["session", "title", &session_id, "--clear"],
    );
    assert!(out.status.success());

    let out = run(
        workspace.path(),
        home.path(),
        &["session", "show", &session_id, "--json"],
    );
    let body = String::from_utf8_lossy(&out.stdout);
    assert!(body.contains("\"title\":null"), "title not cleared: {body}");
}

/// `argo session messages` with `--tail` + `--role` (#484, #487).
#[cfg(unix)]
#[test]
fn session_messages_filters_and_tail() {
    let workspace = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let session_id = boot_session(workspace.path(), home.path(), "messages test");

    for text in ["first steer", "second steer"] {
        let out = run(
            workspace.path(),
            home.path(),
            &["steer", &session_id, text],
        );
        assert!(out.status.success());
    }

    // Default — 2 system messages.
    let out = run(
        workspace.path(),
        home.path(),
        &["session", "messages", &session_id, "--json"],
    );
    let body = String::from_utf8_lossy(&out.stdout);
    let lines: Vec<_> = body.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 2, "expected 2 lines: {body}");

    // `--tail 1` keeps the second.
    let out = run(
        workspace.path(),
        home.path(),
        &["session", "messages", &session_id, "--tail", "1", "--json"],
    );
    let body = String::from_utf8_lossy(&out.stdout);
    let lines: Vec<_> = body.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("second steer"));

    // `--role user` returns nothing — no user messages here.
    let out = run(
        workspace.path(),
        home.path(),
        &[
            "session", "messages", &session_id, "--role", "user", "--json",
        ],
    );
    let body = String::from_utf8_lossy(&out.stdout);
    let lines: Vec<_> = body.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 0);
}

/// `argo session export --format markdown` round-trip (#473, #474).
#[cfg(unix)]
#[test]
fn session_export_markdown_renders_header_and_sections() {
    let workspace = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let session_id = boot_session(workspace.path(), home.path(), "export round-trip");

    let out = run(
        workspace.path(),
        home.path(),
        &["steer", &session_id, "test message"],
    );
    assert!(out.status.success());

    let out = run(
        workspace.path(),
        home.path(),
        &["session", "export", &session_id, "--format", "markdown"],
    );
    assert!(out.status.success());
    let body = String::from_utf8_lossy(&out.stdout);

    assert!(
        body.contains("# Session export round-trip"),
        "missing title header: {body}"
    );
    assert!(body.contains(&format!("`{session_id}`")));
    assert!(body.contains("**Model**: `claude-haiku-4-5`"));
    assert!(body.contains("**Provider**: `anthropic`"));
    assert!(body.contains("**Messages**: 1"));
    assert!(body.contains("**Spend**: $0.0000"));
    assert!(body.contains("## [1] system"));
    assert!(body.contains("test message"));
}

/// `argo session diff --format markdown` (#475). Boots two
/// sessions with different models so the header diffs render.
#[cfg(unix)]
#[test]
fn session_diff_markdown_renders_header_table() {
    let workspace = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();

    assert!(run(workspace.path(), home.path(), &["workspace", "init"]).status.success());
    let out_a = run(
        workspace.path(),
        home.path(),
        &[
            "new",
            "--provider",
            "anthropic",
            "--model",
            "claude-haiku-4-5",
            "--title",
            "a",
        ],
    );
    let id_a = parse_uuid_after(
        "created session",
        &String::from_utf8_lossy(&out_a.stdout),
    );
    let out_b = run(
        workspace.path(),
        home.path(),
        &[
            "new",
            "--provider",
            "anthropic",
            "--model",
            "claude-opus-4-6",
            "--title",
            "b",
        ],
    );
    let id_b = parse_uuid_after(
        "created session",
        &String::from_utf8_lossy(&out_b.stdout),
    );

    let out = run(
        workspace.path(),
        home.path(),
        &["session", "diff", &id_a, &id_b, "--format", "markdown"],
    );
    assert!(out.status.success());
    let body = String::from_utf8_lossy(&out.stdout);

    assert!(body.contains("# Session diff"));
    assert!(body.contains(&format!("`{id_a}`")));
    assert!(body.contains(&format!("`{id_b}`")));
    // Header diffs render as a table; model rows differ.
    assert!(body.contains("`model`"), "missing model header diff: {body}");
    assert!(body.contains("claude-haiku-4-5"));
    assert!(body.contains("claude-opus-4-6"));
}
