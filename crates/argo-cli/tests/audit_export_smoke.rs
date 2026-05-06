mod common;

use argo_audit::compute_chain_hash;
use argo_store::SessionStore;
use argo_types::SessionId;
use chrono::{DateTime, Duration};
use serde_json::{json, Value};
use std::io::Read;
use std::path::Path;
use std::process::{Command, Output};
use uuid::Uuid;

#[tokio::test]
async fn audit_export_formats_redacts_and_checks_chain() {
    let home = tempfile::tempdir().unwrap();
    let out_dir = tempfile::tempdir().unwrap();
    let store = SessionStore::open(home.path().join("sessions.db"))
        .await
        .unwrap();
    store.migrate().await.unwrap();
    let root = home.path().to_string_lossy().to_string();
    let (tenant, workspace) = store.ensure_workspace("audit-export", &root).await.unwrap();
    let session = store
        .create_session(tenant, workspace, "mock", "mock", Some("audit"), None)
        .await
        .unwrap();
    seed_rows(&store, session).await;

    let ndjson_path = out_dir.path().join("audit.ndjson");
    let summary = run_json(
        home.path(),
        &[
            "audit",
            "export",
            "--since",
            "2026-05-06T00:00:00Z",
            "--until",
            "2026-05-06T23:59:59Z",
            "--worker-id",
            "worker-a",
            "--gdpr-redact-emails",
            "--bundle-out",
            ndjson_path.to_str().unwrap(),
            "--json",
        ],
    );
    assert_eq!(summary["export"]["rows"], 1);
    assert_eq!(summary["export"]["format"], "ndjson");
    let ndjson = std::fs::read_to_string(&ndjson_path).unwrap();
    assert!(ndjson.contains("redacted-"));
    assert!(!ndjson.contains("ops@example.com"));
    let row: Value = serde_json::from_str(ndjson.lines().next().unwrap()).unwrap();
    assert_eq!(row["payload"]["worker_id"], "worker-a");

    let csv = run_ok(
        home.path(),
        &[
            "audit",
            "export",
            "--format",
            "csv",
            "--since",
            "2026-05-06",
            "--until",
            "2026-05-06",
        ],
    );
    let csv_body = String::from_utf8(csv.stdout).unwrap();
    assert!(csv_body.starts_with("seq,id,session_id"));
    assert!(csv_body.contains("tool_call"));

    let tar_path = out_dir.path().join("audit.tar");
    run_ok(
        home.path(),
        &[
            "audit",
            "export",
            "--format",
            "tarball",
            "--since",
            "2026-05-06",
            "--until",
            "2026-05-06",
            "--bundle-out",
            tar_path.to_str().unwrap(),
        ],
    );
    let mut archive = tar::Archive::new(std::fs::File::open(&tar_path).unwrap());
    let mut names = Vec::new();
    let mut manifest = Value::Null;
    for entry in archive.entries().unwrap() {
        let mut entry = entry.unwrap();
        let name = entry.path().unwrap().to_string_lossy().to_string();
        if name == "manifest.json" {
            let mut body = String::new();
            entry.read_to_string(&mut body).unwrap();
            manifest = serde_json::from_str(&body).unwrap();
        }
        names.push(name);
    }
    assert_eq!(names, vec!["audit.ndjson", "manifest.json", "README.txt"]);
    assert_eq!(manifest["row_count"], 2);
    assert!(manifest["audit_ndjson_sha256"].as_str().unwrap().len() == 64);

    sqlx::query("UPDATE audit_record SET payload_json = ? WHERE seq = 1")
        .bind(r#"{"worker_id":"worker-a","email":"ops@example.com","tampered":true}"#)
        .execute(store.pool())
        .await
        .unwrap();
    let broken = run(
        home.path(),
        &[
            "audit",
            "export",
            "--since",
            "2026-05-06",
            "--until",
            "2026-05-06",
        ],
    );
    assert!(!broken.status.success());
    let stderr = String::from_utf8_lossy(&broken.stderr);
    assert!(stderr.contains("audit chain verification failed before export"));
    assert!(stderr.contains("self_hash"));
}

async fn seed_rows(store: &SessionStore, session: SessionId) {
    let base = DateTime::parse_from_rfc3339("2026-05-06T12:00:00Z")
        .unwrap()
        .timestamp_micros();
    let first = insert_row(
        store,
        session,
        "tool_call",
        json!({"worker_id":"worker-a","email":"ops@example.com","tool":"shell"}),
        base,
        [0_u8; 32],
    )
    .await;
    insert_row(
        store,
        session,
        "tool_result",
        json!({"worker_id":"worker-b","result":"ok"}),
        base + Duration::seconds(1).num_microseconds().unwrap(),
        first,
    )
    .await;
}

async fn insert_row(
    store: &SessionStore,
    session_id: SessionId,
    kind: &str,
    payload: Value,
    created_at: i64,
    parent_hash: [u8; 32],
) -> [u8; 32] {
    let id = Uuid::now_v7();
    let canonical = serde_json::to_string(&payload).unwrap();
    let self_hash = compute_chain_hash(&parent_hash, kind, &canonical);
    sqlx::query(
        "INSERT INTO audit_record (id, session_id, kind, payload_json, parent_hash, self_hash, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id.as_bytes().to_vec())
    .bind(session_id.0.as_bytes().to_vec())
    .bind(kind)
    .bind(canonical)
    .bind(parent_hash.as_slice())
    .bind(self_hash.as_slice())
    .bind(created_at)
    .execute(store.pool())
    .await
    .unwrap();
    self_hash
}

fn run_json(home: &Path, args: &[&str]) -> Value {
    let out = run_ok(home, args);
    serde_json::from_slice(&out.stdout).unwrap()
}

fn run_ok(home: &Path, args: &[&str]) -> Output {
    let out = run(home, args);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    out
}

fn run(home: &Path, args: &[&str]) -> Output {
    Command::new(common::argo_bin())
        .env("ARGO_HOME", home)
        .args(args)
        .output()
        .unwrap()
}
