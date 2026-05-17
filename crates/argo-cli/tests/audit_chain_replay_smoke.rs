//! `mission-claim-5-runtime-closure-trace` — end-to-end smoke that
//! proves claim 5 ("every action lives on a tamper-evident hash chain
//! that anyone can replay") is runtime-live. See
//! `docs/skycot/operator/audit-chain-forensics.md` for the runbook.
//!
//! ## Sensible deviations from the brief
//!
//! - `argo audit verify-chain` does not take `--session <id>` on
//!   current `origin/main`; it verifies the whole audit table. The
//!   smoke spawns the CLI against an isolated tempdir, so the
//!   "whole table" is exactly the smoke's session.
//! - The brief's `argo replay <session> --output <file>` shape does
//!   not match the shipped CLI: `argo audit replay --format raw`
//!   streams NDJSON to stdout, and the unrelated `argo session
//!   replay` re-runs the model. The smoke drives the same library
//!   functions the `audit_replay::run` handler calls
//!   (`rows_for_session` + `render_replay`) and asserts equivalence
//!   on the parsed NDJSON.

#![allow(clippy::expect_used)]

use std::process::Command;
use std::sync::Arc;

use argo_audit::{rows_for_session, verify_session, AuditSink, ExportedRow, ReplayFormat};
use argo_store::SessionStore;
use argo_types::{SessionId, TenantId, WorkspaceId};

#[path = "fixtures/audit_chain_seed.rs"]
mod seed;

const TENANT_A: &str = "tenant-acme";
const TENANT_B: &str = "tenant-globex";

fn argo_bin() -> &'static str {
    env!("CARGO_BIN_EXE_argo")
}

async fn fresh_store_with_session(home: &std::path::Path) -> (Arc<SessionStore>, SessionId) {
    let db = home.join("sessions.db");
    let store = Arc::new(SessionStore::open(&db).await.expect("open store"));
    store.migrate().await.expect("migrate");
    let tenant = TenantId::new();
    let ws = WorkspaceId::new();
    sqlx::query(
        "INSERT INTO workspace (id, tenant_id, name, root_path, created_at)
         VALUES (?, ?, 'smoke', '.', strftime('%s','now'))",
    )
    .bind(ws.0.as_bytes().as_slice())
    .bind(tenant.0.as_bytes().as_slice())
    .execute(store.pool())
    .await
    .expect("insert workspace");
    let session = store
        .create_session(tenant, ws, "test-model", "test-provider", Some("smoke"), None)
        .await
        .expect("create session");
    (store, session)
}

async fn seed_session(sink: &AuditSink, session: SessionId, seq: &[seed::SeedEvent]) {
    for ev in seq {
        sink.write(Some(session), ev.kind, ev.payload.clone(), None)
            .await
            .expect("audit write");
    }
    sink.flush_pending().await.expect("flush sink");
}

fn assert_chain_links(rows: &[ExportedRow]) {
    assert!(!rows.is_empty(), "no audit rows for session");
    let zero = "00".repeat(32);
    assert_eq!(rows[0].parent_hash, zero, "first row must root at zero");
    for window in rows.windows(2) {
        assert_eq!(
            window[1].parent_hash, window[0].self_hash,
            "row {seq} parent_hash must chain back to row {prev} self_hash",
            seq = window[1].seq,
            prev = window[0].seq,
        );
    }
}

fn assert_trajectory_equivalent(rows: &[ExportedRow], seq: &[seed::SeedEvent]) {
    assert_eq!(rows.len(), seq.len(), "replayed row count != seeded count");
    for (i, (row, seed_ev)) in rows.iter().zip(seq.iter()).enumerate() {
        assert_eq!(row.kind, seed_ev.kind.label(), "kind drift at index {i}");
        // The sink stamps `tenant_id` into the payload at the publish
        // edge, so the on-disk payload is a superset of the seeded
        // payload. Assert every seeded key/value survives unchanged.
        let object = seed_ev.payload.as_object().expect("seed payload is object");
        for (k, v) in object {
            assert_eq!(
                row.payload.get(k),
                Some(v),
                "payload key `{k}` drift at index {i}; row={:?}",
                row.payload
            );
        }
    }
}

/// Waypoints 1–5: chain emission → library verify → CLI verify →
/// rows fetch → render → trajectory equivalence.
#[tokio::test]
async fn closure_smoke_session_emit_verify_render_matches_trajectory() {
    let home = tempfile::tempdir().expect("home tempdir");
    let (store, session) = fresh_store_with_session(home.path()).await;

    let sink = AuditSink::new_with_tenant(Arc::clone(&store), TENANT_A.to_string());
    let seq = seed::happy_path_session();
    seed_session(&sink, session, &seq).await;

    // Waypoint 2 — chain hash links cleanly through every row.
    let rows = rows_for_session(&store, session)
        .await
        .expect("rows_for_session");
    assert_chain_links(&rows);

    // Waypoint 3a — library verify clean.
    let report = verify_session(&store, session)
        .await
        .expect("verify_session");
    assert!(report.ok, "verify_session must pass: {:?}", report.breaks);
    assert_eq!(report.rows, seq.len(), "verify-session row count drift");

    // Waypoint 3b — CLI verify clean (subprocess against same store).
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args(["audit", "verify-chain", "--json"])
        .output()
        .expect("spawn argo audit verify-chain");
    assert!(out.status.success(), "verify-chain exit: {:?}", out.status);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let summary: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("verify-chain JSON summary parse");
    assert_eq!(
        summary["verified"].as_bool(),
        Some(true),
        "CLI must report verified=true; got: {summary}",
    );
    assert_eq!(
        summary["breaks"].as_u64(),
        Some(0),
        "CLI must report 0 breaks; got: {summary}",
    );

    // Waypoint 4 + 5 — render the replay trajectory the same way
    // `argo audit replay --format raw` does, then assert equivalence.
    let trajectory = argo_audit::render_replay(&rows, ReplayFormat::Raw);
    let parsed: Vec<ExportedRow> = trajectory
        .lines()
        .map(|line| serde_json::from_str(line).expect("trajectory NDJSON parse"))
        .collect();
    assert_trajectory_equivalent(&parsed, &seq);
}

