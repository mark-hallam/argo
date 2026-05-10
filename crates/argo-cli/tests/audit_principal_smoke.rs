//! Integration smoke for `mvp-audit-principal-stamp` (Tier 1 MVP
//! — closes mission claim "everything is auditable").
//!
//! Spawns `argo gateway start` with a bearer token configured,
//! POSTs `/sessions/<id>/approve` with both the bearer and a
//! `X-Argo-Principal` header, then re-opens the SQLite DB and
//! asserts the `audit_record` row carries the resolved principal
//! in the new `principal_id` column. A second request without the
//! header asserts the column lands NULL for anonymous callers.
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

const BEARER_TOKEN: &str = "test-bearer-token-aud-principal";
const PRINCIPAL_HEADER: &str = "human:operator@example.com";

fn argo_bin() -> &'static str {
    env!("CARGO_BIN_EXE_argo")
}

fn pick_free_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    port
}

fn await_listening(stderr: &mut BufReader<impl Read>) {
    let mut line = String::new();
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        if std::time::Instant::now() > deadline {
            panic!("listener did not announce in 5s");
        }
        line.clear();
        stderr.read_line(&mut line).expect("read line");
        if line.contains("listening on") {
            break;
        }
    }
}

fn http_post_with_headers(
    port: u16,
    path: &str,
    body: &str,
    extra_headers: &[(&str, &str)],
) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    let mut req = format!(
        "POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\nConnection: close\r\n",
        body.len()
    );
    for (name, value) in extra_headers {
        req.push_str(&format!("{name}: {value}\r\n"));
    }
    req.push_str("\r\n");
    req.push_str(body);
    stream.write_all(req.as_bytes()).expect("write");
    let mut resp = String::new();
    stream.read_to_string(&mut resp).expect("read");
    resp
}

fn body_of(resp: &str) -> &str {
    resp.split_once("\r\n\r\n").map(|(_h, b)| b).unwrap_or("")
}

fn extract_session_id(create_body: &str) -> String {
    create_body
        .split("\"id\":\"")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .expect("session id in create body")
        .to_string()
}

fn spawn_gateway(home: &Path, port: u16, max_requests: u32) -> (Child, BufReader<std::process::ChildStderr>) {
    let mut child = Command::new(argo_bin())
        .env("ARGO_HOME", home)
        .env("ARGO_GATEWAY_BEARER_TOKEN", BEARER_TOKEN)
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
        .expect("spawn");
    let stderr = BufReader::new(child.stderr.take().unwrap());
    (child, stderr)
}

async fn read_principals(home: &Path) -> Vec<(String, Option<String>)> {
    let store = argo_store::SessionStore::open(home.join("sessions.db"))
        .await
        .unwrap();
    store.migrate().await.unwrap();
    sqlx::query_as::<_, (String, Option<String>)>(
        "SELECT kind, principal_id FROM audit_record \
         ORDER BY seq ASC",
    )
    .fetch_all(store.pool())
    .await
    .unwrap()
}

#[test]
fn approve_with_principal_header_stamps_audit_row() {
    let port = pick_free_port();
    let home = tempfile::tempdir().unwrap();
    // 3 requests: create session, approve (with principal), approve (anon).
    let (mut child, mut stderr) = spawn_gateway(home.path(), port, 3);
    await_listening(&mut stderr);

    let bearer = format!("Bearer {BEARER_TOKEN}");
    let auth_header = ("Authorization", bearer.as_str());
    let principal_header = ("X-Argo-Principal", PRINCIPAL_HEADER);

    let create = http_post_with_headers(
        port,
        "/sessions",
        r#"{"model":"m","provider":"p","title":"t"}"#,
        &[auth_header],
    );
    assert!(create.contains("HTTP/1.1 201 Created"), "create={create}");
    let sid = extract_session_id(body_of(&create));

    // Authenticated approve with a resolved principal stamps the
    // header value into `audit_record.principal_id`.
    let approve = http_post_with_headers(
        port,
        &format!("/sessions/{sid}/approve"),
        r#"{"tool":"delegate_task","note":"smoke"}"#,
        &[auth_header, principal_header],
    );
    assert!(approve.contains("HTTP/1.1 200 OK"), "approve={approve}");

    // Anonymous (no principal header) approve emits an audit row
    // whose `principal_id` is NULL — distinct from `"anonymous"`.
    let approve_anon = http_post_with_headers(
        port,
        &format!("/sessions/{sid}/approve"),
        r#"{"tool":"shell","note":"anon"}"#,
        &[auth_header],
    );
    assert!(approve_anon.contains("HTTP/1.1 200 OK"), "anon={approve_anon}");

    let status = child.wait().expect("wait");
    assert!(status.success());

    let rt = tokio::runtime::Runtime::new().unwrap();
    let rows = rt.block_on(read_principals(home.path()));
    let granted: Vec<&Option<String>> = rows
        .iter()
        .filter(|(kind, _)| kind == "approval.granted")
        .map(|(_, p)| p)
        .collect();
    assert_eq!(granted.len(), 2, "rows={rows:?}");
    assert_eq!(
        granted[0].as_deref(),
        Some(PRINCIPAL_HEADER),
        "first approve must carry the resolved principal: {granted:?}"
    );
    assert!(
        granted[1].is_none(),
        "anonymous approve must land NULL: {granted:?}"
    );
}
