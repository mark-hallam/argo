mod common;

use argo_store::SessionStore;
use common::argo_bin;
use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};
use uuid::Uuid;

#[tokio::test]
async fn tenant_inspection_and_id_rename_work_end_to_end() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    write_channel_manifest(work.path(), "email");

    let tenant = create_tenant(home.path(), work.path(), "Acme Active");
    let archived = create_tenant(home.path(), work.path(), "Acme Archived");
    assert_success(&run(
        home.path(),
        work.path(),
        &["tenant", "archive", &archived, "--json"],
    ));
    seed_tenant_rows(home.path(), &tenant).await;

    let visible = json(run(home.path(), work.path(), &["tenant", "list", "--json"]));
    assert_eq!(visible["ok"], true);
    assert!(has_tenant(&visible, &tenant));
    assert!(!has_tenant(&visible, &archived));

    let all = json(run(
        home.path(),
        work.path(),
        &["tenant", "list", "--include-archived", "--json"],
    ));
    assert!(has_tenant(&all, &tenant));
    assert!(has_tenant(&all, &archived));

    let shown = json(run(
        home.path(),
        work.path(),
        &["tenant", "show", &tenant, "--json"],
    ));
    assert_eq!(shown["ok"], true);
    assert_eq!(shown["tenant"]["summary"]["id"], tenant);
    assert_eq!(shown["tenant"]["summary"]["worker_count"], 1);
    assert_eq!(shown["tenant"]["recent_activity"][0]["kind"], "tenant_seed");
    assert_eq!(shown["tenant"]["channels"][0]["manifest_present"], true);
    assert!(shown["tenant"]["classification_rules"].as_array().unwrap().len() >= 6);

    let new_id = Uuid::now_v7().to_string();
    let renamed = json(run(
        home.path(),
        work.path(),
        &["tenant", "rename", &tenant, &new_id, "--json"],
    ));
    assert_eq!(renamed["ok"], true);
    assert_eq!(renamed["tenant_rename"]["old_id"], tenant);
    assert_eq!(renamed["tenant_rename"]["new_id"], new_id);

    assert_renamed_rows(home.path(), &tenant, &new_id).await;
}

async fn seed_tenant_rows(home: &Path, tenant: &str) {
    let store = SessionStore::open(home.join("sessions.db")).await.unwrap();
    store.migrate().await.unwrap();
    let tenant_id = Uuid::parse_str(tenant).unwrap();
    let workspace_id: Vec<u8> =
        sqlx::query_scalar("SELECT id FROM workspace WHERE tenant_id = ? LIMIT 1")
            .bind(tenant_id.as_bytes().as_slice())
            .fetch_one(store.pool())
            .await
            .unwrap();
    let session_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO worker
         (id, tenant_id, vertical_id, persona_id, role_id, memory_scope,
          auto_approval_json, channels_json, created_at)
         VALUES ('w-inspect', ?, 'gtm', 'p', 'r', 'worker_self', '{}',
                 '[{\"channel\":\"email\",\"target\":\"ops@example.test\"}]', 20)",
    )
    .bind(tenant)
    .execute(store.pool())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO session (id, tenant_id, workspace_id, title, started_at, model, provider)
         VALUES (?, ?, ?, 'seed', 30, 'm', 'p')",
    )
    .bind(session_id.as_bytes().as_slice())
    .bind(tenant_id.as_bytes().as_slice())
    .bind(workspace_id)
    .execute(store.pool())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO spend_ledger
         (tenant_id, session_id, charged_at_micros, amount_usd_micros, provider, model)
         VALUES (?, ?, ?, 1200000, 'p', 'm')",
    )
    .bind(tenant_id.as_bytes().as_slice())
    .bind(session_id.as_bytes().as_slice())
    .bind(chrono::Utc::now().timestamp_micros())
    .execute(store.pool())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO audit_record
         (id, session_id, kind, payload_json, parent_hash, self_hash, created_at)
         VALUES (?, ?, 'tenant_seed', ?, ?, ?, ?)",
    )
    .bind(Uuid::now_v7().as_bytes().as_slice())
    .bind(session_id.as_bytes().as_slice())
    .bind(format!(r#"{{"tenant_id":"{tenant}","event":"seed"}}"#))
    .bind([0_u8; 32].as_slice())
    .bind([1_u8; 32].as_slice())
    .bind(chrono::Utc::now().timestamp_micros())
    .execute(store.pool())
    .await
    .unwrap();
}

async fn assert_renamed_rows(home: &Path, old: &str, new_id: &str) {
    let store = SessionStore::open(home.join("sessions.db")).await.unwrap();
    let old_uuid = Uuid::parse_str(old).unwrap();
    let new_uuid = Uuid::parse_str(new_id).unwrap();
    let old_workspace: i64 = count_blob(&store, "workspace", old_uuid).await;
    let new_workspace: i64 = count_blob(&store, "workspace", new_uuid).await;
    let old_spend: i64 = count_blob(&store, "spend_ledger", old_uuid).await;
    let new_spend: i64 = count_blob(&store, "spend_ledger", new_uuid).await;
    let old_worker: i64 = count_text(&store, "worker", old).await;
    let new_worker: i64 = count_text(&store, "worker", new_id).await;
    let payload: String = sqlx::query_scalar("SELECT payload_json FROM audit_record LIMIT 1")
        .fetch_one(store.pool())
        .await
        .unwrap();

    assert_eq!(old_workspace, 0);
    assert_eq!(new_workspace, 1);
    assert_eq!(old_spend, 0);
    assert_eq!(new_spend, 1);
    assert_eq!(old_worker, 0);
    assert_eq!(new_worker, 1);
    assert!(!payload.contains(old));
    assert!(payload.contains(new_id));
}

async fn count_blob(store: &SessionStore, table: &str, tenant: Uuid) -> i64 {
    let sql = format!("SELECT COUNT(*) FROM {table} WHERE tenant_id = ?");
    sqlx::query_scalar(&sql)
        .bind(tenant.as_bytes().as_slice())
        .fetch_one(store.pool())
        .await
        .unwrap()
}

async fn count_text(store: &SessionStore, table: &str, tenant: &str) -> i64 {
    let sql = format!("SELECT COUNT(*) FROM {table} WHERE tenant_id = ?");
    sqlx::query_scalar(&sql)
        .bind(tenant)
        .fetch_one(store.pool())
        .await
        .unwrap()
}

fn create_tenant(home: &Path, cwd: &Path, name: &str) -> String {
    let out = run(home, cwd, &["tenant", "create", name, "--json"]);
    assert_success(&out);
    json(out)["id"].as_str().unwrap().to_string()
}

fn has_tenant(list: &Value, tenant: &str) -> bool {
    list["tenants"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["id"] == tenant)
}

fn write_channel_manifest(cwd: &Path, channel: &str) {
    let path = cwd
        .join("manifests")
        .join("channels")
        .join(format!("{channel}.toml"));
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, format!("[channel]\nid = \"{channel}\"\n")).unwrap();
}

fn json(output: Output) -> Value {
    assert_success(&output);
    serde_json::from_slice(&output.stdout).unwrap()
}

fn run(home: &Path, cwd: &Path, args: &[&str]) -> Output {
    Command::new(argo_bin())
        .env("ARGO_HOME", home)
        .current_dir(cwd)
        .args(args)
        .output()
        .unwrap()
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "status: {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
