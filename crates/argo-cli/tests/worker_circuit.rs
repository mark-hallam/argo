//! Integration lock for `argo worker resume` circuit state.

use argo_memory::{MemoryFile, MemoryStore};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::Arc;

fn argo_bin() -> &'static str {
    env!("CARGO_BIN_EXE_argo")
}

fn verticals_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .join("verticals")
}

fn run(workspace: &Path, home: &Path, args: &[&str]) -> Output {
    Command::new(argo_bin())
        .current_dir(workspace)
        .env("ARGO_HOME", home)
        .env("ARGO_VERTICALS_DIR", verticals_dir())
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("spawn argo {args:?}: {e}"))
}

async fn open_store(home: &Path) -> argo_store::SessionStore {
    let store = argo_store::SessionStore::open(home.join("sessions.db"))
        .await
        .unwrap();
    store.migrate().await.unwrap();
    argo_identity::circuit::ensure_schema(store.pool())
        .await
        .unwrap();
    store
}

#[test]
fn worker_resume_flips_open_circuit_to_half_open() {
    let home = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    let worker = "00000000-0000-0000-0000-000000000001";
    let tenant = "00000000-0000-0000-0000-000000000002";

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let store = open_store(home.path()).await;
        sqlx::query(
            "INSERT INTO worker_circuit_history
             (worker_id, state, failure_count, reason, created_at)
             VALUES (?, 'open', 3, 'three strikes', 1)",
        )
        .bind(worker)
        .execute(store.pool())
        .await
        .unwrap();
    });

    let out = run(
        workspace.path(),
        home.path(),
        &[
            "worker",
            "resume",
            worker,
            "--tenant",
            tenant,
            "--reason",
            "operator probe",
            "--json",
        ],
    );
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["worker_id"], worker);
    assert_eq!(body["circuit_state"]["state"], "half_open");

    rt.block_on(async {
        let store = open_store(home.path()).await;
        let latest: String = sqlx::query_scalar(
            "SELECT state FROM worker_circuit_history
             WHERE worker_id = ? ORDER BY created_at DESC, id DESC LIMIT 1",
        )
        .bind(worker)
        .fetch_one(store.pool())
        .await
        .unwrap();
        assert_eq!(latest, "half_open");
    });
}

#[test]
fn open_circuit_rejects_action_dispatch() {
    let home = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    let worker = "00000000-0000-0000-0000-000000000011";
    let action_id = "act-circuit-open";

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let store = Arc::new(open_store(home.path()).await);
        let name = workspace
            .path()
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("workspace");
        let root_path = std::fs::canonicalize(workspace.path()).unwrap();
        let root = root_path.to_string_lossy();
        let ws = store.ensure_workspace(name, &root).await.unwrap().1;
        let memory = MemoryStore::new(Arc::clone(&store));
        let body = serde_json::json!({
            "schema_key": "worker_action",
            "action_id": action_id,
            "job_id": "job-circuit-test",
            "worker_id": worker,
            "item_id": null,
            "action_type": "verify_price",
            "risk_level": "low",
            "requires_approval": false,
            "state": "pending",
            "payload": {"worker_id": worker},
            "created_at": chrono::Utc::now().to_rfc3339(),
        })
        .to_string();
        memory
            .append_for_vertical(ws, MemoryFile::Memory, body, None, "gtm-engineer")
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO worker_circuit_history
             (worker_id, state, failure_count, reason, created_at)
             VALUES (?, 'open', 3, 'three strikes', 1)",
        )
        .bind(worker)
        .execute(store.pool())
        .await
        .unwrap();
    });

    let out = run(
        workspace.path(),
        home.path(),
        &["worker", "actions", "execute", action_id, "--json"],
    );
    assert!(!out.status.success());
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(combined.contains("circuit_open"), "got: {combined}");
}
