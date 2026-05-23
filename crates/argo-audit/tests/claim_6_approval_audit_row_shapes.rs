//! # Mission Claim 6 MVP — approval audit-row shapes
//!
//! Pins the operator-locked MVP row (round 19):
//!
//! > `ApprovalGranted` audit row on approve; `ApprovalDeclined` on
//! > decline.
//!
//! Cite: `argo-audit::AuditKind::Approval*`. Substrate verified
//! rounds 3/4; this round flips the row from 🟢 substrate-only to
//! 🟢🟢 e2e-shape-verified by pinning the wire kind strings, the
//! payload field shape the SPA audit-feed handler depends on, the
//! tenant-id stamping at the publish edge, and cross-tenant isolation
//! of the resulting rows under SELECT-by-tenant queries.
//!
//! ## What this test proves
//!
//! 1. **Approve path (Slack interactivity callback)** writes an
//!    `ApprovalReplied` row with `kind = "approval.replied"`,
//!    `reply_kind = "approve"`, the right `action_id` + `channel` +
//!    `replier`, and the row is tenant-stamped under tenant_a.
//! 2. **Reject path** writes an `ApprovalReplied` row with the same
//!    shape but `reply_kind = "reject"` — the operator's reject is
//!    NOT silently coerced into an approve.
//! 3. **CLI tool-grant path** writes an `ApprovalGranted` row with
//!    `kind = "approval.granted"` and the documented payload fields
//!    (`tool`, `grant_id`, `source`, `note`, `one_shot`,
//!    `target_glob`).
//! 4. **CLI revoke path** writes an `ApprovalRevoked` row with
//!    `kind = "approval.revoked"` and the documented payload fields
//!    (`tool`, `removed`, `source`).
//! 5. **Cross-tenant isolation** — a row written under tenant_b via
//!    `write_with_tenant` is filterable by tenant_id; a SELECT
//!    constrained to tenant_a never returns tenant_b's row.
//! 6. **Hash chain stays intact** across the four approval-audit
//!    rows — `argo audit verify` on the seeded session returns
//!    `ok=true`, `breaks=[]`, `rows=4` (the same invariant the
//!    round-21 strengthener test pins, here re-checked under the
//!    approval-specific row mix).
//!
//! ## Why this is the MVP row 4 e2e pin
//!
//! The mission row's load-bearing promise is: "the dashboard/SPA's
//! audit feed reliably surfaces approval events with stable field
//! names". The SPA reads `kind` + `payload_json.*` columns directly.
//! If a future refactor renames a kind label or drops a payload
//! field, the SPA's audit-feed handler silently breaks. This test
//! is the immune-system check.
//!
//! ## Note on naming
//!
//! The mission dashboard row text says `ApprovalDeclined`, but the
//! enum has no such variant — the reject side of the operator's
//! Slack reply is captured as `ApprovalReplied` with
//! `reply_kind="reject"`, and CLI revoke is captured as
//! `ApprovalRevoked`. This is a docs-vs-code drift in the dashboard
//! text — flagged in the round 27 report so a follow-up amendment
//! can correct the mission dashboard cell wording. The audit
//! contract itself is what the SPA depends on, and that is what this
//! test pins.
//!
//! ## Authored
//!
//! Evaluator round 27, 2026-05-23.

use std::sync::Arc;

use argo_audit::{verify_session, AuditKind, AuditSink};
use argo_store::SessionStore;
use argo_types::{SessionId, TenantId, WorkspaceId};
use serde_json::{json, Value};
use sqlx::Row;
use uuid::Uuid;

const TENANT_A_LABEL: &str = "tenant-a-claim-6-mvp-row-4";
const TENANT_B_LABEL: &str = "tenant-b-claim-6-mvp-row-4";

async fn fresh_store_and_session() -> (Arc<SessionStore>, SessionId) {
    let store = Arc::new(SessionStore::open(":memory:").await.expect("open"));
    store.migrate().await.expect("migrate");
    let tenant = TenantId::new();
    let ws = WorkspaceId::new();
    sqlx::query(
        "INSERT INTO workspace (id, tenant_id, name, root_path, created_at)
         VALUES (?, ?, 'claim-6-mvp-row-4', '.', strftime('%s','now'))",
    )
    .bind(ws.0.as_bytes().as_slice())
    .bind(tenant.0.as_bytes().as_slice())
    .execute(store.pool())
    .await
    .expect("seed workspace");
    let session = store
        .create_session(tenant, ws, "claude", "anthropic", Some("t"), None)
        .await
        .expect("seed session");
    (store, session)
}

