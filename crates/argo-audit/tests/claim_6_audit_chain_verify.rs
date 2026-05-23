//! # Mission Claim 6 strengthener — `argo audit verify` chain integrity
//!
//! Pins the operator-locked strengthener from MISSION.md (round 15):
//!
//! > **Hash-chained audit as evidence.** Every worker action is hash-
//! > chained into `audit_record`. Each row's `self_hash` is
//! > `SHA-256(prior_row_hash || canonical_content)`, so any tampering
//! > between writes is detectable. Verifiable months later via
//! > `argo audit verify`.
//!
//! And the operator-locked Claim 6 MVP dashboard row (round 19):
//!
//! > Audit chain unbroken across approval + decision — `argo audit
//! > verify` returns OK on a clean DB, FAIL after tampering.
//!
//! This file is the regression contract for the strengthener.
//!
//! ## What this test proves
//!
//! 1. **Clean-DB verify is OK.** Seed N audit rows for one session via
//!    `AuditSink::write`. Run `verify_session`. Assert `ok = true,
//!    breaks.is_empty(), rows = N`.
//! 2. **Tampered-row verify is FAIL.** Mutate one row's `payload_json`
//!    directly via SQL (bypassing the sink). Re-run `verify_session`.
//!    Assert `ok = false, breaks.len() >= 1`.
//!
//! ## Authored
//!
//! Evaluator round 21, 2026-05-23.

use std::sync::Arc;

use argo_audit::{verify_session, AuditKind, AuditSink};
use argo_store::SessionStore;
use argo_types::SessionId;
use serde_json::json;
use sqlx::Row;
use uuid::Uuid;

/// Number of audit rows to seed before the verify pass. Picked >3 so
/// the chain genuinely chains (each row's parent_hash must equal the
/// prior row's self_hash) and a tampered middle row breaks the chain
/// in TWO places (its own self_hash + the next row's parent_hash).
const SEED_ROWS: usize = 5;

/// One session id used across the whole test so all seeded rows share
/// the same chain.
fn fixed_session_id() -> SessionId {
    SessionId(Uuid::from_u128(0x6CC7_C8C8_6CC7_C8C8_6CC7_C8C8_6CC7_C8C8))
}

#[tokio::test]
async fn claim_6_strengthener_audit_chain_verify() {
    // ── Phase 1: clean store, seed N audit rows, verify_session = OK.
    let store = Arc::new(SessionStore::open(":memory:").await.expect("open"));
    store.migrate().await.expect("migrate");
    let sink =
        AuditSink::new_with_tenant(Arc::clone(&store), "claim-6-strengthener".into());
    let session_id = fixed_session_id();
    for i in 0..SEED_ROWS {
        sink.write(
            Some(session_id),
            AuditKind::ToolCall,
            json!({
                "i": i,
                "tool": "echo",
                "marker": format!("claim-6-strengthener-row-{i}"),
            }),
            Some("system:claim-6-strengthener-test".into()),
        )
        .await
        .expect("audit write");
    }
    sink.flush_pending().await.expect("flush");

    let clean_report = verify_session(store.as_ref(), session_id)
        .await
        .expect("verify_session phase 1");
    assert!(
        clean_report.ok,
        "claim-6 strengthener: clean DB verify must be OK; got breaks={:?}",
        clean_report.breaks
    );
    assert_eq!(
        clean_report.rows, SEED_ROWS,
        "claim-6 strengthener: expected {SEED_ROWS} rows verified, got {}",
        clean_report.rows
    );
    assert!(
        clean_report.breaks.is_empty(),
        "claim-6 strengthener: clean DB must have zero chain breaks; got {:?}",
        clean_report.breaks
    );

    // ── Phase 2: tamper with the THIRD row's payload_json directly via
    // SQL (bypassing the sink — this is the "attacker mutates the DB"
    // scenario). The verify pass MUST detect it.
    let session_bytes = session_id.0.as_bytes().to_vec();
    let target_seq: i64 = sqlx::query_scalar(
        "SELECT seq FROM audit_record WHERE session_id = ? ORDER BY seq ASC LIMIT 1 OFFSET 2",
    )
    .bind(session_bytes.as_slice())
    .fetch_one(store.pool())
    .await
    .expect("pick target seq");
    let updated = sqlx::query(
        "UPDATE audit_record SET payload_json = ? WHERE session_id = ? AND seq = ?",
    )
    .bind(r#"{"tampered":true,"original_content":"erased"}"#)
    .bind(session_bytes.as_slice())
    .bind(target_seq)
    .execute(store.pool())
    .await
    .expect("tamper UPDATE");
    assert_eq!(
        updated.rows_affected(),
        1,
        "claim-6 strengthener: tamper UPDATE must affect exactly 1 row"
    );

    // Sanity: confirm the tampered row's payload is what we wrote
    // (rules out test plumbing failure before we trust the verify result).
    let tampered_payload: String =
        sqlx::query("SELECT payload_json FROM audit_record WHERE session_id = ? AND seq = ?")
            .bind(session_bytes.as_slice())
            .bind(target_seq)
            .fetch_one(store.pool())
            .await
            .expect("re-fetch tampered row")
            .try_get("payload_json")
            .expect("decode payload_json");
    assert!(
        tampered_payload.contains("\"tampered\":true"),
        "claim-6 strengthener: tamper UPDATE didn't land — got payload {tampered_payload:?}"
    );

    // ── Phase 3: re-verify. MUST detect the tamper.
    let tampered_report = verify_session(store.as_ref(), session_id)
        .await
        .expect("verify_session phase 3");
    assert!(
        !tampered_report.ok,
        "claim-6 strengthener: tampered DB verify must FAIL; got ok=true, breaks={:?}",
        tampered_report.breaks
    );
    assert!(
        !tampered_report.breaks.is_empty(),
        "claim-6 strengthener: tampered DB must report at least one ChainBreak; got breaks={:?}",
        tampered_report.breaks
    );

    // ── Phase 4: structural — at least one ChainBreak must reference
    // the tampered row's seq (or the immediately-following row, whose
    // parent_hash now mismatches). This pins that the verifier
    // actually points at the right location, not just "something
    // somewhere is wrong."
    let break_seqs: Vec<i64> = tampered_report.breaks.iter().map(|b| b.seq).collect();
    assert!(
        break_seqs.contains(&target_seq) || break_seqs.contains(&(target_seq + 1)),
        "claim-6 strengthener: ChainBreak.seq must point at tampered row {target_seq} OR the next row {} — got {break_seqs:?}",
        target_seq + 1
    );
}
