mod common;

use common::argo_bin;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::process::{Command, Output};
use std::thread;

#[test]
fn connector_authorize_status_test_revoke_round_trip() {
    let workspace = tempfile::tempdir().unwrap(); let home = tempfile::tempdir().unwrap(); let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap(); let addr = listener.local_addr().unwrap();
    let server = thread::spawn(move || { for _ in 0..2 { let (mut stream, _) = listener.accept().unwrap(); let mut buf = [0; 2048]; let n = stream.read(&mut buf).unwrap(); let req = String::from_utf8_lossy(&buf[..n]); assert!(req.contains("POST /auth.test")); assert!(req.to_ascii_lowercase().contains("authorization: bearer smoke-token")); let body = r#"{"ok":true}"#; let resp = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}", body.len()); stream.write_all(resp.as_bytes()).unwrap(); } });
    let base_url = format!("http://{addr}");
    let authorized = run(workspace.path(), home.path(), &base_url, &["connector", "authorize", "slack", "--token", "smoke-token", "--json"]); assert_success(&authorized); assert!(stdout(&authorized).contains("connector_authorize")); assert!(workspace.path().join(".argo/connectors/slack/auth.json").exists());
    let status = run(workspace.path(), home.path(), &base_url, &["connector", "list-status", "--json"]); assert_success(&status); let status_stdout = stdout(&status); assert!(status_stdout.contains("slack")); assert!(status_stdout.contains("authorized"));
    let tested = run(workspace.path(), home.path(), &base_url, &["connector", "test", "slack", "--json"]); assert_success(&tested); assert!(stdout(&tested).contains("connector_test"));
    let revoked = run(workspace.path(), home.path(), &base_url, &["connector", "revoke", "slack", "--json"]); assert_success(&revoked); assert!(!workspace.path().join(".argo/connectors/slack").exists()); server.join().unwrap();
}

fn run(workspace: &Path, home: &Path, base_url: &str, args: &[&str]) -> Output { Command::new(argo_bin()).current_dir(workspace).env("ARGO_HOME", home).env("SLACK_API_BASE_URL", base_url).env_remove("SLACK_BOT_TOKEN").args(args).output().unwrap() }
fn assert_success(output: &Output) { assert!(output.status.success(), "status: {:?}\nstdout:\n{}\nstderr:\n{}", output.status, String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr)); }
fn stdout(output: &Output) -> String { String::from_utf8_lossy(&output.stdout).into_owned() }
