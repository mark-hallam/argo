//! End-to-end `argo compliance ropa` smoke test.
//!
//! Seeds two tenants, exports tenant A's Article 30 ROPA document,
//! and verifies the JSON shape, tenant isolation, and audit row.
mod common;

use argo_store::SessionStore;
use argo_types::{TenantId, WorkspaceId};
use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};

#[tokio::test]
async fn compliance_ropa_exports_article_30_shape_and_logs_audit() {
    let home = tempfile::tempdir().unwrap();
    let out_dir = tempfile::tempdir().unwrap();
    let store = SessionStore::open(home.path().join("sessions.db"))
        .await
        .unwrap();
    store.migrate().await.unwrap();

    let alice = TenantId::new();
    let bob = TenantId::new();
    seed_workspace(&store, alice, "alice-ropa-ws").await;
    seed_workspace(&store, bob, "bob-ropa-ws").await;

    let json_path = out_dir.path().join("ropa.json");
    let out = run_ok(
        home.path(),
        &[
            "compliance",
            "ropa",
            "--tenant",
            &alice.to_string(),
            "--output",
            json_path.to_str().unwrap(),
        ],
    );
    assert!(String::from_utf8(out.stdout)
        .unwrap()
        .contains("Article 30 ROPA export"));

    let body = std::fs::read_to_string(&json_path).unwrap();
    let report: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(report["schema"], "argo.compliance.ropa.v1");
    assert_eq!(report["article"], "GDPR Article 30");
    assert_eq!(report["tenant_id"], alice.to_string());
    assert_eq!(report["exported_by"], "ropa-tester");
    assert!(report["generated_at"].is_string());
    assert_eq!(report["controller"]["role"], "controller");
    assert_eq!(report["processor"]["role"], "processor");
    assert_eq!(report["records"][0]["name"], "Delegated AI operations");

    let workspaces = report["source_inventory"]["workspaces"].as_array().unwrap();
    assert_eq!(workspaces.len(), 1);
    assert_eq!(workspaces[0]["name"], "alice-ropa-ws");
    assert!(
        !body.contains("bob-ropa-ws"),
        "bob workspace must not appear in alice ROPA export"
    );

    let (kind, payload_json, tenant_id): (String, String, String) = sqlx::query_as(
        "SELECT kind, payload_json, tenant_id FROM audit_record ORDER BY seq DESC LIMIT 1",
    )
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert_eq!(kind, "compliance.ropa.exported");
    assert_eq!(tenant_id, alice.to_string());

    let payload: Value = serde_json::from_str(&payload_json).unwrap();
    assert_eq!(payload["tenant_id"], alice.to_string());
    assert_eq!(payload["exported_by"], "ropa-tester");
    let hash = payload["output_path_hash"].as_str().unwrap();
    assert_eq!(hash.len(), 64);
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    assert!(payload.get("output_path").is_none());
}

async fn seed_workspace(store: &SessionStore, tenant: TenantId, name: &str) -> WorkspaceId {
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
    let out = Command::new(common::argo_bin())
        .env("ARGO_HOME", home)
        .env("ARGO_PRINCIPAL", "ropa-tester")
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    out
}
