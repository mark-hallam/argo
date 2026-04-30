//! End-to-end stdio test for `argo-mcp-browser`.
//!
//! Spawns the binary, drives the JSON-RPC stdio loop, and verifies:
//!   1. tools/list advertises the 12 spec tools + the 2 HTTP probes
//!      (list_tabs, chrome_version).
//!   2. list_tabs against a known-closed port → clean is_error
//!      response with the "no Chrome on port" hint.
//!   3. chrome_version against the same port → same.
//!   4. one of the 12 spec tools (read_page) → existing
//!      "backend not yet wired" stub.
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

fn browser_bin() -> &'static str {
    env!("CARGO_BIN_EXE_argo-mcp-browser")
}

fn pick_free_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    port
}

fn drive(requests: &[String]) -> Vec<serde_json::Value> {
    let mut child = Command::new(browser_bin())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn browser MCP");

    {
        let stdin = child.stdin.as_mut().expect("stdin");
        for r in requests {
            writeln!(stdin, "{r}").expect("write request");
        }
    }
    drop(child.stdin.take());

    let stdout = child.stdout.take().expect("stdout");
    let mut out = Vec::new();
    for line in BufReader::new(stdout).lines() {
        let l = line.expect("read stdout");
        if l.is_empty() {
            continue;
        }
        let v: serde_json::Value = serde_json::from_str(&l)
            .unwrap_or_else(|e| panic!("response is not JSON: {e}\nline: {l}"));
        out.push(v);
    }
    let _ = child.wait();
    out
}

#[test]
fn tools_list_advertises_twelve_spec_tools_plus_two_http_probes() {
    let resps = drive(&[r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#.into()]);
    assert_eq!(resps.len(), 1);
    let tools = resps[0]["result"]["tools"].as_array().expect("tools array");
    assert_eq!(tools.len(), 14);
    let names: Vec<String> = tools
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect();
    for spec in &[
        "navigate",
        "click",
        "fill",
        "submit",
        "screenshot",
        "read_page",
        "extract_links",
        "wait_for",
        "scroll",
        "eval",
        "cookies",
        "pdf",
    ] {
        assert!(names.contains(&(*spec).to_string()), "missing {spec}");
    }
    assert!(names.contains(&"list_tabs".to_string()));
    assert!(names.contains(&"chrome_version".to_string()));
}

#[test]
fn list_tabs_against_closed_port_surfaces_clean_error() {
    let port = pick_free_port();
    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"list_tabs","arguments":{{"port":{port}}}}}}}"#
    );
    let resps = drive(&[req]);
    assert_eq!(resps.len(), 1);
    let r = &resps[0]["result"];
    assert_eq!(r["isError"], true);
    let text = r["content"][0]["text"].as_str().unwrap();
    assert!(
        text.contains("no Chrome on port") || text.contains("/json"),
        "unexpected text: {text}"
    );
}

#[test]
fn chrome_version_against_closed_port_surfaces_clean_error() {
    let port = pick_free_port();
    let req = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"chrome_version","arguments":{{"port":{port}}}}}}}"#
    );
    let resps = drive(&[req]);
    assert_eq!(resps.len(), 1);
    let r = &resps[0]["result"];
    assert_eq!(r["isError"], true);
    let text = r["content"][0]["text"].as_str().unwrap();
    assert!(
        text.contains("no Chrome on port") || text.contains("/json/version"),
        "unexpected text: {text}"
    );
}

#[test]
fn read_page_returns_not_wired_stub() {
    let resps = drive(&[
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"read_page","arguments":{}}}"#.into(),
    ]);
    assert_eq!(resps.len(), 1);
    let r = &resps[0]["result"];
    assert_eq!(r["isError"], true);
    let text = r["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("not yet wired"));
}
