//! Claim 6 row-4 production pin: gateway refusal emits an
//! `approval.declined` audit row with tenant-stamped payload.

mod common;

use argo_memory::{MemoryFile, MemoryStore};
use argo_store::SessionStore;
use common::{body_of, listener_signal::await_listener_signal, pick_free_port};
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;

const SMOKE_DB_KEY_B64: &str = "QkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkI=";
const VERTICAL_ID: &str = "gtm-engineer";
const PRINCIPAL: &str = "human:operator@example.test";

#[test]
fn approval_declined_gateway_e2e_contracts() {
    let _gateway_lock = common::gateway_lock_guard();
    std::env::set_var("SKYCOT_DB_KEY_BASE64", SMOKE_DB_KEY_B64);

    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        refusal_emits_approval_declined_payload().await;
        approve_does_not_emit_approval_declined().await;
        cross_tenant_refuse_attempt_emits_no_declined_row().await;
    });
}

async fn refusal_emits_approval_declined_payload() {
    let home = tempfile::tempdir().expect("home");
    let workspace = tempfile::tempdir().expect("workspace");
    let store = open_store(home.path()).await;
    let tenant = seed_action(
        &store,
        workspace.path(),
        "approval-refuse",
        "delete the Q4 customer export",
    )
    .await;

    let port = pick_free_port();
    let mut child = spawn_gateway(home.path(), workspace.path(), port);
    let resp = http_post_principal(
        port,
        "/api/v1/approvals/approval-refuse/decide",
        r#"{"decision":"refuse"}"#,
    );
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp: {resp}");
    let body: Value = serde_json::from_str(body_of(&resp)).expect("json body");
    assert_eq!(body["decision"], "refuse");
    let _ = child.wait();

    let rows = declined_rows(&store).await;
    assert_eq!(rows.len(), 1, "expected one declined row: {rows:?}");
    let row = &rows[0];
    assert_eq!(row.tenant_id, tenant);
    assert_eq!(row.payload["tenant_id"], tenant);
    assert_eq!(row.payload["approval_id"], "approval-refuse");
    assert_eq!(row.payload["decided_by"], PRINCIPAL);
    assert_eq!(row.payload["decision"], "refuse");
    assert!(
        row.payload["decided_at"]
            .as_str()
            .is_some_and(|s| s.contains('T')),
        "decided_at must be RFC3339-ish: {:?}",
        row.payload["decided_at"]
    );
    assert_eq!(
        row.payload["request_summary"],
        "delete the Q4 customer export"
    );
}

async fn approve_does_not_emit_approval_declined() {
    let home = tempfile::tempdir().expect("home");
    let workspace = tempfile::tempdir().expect("workspace");
    let store = open_store(home.path()).await;
    seed_action(
        &store,
        workspace.path(),
        "approval-approve",
        "publish the safe changelog",
    )
    .await;

    let port = pick_free_port();
    let mut child = spawn_gateway(home.path(), workspace.path(), port);
    let resp = http_post_principal(
        port,
        "/api/v1/approvals/approval-approve/decide",
        r#"{"decision":"approve"}"#,
    );
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp: {resp}");
    let _ = child.wait();

    assert!(
        declined_rows(&store).await.is_empty(),
        "approve path must not emit approval.declined"
    );
}

async fn cross_tenant_refuse_attempt_emits_no_declined_row() {
    let home = tempfile::tempdir().expect("home");
    let tenant_a_workspace = tempfile::tempdir().expect("tenant a workspace");
    let tenant_b_workspace = tempfile::tempdir().expect("tenant b workspace");
    let store = open_store(home.path()).await;
    let tenant_a = seed_action(
        &store,
        tenant_a_workspace.path(),
        "tenant-a-approval",
        "private tenant A request",
    )
    .await;
    let tenant_b = ensure_workspace(&store, tenant_b_workspace.path()).await;
    assert_ne!(tenant_a, tenant_b, "test must use distinct tenants");

    let port = pick_free_port();
    let mut child = spawn_gateway(home.path(), tenant_b_workspace.path(), port);
    let resp = http_post_principal(
        port,
        "/api/v1/approvals/tenant-a-approval/decide",
        r#"{"decision":"refuse"}"#,
    );
    assert!(resp.contains("HTTP/1.1 404 Not Found"), "resp: {resp}");
    let _ = child.wait();

    assert!(
        declined_rows(&store).await.is_empty(),
        "cross-tenant miss must not leak an approval.declined audit row"
    );
}

