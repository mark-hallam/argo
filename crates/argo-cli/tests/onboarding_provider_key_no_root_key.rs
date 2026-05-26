mod common;

use common::listener_signal::{await_listener_signal, StderrDrain};
use common::{argo_bin, body_of, http_get, pick_free_port};
use serde_json::Value;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

const PRINCIPAL: &str = "human:no-root-key@example.com";
const SMOKE_DB_KEY_B64: &str = "QkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkI=";

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
}

fn boot_without_workspace_root_key(port: u16, max_requests: u32) -> (Child, StderrDrain) {
    let root = workspace_root();
    let home = tempfile::tempdir().expect("tempdir").keep();
    let mut child = Command::new(argo_bin())
        .current_dir(&root)
        .env("ARGO_HOME", &home)
        .env("HOME", &home)
        .env("ARGO_VERTICALS_DIR", root.join("verticals"))
        .env("OPENAI_API_KEY", "sk-test-only-not-real")
        .env("SKYCOT_DB_KEY_BASE64", SMOKE_DB_KEY_B64)
        .env_remove("SKYCOT_WORKSPACE_ROOT_KEY_BASE64")
        .env_remove("ARGO_CREDENTIALS_MASTER_KEY")
        .args([
            "gateway",
            "start",
            "--port",
            &port.to_string(),
            "--max-requests",
            &(max_requests + 1).to_string(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    let stderr = child.stderr.take().expect("stderr piped");
    let drain = await_listener_signal(stderr).expect("gateway listener signal");
    let resp = http_get(port, "/sessions");
    assert!(
        resp.contains("HTTP/1.1 200 OK"),
        "gateway store not ready: {resp}"
    );
    (child, drain)
}

fn http_post_json(port: u16, path: &str, body: &str, headers: &[(&str, &str)]) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    let mut req =
        format!("POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\n");
    for (name, value) in headers {
        req.push_str(name);
        req.push_str(": ");
        req.push_str(value);
        req.push_str("\r\n");
    }
    req.push_str(&format!(
        "Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    ));
    stream.write_all(req.as_bytes()).expect("write");
    let mut resp = String::new();
    stream.read_to_string(&mut resp).expect("read");
    resp
}

#[test]
fn provider_key_no_root_key_returns_503_without_secret_names() {
    let port = pick_free_port();
    let (mut child, _drain) = boot_without_workspace_root_key(port, 2);

    let provision = http_post_json(
        port,
        "/api/v1/onboarding/provision",
        r#"{"workspace_name":"Acme","company_name":"Acme Co"}"#,
        &[("X-Argo-Principal", PRINCIPAL)],
    );
    assert!(
        provision.contains("HTTP/1.1 201 Created") || provision.contains("HTTP/1.1 200 OK"),
        "provision response: {provision}"
    );

    let resp = http_post_json(
        port,
        "/api/v1/onboarding/provider-key",
        r#"{"provider":"anthropic","api_key":"sk-ant-valid-shape-test-key-000000"}"#,
        &[("X-Argo-Principal", PRINCIPAL)],
    );
    assert!(
        resp.contains("HTTP/1.1 503 Service Unavailable"),
        "provider-key response: {resp}"
    );
    let body = body_of(&resp);
    let parsed: Value = serde_json::from_str(body).expect("json body");
    assert_eq!(parsed["error"]["code"], "provider_validation_unconfigured");
    assert!(
        parsed["error"]["message"]
            .as_str()
            .is_some_and(|message| message.contains("skip this step")),
        "body: {body}"
    );
    for forbidden in [
        "SKYCOT_WORKSPACE_ROOT_KEY_BASE64",
        "ARGO_CREDENTIALS_MASTER_KEY",
        "wrap key",
        "no workspace root key",
    ] {
        assert!(!body.contains(forbidden), "body leaked {forbidden}: {body}");
    }

    let status = child.wait().expect("wait");
    assert!(status.success(), "gateway exit: {status}");
}
