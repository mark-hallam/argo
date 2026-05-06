mod common;

use argo_audit::{compute_chain_hash, verify_all};
use argo_store::SessionStore;
use argo_types::{MessageRole, SessionId};
use serde_json::Value;
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Output, Stdio};
use uuid::Uuid;

const SUBJECT: &str = "alice@example.com";

#[tokio::test]
async fn gdpr_delete_dry_run_confirm_bundle_and_redacts_all_surfaces() {
    let home = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    let bundle = home.path().join("gdpr.tar");
    let store = SessionStore::open(home.path().join("sessions.db"))
        .await
        .unwrap();
    store.migrate().await.unwrap();
    let (tenant, workspace_id) = store
        .ensure_workspace("gdpr-smoke", workspace.path().to_str().unwrap())
        .await
        .unwrap();
    let session = store
        .create_session(tenant, workspace_id, "mock", "mock", Some(SUBJECT), None)
        .await
        .unwrap();
    store
        .append_message(
            session,
            MessageRole::User,
            r#"[{"type":"text","text":"alice@example.com asked for erasure"}]"#,
        )
        .await
        .unwrap();
    seed_memory(&store, workspace_id).await;
    seed_skill_candidate(&store).await;
    seed_worker(&store).await;
    seed_audit(&store, session).await;

    let dry = run_json(
        workspace.path(),
        home.path(),
        &["gdpr", "delete", "--subject", SUBJECT, "--json"],
    );
    assert_eq!(dry["gdpr_delete"]["dry_run"], true);
    assert_eq!(dry["gdpr_delete"]["counts"]["memory_rows"], 1);
    assert_eq!(dry["gdpr_delete"]["counts"]["audit_records"], 1);
    assert_eq!(dry["gdpr_delete"]["counts"]["skill_candidates"], 1);
    assert_eq!(dry["gdpr_delete"]["counts"]["notify_channels"], 1);
    assert_eq!(dry["gdpr_delete"]["counts"]["worker_operator_refs"], 1);
    assert_eq!(dry["gdpr_delete"]["counts"]["session_payloads"], 2);

    let out = run_with_stdin(
        workspace.path(),
        home.path(),
        &[
            "gdpr",
            "delete",
            "--subject",
            SUBJECT,
            "--confirm",
            "--bundle-out",
            bundle.to_str().unwrap(),
            "--json",
        ],
        "alice@example.com\n",
    );
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let done: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(done["gdpr_delete"]["dry_run"], false);
    assert_eq!(done["gdpr_delete"]["audit_event_kind"], "gdpr_subject_erasure");
    assert_tar(&bundle);

    let leaked: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_record WHERE instr(lower(payload_json), ?) > 0",
    )
    .bind(SUBJECT)
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert_eq!(leaked, 0);
    let memory_left: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM memory_entry")
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(memory_left, 0);
    let skill_left: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM skill")
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(skill_left, 0);
    let worker: (String, String) =
        sqlx::query_as("SELECT channels_json, auto_approval_json FROM worker WHERE id = 'w1'")
            .fetch_one(store.pool())
            .await
            .unwrap();
    assert!(!worker.0.contains(SUBJECT));
    assert!(!worker.1.contains(SUBJECT));
    let message: String = sqlx::query_scalar("SELECT content_json FROM message LIMIT 1")
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert!(!message.contains(SUBJECT));
    let event: String = sqlx::query_scalar(
        "SELECT payload_json FROM audit_record WHERE session_id IS NULL ORDER BY seq DESC LIMIT 1",
    )
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert!(event.contains("gdpr_subject_erasure"));
    assert!(verify_all(&store).await.unwrap().ok);
}

async fn seed_memory(store: &SessionStore, workspace_id: argo_types::WorkspaceId) {
    sqlx::query(
        "INSERT INTO memory_entry (id, workspace_id, file, body, learned_at, hash)
         VALUES (?, ?, 'MEMORY.md', ?, 1, ?)",
    )
    .bind(Uuid::now_v7().as_bytes().to_vec())
    .bind(workspace_id.0.as_bytes().to_vec())
    .bind(r#"{"author":"alice@example.com","note":"delete me"}"#)
    .bind(vec![1_u8; 32])
    .execute(store.pool())
    .await
    .unwrap();
}

async fn seed_skill_candidate(store: &SessionStore) {
    sqlx::query(
        "INSERT INTO skill (id, name, version, body, frontmatter, source, hash, created_at, lifecycle_state)
         VALUES (?, 'candidate', '0.1', ?, '{}', 'test', ?, 1, 'pending_review')",
    )
    .bind(Uuid::now_v7().as_bytes().to_vec())
    .bind(r#"{"proposed_by":"alice@example.com","schema_key":"learned_skill_candidates"}"#)
    .bind(vec![2_u8; 32])
    .execute(store.pool())
    .await
    .unwrap();
}

async fn seed_worker(store: &SessionStore) {
    sqlx::query(
        "CREATE TABLE worker (id TEXT PRIMARY KEY, auto_approval_json TEXT NOT NULL, channels_json TEXT NOT NULL)",
    )
    .execute(store.pool())
    .await
    .unwrap();
    sqlx::query("INSERT INTO worker VALUES ('w1', ?, ?)")
        .bind(r#"{"operator_email":"alice@example.com"}"#)
        .bind(r#"[{"channel":"email","target":"alice@example.com"},{"channel":"slack","target":"C1"}]"#)
        .execute(store.pool())
        .await
        .unwrap();
}

async fn seed_audit(store: &SessionStore, session: SessionId) {
    let payload = serde_json::json!({"email": SUBJECT, "tool": "erase"});
    let canonical = serde_json::to_string(&payload).unwrap();
    let parent = [0_u8; 32];
    let self_hash = compute_chain_hash(&parent, "tool_call", &canonical);
    sqlx::query(
        "INSERT INTO audit_record (id, session_id, kind, payload_json, parent_hash, self_hash, created_at)
         VALUES (?, ?, 'tool_call', ?, ?, ?, 1)",
    )
    .bind(Uuid::now_v7().as_bytes().to_vec())
    .bind(session.0.as_bytes().to_vec())
    .bind(canonical)
    .bind(parent.as_slice())
    .bind(self_hash.as_slice())
    .execute(store.pool())
    .await
    .unwrap();
}

fn run_json(cwd: &Path, home: &Path, args: &[&str]) -> Value {
    let out = Command::new(common::argo_bin())
        .current_dir(cwd)
        .env("ARGO_HOME", home)
        .args(args)
        .output()
        .unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    serde_json::from_slice(&out.stdout).unwrap()
}

fn run_with_stdin(cwd: &Path, home: &Path, args: &[&str], stdin: &str) -> Output {
    let mut child = Command::new(common::argo_bin())
        .current_dir(cwd)
        .env("ARGO_HOME", home)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.as_mut().unwrap().write_all(stdin.as_bytes()).unwrap();
    child.wait_with_output().unwrap()
}

fn assert_tar(path: &Path) {
    let mut names = Vec::new();
    let mut archive = tar::Archive::new(std::fs::File::open(path).unwrap());
    for entry in archive.entries().unwrap() {
        let mut entry = entry.unwrap();
        let mut ignored = String::new();
        let _ = entry.read_to_string(&mut ignored);
        names.push(entry.path().unwrap().to_string_lossy().to_string());
    }
    assert_eq!(names, vec!["manifest.json", "records.ndjson", "README.txt"]);
}
