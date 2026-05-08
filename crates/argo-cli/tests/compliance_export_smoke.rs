//! End-to-end `argo compliance export` smoke test.
//!
//! Seeds two tenants in a fresh `ARGO_HOME`, exports tenant A, asserts
//! that the JSON envelope contains tenant A's rows and excludes tenant
//! B's, and that the export itself was audit-logged.
mod common;

use argo_store::SessionStore;
use argo_types::{TenantId, WorkspaceId};
use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};

#[tokio::test]
async fn compliance_export_isolates_tenants_and_logs_audit() {
    let home = tempfile::tempdir().unwrap();
    let out_dir = tempfile::tempdir().unwrap();
    let store = SessionStore::open(home.path().join("sessions.db"))
        .await
        .unwrap();
    store.migrate().await.unwrap();

    let alice = TenantId::new();
    let bob = TenantId::new();
    seed_workspace(&store, alice, "alice-ws").await;
    seed_workspace(&store, bob, "bob-ws").await;

    let json_path = out_dir.path().join("export.json");
    let out = run_ok(
        home.path(),
        &[
            "compliance",
            "export",
            "--tenant",
            &alice.to_string(),
            "--format",
            "gdpr",
            "--out",
            json_path.to_str().unwrap(),
        ],
    );
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let body = std::fs::read_to_string(&json_path).unwrap();
    let envelope: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(envelope["subject"], alice.to_string());
    assert!(envelope["argo_version"].is_string());
    assert!(envelope["exported_at"].is_string());

    let workspace = envelope["tables"]["workspace"].as_object().unwrap();
    let rows = workspace["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 1, "alice's export should contain exactly 1 workspace row");
    assert_eq!(rows[0]["name"], "alice-ws");
    assert!(
        !body.contains("bob-ws"),
        "bob-ws must NOT appear in alice's export bundle"
    );

    let kind: String =
        sqlx::query_scalar("SELECT kind FROM audit_record ORDER BY seq DESC LIMIT 1")
            .fetch_one(store.pool())
            .await
            .unwrap();
    assert_eq!(kind, "compliance.export");
}

#[tokio::test]
async fn compliance_export_rejects_bad_tenant_id() {
    let home = tempfile::tempdir().unwrap();
    let out = run(
        home.path(),
        &[
            "compliance",
            "export",
            "--tenant",
            "not-a-uuid",
            "--format",
            "gdpr",
        ],
    );
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("compliance export") && stderr.contains("not-a-uuid"),
        "stderr did not mention tenant validation: {stderr}"
    );
}

async fn seed_workspace(
    store: &SessionStore,
    tenant: TenantId,
    name: &str,
) -> WorkspaceId {
    let workspace = WorkspaceId::new();
    sqlx::query(
        "INSERT INTO workspace (id, tenant_id, name, root_path, created_at) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(workspace.0.as_bytes().as_slice())
    .bind(tenant.0.as_bytes().as_slice())
    .bind(name)
    .bind(format!("/tmp/{name}"))
    .bind(1_700_000_000_i64)
    .execute(store.pool())
    .await
    .unwrap();
    workspace
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