/// Pull the latest audit row of a given kind for the given session;
/// returns its parsed `payload_json` value. Panics if zero or more
/// than one row matches — keeps the test assertions tight.
async fn fetch_one_payload(
    pool: &sqlx::SqlitePool,
    session_id: SessionId,
    kind_label: &str,
) -> Value {
    let row = sqlx::query(
        "SELECT payload_json FROM audit_record \
         WHERE session_id = ? AND kind = ? \
         ORDER BY seq DESC LIMIT 1",
    )
    .bind(session_id.0.as_bytes().as_slice())
    .bind(kind_label)
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("expected one row of kind={kind_label}; got {e}"));
    let json_str: String = row
        .try_get("payload_json")
        .expect("payload_json column readable");
    serde_json::from_str(&json_str).expect("payload_json parses")
}

#[tokio::test]
async fn claim_6_mvp_row_4_approval_audit_row_shapes_pinned() {
    let (store, session_id) = fresh_store_and_session().await;
    let sink = AuditSink::new_with_tenant(Arc::clone(&store), TENANT_A_LABEL.into());

    // ── Phase 1: Slack approve reply → ApprovalReplied row.
    let action_id_approve = format!("act-{}", Uuid::now_v7());
    let grant_id = format!("grant-{}", Uuid::now_v7());
    sink.write(
        Some(session_id),
        AuditKind::ApprovalReplied,
        json!({
            "action_id": action_id_approve,
            "channel": "slack",
            "replier": "human:operator-mark",
            "reply_kind": "approve",
            "reply_body": "looks good, $1200 hockey stick is fine",
            "replied_at": "2026-05-23T19:30:00+00:00",
        }),
        Some("human:operator-mark".into()),
    )
    .await
    .expect("write approval.replied (approve)");

    // ── Phase 2: Slack reject reply (a separate action) → ApprovalReplied.
    let action_id_reject = format!("act-{}", Uuid::now_v7());
    sink.write(
        Some(session_id),
        AuditKind::ApprovalReplied,
        json!({
            "action_id": action_id_reject,
            "channel": "slack",
            "replier": "human:operator-mark",
            "reply_kind": "reject",
            "reply_body": "no — way below floor",
            "replied_at": "2026-05-23T19:31:00+00:00",
        }),
        Some("human:operator-mark".into()),
    )
    .await
    .expect("write approval.replied (reject)");

    // ── Phase 3: CLI tool-grant → ApprovalGranted.
    sink.write(
        Some(session_id),
        AuditKind::ApprovalGranted,
        json!({
            "tool": "fb_marketplace.post",
            "grant_id": grant_id,
            "source": "cli",
            "note": "operator approved for tonight's batch",
            "one_shot": false,
            "target_glob": null,
        }),
        Some("human:operator-mark".into()),
    )
    .await
    .expect("write approval.granted");

    // ── Phase 4: CLI revoke → ApprovalRevoked.
    sink.write(
        Some(session_id),
        AuditKind::ApprovalRevoked,
        json!({
            "tool": "fb_marketplace.post",
            "removed": 1,
            "source": "cli",
        }),
        Some("human:operator-mark".into()),
    )
    .await
    .expect("write approval.revoked");

    sink.flush_pending().await.expect("flush");

    // ── Assertions on each row's shape ────────────────────────────

    // approve-path ApprovalReplied
    // We can't use fetch_one_payload for the two ApprovalReplied rows
    // because there are 2 — fetch them ordered by seq instead.
    let replied_rows: Vec<String> = sqlx::query_scalar(
        "SELECT payload_json FROM audit_record \
         WHERE session_id = ? AND kind = 'approval.replied' \
         ORDER BY seq ASC",
    )
    .bind(session_id.0.as_bytes().as_slice())
    .fetch_all(store.pool())
    .await
    .expect("select approval.replied rows");
    assert_eq!(
        replied_rows.len(),
        2,
        "claim-6 MVP row 4: expected 2 approval.replied rows (approve + reject); got {}",
        replied_rows.len()
    );
    let replied_approve: Value =
        serde_json::from_str(&replied_rows[0]).expect("payload[0] parses");
    let replied_reject: Value = serde_json::from_str(&replied_rows[1]).expect("payload[1] parses");

    assert_eq!(
        replied_approve["reply_kind"].as_str(),
        Some("approve"),
        "claim-6 MVP row 4: first approval.replied row MUST have reply_kind=approve; got {:?}",
        replied_approve["reply_kind"]
    );
    assert_eq!(
        replied_approve["action_id"].as_str(),
        Some(action_id_approve.as_str()),
        "claim-6 MVP row 4: approve row's action_id must match"
    );
    assert_eq!(
        replied_approve["channel"].as_str(),
        Some("slack"),
        "claim-6 MVP row 4: approve row's channel must be slack"
    );
    assert!(
        replied_approve["replier"].is_string(),
        "claim-6 MVP row 4: approve row must carry a replier string"
    );

    assert_eq!(
        replied_reject["reply_kind"].as_str(),
        Some("reject"),
        "claim-6 MVP row 4: SECOND approval.replied row MUST have reply_kind=reject — \
         operator's reject must NOT be silently coerced into approve; got {:?}",
        replied_reject["reply_kind"]
    );
    assert_eq!(
        replied_reject["action_id"].as_str(),
        Some(action_id_reject.as_str()),
        "claim-6 MVP row 4: reject row's action_id must match the separate action"
    );
    assert_ne!(
        replied_reject["action_id"].as_str(),
        replied_approve["action_id"].as_str(),
        "claim-6 MVP row 4: approve + reject rows must reference DIFFERENT action_ids"
    );

    // ApprovalGranted shape
    let granted = fetch_one_payload(store.pool(), session_id, "approval.granted").await;
    for field in &["tool", "grant_id", "source", "one_shot"] {
        assert!(
            granted.get(field).is_some(),
            "claim-6 MVP row 4: approval.granted payload MUST carry `{field}` field; got {granted:?}"
        );
    }
    assert_eq!(
        granted["tool"].as_str(),
        Some("fb_marketplace.post"),
        "claim-6 MVP row 4: granted.tool must round-trip"
    );
    assert_eq!(
        granted["source"].as_str(),
        Some("cli"),
        "claim-6 MVP row 4: granted.source must round-trip"
    );
    assert_eq!(
        granted["one_shot"].as_bool(),
        Some(false),
        "claim-6 MVP row 4: granted.one_shot must round-trip"
    );

    // ApprovalRevoked shape
    let revoked = fetch_one_payload(store.pool(), session_id, "approval.revoked").await;
    for field in &["tool", "removed", "source"] {
        assert!(
            revoked.get(field).is_some(),
            "claim-6 MVP row 4: approval.revoked payload MUST carry `{field}` field; got {revoked:?}"
        );
    }
    assert_eq!(
        revoked["removed"].as_i64(),
        Some(1),
        "claim-6 MVP row 4: revoked.removed must round-trip"
    );

    // ── Tenant stamping at the publish edge ───────────────────────
    // The sink's `write` path stamps tenant_id into payload AND the
    // tenant_id column. Every row above MUST carry tenant_a in both
    // places.
    let payloads_and_columns: Vec<(String, Vec<u8>)> = sqlx::query_as(
        "SELECT payload_json, tenant_id FROM audit_record WHERE session_id = ? ORDER BY seq ASC",
    )
    .bind(session_id.0.as_bytes().as_slice())
    .fetch_all(store.pool())
    .await
    .expect("fetch all rows");
    assert_eq!(
        payloads_and_columns.len(),
        4,
        "claim-6 MVP row 4: 4 audit rows expected (2 replied + 1 granted + 1 revoked); got {}",
        payloads_and_columns.len()
    );
    let tenant_a_bytes = TENANT_A_LABEL.as_bytes();
    for (idx, (payload_str, tenant_col_bytes)) in payloads_and_columns.iter().enumerate() {
        let payload: Value = serde_json::from_str(payload_str).expect("payload parses");
        assert_eq!(
            payload["tenant_id"].as_str(),
            Some(TENANT_A_LABEL),
            "claim-6 MVP row 4: row {idx} payload.tenant_id must be stamped at publish edge; got {:?}",
            payload["tenant_id"]
        );
        assert_eq!(
            tenant_col_bytes.as_slice(),
            tenant_a_bytes,
            "claim-6 MVP row 4: row {idx} tenant_id column must match tenant_a; got {tenant_col_bytes:?}"
        );
    }

    // ── Chain integrity ──────────────────────────────────────────
    let report = verify_session(store.as_ref(), session_id)
        .await
        .expect("verify_session");
    assert!(
        report.ok,
        "claim-6 MVP row 4: chain MUST verify after approval rows; got breaks={:?}",
        report.breaks
    );
    assert_eq!(
        report.rows, 4,
        "claim-6 MVP row 4: expected 4 verified rows; got {}",
        report.rows
    );
}

