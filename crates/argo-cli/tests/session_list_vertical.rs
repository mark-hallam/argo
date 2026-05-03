//! `argo session list` and `GET /sessions` surface a `vertical`
//! column (#690). Completes the operator-visibility trifecta:
//! - `/audit/by-vertical` (#688) tells you which vertical is busy
//! - `argo session show` (#689) confirms one session's binding
//! - `argo session list` (this PR) shows every session's binding
//!   in one paste-able table without per-row drill-down
//!
//! Coverage:
//! - CLI markdown table includes `| vertical |` header + per-row cell
//! - CLI text view includes `vertical` column header
//! - CLI JSON list output already carries `vertical_id` (regression
//!   guard that #690 hasn't accidentally dropped it)
//! - Gateway `GET /sessions` JSON includes `vertical_id` field
//! - Gateway `GET /sessions?format=markdown` table includes
//!   `vertical` column

mod common;

use common::{argo_bin, await_listening, body_of, http_get, http_post, pick_free_port};
use std::io::BufReader;
use std::path::Path;
use std::process::{Command, Stdio};

const MIN_TOML: &str = "schema_version = \"1\"\n\n\
[vertical]\n\
id = \"demo\"\n\
name = \"Demo\"\n\
version = \"0.1.0\"\n";

fn write_demo_vertical(verticals_dir: &Path) {
    let demo = verticals_dir.join("demo");
    std::fs::create_dir_all(&demo).unwrap();
    std::fs::write(demo.join("vertical.toml"), MIN_TOML).unwrap();
}

