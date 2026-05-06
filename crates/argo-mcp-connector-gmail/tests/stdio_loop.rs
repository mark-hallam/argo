use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

fn gmail_bin() -> &'static str {
    env!("CARGO_BIN_EXE_argo-mcp-connector-gmail")
}

fn drive(requests: &[&str]) -> Vec<serde_json::Value> {
    let mut child = Command::new(gmail_bin())
        .env_remove("ARGO_GMAIL_CLIENT_ID")
        .env_remove("ARGO_GMAIL_CLIENT_SECRET")
        .env_remove("ARGO_GMAIL_REFRESH_TOKEN")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn gmail MCP");

    {
        let stdin = child.stdin.as_mut().expect("stdin");
        for request in requests {
            writeln!(stdin, "{request}").expect("write request");
        }
    }
    drop(child.stdin.take());

    let stdout = child.stdout.take().expect("stdout");
    let mut out = Vec::new();
    for line in BufReader::new(stdout).lines() {
        let line = line.expect("read stdout");
        if !line.is_empty() {
            out.push(serde_json::from_str(&line).expect("json response"));
        }
    }
    let _ = child.wait();
    out
}

#[test]
fn tools_list_advertises_all_five_tools() {
    let resps = drive(&[r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#]);
    let tools = resps[0]["result"]["tools"].as_array().expect("tools");
    let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
    assert_eq!(
        names,
        vec![
            "gmail.search",
            "gmail.read",
            "gmail.draft",
            "gmail.send",
            "gmail.label"
        ]
    );
}

#[test]
fn missing_oauth_env_fails_clearly() {
    let resps = drive(&[
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"gmail.search","arguments":{}}}"#,
    ]);
    let result = &resps[0]["result"];
    assert_eq!(result["isError"], true);
    let text = result["content"][0]["text"].as_str().expect("text");
    assert!(text.contains("ARGO_GMAIL_CLIENT_ID"));
}
