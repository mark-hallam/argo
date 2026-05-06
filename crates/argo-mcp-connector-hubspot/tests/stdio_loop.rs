use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

fn hubspot_bin() -> &'static str {
    env!("CARGO_BIN_EXE_argo-mcp-connector-hubspot")
}

fn drive(requests: &[&str]) -> Vec<serde_json::Value> {
    let mut child = Command::new(hubspot_bin())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn hubspot MCP");

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
            "hubspot.search_contacts",
            "hubspot.create_contact",
            "hubspot.create_deal",
            "hubspot.log_activity",
            "hubspot.get_pipeline"
        ]
    );
}

#[test]
fn missing_token_fails_clearly() {
    std::env::remove_var("HUBSPOT_PRIVATE_APP_TOKEN");
    let resps = drive(&[
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"hubspot.get_pipeline","arguments":{}}}"#,
    ]);
    let result = &resps[0]["result"];
    assert_eq!(result["isError"], true);
    let text = result["content"][0]["text"].as_str().expect("text");
    assert!(text.contains("HUBSPOT_PRIVATE_APP_TOKEN"));
}
