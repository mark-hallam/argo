//! Integration smoke for the per-request gateway audit table.
mod common;

use common::{argo_bin, await_listening, gateway_lock_guard, http_get, pick_free_port};
use std::io::BufReader;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[test]
fn gateway_writes_request_audit_rows_for_public_gets() {
    let _gateway_lock = gateway_lock_guard();
    let port = pick_free_port();
    let home = tempfile::tempdir().unwrap();

    let mut child = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_IP_RATE_LIMIT", "0")
        .env("ANTHROPIC_API_KEY", "sk-ant-test-only-not-real")
        .args([
            "gateway",
            "start",
            "--port",
            &port.to_string(),
            "--max-requests",
            "3",
            "--skip-startup-check",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");

    let mut stderr = BufReader::new(child.stderr.take().expect("stderr"));
    await_listening(&mut stderr);

    let health = http_get(port, "/health");
    assert!(health.contains("HTTP/1.1 200 OK"), "{health}");
    let version = http_get(port, "/version");
    assert!(version.contains("HTTP/1.1 200 OK"), "{version}");

    wait_for_rows(home.path().join("sessions.db"));
    let livez = http_get(port, "/livez");
    assert!(livez.contains("HTTP/1.1 200 OK"), "{livez}");
    let _ = child.wait();

    let list = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args(["gateway", "audit", "list", "--since", "1h ago", "--json"])
        .output()
        .expect("audit list");
    assert!(
        list.status.success(),
        "audit list failed: {}",
        String::from_utf8_lossy(&list.stderr)
    );
    let list_json: serde_json::Value = serde_json::from_slice(&list.stdout).unwrap();
    let rows = list_json["rows"].as_array().unwrap();
    assert!(rows.len() >= 2, "{rows:?}");
    assert!(rows.iter().any(|row| row["path"] == "/health"));
    assert!(rows.iter().any(|row| row["path"] == "/version"));

    let request_id = rows
        .iter()
        .find(|row| row["path"] == "/health" || row["path"] == "/version")
        .and_then(|row| row["request_id"].as_str())
        .unwrap();
    let show = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args(["gateway", "audit", "show", request_id, "--json"])
        .output()
        .expect("audit show");
    assert!(
        show.status.success(),
        "audit show failed: {}",
        String::from_utf8_lossy(&show.stderr)
    );
    let show_json: serde_json::Value = serde_json::from_slice(&show.stdout).unwrap();
    assert_eq!(show_json["request_id"], request_id);
    assert!(show_json["path"] == "/health" || show_json["path"] == "/version");
}

fn wait_for_rows(db_path: std::path::PathBuf) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let rows = rt.block_on(async {
            let store = argo_store::SessionStore::open(&db_path).await.unwrap();
            store.migrate().await.unwrap();
            sqlx::query_as::<_, (String, i64, Option<String>)>(
                "SELECT path, status_code, principal \
                 FROM gateway_request_audit ORDER BY received_at",
            )
            .fetch_all(store.pool())
            .await
            .unwrap()
        });
        if rows.len() >= 2 {
            assert!(rows
                .iter()
                .any(|row| row == &("/health".to_string(), 200, None)));
            assert!(rows
                .iter()
                .any(|row| row == &("/version".to_string(), 200, None)));
            return;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for gateway_request_audit rows, got {rows:?}"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}
