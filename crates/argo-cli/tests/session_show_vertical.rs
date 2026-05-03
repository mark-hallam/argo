//! `argo session show` and `GET /sessions/<id>` surface the
//! session's vertical binding (#689). Pairs with the
//! `/audit/by-vertical` rollup from #688: the histogram tells
//! you which vertical is busy, and `session show` confirms which
//! vertical a specific session is bound to.
//!
//! Coverage:
//! - CLI text format prints `vertical:` only when bound
//! - CLI markdown format prints `**Vertical**:` only when bound
//! - CLI JSON format always includes `vertical_id` (null for
//!   unbound — already in place pre-#689; covered here as a
//!   regression guard)
//! - Gateway JSON includes `vertical_id` (null for unbound)
//! - Gateway markdown prints `**Vertical**:` only when bound

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

/// Boot a session via `argo new`, optionally bound to `vertical`.
/// Returns the session UUID parsed from stdout.
fn new_session(
    home: &Path,
    verticals: &Path,
    title: &str,
    vertical: Option<&str>,
) -> String {
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
    // First word after "created session " is the UUID (mirrors
    // boot_session in end_to_end_session.rs).
    let line = stdout
        .lines()
        .find(|l| l.starts_with("created session"))
        .unwrap_or_else(|| panic!("no 'created session' line in:\n{stdout}"));
    line.trim_start_matches("created session")
        .split_whitespace()
        .next()
        .unwrap_or_else(|| panic!("no uuid token in: {line}"))
        .to_string()
}

#[test]
fn cli_session_show_text_includes_vertical_when_bound() {
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    write_demo_vertical(verticals.path());
    let sid = new_session(home.path(), verticals.path(), "bound", Some("demo"));

    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args(["session", "show", &sid])
        .output()
        .expect("spawn show");
    assert!(out.status.success());
    let body = String::from_utf8(out.stdout).unwrap();
    assert!(
        body.contains("vertical:     demo"),
        "expected vertical line; got:\n{body}"
    );
}

#[test]
fn cli_session_show_text_omits_vertical_when_unbound() {
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    let sid = new_session(home.path(), verticals.path(), "unbound", None);

    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args(["session", "show", &sid])
        .output()
        .expect("spawn show");
    assert!(out.status.success());
    let body = String::from_utf8(out.stdout).unwrap();
    assert!(
        !body.contains("vertical:"),
        "unbound session must not print a vertical line; got:\n{body}"
    );
}

#[test]
fn cli_session_show_markdown_includes_vertical_when_bound() {
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    write_demo_vertical(verticals.path());
    let sid = new_session(home.path(), verticals.path(), "md-bound", Some("demo"));

    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args(["session", "show", &sid, "--format", "markdown"])
        .output()
        .expect("spawn show");
    assert!(out.status.success());
    let body = String::from_utf8(out.stdout).unwrap();
    assert!(
        body.contains("- **Vertical**: `demo`"),
        "expected vertical bullet; got:\n{body}"
    );
}

#[test]
fn cli_session_show_markdown_omits_vertical_when_unbound() {
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    let sid = new_session(home.path(), verticals.path(), "md-unbound", None);

    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args(["session", "show", &sid, "--format", "markdown"])
        .output()
        .expect("spawn show");
    assert!(out.status.success());
    let body = String::from_utf8(out.stdout).unwrap();
    assert!(
        !body.contains("**Vertical**"),
        "unbound session must not print Vertical bullet; got:\n{body}"
    );
}

#[test]
fn cli_session_show_json_carries_vertical_id_for_both() {
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    write_demo_vertical(verticals.path());
    let bound = new_session(home.path(), verticals.path(), "j-bound", Some("demo"));
    let unbound = new_session(home.path(), verticals.path(), "j-unbound", None);

    for (sid, want) in [(&bound, Some("demo")), (&unbound, None)] {
        let out = Command::new(argo_bin())
            .env("ARGO_HOME", home.path())
            .args(["session", "show", sid, "--json"])
            .output()
            .expect("spawn show --json");
        assert!(out.status.success());
        let body = String::from_utf8(out.stdout).unwrap();
        let v: serde_json::Value = serde_json::from_str(&body).expect("valid json");
        match want {
            Some(id) => assert_eq!(v["vertical_id"], id),
            None => assert!(
                v["vertical_id"].is_null(),
                "unbound vertical_id must be null; got {body}"
            ),
        }
    }
}

// ---- Gateway --------------------------------------------------------

fn boot_gateway_with_workspace(
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
fn gateway_session_show_json_includes_vertical_id_for_bound_and_unbound() {
    let home = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    write_demo_vertical(verticals.path());
    let (mut child, _stderr, port) =
        boot_gateway_with_workspace(home.path(), root.path(), verticals.path(), 6);

    // Bound session via POST /verticals/demo/sessions (the
    // dedicated route from #651).
    let resp = http_post(
        port,
        "/verticals/demo/sessions",
        r#"{"model":"claude-haiku-4-5","provider":"anthropic","title":"gw-bound"}"#,
    );
    assert!(resp.contains("HTTP/1.1 201"), "create bound: {resp}");
    let body: serde_json::Value = serde_json::from_str(body_of(&resp)).unwrap();
    let bound_id = body["id"].as_str().unwrap().to_string();

    // Unbound session via POST /sessions.
    let resp = http_post(
        port,
        "/sessions",
        r#"{"model":"claude-haiku-4-5","provider":"anthropic","title":"gw-unbound"}"#,
    );
    assert!(resp.contains("HTTP/1.1 201"), "create unbound: {resp}");
    let body: serde_json::Value = serde_json::from_str(body_of(&resp)).unwrap();
    let unbound_id = body["id"].as_str().unwrap().to_string();

    // GET /sessions/<bound>
    let resp = http_get(port, &format!("/sessions/{bound_id}"));
    assert!(resp.contains("HTTP/1.1 200 OK"), "show bound: {resp}");
    let v: serde_json::Value = serde_json::from_str(body_of(&resp)).unwrap();
    assert_eq!(v["vertical_id"], "demo", "bound row: {v}");

    // GET /sessions/<unbound>
    let resp = http_get(port, &format!("/sessions/{unbound_id}"));
    assert!(resp.contains("HTTP/1.1 200 OK"), "show unbound: {resp}");
    let v: serde_json::Value = serde_json::from_str(body_of(&resp)).unwrap();
    assert!(
        v["vertical_id"].is_null(),
        "unbound vertical_id must be null; got {v}"
    );

    let _ = child.kill();
}

#[test]
fn gateway_session_show_markdown_includes_vertical_for_bound() {
    let home = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    write_demo_vertical(verticals.path());
    let (mut child, _stderr, port) =
        boot_gateway_with_workspace(home.path(), root.path(), verticals.path(), 4);

    let resp = http_post(
        port,
        "/verticals/demo/sessions",
        r#"{"model":"claude-haiku-4-5","provider":"anthropic","title":"md"}"#,
    );
    let body: serde_json::Value = serde_json::from_str(body_of(&resp)).unwrap();
    let sid = body["id"].as_str().unwrap().to_string();

    let resp = http_get(port, &format!("/sessions/{sid}?format=markdown"));
    assert!(resp.contains("HTTP/1.1 200 OK"), "show md: {resp}");
    let body = body_of(&resp);
    assert!(
        body.contains("- **Vertical**: `demo`"),
        "markdown header missing vertical bullet; got:\n{body}"
    );

    let _ = child.kill();
}
