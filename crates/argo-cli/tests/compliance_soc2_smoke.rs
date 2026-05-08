//! Integration smoke for `argo compliance soc2`.
//!
//! Seeds a SQLite store with one event in each SOC2 category plus an
//! unmapped action, runs the JSON path of the CLI, and asserts the
//! per-category aggregates match.
mod common;

use argo_audit::compute_chain_hash;
use argo_store::SessionStore;
use chrono::{DateTime, Duration, Utc};
use serde_json::{json, Value};
use std::path::Path;
use std::process::{Command, Output};
use uuid::Uuid;

#[tokio::test]
async fn soc2_report_aggregates_categories_via_cli() {
    let home = tempfile::tempdir().unwrap();
    let store = SessionStore::open(home.path().join("sessions.db"))
        .await
        .unwrap();
    store.migrate().await.unwrap();

    let base = DateTime::parse_from_rfc3339("2026-05-08T12:00:00Z")
        .unwrap()
        .with_timezone(&Utc)
        .timestamp_micros();
    let mut prev = [0_u8; 32];
    let seeds: &[(&str, Value)] = &[
        ("auth.login", json!({"actor":"alice"})),
        (
            "worker.deploy",
            json!({"actor":"bob","tenant_id":"t-1","target":"worker-x"}),
        ),
        ("compliance.export", json!({"actor":"carol"})),
        ("policy.update", json!({"actor":"dave"})),
        ("weird.unmapped", json!({})),
    ];
    for (i, (kind, payload)) in seeds.iter().enumerate() {
        let ts = base + Duration::seconds(i as i64).num_microseconds().unwrap();
        prev = insert_row(&store, kind, payload.clone(), ts, prev).await;
    }

    let from = "2026-05-08T00:00:00Z";
    let to = "2026-05-09T00:00:00Z";
    let out = run_ok(
        home.path(),
        &[
            "compliance", "soc2", "--from", from, "--to", to, "--format", "json",
        ],
    );
    let body = String::from_utf8(out.stdout).unwrap();
    let value: Value = serde_json::from_str(&body).expect("valid json");

    assert_eq!(value["window"]["from"], from);
    assert_eq!(value["window"]["to"], to);
    assert_eq!(value["events"].as_array().unwrap().len(), 5);
    assert_eq!(value["categories"]["Access"]["count"], 1);
    assert_eq!(value["categories"]["ChangeMgmt"]["count"], 1);
    assert_eq!(value["categories"]["DataMgmt"]["count"], 1);
    assert_eq!(value["categories"]["Security"]["count"], 1);
    assert_eq!(value["categories"]["Other"]["count"], 1);

    // Action filter narrows the output to one row.
    let narrow = run_ok(
        home.path(),
        &[
            "compliance",
            "soc2",
            "--from",
            from,
            "--to",
            to,
            "--format",
            "json",
            "--action",
            "auth.login",
        ],
    );
    let narrow: Value = serde_json::from_slice(&narrow.stdout).unwrap();
    let events = narrow["events"].as_array().unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["action"], "auth.login");
    assert_eq!(events[0]["category"], "Access");

    // Markdown is the default.
    let md = run_ok(home.path(), &["compliance", "soc2", "--from", from, "--to", to]);
    let md = String::from_utf8(md.stdout).unwrap();
    assert!(md.starts_with("# SOC2 audit trail report"));
    assert!(md.contains("## Access"));
    assert!(md.contains("auth.login"));
}

async fn insert_row(
    store: &SessionStore,
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
         VALUES (?, NULL, ?, ?, ?, ?, ?)",
    )
    .bind(id.as_bytes().to_vec())
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

fn run_ok(home: &Path, args: &[&str]) -> Output {
    let out = Command::new(common::argo_bin())
        .env("ARGO_HOME", home)
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "argo {args:?} failed; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    out
}