/// Waypoint 6 — hand-corrupt one row's `payload_json` in an isolated
/// session and assert `verify_session` flags the corrupted seq.
#[tokio::test]
async fn tamper_evidence_corrupted_payload_detected_at_offset() {
    let home = tempfile::tempdir().expect("home tempdir");
    let (store, session) = fresh_store_with_session(home.path()).await;
    let sink = AuditSink::new_with_tenant(Arc::clone(&store), TENANT_A.to_string());
    seed_session(&sink, session, &seed::happy_path_session()).await;

    // Pick the ToolResult row (seq 3 in the canonical sequence) and
    // corrupt its on-disk payload. The sink's chain hash was computed
    // over the original canonical string, so re-extracting it will no
    // longer match self_hash.
    let target_seq: i64 = sqlx::query_scalar(
        "SELECT seq FROM audit_record \
         WHERE session_id = ? AND kind = 'tool_result' \
         ORDER BY seq ASC LIMIT 1",
    )
    .bind(session.0.as_bytes().as_slice())
    .fetch_one(store.pool())
    .await
    .expect("locate tamper target");
    sqlx::query("UPDATE audit_record SET payload_json = ? WHERE seq = ?")
        .bind(r#"{"tampered":true}"#)
        .bind(target_seq)
        .execute(store.pool())
        .await
        .expect("apply tamper");

    let report = verify_session(&store, session)
        .await
        .expect("verify_session");
    assert!(!report.ok, "tamper must flip verify to false");
    assert!(
        report.breaks.iter().any(|b| b.seq == target_seq),
        "verifier must flag the corrupted seq ({target_seq}); breaks: {:?}",
        report.breaks,
    );
}

/// Waypoint 7 — per-session reads are tenant-scoped, and tampering a
/// row's `tenant_id` denorm column (the natural cross-tenant spoof
/// vector) is detected by the verifier.
#[tokio::test]
async fn cross_tenant_isolation_and_spoof_detected() {
    let home = tempfile::tempdir().expect("home tempdir");
    let (store, session_a) = fresh_store_with_session(home.path()).await;
    let (_, session_b) = fresh_store_with_session(home.path()).await;

    let sink_a = AuditSink::new_with_tenant(Arc::clone(&store), TENANT_A.to_string());
    let sink_b = AuditSink::new_with_tenant(Arc::clone(&store), TENANT_B.to_string());
    seed_session(&sink_a, session_a, &seed::happy_path_session()).await;
    seed_session(&sink_b, session_b, &seed::happy_path_session()).await;

    // Per-session reads never bleed across tenants — the brief's
    // "replay attempt across tenant boundary fails" assertion.
    let rows_a = rows_for_session(&store, session_a)
        .await
        .expect("rows for A");
    let rows_b = rows_for_session(&store, session_b)
        .await
        .expect("rows for B");
    assert!(
        rows_a
            .iter()
            .all(|r| r.payload["tenant_id"].as_str() == Some(TENANT_A)),
        "session-A rows must all be stamped tenant-A",
    );
    assert!(
        rows_b
            .iter()
            .all(|r| r.payload["tenant_id"].as_str() == Some(TENANT_B)),
        "session-B rows must all be stamped tenant-B",
    );
    // Belt-and-braces: cross-(tenant, session) lookup is empty.
    let bleed: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_record \
         WHERE session_id = ? AND tenant_id = ?",
    )
    .bind(session_a.0.as_bytes().as_slice())
    .bind(TENANT_B)
    .fetch_one(store.pool())
    .await
    .expect("count cross-tenant bleed");
    assert_eq!(bleed, 0, "no rows must straddle tenant + session");

    // Spoof vector: rename session-A row's tenant_id column to
    // tenant-B without re-hashing. The verifier's denorm sanity belt
    // must flag the row.
    let spoof_seq: i64 = sqlx::query_scalar(
        "SELECT seq FROM audit_record WHERE session_id = ? ORDER BY seq ASC LIMIT 1",
    )
    .bind(session_a.0.as_bytes().as_slice())
    .fetch_one(store.pool())
    .await
    .expect("locate spoof target");
    sqlx::query("UPDATE audit_record SET tenant_id = ? WHERE seq = ?")
        .bind(TENANT_B)
        .bind(spoof_seq)
        .execute(store.pool())
        .await
        .expect("apply tenant_id spoof");

    let report = verify_session(&store, session_a)
        .await
        .expect("verify_session A");
    assert!(!report.ok, "tenant_id spoof must flip verify to false");
    assert!(
        report.breaks.iter().any(|b| b.seq == spoof_seq
            && b.reason.contains("tenant_id denorm drift")),
        "verifier must flag tenant_id spoof at seq {spoof_seq}; breaks: {:?}",
        report.breaks,
    );
}
