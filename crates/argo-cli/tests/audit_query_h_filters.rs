mod common;

use argo_audit::compute_chain_hash;
use argo_store::SessionStore;
use argo_types::SessionId;
use chrono::{DateTime, Duration};
use serde_json::{json, Value};
use std::path::Path;
use std::process::{Command, Output};
use uuid::Uuid;

#[tokio::test]
async fn audit_query_h_filters_narrow_rows() {
    let home = tempfile::tempdir().unwrap();
    let store = SessionStore::open(home.path().join("sessions.db"))
        .await
        .unwrap();
    store.migrate().await.unwrap();

    let root = home.path().to_string_lossy().to_string();
    let (tenant, workspace) = store
        .ensure_workspace("audit-query-test", &root)
        .await
        .unwrap();
    let session_a = store
        .create_session(tenant, workspace, "mock", "mock", Some("a"), None)
        .await
        .unwrap();
    let session_b = store
        .create_session(tenant, workspace, "mock", "mock", Some("b"), None)
        .await
        .unwrap();
    seed_rows(&store, session_a, session_b).await;

    let approved = run_json(
        home.path(),
        &["audit", "query", "--decision", "auto_approve", "--json"],
    );
    assert_eq!(approved["query"]["decision"], "auto_approve");
    assert_eq!(approved["count"], 1);
    assert_eq!(approved["rows"][0]["kind"], "auto_approval_decision");
    assert_eq!(approved["rows"][0]["payload"]["decision"], "auto_approve");

    let session_a_raw = session_a.to_string();
    let composed = run_json(
        home.path(),
        &[
            "audit",
            "query",
            "--worker",
            "worker-a",
            "--session",
            &session_a_raw,
            "--since",
            "2026-05-06T12:00:01Z",
            "--until",
            "2026-05-06T12:00:03Z",
            "--json",
        ],
    );
    assert_eq!(composed["query"]["worker"], "worker-a");
    assert_eq!(composed["query"]["session"], session_a_raw);
    assert_eq!(composed["query"]["since"], "2026-05-06T12:00:01Z");
    assert_eq!(composed["query"]["until"], "2026-05-06T12:00:03Z");
    assert_eq!(composed["count"], 1);
    assert_eq!(composed["rows"][0]["kind"], "tool_call");

    let escalations = run_json(
        home.path(),
        &[
            "audit",
            "query",
            "--escalation",
            "--worker",
            "worker-a",
            "--json",
        ],
    );
    assert_eq!(escalations["query"]["escalation"], true);
    assert_eq!(escalations["query"]["worker"], "worker-a");
    assert_eq!(escalations["count"], 2);
    let kinds: Vec<&str> = escalations["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["kind"].as_str().unwrap())
        .collect();
    assert!(kinds.contains(&"handoff_emitted"));
    assert!(kinds.contains(&"escalation_exhausted"));

    let limited = run_json(
        home.path(),
        &["audit", "query", "--escalation", "--limit", "2", "--json"],
    );
    assert_eq!(limited["query"]["limit"], 2);
    assert_eq!(limited["count"], 2);

    let invalid = run(
        home.path(),
        &["audit", "query", "--decision", "approve", "--json"],
    );
    assert!(!invalid.status.success());
    let stderr = String::from_utf8_lossy(&invalid.stderr);
    assert!(stderr.contains("invalid decision 'approve'"));
}

async fn seed_rows(store: &SessionStore, session_a: SessionId, session_b: SessionId) {
    let base = DateTime::parse_from_rfc3339("2026-05-06T12:00:00Z")
        .unwrap()
        .timestamp_micros();
    insert_row(
        store,
        Some(session_a),
        "auto_approval_decision",
        json!({"decision":"auto_approve","worker_id":"worker-a"}),
        base,
    )
    .await;
    insert_row(
        store,
        Some(session_a),
        "auto_approval_decision",
        json!({"decision":"ask","worker_id":"worker-b"}),
        base + Duration::seconds(1).num_microseconds().unwrap(),
    )
    .await;
    insert_row(
        store,
        Some(session_a),
        "tool_call",
        json!({"tool":"shell","worker_id":"worker-a"}),
        base + Duration::seconds(2).num_microseconds().unwrap(),
    )
    .await;
    insert_row(
        store,
        Some(session_b),
        "escalation_exhausted",
        json!({"worker_id":"worker-a","reason":"timeout"}),
        base + Duration::seconds(3).num_microseconds().unwrap(),
    )
    .await;
    insert_row(
        store,
        Some(session_b),
        "handoff_emitted",
        json!({"from_worker_id":"worker-b","to":{"kind":"worker","worker_id":"worker-a"}}),
        base + Duration::seconds(4).num_microseconds().unwrap(),
    )
    .await;
    insert_row(
        store,
        Some(session_a),
        "handoff_settled",
        json!({"worker_id":"worker-c","outcome":"done"}),
        base + Duration::seconds(5).num_microseconds().unwrap(),
    )
    .await;
    insert_row(
        store,
        None,
        "turn_start",
        json!({"worker_id":"worker-a"}),
        base + Duration::seconds(6).num_microseconds().unwrap(),
    )
    .await;
}

async fn insert_row(
    store: &SessionStore,
    session_id: Option<SessionId>,
    kind: &str,
    payload: Value,
    created_at: i64,
) {
    let id = Uuid::now_v7();
    let parent_hash = [0_u8; 32];
    let canonical = serde_json::to_string(&payload).unwrap();
    let self_hash = compute_chain_hash(&parent_hash, kind, &canonical);
    sqlx::query(
        "INSERT INTO audit_record (id, session_id, kind, payload_json, parent_hash, self_hash, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id.as_bytes().to_vec())
    .bind(session_id.map(|s| s.0.as_bytes().to_vec()))
    .bind(kind)
    .bind(canonical)
    .bind(parent_hash.as_slice())
    .bind(self_hash.as_slice())
    .bind(created_at)
    .execute(store.pool())
    .await
    .unwrap();
}

fn run_json(home: &Path, args: &[&str]) -> Value {
    let output = run(home, args);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn run(home: &Path, args: &[&str]) -> Output {
    Command::new(common::argo_bin())
        .env("ARGO_HOME", home)
        .args(args)
        .output()
        .unwrap()
}