async fn open_store(home: &Path) -> Arc<SessionStore> {
    let store = SessionStore::open(home.join("sessions.db"))
        .await
        .expect("open store");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

async fn ensure_workspace(store: &SessionStore, workspace: &Path) -> String {
    let (name, root) = workspace_name_and_root(workspace);
    let (tenant, _workspace) = store
        .ensure_workspace(&name, &root)
        .await
        .expect("workspace");
    tenant.to_string()
}

async fn seed_action(
    store: &Arc<SessionStore>,
    workspace: &Path,
    action_id: &str,
    summary: &str,
) -> String {
    let (name, root) = workspace_name_and_root(workspace);
    let (tenant, workspace_id) = store
        .ensure_workspace(&name, &root)
        .await
        .expect("workspace");
    let memory = MemoryStore::new(Arc::clone(store));
    let record = json!({
        "schema_key": "worker_action",
        "action_id": action_id,
        "job_id": "claim-6-row-4",
        "worker_id": "00000000-0000-0000-0000-000000000000",
        "item_id": Value::Null,
        "action_type": "post_listing",
        "risk_level": "high",
        "requires_approval": true,
        "state": "pending",
        "payload": { "summary": summary },
        "approved_by": Value::Null,
        "approved_at": Value::Null,
        "executed_at": Value::Null,
        "platform": Value::Null,
        "result": Value::Null,
        "evidence": [],
        "blocked_reason": Value::Null,
        "created_at": chrono::Utc::now().to_rfc3339(),
    });
    memory
        .append_for_vertical(
            workspace_id,
            MemoryFile::Memory,
            record.to_string(),
            None,
            VERTICAL_ID,
        )
        .await
        .expect("append action");
    tenant.to_string()
}

fn workspace_name_and_root(workspace: &Path) -> (String, String) {
    let root_path = workspace
        .canonicalize()
        .unwrap_or_else(|_| workspace.to_path_buf());
    let name = root_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace")
        .to_string();
    (name, root_path.to_string_lossy().into_owned())
}

#[derive(Debug)]
struct DeclinedRow {
    tenant_id: String,
    payload: Value,
}

async fn declined_rows(store: &SessionStore) -> Vec<DeclinedRow> {
    let rows = sqlx::query_as::<_, (String, String)>(
        "SELECT tenant_id, payload_json FROM audit_record \
         WHERE kind = 'approval.declined' ORDER BY seq ASC",
    )
    .fetch_all(store.pool())
    .await
    .expect("select declined rows");
    rows.into_iter()
        .map(|(tenant_id, payload_json)| DeclinedRow {
            tenant_id,
            payload: serde_json::from_str(&payload_json).expect("payload json"),
        })
        .collect()
}

fn spawn_gateway(home: &Path, workspace: &Path, port: u16) -> Child {
    let mut child = Command::new(common::argo_bin())
        .current_dir(workspace)
        .env("ARGO_HOME", home)
        .env("ARGO_VERTICALS_DIR", workspace_root().join("verticals"))
        .env("SKYCOT_DB_KEY_BASE64", SMOKE_DB_KEY_B64)
        .env("OPENAI_API_KEY", "test-key")
        .args([
            "gateway",
            "start",
            "--port",
            &port.to_string(),
            "--max-requests",
            "1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn gateway");
    let stderr = child.stderr.take().expect("stderr piped");
    let _drain = await_listener_signal(stderr).expect("gateway listener signal");
    child
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
}

fn http_post_principal(port: u16, path: &str, body: &str) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    let req = format!(
        "POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\n\
         X-Argo-Principal: {PRINCIPAL}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(req.as_bytes()).expect("write");
    let mut resp = String::new();
    stream.read_to_string(&mut resp).expect("read");
    resp
}
