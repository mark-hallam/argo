//! End-to-end smoke for `argo handoff-stats` (Stream
//! `codex-handoff-stats-cli`). Seeds two tenants with mixed-status
//! handoffs over a session and asserts the JSON envelope shape for
//! the report and histogram modes.

mod common;

use argo_identity::WorkerId;
use argo_orchestrator::{EscalationStep, Handoff, HandoffStatus, OrchestratorStore, SessionId};
use argo_store::SessionStore;
use argo_types::TenantId;
use common::argo_bin;
use std::{process::Command, sync::Arc};

#[tokio::test]
async fn handoff_stats_report_and_histogram_round_trip_via_cli() {
    let home = tempfile::tempdir().unwrap();
    let tenant = TenantId::new();
    let tenant_s = tenant.to_string();
    let start = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args([
            "session", "start", "--tenant", &tenant_s, "--title", "stats demo", "--json",
        ])
        .output()
        .unwrap();
    assert!(
        start.status.success(),
        "session start failed: {}",
        String::from_utf8_lossy(&start.stderr)
    );
    let session: serde_json::Value = serde_json::from_slice(&start.stdout).unwrap();
    let sid = session["id"].as_str().unwrap().to_string();

    let store = Arc::new(
        SessionStore::open(home.path().join("sessions.db"))
            .await
            .unwrap(),
    );
    store.migrate().await.unwrap();
    let repo = OrchestratorStore::new(Arc::clone(&store));
    repo.ensure_schema().await.unwrap();

    let from = WorkerId::new();
    let receiver = WorkerId::new();
    for status in [
        HandoffStatus::Settled,
        HandoffStatus::Settled,
        HandoffStatus::Cancelled,
        HandoffStatus::Pending,
    ] {
        let handoff = Handoff::new(
            SessionId::from_string(sid.clone()),
            from,
            EscalationStep::Worker(receiver),
            "ship",
            serde_json::json!({"case": "demo"}),
        );
        let hid = handoff.id.clone();
        repo.insert_handoff(tenant, &handoff).await.unwrap();
        if !matches!(status, HandoffStatus::Pending) {
            repo.update_handoff_status(tenant, &hid, HandoffStatus::Acknowledged)
                .await
                .unwrap();
            repo.update_handoff_status(tenant, &hid, status).await.unwrap();
        }
    }

    let report = run_json(home.path(), &[
        "handoff-stats",
        "--tenant",
        &tenant_s,
        "--since",
        "1d",
        "--json",
    ]);
    assert_eq!(report["tenant_id"], tenant_s);
    assert_eq!(report["overview"]["total"], 4);
    assert_eq!(report["overview"]["resolved"], 2);
    assert_eq!(report["overview"]["abandoned"], 1);
    assert_eq!(report["overview"]["pending"], 1);
    assert!(
        (report["overview"]["success_rate"].as_f64().unwrap() - 2.0 / 3.0).abs() < 1e-9,
        "success_rate envelope: {}",
        report["overview"]["success_rate"],
    );
    let top_from = report["top_from"].as_array().unwrap();
    assert_eq!(top_from.len(), 1);
    assert_eq!(top_from[0]["key"], from.to_string());
    assert_eq!(top_from[0]["count"], 4);
    assert_eq!(report["by"], "worker");

    let by_skill = run_json(home.path(), &[
        "handoff-stats",
        "--tenant",
        &tenant_s,
        "--by",
        "skill",
        "--since",
        "1d",
        "--json",
    ]);
    let breakdown = by_skill["breakdown"].as_array().unwrap();
    assert_eq!(breakdown.len(), 1);
    assert_eq!(breakdown[0]["key"], "ship");
    assert_eq!(breakdown[0]["count"], 4);
    assert_eq!(by_skill["by"], "skill");

    let histogram = run_json(home.path(), &[
        "handoff-stats",
        "--tenant",
        &tenant_s,
        "--histogram",
        "--bucket-mins",
        "60",
        "--json",
    ]);
    let buckets = histogram["buckets"].as_array().unwrap();
    assert_eq!(buckets.len(), 24, "1d window / 60min buckets = 24");
    let total: u64 = buckets.iter().map(|b| b["count"].as_u64().unwrap()).sum();
    assert_eq!(total, 4);
}

fn run_json(home: &std::path::Path, args: &[&str]) -> serde_json::Value {
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home)
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "argo {args:?} failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "argo {args:?} stdout was not JSON: {e}\nstdout:\n{}",
            String::from_utf8_lossy(&out.stdout)
        )
    })
}
