//! End-to-end smoke for the `mvp-vertical-introspection` surface.
//! Exercises the operator-facing discovery seam against the actual
//! demo vertical that ships with the binary (`finance-ar`) so a
//! future edit to `verticals/` or to the loader can't silently
//! break the catalog API. `vertical_discovery.rs` already covers
//! synthetic fixtures.

mod common;

use common::{argo_bin, await_listening, body_of, http_get, pick_free_port};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn repo_verticals_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(|p| p.join("verticals"))
        .expect("workspace verticals dir resolves")
}

fn argo() -> Command {
    let mut cmd = Command::new(argo_bin());
    cmd.env("ARGO_VERTICALS_DIR", repo_verticals_dir());
    cmd
}

#[test]
fn cli_list_json_includes_finance_ar() {
    let out = argo()
        .args(["vertical", "list", "--source", "workspace", "--json"])
        .output()
        .expect("spawn list");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("list --json emits valid JSON");
    let loaded = body["loaded"]
        .as_array()
        .expect("`loaded` is an array per brief AC #1");
    let finance = loaded
        .iter()
        .find(|v| v["id"] == "finance-ar")
        .unwrap_or_else(|| panic!("finance-ar missing from catalog: {body}"));
    // The list projection is intentionally narrow (id/version/persona/
    // action_count/source) — full manifest details are reachable via
    // `describe --json` (covered separately below).
    assert_eq!(finance["version"], "0.1.0");
    assert_eq!(finance["source"], "workspace");
    assert!(
        finance["action_count"].as_u64().is_some_and(|n| n > 0),
        "finance-ar declares actions: {finance}"
    );
}

#[test]
fn cli_describe_json_emits_full_manifest_for_finance_ar() {
    let out = argo()
        .args(["vertical", "describe", "finance-ar", "--json"])
        .output()
        .expect("spawn describe");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("describe --json emits valid JSON");
    // Brief AC #2: full manifest, not the one-paragraph summary the
    // text form returns.
    assert_eq!(v["id"], "finance-ar");
    assert_eq!(v["name"], "Finance — Accounts Receivable");
    assert_eq!(v["version"], "0.1.0");
    let actions = v["actions"]
        .as_array()
        .expect("describe --json must include the action list");
    assert!(
        actions.iter().any(|a| a["name"] == "send_reminder"),
        "send_reminder must appear in the executor matrix: {v}"
    );
    assert!(
        v["vertical_toml"].as_str().is_some_and(|s| !s.is_empty()),
        "describe --json must include the raw manifest body"
    );
}

#[test]
fn cli_describe_text_keeps_one_paragraph_summary() {
    // Text output stays the narrative blurb so callers of the
    // existing `argo vertical describe <id>` shape don't suddenly
    // get the full manifest dumped at them.
    let out = argo()
        .args(["vertical", "describe", "finance-ar"])
        .output()
        .expect("spawn describe text");
    assert!(out.status.success());
    let body = String::from_utf8(out.stdout).unwrap();
    assert!(
        body.contains("Finance — Accounts Receivable"),
        "summary should mention vertical name: {body}"
    );
    assert!(
        body.contains("approval-gated"),
        "summary should mention approval gating: {body}"
    );
}

fn spawn_gateway(
    port: u16,
    home: &Path,
) -> (
    std::process::Child,
    std::io::BufReader<std::process::ChildStderr>,
) {
    // `--max-requests 1`: each test makes one HTTP call; the gateway
    // self-shuts after that so `child.wait()` returns. Bumping the
    // budget would hang the test forever.
    let mut child = Command::new(argo_bin())
        .env("ARGO_HOME", home)
        .env("ARGO_VERTICALS_DIR", repo_verticals_dir())
        .args([
            "gateway",
            "start",
            "--port",
            &port.to_string(),
            "--max-requests",
            "1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn gateway");
    // Caller MUST hold the stderr reader for the gateway's lifetime —
    // dropping it SIGPIPEs the gateway on its next stderr write.
    let mut stderr = std::io::BufReader::new(child.stderr.take().expect("stderr piped"));
    await_listening(&mut stderr);
    (child, stderr)
}

#[test]
fn gateway_get_verticals_lists_finance_ar() {
    let port = pick_free_port();
    let home = tempfile::tempdir().unwrap();
    let (mut child, _stderr) = spawn_gateway(port, home.path());

    let resp = http_get(port, "/verticals");
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp={resp}");
    let v: serde_json::Value = serde_json::from_str(body_of(&resp)).expect("body parses");
    let loaded = v["loaded"].as_array().expect("loaded array");
    assert!(
        loaded.iter().any(|x| x["id"] == "finance-ar"),
        "finance-ar must appear in /verticals: {v}"
    );

    let _ = child.wait();
}

#[test]
fn gateway_get_vertical_finance_ar_returns_full_manifest() {
    let port = pick_free_port();
    let home = tempfile::tempdir().unwrap();
    let (mut child, _stderr) = spawn_gateway(port, home.path());

    let resp = http_get(port, "/verticals/finance-ar");
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp={resp}");
    let v: serde_json::Value = serde_json::from_str(body_of(&resp)).expect("body parses");
    assert_eq!(v["id"], "finance-ar");
    assert_eq!(v["name"], "Finance — Accounts Receivable");
    assert!(
        v["tools"]["denied"]
            .as_array()
            .is_some_and(|d| d.iter().any(|t| t == "file_write")),
        "finance-ar manifest denies file_write: {v}"
    );

    let _ = child.wait();
}

#[test]
fn gateway_get_vertical_nonexistent_returns_404() {
    let port = pick_free_port();
    let home = tempfile::tempdir().unwrap();
    let (mut child, _stderr) = spawn_gateway(port, home.path());

    let resp = http_get(port, "/verticals/this-is-not-a-real-vertical");
    assert!(
        resp.contains("HTTP/1.1 404 Not Found"),
        "missing vertical must 404: resp={resp}"
    );
    assert!(
        body_of(&resp).contains("not found"),
        "404 body must explain why: {resp}"
    );

    let _ = child.wait();
}
