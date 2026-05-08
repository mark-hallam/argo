//! End-to-end lock for `argo worker show` rich introspection.
//! Drives the real `argo` binary against a temp store seeded with a
//! Worker (charter + 1 channel), 1 session, 1 audit row, 1 duty, and
//! 1 budget charge - asserts every brief-required section heading
//! lands in markdown, JSON envelope shape stays stable, and unknown
//! worker_id exits non-zero.

mod common;

use argo_audit::compute_chain_hash;
use argo_identity::{IdentityStore, NotifyChannelRef, Persona, Role, Worker};
use argo_orchestrator::{DutyRecord, DutyStore, OrchestratorStore};
use argo_store::SessionStore;
use argo_types::SessionId;
use chrono::Utc;
use serde_json::{json, Value};
use std::path::Path;
use std::process::{Command, Output};
use std::sync::Arc;
use tempfile::TempDir;
use uuid::Uuid;

#[tokio::test]
async fn worker_show_renders_every_brief_section() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();

    let store = SessionStore::open(home.path().join("sessions.db"))
        .await
        .unwrap();
    store.migrate().await.unwrap();
    let (tenant, ws) = store
        .ensure_workspace("worker-show-smoke", &work.path().to_string_lossy())
        .await
        .unwrap();

    // Seed a worker with charter + 1 channel.
    let identities = IdentityStore::new(&store);
    identities.ensure_schema().await.unwrap();
    let mut worker = Worker::draft(
        tenant,
        "support",
        Persona::named("Maya", "operator@example.test"),
        Role::new("Triage approved support tickets."),
    );
    worker.charter = Some("# Charter\n\nReply within 24h.\n".to_string());
    worker.channels = vec![NotifyChannelRef::new("slack", "C123")];
    identities.create_worker(&worker).await.unwrap();
    let worker_id = worker.id.to_string();
    let tenant_str = tenant.to_string();
    let store_arc = Arc::new(SessionStore::open(home.path().join("sessions.db")).await.unwrap());
    store_arc.migrate().await.unwrap();
    OrchestratorStore::new(Arc::clone(&store_arc))
        .ensure_schema()
        .await
        .unwrap();

    // Seed 1 session.
    let session_id = SessionId::new();
    sqlx::query(
        "INSERT INTO session (id, tenant_id, workspace_id, model, provider, started_at, vertical_id, participants) \
         VALUES (?, ?, ?, 'mock', 'mock', ?, ?, ?)",
    )
    .bind(session_id.0.as_bytes().as_slice())
    .bind(tenant.0.as_bytes().as_slice())
    .bind(ws.0.as_bytes().as_slice())
    .bind(Utc::now().timestamp_micros())
    .bind("support")
    .bind(serde_json::to_string(&vec![worker_id.clone()]).unwrap())
    .execute(store.pool())
    .await
    .unwrap();

    // Seed 1 audit row tagged with this worker_id.
    let audit_id = Uuid::now_v7();
    let parent = [0_u8; 32];
    let payload = json!({"worker_id": worker_id, "decision": "auto_approve"});
    let canonical = serde_json::to_string(&payload).unwrap();
    let self_hash = compute_chain_hash(&parent, "auto_approval_decision", &canonical);
    sqlx::query(
        "INSERT INTO audit_record (id, session_id, kind, payload_json, parent_hash, self_hash, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(audit_id.as_bytes().to_vec())
    .bind(Some(session_id.0.as_bytes().to_vec()))
    .bind("auto_approval_decision")
    .bind(canonical)
    .bind(parent.as_slice())
    .bind(self_hash.as_slice())
    .bind(Utc::now().timestamp_micros())
    .execute(store.pool())
    .await
    .unwrap();

    // Seed 1 duty.
    let duty_store = DutyStore::new(Arc::clone(&store_arc));
    duty_store.ensure_schema().await.unwrap();
    let duty = DutyRecord::new(
        tenant,
        worker.id,
        "0 9 * * 1-5",
        "morning_check",
        "Sweep new tickets",
    );
    duty_store.insert(&duty).await.unwrap();

    // Seed 1 budget charge for spend YTD.
    let budget_schema =
        include_str!("../../argo-store/migrations/14__worker_budget.sql");
    for stmt in budget_schema.split_terminator(';') {
        let sql = stmt.trim();
        if !sql.is_empty() {
            sqlx::query(sql).execute(store.pool()).await.unwrap();
        }
    }
    sqlx::query(
        "INSERT INTO worker_budget (tenant_id, worker_id, action_id, usd_micros, charged_at) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(tenant.0.as_bytes().as_slice())
    .bind(&worker_id)
    .bind(Uuid::now_v7().to_string())
    .bind(7_500_000_i64)
    .bind(Utc::now().timestamp())
    .execute(store.pool())
    .await
    .unwrap();
    drop(store);
    drop(store_arc);

    // Markdown render: every section heading present.
    let md = run(
        &home,
        work.path(),
        &[
            "worker",
            "show",
            &worker_id,
            "--tenant",
            &tenant_str,
        ],
    );
    assert_success(&md);
    let body = String::from_utf8_lossy(&md.stdout);
    for heading in [
        "## Persona",
        "## Role",
        "## Charter",
        "## Channels",
        "## Recent Sessions",
        "## Recent Audit",
        "## Duties",
        "## Spend YTD",
    ] {
        assert!(
            body.contains(heading),
            "missing {heading} in markdown output:\n{body}"
        );
    }
    assert!(body.contains("Reply within 24h"), "charter body missing");
    assert!(body.contains("`slack` -> `C123`"), "channel missing");
    assert!(body.contains("morning_check"), "duty missing");
    assert!(body.contains("$7.50"), "spend YTD missing");
    assert!(body.contains("auto_approval_decision"), "audit row missing");

    // JSON envelope shape stable.
    let json_out = run(
        &home,
        work.path(),
        &[
            "worker",
            "show",
            &worker_id,
            "--tenant",
            &tenant_str,
            "--json",
        ],
    );
    assert_success(&json_out);
    let envelope: Value = serde_json::from_slice(&json_out.stdout).expect("valid json");
    assert_eq!(envelope["ok"], true);
    let report = &envelope["worker_show"];
    assert_eq!(report["worker"]["persona"]["name"], "Maya");
    assert!(report["worker"]["charter"].as_str().unwrap_or("").contains("Reply within 24h"));
    assert_eq!(report["spend_ytd_usd_micros"], 7_500_000);
    assert!(!report["recent_sessions"].as_array().unwrap().is_empty());
    assert!(!report["recent_audit"].as_array().unwrap().is_empty());
    assert!(!report["duties"].as_array().unwrap().is_empty());

    // Unknown worker_id exits non-zero.
    let bogus = Uuid::now_v7().to_string();
    let bad = run(
        &home,
        work.path(),
        &[
            "worker",
            "show",
            &bogus,
            "--tenant",
            &tenant_str,
        ],
    );
    assert!(
        !bad.status.success(),
        "expected exit failure for missing worker, got success\nstdout:\n{}",
        String::from_utf8_lossy(&bad.stdout)
    );
    let stderr = String::from_utf8_lossy(&bad.stderr);
    assert!(
        stderr.to_lowercase().contains("no worker") || stderr.contains(&bogus),
        "unexpected stderr: {stderr}"
    );
}

fn run(home: &TempDir, cwd: &Path, args: &[&str]) -> Output {
    Command::new(common::argo_bin())
        .env("ARGO_HOME", home.path())
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