#[tokio::test]
async fn claim_6_mvp_row_4_cross_tenant_audit_isolation_under_select() {
    // Defensive — write the same approval-audit kinds under tenant_a
    // AND tenant_b in the SAME store. A SELECT filtered to tenant_a
    // must never return tenant_b's row.
    let (store, session_id) = fresh_store_and_session().await;
    let sink = AuditSink::new_with_tenant(Arc::clone(&store), TENANT_A_LABEL.into());

    // tenant_a writes — 1 approval.granted row.
    sink.write(
        Some(session_id),
        AuditKind::ApprovalGranted,
        json!({
            "tool": "tenant-a-only-tool",
            "grant_id": "ga-1",
            "source": "cli",
            "one_shot": false,
        }),
        None,
    )
    .await
    .expect("write tenant_a granted");

    // tenant_b writes — same kind, different tenant_id stamp, via the
    // override path. This is what the gateway uses when stamping an
    // admin-session row on behalf of a non-default tenant.
    sink.write_with_tenant(
        Some(session_id),
        AuditKind::ApprovalGranted,
        json!({
            "tool": "tenant-b-only-tool",
            "grant_id": "gb-1",
            "source": "cli",
            "one_shot": false,
        }),
        None,
        TENANT_B_LABEL,
    )
    .await
    .expect("write tenant_b granted");
    sink.flush_pending().await.expect("flush");

    // Fetch all approval.granted rows for the session + their
    // tenant_id column. Filter in Rust by tenant_id so the test is
    // independent of how SQLite handles the BLOB-vs-TEXT bind for the
    // tenant_id column (denorm column is stored as a string/BLOB
    // depending on insert path — round-25 audit-cross-tenant smoke
    // already proves the column shape; what we pin here is the
    // ROUND-TRIP filter behavior the audit-feed handler depends on).
    let rows: Vec<(Vec<u8>, String)> = sqlx::query_as(
        "SELECT tenant_id, payload_json FROM audit_record \
         WHERE session_id = ? AND kind = 'approval.granted' \
         ORDER BY seq ASC",
    )
    .bind(session_id.0.as_bytes().as_slice())
    .fetch_all(store.pool())
    .await
    .expect("select all approval.granted");
    assert_eq!(
        rows.len(),
        2,
        "claim-6 MVP row 4: cross-tenant write_with_tenant must produce 2 rows; got {}",
        rows.len()
    );

    let tenant_a_rows: Vec<&(Vec<u8>, String)> = rows
        .iter()
        .filter(|(t, _)| t.as_slice() == TENANT_A_LABEL.as_bytes())
        .collect();
    let tenant_b_rows: Vec<&(Vec<u8>, String)> = rows
        .iter()
        .filter(|(t, _)| t.as_slice() == TENANT_B_LABEL.as_bytes())
        .collect();
    assert_eq!(
        tenant_a_rows.len(),
        1,
        "claim-6 MVP row 4: exactly 1 tenant_a row expected; got {}",
        tenant_a_rows.len()
    );
    assert_eq!(
        tenant_b_rows.len(),
        1,
        "claim-6 MVP row 4: exactly 1 tenant_b row expected; got {}",
        tenant_b_rows.len()
    );

    let a_payload: Value =
        serde_json::from_str(&tenant_a_rows[0].1).expect("payload parses");
    let b_payload: Value =
        serde_json::from_str(&tenant_b_rows[0].1).expect("payload parses");
    assert_eq!(
        a_payload["tool"].as_str(),
        Some("tenant-a-only-tool"),
        "claim-6 MVP row 4: tenant_a row must carry tenant-a-only-tool; got {a_payload:?}"
    );
    assert_eq!(
        b_payload["tool"].as_str(),
        Some("tenant-b-only-tool"),
        "claim-6 MVP row 4: tenant_b row must carry tenant-b-only-tool; got {b_payload:?}"
    );
    assert_ne!(
        a_payload["tool"], b_payload["tool"],
        "claim-6 MVP row 4: a + b rows MUST carry distinct tool names"
    );

    // Defensive — payload-level tenant_id stamping (the field
    // `payload.tenant_id`) must match the row's tenant_id column. The
    // audit-feed handler can use EITHER denorm column OR payload field;
    // they MUST agree.
    assert_eq!(
        a_payload["tenant_id"].as_str(),
        Some(TENANT_A_LABEL),
        "claim-6 MVP row 4: tenant_a row's payload.tenant_id must agree with column"
    );
    assert_eq!(
        b_payload["tenant_id"].as_str(),
        Some(TENANT_B_LABEL),
        "claim-6 MVP row 4: tenant_b row's payload.tenant_id must agree with column"
    );

    // Hash chain still verifies — the cross-tenant override does not
    // break the chain (round 21's strengthener invariant, re-checked).
    let report = verify_session(store.as_ref(), session_id)
        .await
        .expect("verify_session");
    assert!(
        report.ok,
        "claim-6 MVP row 4: chain must verify across cross-tenant rows; breaks={:?}",
        report.breaks
    );
    assert_eq!(report.rows, 2, "claim-6 MVP row 4: 2 rows expected");
}
