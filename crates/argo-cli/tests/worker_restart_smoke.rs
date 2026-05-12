//! Smoke test for the OOM-restart classifier + audit-emission
//! pipeline (mission-oom-restart-handler).
//!
//! Synthesises a child process that exits with code 137 (the
//! shell-wrapped Linux SIGKILL signature) — we do not actually
//! allocate to OOM-out, since that is flaky and slow. The test
//! drives the classifier on the real `ExitStatus`, asks the
//! coordinator for a restart decision, persists the audit row
//! via `AuditSink::write`, and then queries the same store
//! through `argo worker restarts --since 1h` to confirm the row
//! round-trips.
//!
//! Brief acceptance ticks covered: classify-exit-137,
//! audit-row appended for non-clean exits, `argo worker
//! restarts --since` lists recent events. On Windows the
//! classifier falls through to `OtherSignal`, so the assertion
//! is widened there; the test does not skip.

mod common;

use argo_audit::{AuditKind, AuditSink};
use argo_runtime::oom_restart::{
    build_audit_payload, classify_exit, decide_restart, OomRestartCoordinator, RestartDecision,
    WorkerExitReason,
};
use argo_store::SessionStore;
use chrono::Utc;
use std::path::Path;
use std::process::{Command, Output};
use std::sync::Arc;

#[tokio::test]
async fn synthetic_exit_137_classifies_audits_and_lists() {
    let home = tempfile::tempdir().unwrap();
    let store = Arc::new(
        SessionStore::open(home.path().join("sessions.db"))
            .await
            .unwrap(),
    );
    store.migrate().await.unwrap();
    let sink = AuditSink::new(store.clone());

    // 1) Synthesise a child that exits 137 — no real OOM, just
    //    the same ExitStatus the supervisor would see.
    let status = spawn_exit_137();
    let reason = classify_exit(status, None);
    #[cfg(unix)]
    {
        assert!(
            matches!(reason, WorkerExitReason::OomKilled { .. }),
            "expected OomKilled on unix, got {reason:?}"
        );
    }
    #[cfg(not(unix))]
    {
        // Windows fallthrough: exit 137 still routes through
        // `code() == Some(137)`, so we land in OomKilled too.
        assert!(matches!(
            reason,
            WorkerExitReason::OomKilled { .. } | WorkerExitReason::Clean { code: 137 }
        ));
    }

    // 2) Drive the coordinator + emit the audit row.
    let mut coord = OomRestartCoordinator::with_defaults();
    let decision = decide_restart(&mut coord, &reason, Utc::now());
    assert!(matches!(decision, RestartDecision::Restart { .. }));
    let payload = build_audit_payload("wkr-smoke", &reason, &decision, 1);
    sink.write(None, AuditKind::WorkerOomRestart, payload, None)
        .await
        .unwrap();

    // 3) Exhaust the budget + verify give-up path produces a
    //    salvage note pointing at the runbook.
    let mut budget_coord = OomRestartCoordinator::with_defaults();
    for _ in 0..3 {
        let _ = decide_restart(&mut budget_coord, &reason, Utc::now());
    }
    let exhausted = decide_restart(&mut budget_coord, &reason, Utc::now());
    match &exhausted {
        RestartDecision::GiveUp { salvage_note } => {
            assert!(salvage_note.contains("oom-restart.md"));
            assert!(salvage_note.contains("restarts in window exceeded"));
        }
        other => panic!("expected GiveUp after budget exhaustion, got {other:?}"),
    }
    let give_up_payload = build_audit_payload("wkr-budget", &reason, &exhausted, 4);
    sink.write(None, AuditKind::WorkerOomRestart, give_up_payload, None)
        .await
        .unwrap();

    // 4) Drop the sink so its broadcast channel doesn't keep a
    //    background-task receiver alive; SessionStore stays open
    //    because the CLI sub-process re-opens the same db file.
    drop(sink);

    // 5) Query via the live CLI binary — confirms the dispatch
    //    arm, the handler, and the audit_record schema all line
    //    up end-to-end. `--since 1h` is wide enough to catch the
    //    fresh rows under any plausible clock skew.
    let out = run_ok(
        home.path(),
        &["worker", "restarts", "--since", "1h"],
    );
    let body = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = body.lines().filter(|l| !l.trim().is_empty()).collect();
    assert!(
        lines.len() >= 2,
        "expected at least 2 restart rows, got {}\n----\n{body}",
        lines.len()
    );
    let restart_row: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    let payload = &restart_row["payload"];
    // Rows are sorted DESC by created_at — the most recent (the
    // GiveUp one) lands first.
    assert_eq!(payload["worker_id"], "wkr-budget");
    assert_eq!(payload["decision"]["kind"], "give_up");

    // 6) Worker-id filter narrows down.
    let filtered = run_ok(
        home.path(),
        &["worker", "restarts", "--worker", "wkr-smoke", "--since", "1h"],
    );
    let filtered_body = String::from_utf8(filtered.stdout).unwrap();
    let filtered_lines: Vec<&str> = filtered_body
        .lines()
        .filter(|l| !l.trim().is_empty())
        .collect();
    assert_eq!(filtered_lines.len(), 1);
    let only: serde_json::Value = serde_json::from_str(filtered_lines[0]).unwrap();
    assert_eq!(only["payload"]["worker_id"], "wkr-smoke");
    assert_eq!(only["payload"]["decision"]["kind"], "restart");
}

fn spawn_exit_137() -> std::process::ExitStatus {
    // `sh -c 'exit 137'` works on every POSIX host; on Windows
    // we fall back to `cmd /c exit 137`. Both surface as
    // `ExitStatus::code() == Some(137)` to the parent.
    let mut command = if cfg!(windows) {
        let mut c = Command::new("cmd");
        c.args(["/C", "exit", "137"]);
        c
    } else {
        let mut c = Command::new("sh");
        c.args(["-c", "exit 137"]);
        c
    };
    command.status().expect("spawn synthetic worker")
}

fn run_ok(home: &Path, args: &[&str]) -> Output {
    let out = Command::new(common::argo_bin())
        .env("ARGO_HOME", home)
        .args(args)
        .output()
        .expect("argo cli");
    assert!(
        out.status.success(),
        "argo {:?} failed\nstderr: {}",
        args,
        String::from_utf8_lossy(&out.stderr)
    );
    out
}
