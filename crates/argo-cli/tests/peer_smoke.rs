//! 2-host smoke test for the multi-host orchestrator POC
//! (see docs/skycot/MULTI_HOST_ORCHESTRATOR.md).
//!
//! Spawns two gateways on `127.0.0.1` ephemeral ports, peers them
//! with each other (poll interval 100ms, shared `ARGO_PEER_SECRET`),
//! creates a session on g1, and asserts:
//!
//! 1. `GET /peer/state` on g2 surfaces a snapshot of g1 within ≤2s.
//! 2. `GET /peer/sync` on g1 — once authenticated — returns the
//!    new session in its `sessions` array.
//! 3. `GET /peer/sync` without the bearer secret returns 401.

mod common;
use common::{argo_bin, body_of, http_get, http_post, pick_free_port};
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const PEER_SECRET: &str = "peer-test-secret";

struct GatewayProc {
    child: Child,
    port: u16,
    _home: tempfile::TempDir,
    _workspace: tempfile::TempDir,
}

impl Drop for GatewayProc {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn spawn_peer_gateway(self_port: u16, peer_url: &str, max_requests: u32) -> GatewayProc {
    let home = tempfile::tempdir().expect("home tempdir");
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    Command::new(argo_bin())
        .current_dir(workspace.path())
        .env("ARGO_HOME", home.path())
        .args(["workspace", "init"])
        .output()
        .expect("workspace init");

    let mut child = Command::new(argo_bin())
        .current_dir(workspace.path())
        .env("ARGO_HOME", home.path())
        .env("ARGO_GATEWAY_PEERS", peer_url)
        .env("ARGO_PEER_SECRET", PEER_SECRET)
        .env("ARGO_PEER_POLL_MS", "100")
        .env("ARGO_PEER_TIMEOUT_MS", "1500")
        .args([
            "gateway",
            "start",
            "--port",
            &self_port.to_string(),
            "--max-requests",
            &max_requests.to_string(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn gateway");

    let stderr = child.stderr.take().expect("stderr");
    let mut stderr = BufReader::new(stderr);
    let mut line = String::new();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if Instant::now() > deadline {
            let _ = child.kill();
            panic!("gateway port {self_port} did not announce listener in 10s");
        }
        line.clear();
        match stderr.read_line(&mut line) {
            Ok(0) => {
                let _ = child.kill();
                panic!("gateway port {self_port} stderr closed before announce");
            }
            Ok(_) => {
                if line.contains("listening on") {
                    break;
                }
            }
            Err(e) => panic!("read stderr: {e}"),
        }
    }
    // Drain stderr in the background so the child's pipe doesn't fill.
    std::thread::spawn(move || {
        let mut s = String::new();
        while stderr.read_line(&mut s).unwrap_or(0) > 0 {
            s.clear();
        }
    });

    GatewayProc {
        child,
        port: self_port,
        _home: home,
        _workspace: workspace,
    }
}

fn http_get_with_bearer(port: u16, path: &str, bearer: &str) -> String {
    use std::io::{Read, Write};
    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).expect("connect");
    let req = format!(
        "GET {path} HTTP/1.1\r\nHost: localhost\r\n\
         Authorization: Bearer {bearer}\r\nConnection: close\r\n\r\n"
    );
    stream.write_all(req.as_bytes()).expect("write");
    let mut resp = String::new();
    stream.read_to_string(&mut resp).expect("read");
    resp
}

#[test]
fn two_gateways_peer_and_session_replicates_to_peer_state() {
    let _lock = common::gateway_lock_guard();
    let g1_port = pick_free_port();
    let g2_port = pick_free_port();
    let g1_url = format!("http://127.0.0.1:{g1_port}");
    let g2_url = format!("http://127.0.0.1:{g2_port}");

    // High max-requests so the gateways don't auto-exit while the
    // test is still polling. Each gateway absorbs:
    //   - peer-poll traffic from the other gateway (~10/sec at 100ms)
    //   - the test's session POST + /peer/state polls
    let mut g1 = spawn_peer_gateway(g1_port, &g2_url, 5000);
    let mut g2 = spawn_peer_gateway(g2_port, &g1_url, 5000);

    // Sanity: /health works on both before we exercise the mesh.
    let h1 = http_get(g1.port, "/health");
    assert!(h1.contains("HTTP/1.1 200 OK"), "g1 /health: {h1}");
    let h2 = http_get(g2.port, "/health");
    assert!(h2.contains("HTTP/1.1 200 OK"), "g2 /health: {h2}");

    // Create a session on g1.
    let create = http_post(
        g1.port,
        "/sessions",
        r#"{"model":"claude-haiku","provider":"anthropic","title":"peer-smoke"}"#,
    );
    assert!(
        create.contains("HTTP/1.1 201 Created"),
        "g1 POST /sessions: {create}"
    );
    let parsed: serde_json::Value =
        serde_json::from_str(body_of(&create)).expect("valid session json");
    let session_id = parsed["id"].as_str().expect("id").to_string();

    // Wait for g2's peer poll to pick up the new session. We poll
    // /peer/state up to ~3s — longer than the 100ms ARGO_PEER_POLL_MS.
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut last = String::new();
    let mut saw_session = false;
    while Instant::now() < deadline {
        last = http_get(g2.port, "/peer/state");
        if let Some(body) = last.split_once("\r\n\r\n").map(|(_h, b)| b) {
            if let Ok(j) = serde_json::from_str::<serde_json::Value>(body) {
                if let Some(peers) = j["peers"].as_array() {
                    if peers.iter().any(|p| {
                        p["url"] == g1_url
                            && p["session_count"].as_u64().unwrap_or(0) >= 1
                            && p["last_error"].is_null()
                    }) {
                        saw_session = true;
                        break;
                    }
                }
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(
        saw_session,
        "g2 never observed g1's session within 3s; last /peer/state: {last}"
    );

    // /peer/sync on g1 with the right bearer returns the session row.
    let sync = http_get_with_bearer(g1.port, "/peer/sync", PEER_SECRET);
    assert!(
        sync.contains("HTTP/1.1 200 OK"),
        "g1 /peer/sync (auth): {sync}"
    );
    let sync_body = body_of(&sync);
    let sync_json: serde_json::Value =
        serde_json::from_str(sync_body).expect("valid sync json");
    let sessions = sync_json["sessions"].as_array().expect("sessions array");
    assert!(
        sessions.iter().any(|s| s["id"].as_str() == Some(&session_id)),
        "g1 /peer/sync did not include created session: {sync_body}"
    );

    // /peer/sync without bearer is 401.
    let sync_unauth = http_get(g1.port, "/peer/sync");
    assert!(
        sync_unauth.contains("HTTP/1.1 401 Unauthorized"),
        "g1 /peer/sync (no auth): {sync_unauth}"
    );

    // Tear down explicitly so the assertions above own the test
    // outcome — Drop's kill() runs on panic too, but explicit
    // shutdown produces cleaner test logs on success.
    let _ = g1.child.kill();
    let _ = g2.child.kill();
    let _ = g1.child.wait();
    let _ = g2.child.wait();
}