fn workspace_init(home: &Path, root: &Path) {
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home)
        .current_dir(root)
        .args(["workspace", "init"])
        .output()
        .expect("spawn workspace init");
    assert!(
        out.status.success(),
        "workspace init: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Boot two sessions: one bound to `demo`, one unbound. Returns
/// the home + verticals tempdirs (kept alive by caller) plus the
/// two session ids.
fn boot_two_sessions(
    home: &Path,
    verticals: &Path,
) -> (String, String) {
    let bound = run_new(home, verticals, "bound", Some("demo"));
    let unbound = run_new(home, verticals, "unbound", None);
    (bound, unbound)
}

fn run_new(home: &Path, verticals: &Path, title: &str, vertical: Option<&str>) -> String {
    let mut args: Vec<String> = vec![
        "new".into(),
        "--provider".into(),
        "anthropic".into(),
        "--model".into(),
        "claude-haiku-4-5".into(),
        "--title".into(),
        title.into(),
    ];
    if let Some(v) = vertical {
        args.push("--vertical".into());
        args.push(v.into());
    }
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home)
        .env("ARGO_VERTICALS_DIR", verticals)
        .args(&args)
        .output()
        .expect("spawn new");
    assert!(
        out.status.success(),
        "argo new: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let line = stdout
        .lines()
        .find(|l| l.starts_with("created session"))
        .unwrap_or_else(|| panic!("no 'created session' line in:\n{stdout}"));
    line.trim_start_matches("created session")
        .trim()
        .split_whitespace()
        .next()
        .unwrap_or_else(|| panic!("no uuid token in: {line}"))
        .to_string()
}

#[test]
fn cli_session_list_markdown_includes_vertical_column() {
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    write_demo_vertical(verticals.path());
    let (_bound, _unbound) = boot_two_sessions(home.path(), verticals.path());

    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args(["session", "list", "--format", "markdown"])
        .output()
        .expect("spawn list");
    assert!(out.status.success());
    let body = String::from_utf8(out.stdout).unwrap();
    assert!(
        body.contains("| vertical |"),
        "markdown header missing vertical column; got:\n{body}"
    );
    // Bound row carries the vertical id; unbound shows '-'.
    assert!(
        body.contains("`demo`"),
        "missing demo vertical cell; got:\n{body}"
    );
    assert!(
        body.contains("`-`"),
        "missing - cell for unbound row; got:\n{body}"
    );
}

#[test]
fn cli_session_list_text_includes_vertical_column() {
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    write_demo_vertical(verticals.path());
    let (_bound, _unbound) = boot_two_sessions(home.path(), verticals.path());

    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args(["session", "list"])
        .output()
        .expect("spawn list");
    assert!(out.status.success());
    let body = String::from_utf8(out.stdout).unwrap();
    assert!(
        body.contains("vertical"),
        "text header missing vertical column; got:\n{body}"
    );
    // Bound vertical name appears in some row; unbound row uses
    // the `-` placeholder.
    assert!(body.contains("demo"), "missing demo vertical cell; got:\n{body}");
}

#[test]
fn cli_session_list_json_carries_vertical_id() {
    // Regression guard for #689 + #690: each session row in the
    // JSON list still includes vertical_id (null for unbound).
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    write_demo_vertical(verticals.path());
    let (bound, unbound) = boot_two_sessions(home.path(), verticals.path());

    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args(["session", "list", "--json"])
        .output()
        .expect("spawn list --json");
    assert!(out.status.success());
    let body = String::from_utf8(out.stdout).unwrap();
    let mut found_bound = false;
    let mut found_unbound = false;
    for line in body.lines().filter(|l| !l.trim().is_empty()) {
        let v: serde_json::Value = serde_json::from_str(line).expect("valid json");
        let id = v["id"].as_str().unwrap_or("");
        if id == bound {
            assert_eq!(v["vertical_id"], "demo", "bound row: {v}");
            found_bound = true;
        } else if id == unbound {
            assert!(
                v["vertical_id"].is_null(),
                "unbound row vertical_id must be null; got {v}"
            );
            found_unbound = true;
        }
    }
    assert!(found_bound && found_unbound, "missed a row in:\n{body}");
}

// ---- Gateway --------------------------------------------------------

fn boot_gateway(
    home: &Path,
    workspace_root: &Path,
    verticals: &Path,
    max_requests: u32,
) -> (
    std::process::Child,
    BufReader<std::process::ChildStderr>,
    u16,
) {
    workspace_init(home, workspace_root);
    let port = pick_free_port();
    let mut child = Command::new(argo_bin())
        .env("ARGO_HOME", home)
        .env("ARGO_VERTICALS_DIR", verticals)
        .current_dir(workspace_root)
        .args([
            "gateway",
            "start",
            "--port",
            &port.to_string(),
            "--max-requests",
            &max_requests.to_string(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn gateway");
    let mut stderr = BufReader::new(child.stderr.take().unwrap());
    await_listening(&mut stderr);
    (child, stderr, port)
}

#[test]
fn gateway_sessions_list_json_carries_vertical_id_for_each_row() {
    let home = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    write_demo_vertical(verticals.path());
    let (mut child, _stderr, port) =
        boot_gateway(home.path(), root.path(), verticals.path(), 6);

    // One bound + one unbound session.
    http_post(
        port,
        "/verticals/demo/sessions",
        r#"{"model":"claude-haiku-4-5","provider":"anthropic","title":"gw-bound"}"#,
    );
    http_post(
        port,
        "/sessions",
        r#"{"model":"claude-haiku-4-5","provider":"anthropic","title":"gw-unbound"}"#,
    );

    let resp = http_get(port, "/sessions");
    assert!(resp.contains("HTTP/1.1 200 OK"), "list: {resp}");
    let arr: serde_json::Value = serde_json::from_str(body_of(&resp)).unwrap();
    let arr = arr.as_array().unwrap();
    assert_eq!(arr.len(), 2, "expected 2 rows; got {arr:?}");
    let mut bound_ok = false;
    let mut unbound_ok = false;
    for row in arr {
        // Every row carries the field even when unbound.
        assert!(row.get("vertical_id").is_some(), "missing vertical_id key: {row}");
        if row["title"] == "gw-bound" {
            assert_eq!(row["vertical_id"], "demo");
            bound_ok = true;
        } else if row["title"] == "gw-unbound" {
            assert!(row["vertical_id"].is_null(), "unbound row: {row}");
            unbound_ok = true;
        }
    }
    assert!(bound_ok && unbound_ok);

    let _ = child.kill();
}

#[test]
fn gateway_sessions_list_markdown_includes_vertical_column() {
    let home = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    write_demo_vertical(verticals.path());
    let (mut child, _stderr, port) =
        boot_gateway(home.path(), root.path(), verticals.path(), 5);

    http_post(
        port,
        "/verticals/demo/sessions",
        r#"{"model":"claude-haiku-4-5","provider":"anthropic","title":"md-bound"}"#,
    );
    http_post(
        port,
        "/sessions",
        r#"{"model":"claude-haiku-4-5","provider":"anthropic","title":"md-unbound"}"#,
    );

    let resp = http_get(port, "/sessions?format=markdown");
    assert!(resp.contains("HTTP/1.1 200 OK"), "list md: {resp}");
    let body = body_of(&resp);
    assert!(
        body.contains("| vertical |"),
        "markdown header missing vertical column; got:\n{body}"
    );
    assert!(
        body.contains("`demo`"),
        "bound row missing demo cell; got:\n{body}"
    );
    assert!(
        body.contains("`-`"),
        "unbound row missing `-` cell; got:\n{body}"
    );

    let _ = child.kill();
}
