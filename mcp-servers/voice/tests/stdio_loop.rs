//! End-to-end stdio test for `argo-mcp-voice`.
//!
//! Spawns the binary, drives the JSON-RPC stdio loop, and verifies:
//!   1. `tools/list` returns the three tools in spec order, and the
//!      `transcribe` schema advertises the new `backend` field
//!      (whisper|openai).
//!   2. `synthesize` with `backend=openai` and an unset API key
//!      surfaces a clean `OPENAI_API_KEY not set` content block
//!      with `is_error: true` (no panic, no hang).
//!   3. `transcribe` with `backend=openai` and an unset API key
//!      same — error, no hang.
//!   4. `synthesize` with the default backend (`piper`) returns the
//!      pre-existing `not yet wired` stub.
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

fn voice_bin() -> &'static str {
    env!("CARGO_BIN_EXE_argo-mcp-voice")
}

fn drive(requests: &[&str]) -> Vec<serde_json::Value> {
    let mut child = Command::new(voice_bin())
        .env_remove("OPENAI_API_KEY")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn voice MCP");

    {
        let stdin = child.stdin.as_mut().expect("stdin");
        for r in requests {
            writeln!(stdin, "{r}").expect("write request");
        }
        // dropping stdin signals EOF so the run-loop exits cleanly
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
fn tools_list_advertises_three_tools_with_backend_field_on_transcribe() {
    let resps = drive(&[r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#]);
    assert_eq!(resps.len(), 1);
    let tools = resps[0]["result"]["tools"].as_array().expect("tools array");
    assert_eq!(tools.len(), 3);
    assert_eq!(tools[0]["name"], "transcribe");
    assert_eq!(tools[1]["name"], "synthesize");
    assert_eq!(tools[2]["name"], "voice_chat");
    let transcribe_schema = &tools[0]["inputSchema"]["properties"]["backend"];
    assert!(
        !transcribe_schema.is_null(),
        "transcribe schema must advertise a `backend` field after #96"
    );
    let enums = transcribe_schema["enum"].as_array().expect("backend enum");
    let names: Vec<String> = enums
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert!(names.contains(&"openai".to_string()));
    assert!(names.contains(&"whisper".to_string()));
}

#[test]
fn synthesize_openai_with_no_key_returns_clean_error() {
    let resps = drive(&[
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"synthesize","arguments":{"text":"hi","backend":"openai"}}}"#,
    ]);
    assert_eq!(resps.len(), 1);
    let r = &resps[0]["result"];
    assert_eq!(r["isError"], true);
    let text = r["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("OPENAI_API_KEY"));
}

#[test]
fn transcribe_openai_with_no_key_returns_clean_error() {
    let resps = drive(&[
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"transcribe","arguments":{"audio":"/tmp/x.wav","backend":"openai"}}}"#,
    ]);
    assert_eq!(resps.len(), 1);
    let r = &resps[0]["result"];
    assert_eq!(r["isError"], true);
    let text = r["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("OPENAI_API_KEY"));
}

#[test]
fn synthesize_default_backend_returns_not_wired() {
    let resps = drive(&[
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"synthesize","arguments":{"text":"hi"}}}"#,
    ]);
    assert_eq!(resps.len(), 1);
    let r = &resps[0]["result"];
    assert_eq!(r["isError"], true);
    let text = r["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("not yet wired"));
}
