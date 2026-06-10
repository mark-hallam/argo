//! Public-surface contract pins for `argo-audit`.
//!
//! Authored by `pin-surface-contract-argo-types-store-audit`
//! (test-health audit 2026-06-10, F4). `argo-audit` is the
//! hash-chained audit writer — differentiator #1 (tamper-evident
//! audit). The chain shape
//! `self_hash = sha256(parent_hash || kind || 0x00 || payload)`, the
//! `AuditKind` wire labels, the `AuditSink` write entry points, and the
//! `verify_session` / `verify_all` chain checkers are all load-bearing:
//! a silent drift in any of them weakens the audit guarantee the
//! product is built on. These import + type-level fences plus a live
//! write→verify round-trip make such a drift fail loud.

#![allow(dead_code)]

use std::sync::Arc;

// 1. Re-export inventory fence — fails to COMPILE if any canonical
//    audit symbol is renamed or dropped from `lib.rs`.
use argo_audit::{
    compute_chain_hash, is_metadata_verb, verify_all, verify_session, AuditEvent, AuditKind,
    AuditPostCommit, AuditSink, CapabilityGap, ChainBreak, ExportedRow, GapKind, ReplayFormat,
    TailFilters, VerifyReport, WatchdogConfig, ANONYMOUS_AUDIT_TENANT_ID, AUDIT_BROADCAST_CAPACITY,
    CSV_HEADER, DEFAULT_AUDIT_TENANT_ID, METADATA_VERBS, SYSTEM_AUDIT_TENANT_ID,
};
use argo_store::SessionStore;
use argo_types::SessionId;

// ---------------------------------------------------------------------------
// 2. `AuditKind` wire-label contract. The `kind` column persists
//    `label()`; `from_label()` round-trips it. A silent rename would
//    orphan historical rows from `argo audit verify`.
// ---------------------------------------------------------------------------

#[test]
fn audit_kind_label_round_trip_is_total() {
    let _: fn(AuditKind) -> &'static str = AuditKind::label;
    let _: fn(&str) -> Option<AuditKind> = AuditKind::from_label;
    let _: fn() -> Vec<AuditKind> = AuditKind::all;

    // Every known kind round-trips label → from_label → identity.
    for kind in AuditKind::all() {
        let label = kind.label();
        assert_eq!(
            AuditKind::from_label(label),
            Some(kind),
            "label {label:?} failed to round-trip"
        );
    }
    // Unknown labels are `None`, not a silent default.
    assert_eq!(AuditKind::from_label("definitely.not.a.kind"), None);

    // A few canonical Phase-1 + differentiator labels are pinned
    // verbatim — these strings are persisted on disk forever.
    assert_eq!(AuditKind::TurnStart.label(), "turn_start");
    assert_eq!(AuditKind::ToolCall.label(), "tool_call");
    assert_eq!(AuditKind::ApprovalGranted.label(), "approval.granted");
    assert_eq!(AuditKind::RbacDeny.label(), "rbac.deny");
}

// ---------------------------------------------------------------------------
// 3. Chain-hash + writer + verifier signatures.
// ---------------------------------------------------------------------------

#[test]
fn audit_primitive_signatures_are_stable() {
    // `compute_chain_hash(parent, kind, payload) -> [u8; 32]`.
    let _: fn(&[u8; 32], &str, &str) -> [u8; 32] = compute_chain_hash;

    // The broadcast capacity sentinel + tenant constants.
    let _: usize = AUDIT_BROADCAST_CAPACITY;
    assert_eq!(DEFAULT_AUDIT_TENANT_ID, "ws-default");
    assert_eq!(SYSTEM_AUDIT_TENANT_ID, "system");
    assert_eq!(ANONYMOUS_AUDIT_TENANT_ID, "anonymous");

    // `CSV_HEADER` export contract is a `&[&str]` column list.
    let _: &[&str] = CSV_HEADER;
    // `METADATA_VERBS` + `is_metadata_verb` agree on membership.
    let _: &[&str] = METADATA_VERBS;
    assert!(METADATA_VERBS.iter().all(|v| is_metadata_verb(v)));
}

#[test]
fn verify_report_field_shape_is_stable() {
    // A default report is the "empty + clean" base the verifier builds
    // on; pin the load-bearing field types.
    let report = VerifyReport::default();
    let _: usize = report.rows;
    let _: bool = report.ok;
    let _: Vec<ChainBreak> = report.breaks;

    let brk = ChainBreak {
        seq: 1,
        kind: "tool_call".into(),
        reason: "parent mismatch".into(),
    };
    let _: i64 = brk.seq;
    let _: String = brk.kind;
    let _: String = brk.reason;
}

// ---------------------------------------------------------------------------
// 4. Live write → verify round-trip. This is the actual differentiator
//    contract: a row written through `AuditSink` produces a chain that
//    `verify_session` confirms clean. Pins `AuditSink::new_with_tenant`,
//    `write`, and `verify_session` end-to-end.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn sink_write_then_verify_session_is_clean() {
    let store = Arc::new(SessionStore::open(":memory:").await.expect("open store"));
    store.migrate().await.expect("migrate");

    // Constructor fence exercised by real construction. (`AuditSink::new`
    // is deprecated — production callers use `new_with_tenant`.)
    let sink = AuditSink::new_with_tenant(store.clone(), SYSTEM_AUDIT_TENANT_ID.to_string());
    assert_eq!(sink.tenant_id(), SYSTEM_AUDIT_TENANT_ID);

    let session = SessionId::new();
    sink.write(
        Some(session),
        AuditKind::TurnStart,
        serde_json::json!({ "k": 1 }),
        None,
    )
    .await
    .expect("write audit row");
    sink.write(
        Some(session),
        AuditKind::ToolCall,
        serde_json::json!({ "tool": "fs.read" }),
        Some("principal:test".into()),
    )
    .await
    .expect("write second audit row");
    sink.flush_pending().await.expect("flush");

    let report = verify_session(store.as_ref(), session)
        .await
        .expect("verify session chain");
    assert!(report.ok, "freshly-written chain must verify clean");
    assert!(report.breaks.is_empty());
    assert_eq!(report.rows, 2);

    // `verify_all` walks every session; the round-trip chain is clean.
    let all = verify_all(store.as_ref()).await.expect("verify all");
    assert!(all.ok, "whole-store verify must be clean");
}

// ---------------------------------------------------------------------------
// 5. Read-path / config type inventory fence. These export, replay,
//    gap, and watchdog data shapes are consumed by the CLI + gateway
//    audit endpoints; naming each makes a rename or removal a compile
//    error.
// ---------------------------------------------------------------------------

#[test]
fn audit_read_path_inventory_is_named() {
    // Value-type surfaces.
    let _: Option<AuditEvent> = None;
    let _: Option<AuditPostCommit> = None;
    let _: Option<CapabilityGap> = None;
    let _: Option<ExportedRow> = None;
    let _: Option<GapKind> = None;
    let _: Option<ReplayFormat> = None;
    let _: Option<TailFilters> = None;
    let _: Option<WatchdogConfig> = None;
}
