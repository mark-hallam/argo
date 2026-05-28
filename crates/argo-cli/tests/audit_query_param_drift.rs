//! Contract test for the audit-feed query-param vocabulary.
//!
//! Pins the gateway behavior for the three drift cases that motivated
//! `fix-audit-filter-key-action-type-vs-kind-vocab-alignment`:
//!
//! - `?kind=<X>` is an alias for `?action_type=<X>` — both filter
//!   the audit feed by `action_type` and return the same rows.
//! - An unknown query parameter (e.g. `?bogus=…`) is rejected with
//!   `400` and `{ error: { code: "unknown_query_param", … } }`. The
//!   silent-ignore behavior that let the v1.0 demo runbook drift
//!   stays gone.
//! - The list-endpoint response shape (`items` array; `action_type`
//!   field per row) is unchanged.

mod common;

use argo_audit::compute_chain_hash;
use argo_store::wal_tuning::WalTuningProfile;
use argo_store::SessionStore;
use common::listener_signal::await_listener_signal;
use common::{argo_bin, body_of, gateway_lock_guard, http_get, pick_free_port};
use serde_json::{json, Value};
use std::process::{Command, Stdio};
use uuid::Uuid;

const SMOKE_DB_KEY_B64: &str = "QkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkI=";
const TENANT_ID: &str = "ws-default";
const DUTY_KIND: &str = "worker_duty.fired";
const TOOL_KIND: &str = "tool_call";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn audit_query_param_drift_pins_kind_alias_and_strict_mode() {
    let _gateway_lock = gateway_lock_guard();
    let port = pick_free_port();
    let home = tempfile::tempdir().expect("ARGO_HOME");
    let store = open_store(home.path()).await;
    let duty_id = insert_row(&store, DUTY_KIND, "fired the schedule").await;
    let tool_id = insert_row(&store, TOOL_KIND, "ran a shell tool").await;
    drop(store);

    let mut child = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("SKYCOT_DB_KEY_BASE64", SMOKE_DB_KEY_B64)
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
        .expect("spawn gateway");
    let stderr = child.stderr.take().expect("stderr piped");
    let _drain = await_listener_signal(stderr).expect("gateway listener signal");

    let action_type_resp = http_get(
        port,
        &format!("/api/v1/audit?action_type={DUTY_KIND}&limit=10"),
    );
    let kind_resp = http_get(port, &format!("/api/v1/audit?kind={DUTY_KIND}&limit=10"));
    let bogus_resp = http_get(port, "/api/v1/audit?bogus=x&limit=10");

    let status = child.wait().expect("wait gateway");
    assert!(status.success(), "gateway exit: {status}");

    // Both wire names must return the same row set — the duty row only,
    // not the tool row.
    let action_type_ids = parse_ids(&action_type_resp);
    let kind_ids = parse_ids(&kind_resp);
    assert_eq!(
        action_type_ids, kind_ids,
        "?kind= and ?action_type= must return the same rows; \
         action_type={action_type_ids:?} kind={kind_ids:?}"
    );
    assert!(
        action_type_ids.contains(&duty_id),
        "expected duty row {duty_id} in ?action_type= response: {action_type_resp}"
    );
    assert!(
        !action_type_ids.contains(&tool_id),
        "tool row {tool_id} must not surface when filtering by {DUTY_KIND}: {action_type_resp}"
    );

    // Unknown param → 400 unknown_query_param. No silent-ignore.
    assert!(
        bogus_resp.contains("HTTP/1.1 400 Bad Request"),
        "bogus param must 400: {bogus_resp}"
    );
    let bogus_body: Value =
        serde_json::from_str(body_of(&bogus_resp)).expect("error envelope is JSON");
    assert_eq!(bogus_body["error"]["code"], "unknown_query_param");
    assert!(
        bogus_body["error"]["message"]
            .as_str()
            .is_some_and(|m| m.contains("bogus")),
        "error message must name the offending key: {bogus_body}"
    );
}

async fn open_store(home: &std::path::Path) -> SessionStore {
    let hex = argo_store::encryption::base64_to_hex(SMOKE_DB_KEY_B64).expect("hex key");
    let store = SessionStore::open_with_hex_key(
        home.join("sessions.db"),
        WalTuningProfile::default(),
        &hex,
    )
    .await
    .expect("open encrypted store");
    store.migrate().await.expect("migrate store");
    store
}

async fn insert_row(store: &SessionStore, kind_label: &str, verb: &str) -> String {
    let id = Uuid::now_v7();
    let id_hex = hex::encode(id.as_bytes());
    let parent_hash = [0x4d_u8; 32];
    let payload = json!({
        "tenant_id": TENANT_ID,
        "worker_id": "wkr_drift_regression",
        "worker_name": "Drift Regression",
        "verb": verb,
        "outcome_status": "succeeded",
        "channel": "web",
        "data_classification": "internal",
    });
    let canonical = serde_json::to_string(&payload).expect("canonical payload");
    let self_hash = compute_chain_hash(&parent_hash, kind_label, &canonical);
    sqlx::query(
        "INSERT INTO audit_record \
         (id, session_id, kind, payload_json, parent_hash, self_hash, created_at, tenant_id) \
         VALUES (?, NULL, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id.as_bytes().to_vec())
    .bind(kind_label)
    .bind(canonical)
    .bind(parent_hash.as_slice())
    .bind(self_hash.as_slice())
    .bind(chrono::Utc::now().timestamp_micros())
    .bind(TENANT_ID)
    .execute(store.pool())
    .await
    .expect("insert drift row");
    id_hex
}

fn parse_ids(resp: &str) -> Vec<String> {
    assert!(
        resp.contains("HTTP/1.1 200 OK"),
        "expected 200 OK, got: {resp}"
    );
    let body: Value = serde_json::from_str(body_of(resp)).unwrap_or_else(|e| {
        panic!("audit feed body parse: {e}; body={}", body_of(resp));
    });
    body["items"]
        .as_array()
        .expect("items array")
        .iter()
        .filter_map(|row| row["id"].as_str().map(str::to_string))
        .collect()
}

