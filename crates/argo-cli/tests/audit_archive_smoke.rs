//! End-to-end smoke for `argo audit archive` → `list-archives` →
//! `verify-archive` (#228). Spawns the `argo` binary against a
//! temp `ARGO_HOME`, seeds backdated audit rows, runs the archival
//! pipeline through the CLI, and asserts the tarball + manifest
//! land on disk and round-trip through verify.
mod common;

use argo_audit::compute_chain_hash;
use argo_store::SessionStore;
use argo_types::SessionId;
use chrono::{Duration, Utc};
use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};
use uuid::Uuid;

#[tokio::test]
async fn audit_archive_creates_bundle_and_verifies() {
    let home = tempfile::tempdir().unwrap();
    let store = SessionStore::open(home.path().join("sessions.db"))
        .await
        .unwrap();
    store.migrate().await.unwrap();
    let root = home.path().to_string_lossy().to_string();
    let (tenant, workspace) = store
        .ensure_workspace("audit-archive", &root)
        .await
        .unwrap();
    let session = store
        .create_session(tenant, workspace, "mock", "mock", Some("archive"), None)
        .await
        .unwrap();
    seed_old_rows(&store, session, 3).await;

    let archive_dir = home.path().join("audit-archive");
    let summary = run_json(
        home.path(),
        &[
            "audit",
            "archive",
            "--retention-days",
            "90",
            "--archive-dir",
            archive_dir.to_str().unwrap(),
            "--json",
        ],
    );
    assert_eq!(summary["archive"]["ok"], true);
    assert_eq!(summary["archive"]["row_count"], 3);
    assert_eq!(summary["archive"]["pruned"], false);
    let filename = summary["archive"]["filename"].as_str().unwrap();
    let bundle_id = summary["archive"]["id"].as_str().unwrap().to_string();
    assert!(archive_dir.join(filename).exists(), "tarball must exist");
    assert!(
        archive_dir.join("manifest.jsonl").exists(),
        "manifest.jsonl must exist"
    );

    // Second run on the same day with the same cutoff is idempotent.
    let again = run_json(
        home.path(),
        &[
            "audit",
            "archive",
            "--retention-days",
            "90",
            "--archive-dir",
            archive_dir.to_str().unwrap(),
            "--json",
        ],
    );
    assert_eq!(again["archive"]["id"], bundle_id);

    let listed = run_json(
        home.path(),
        &[
            "audit",
            "list-archives",
            "--archive-dir",
            archive_dir.to_str().unwrap(),
            "--json",
        ],
    );
    let archives = listed["archives"].as_array().unwrap();
    assert_eq!(archives.len(), 1, "idempotent: still one bundle");
    assert_eq!(archives[0]["id"], bundle_id);

    let verify = run_json(
        home.path(),
        &[
            "audit",
            "verify-archive",
            &bundle_id,
            "--archive-dir",
            archive_dir.to_str().unwrap(),
            "--json",
        ],
    );
    assert_eq!(verify["verify"]["ok"], true);

    // Tamper: flip a byte in the tarball; verify must fail.
    let path = archive_dir.join(filename);
    let mut bytes = std::fs::read(&path).unwrap();
    let mid = bytes.len() / 2;
    bytes[mid] ^= 0xff;
    std::fs::write(&path, &bytes).unwrap();
    let failed = run(
        home.path(),
        &[
            "audit",
            "verify-archive",
            &bundle_id,
            "--archive-dir",
            archive_dir.to_str().unwrap(),
        ],
    );
    assert!(!failed.status.success(), "tampered bundle must fail verify");
}

#[tokio::test]
async fn audit_archive_prune_respects_hard_keep_days() {
    let home = tempfile::tempdir().unwrap();
    let store = SessionStore::open(home.path().join("sessions.db"))
        .await
        .unwrap();
    store.migrate().await.unwrap();
    let root = home.path().to_string_lossy().to_string();
    let (tenant, workspace) = store.ensure_workspace("hk", &root).await.unwrap();
    let session = store
        .create_session(tenant, workspace, "mock", "mock", Some("hk"), None)
        .await
        .unwrap();
    let now = Utc::now().timestamp_micros();
    let day = Duration::days(1).num_microseconds().unwrap();
    // Three rows: 200d old, 50d old, 5d old.
    seed_at(&store, session, "tool_call", now - day * 200, [0_u8; 32]).await;
    seed_at(&store, session, "tool_call", now - day * 50, [1_u8; 32]).await;
    seed_at(&store, session, "tool_call", now - day * 5, [2_u8; 32]).await;

    let archive_dir = home.path().join("audit-archive");
    // --retention-days=9 would otherwise wipe even the 5-day row;
    // --hard-keep-days=30 must protect it.
    let summary = run_json(
        home.path(),
        &[
            "audit",
            "archive",
            "--retention-days",
            "9",
            "--hard-keep-days",
            "30",
            "--prune",
            "--archive-dir",
            archive_dir.to_str().unwrap(),
            "--json",
        ],
    );
    assert_eq!(summary["archive"]["row_count"], 2);
    assert_eq!(summary["archive"]["pruned"], true);
    let surviving: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM audit_record")
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(surviving, 1, "5-day-old row must survive hard_keep_days=30");
}

async fn seed_old_rows(store: &SessionStore, session: SessionId, n: usize) {
    let now = Utc::now().timestamp_micros();
    let day = Duration::days(1).num_microseconds().unwrap();
    let mut parent = [0_u8; 32];
    for i in 0..n {
        parent = seed_at(
            store,
            session,
            "tool_call",
            now - day * (100 + i as i64),
            parent,
        )
        .await;
    }
}

async fn seed_at(
    store: &SessionStore,
    session_id: SessionId,
    kind: &str,
    created_at: i64,
    parent_hash: [u8; 32],
) -> [u8; 32] {
    let id = Uuid::now_v7();
    let payload = serde_json::json!({"seed": true});
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
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "stdout did not parse as JSON: {e}\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    })
}

fn run_ok(home: &Path, args: &[&str]) -> Output {
    let out = run(home, args);
    assert!(
        out.status.success(),
        "argo {} failed\nstderr: {}",
        args.join(" "),
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
