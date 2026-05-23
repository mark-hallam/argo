//! # Mission Claim 6 LIVE — cross-tenant approval-row existence-leak prevention
//!
//! Pins sub-test (b) of the operator-locked Claim 6 LIVE criteria
//! (round 18 draft, cited in `docs/skycot/evals/scheduled/2026-05-23/
//! mission-vs-code-gap-manifest.md`):
//!
//! > **Claim 6b LIVE (cross-tenant)** — tenant_b cannot see tenant_a's
//! > pending approvals via any API (`GET /api/v1/approvals`,
//! > `GET /api/v1/approvals/bundles`, `POST /api/v1/audit/export`);
//! > attempting to decide on an approval id from another tenant
//! > returns 404 (not 403) per the operator-locked existence-leak
//! > policy.
//!
//! The gateway-level "404 not 403" test lives outside this brief's
//! may-touch list (it touches `argo-cli/tests/`). What this file
//! pins is the **audit-substrate contract that lets the gateway
//! return 404 in the first place**: a SELECT scoped to tenant_b's
//! tenant_id must return ZERO rows for an `approval_id` owned by
//! tenant_a — even when the approval EXISTS in the table. If this
//! contract held but the gateway returned 403, the gateway would be
//! the bug; if this contract fails (tenant_b's SELECT sees
//! tenant_a's row), the bug is here.
//!
//! ## What this test proves
//!
//! 1. **Approval-row tenant filter — happy path.** A `SELECT WHERE
//!    tenant_id = ? AND kind = 'approval.asked'` for tenant_a returns
//!    exactly the rows tenant_a wrote. The substrate honors the
//!    tenant column.
//! 2. **Approval-row tenant filter — wrong-tenant path.** The SAME
//!    SELECT for tenant_b returns ZERO rows for tenant_a's
//!    approval_ids. Tenant_b CANNOT observe tenant_a's approval
//!    existence through this query — the "404 not 403" substrate.
//! 3. **Cross-kind sweep.** The 404-substrate contract holds across
//!    every approval-related `AuditKind`:
//!     - `ApprovalAsked` (outbound notification audit row)
//!     - `ApprovalGranted` (CLI tool-grant audit row)
//!     - `ApprovalReplied` (operator's Slack/email reply)
//!     - `ApprovalRevoked` (CLI revoke)
//!
//!    Every one of them must be invisible to the other tenant.
//! 4. **action_id existence-leak prevention.** A
//!    `SELECT WHERE tenant_id = ? AND json_extract(payload_json,
//!    '$.action_id') = ?` query — the substrate query the gateway's
//!    approval-by-id lookup will use — returns ZERO rows when
//!    tenant_b queries an action_id owned by tenant_a. This is the
//!    load-bearing pin for "decide-on-other-tenant's-approval → 404,
//!    not 403."
//! 5. **Audit-export substrate.** A blanket `SELECT WHERE
//!    tenant_id = ?` (the shape used by `POST /api/v1/audit/export`)
//!    returns ONLY the rows for the matching tenant — tenant_b's
//!    export must never contain tenant_a's approval rows.
//! 6. **Hash chain stays intact.** After the cross-tenant write mix,
//!    `verify_session` still returns `ok = true, breaks = []`. The
//!    tenant-overlay path doesn't disturb the chain contract pinned
//!    by `claim_6_audit_chain_verify`.
//!
//! ## Why this is the missing pin
//!
//! `claim_6_approval_audit_row_shapes.rs::claim_6_mvp_row_4_cross_tenant_audit_isolation_under_select`
//! already pins cross-tenant SELECT isolation for `ApprovalGranted`
//! at the column level — that's the round-27 baseline. This file
//! deepens that baseline along three axes the round-18 LIVE criterion
//! cares about specifically:
//!
//!  * Coverage across all FOUR approval `AuditKind` variants (not
//!    just `ApprovalGranted`).
//!  * action_id-style existence-leak query (the "decide by id →
//!    404" substrate), not just kind-and-tenant filtering.
//!  * Audit-export shape (the `POST /api/v1/audit/export` substrate).
//!
//! Together with the gateway-side route tests, these pin the
//! end-to-end "404 not 403" contract for tenant_b's view of
//! tenant_a's approvals.
//!
//! ## Authored
//!
//! Eval brief: `eval-claim-6-live-cross-tenant-audit-chain-test`
//! (round 18 draft, see `docs/skycot/evals/scheduled/2026-05-23/
//! draft-live-criteria-claims-4-5-6.md`).

use std::sync::Arc;

use argo_audit::{verify_session, AuditKind, AuditSink};
use argo_store::SessionStore;
use argo_types::{SessionId, TenantId, WorkspaceId};
use serde_json::{json, Value};

const TENANT_A_LABEL: &str = "tenant-a-claim-6-live-cross-tenant";
const TENANT_B_LABEL: &str = "tenant-b-claim-6-live-cross-tenant";

/// Approval kinds the gateway exposes; each must satisfy the
/// "tenant_b SELECT → 0 rows" substrate contract.
const APPROVAL_KINDS: &[(AuditKind, &str)] = &[
    (AuditKind::ApprovalAsked, "approval.asked"),
    (AuditKind::ApprovalGranted, "approval.granted"),
    (AuditKind::ApprovalReplied, "approval.replied"),
    (AuditKind::ApprovalRevoked, "approval.revoked"),
];

/// Build a fresh in-memory store + a seeded session row used as the
/// `session_id` foreign key on every audit write. Returns the store
/// + session so callers can issue `write_with_tenant` for each side.
async fn fresh_store_and_session() -> (Arc<SessionStore>, SessionId) {
    let store = Arc::new(SessionStore::open(":memory:").await.expect("open"));
    store.migrate().await.expect("migrate");
    let tenant = TenantId::new();
    let workspace = WorkspaceId::new();
    sqlx::query(
        "INSERT INTO workspace (id, tenant_id, name, root_path, created_at)
         VALUES (?, ?, 'claim-6-live-cross-tenant', '.', strftime('%s','now'))",
    )
    .bind(workspace.0.as_bytes().as_slice())
    .bind(tenant.0.as_bytes().as_slice())
    .execute(store.pool())
    .await
    .expect("seed workspace");
    let session = store
        .create_session(tenant, workspace, "claude", "anthropic", Some("t"), None)
        .await
        .expect("seed session");
    (store, session)
}

fn approval_payload(kind: AuditKind, action_id: &str, tag: &str) -> Value {
    match kind {
        AuditKind::ApprovalAsked => json!({
            "action_id": action_id,
            "channel": "slack",
            "risk_level": "high",
            "tag": tag,
        }),
        AuditKind::ApprovalGranted => json!({
            "action_id": action_id,
            "tool": "fb_marketplace.post",
            "grant_id": format!("grant-{tag}"),
            "source": "cli",
            "one_shot": false,
            "tag": tag,
        }),
        AuditKind::ApprovalReplied => json!({
            "action_id": action_id,
            "channel": "slack",
            "replier": "human:operator",
            "reply_kind": "approve",
            "tag": tag,
        }),
        AuditKind::ApprovalRevoked => json!({
            "action_id": action_id,
            "tool": "fb_marketplace.post",
            "removed": 1,
            "source": "cli",
            "tag": tag,
        }),
        // Test seeds only the 4 approval kinds; any other kind reaching
        // this helper is a programmer error.
        other => panic!("approval_payload: unexpected kind {other:?}"),
    }
}

#[tokio::test]
async fn claim_6_live_cross_tenant_select_no_existence_leak() {
    let (store, session_id) = fresh_store_and_session().await;
    // Use a single sink and the override path (write_with_tenant) so
    // both tenants' rows land in the same chain — exactly what
    // happens when the gateway's admin-session writes a row on behalf
    // of a non-default tenant.
    let sink = AuditSink::new_with_tenant(Arc::clone(&store), TENANT_A_LABEL.into());

    // ── Seed: tenant_a writes one row of each approval kind with a
    // distinct action_id. tenant_b writes the SAME 4 kinds with
    // DIFFERENT action_ids.
    for (kind, _label) in APPROVAL_KINDS {
        let action_id = format!("act-a-{}", kind.label());
        sink.write_with_tenant(
            Some(session_id),
            *kind,
            approval_payload(*kind, &action_id, "a-side"),
            None,
            TENANT_A_LABEL,
        )
        .await
        .unwrap_or_else(|e| panic!("write tenant_a {kind:?} failed: {e}"));
    }
    for (kind, _label) in APPROVAL_KINDS {
        let action_id = format!("act-b-{}", kind.label());
        sink.write_with_tenant(
            Some(session_id),
            *kind,
            approval_payload(*kind, &action_id, "b-side"),
            None,
            TENANT_B_LABEL,
        )
        .await
        .unwrap_or_else(|e| panic!("write tenant_b {kind:?} failed: {e}"));
    }
    sink.flush_pending().await.expect("flush");

    // ── Assertion 1+2: kind-and-tenant filter.
    // For every approval kind, the tenant_a-scoped SELECT must return
    // exactly 1 row (the row tenant_a wrote), and the tenant_b-scoped
    // SELECT must NOT see any of tenant_a's rows for that kind. The
    // total row count for the kind is 2 (1 per side); each side's
    // SELECT must see exactly its own.
    for (_kind, label) in APPROVAL_KINDS {
        let a_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM audit_record \
             WHERE session_id = ? AND tenant_id = ? AND kind = ?",
        )
        .bind(session_id.0.as_bytes().as_slice())
        .bind(TENANT_A_LABEL)
        .bind(label)
        .fetch_one(store.pool())
        .await
        .unwrap_or_else(|e| panic!("SELECT tenant_a count for {label} failed: {e}"));
        assert_eq!(
            a_count, 1,
            "claim-6 LIVE cross-tenant: tenant_a SELECT for kind={label} must return its own row"
        );

        let b_sees_a: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM audit_record \
             WHERE session_id = ? AND tenant_id = ? AND kind = ? \
             AND json_extract(payload_json, '$.action_id') LIKE 'act-a-%'",
        )
        .bind(session_id.0.as_bytes().as_slice())
        .bind(TENANT_B_LABEL)
        .bind(label)
        .fetch_one(store.pool())
        .await
        .unwrap_or_else(|e| panic!("SELECT tenant_b seeing-a count for {label} failed: {e}"));
        assert_eq!(
            b_sees_a, 0,
            "claim-6 LIVE cross-tenant: tenant_b SELECT for kind={label} must NEVER see tenant_a's row — \
             this is the 404-not-403 substrate contract"
        );
    }

    // ── Assertion 3: action_id existence-leak prevention.
    // The substrate query the gateway's "decide by approval id" path
    // uses: SELECT by (tenant_id, action_id). When tenant_b queries
    // for tenant_a's action_id, the result MUST be empty — that's
    // what lets the gateway return 404 (not 403, which would confirm
    // existence). Cover all 4 kinds.
    for (kind, label) in APPROVAL_KINDS {
        let a_action_id = format!("act-a-{}", kind.label());
        // First confirm the row IS in the table under tenant_a (sanity:
        // if this fails, the test plumbing is wrong before we trust
        // the negative result).
        let a_sees_own: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM audit_record \
             WHERE session_id = ? AND tenant_id = ? AND kind = ? \
             AND json_extract(payload_json, '$.action_id') = ?",
        )
        .bind(session_id.0.as_bytes().as_slice())
        .bind(TENANT_A_LABEL)
        .bind(label)
        .bind(&a_action_id)
        .fetch_one(store.pool())
        .await
        .expect("a-sees-own SELECT");
        assert_eq!(
            a_sees_own, 1,
            "claim-6 LIVE cross-tenant: sanity — tenant_a must see its own action_id={a_action_id}"
        );

        // Now the load-bearing assertion: tenant_b's same-action_id
        // SELECT MUST return 0.
        let b_sees_a_by_action_id: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM audit_record \
             WHERE session_id = ? AND tenant_id = ? AND kind = ? \
             AND json_extract(payload_json, '$.action_id') = ?",
        )
        .bind(session_id.0.as_bytes().as_slice())
        .bind(TENANT_B_LABEL)
        .bind(label)
        .bind(&a_action_id)
        .fetch_one(store.pool())
        .await
        .expect("b-sees-a-by-action-id SELECT");
        assert_eq!(
            b_sees_a_by_action_id, 0,
            "claim-6 LIVE cross-tenant: tenant_b SELECT for action_id={a_action_id} (owned by tenant_a) \
             must return 0 rows — substrate contract for 404-not-403 decide-by-id"
        );
    }

    // ── Assertion 4: audit-export shape.
    // The gateway's `POST /api/v1/audit/export` filters by tenant_id
    // at the SQL layer (the round-13 fix landed in #2051). At the
    // substrate, that shape is `SELECT * FROM audit_record WHERE
    // tenant_id = ?`. tenant_b's export must NEVER contain
    // tenant_a's approval rows.
    let b_export_rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT kind, payload_json FROM audit_record WHERE tenant_id = ? ORDER BY seq ASC",
    )
    .bind(TENANT_B_LABEL)
    .fetch_all(store.pool())
    .await
    .expect("tenant_b export-shape SELECT");
    assert_eq!(
        b_export_rows.len(),
        APPROVAL_KINDS.len(),
        "claim-6 LIVE cross-tenant: tenant_b export must return exactly its own approval rows; got {}",
        b_export_rows.len()
    );
    for (_kind_label, payload_str) in &b_export_rows {
        let payload: Value = serde_json::from_str(payload_str).expect("payload parses");
        let action_id = payload["action_id"]
            .as_str()
            .expect("each approval payload carries action_id");
        assert!(
            action_id.starts_with("act-b-"),
            "claim-6 LIVE cross-tenant: tenant_b export row must reference an act-b-* action_id; got {action_id}"
        );
        assert!(
            !action_id.starts_with("act-a-"),
            "claim-6 LIVE cross-tenant: tenant_b export row LEAKED a tenant_a action_id ({action_id}) — \
             this is the round-13 audit-export tenant-blind regression class"
        );
        // Payload-level tenant_id stamp must agree.
        assert_eq!(
            payload["tenant_id"].as_str(),
            Some(TENANT_B_LABEL),
            "claim-6 LIVE cross-tenant: tenant_b export row's payload.tenant_id must match column"
        );
    }

    // ── Assertion 5: chain integrity across mixed-tenant rows.
    // The cross-tenant override path must not break the hash chain
    // (round 21 strengthener invariant — re-checked here under the
    // mixed-tenant approval seed).
    let report = verify_session(store.as_ref(), session_id)
        .await
        .expect("verify_session");
    assert!(
        report.ok,
        "claim-6 LIVE cross-tenant: chain must verify across mixed-tenant approval rows; breaks={:?}",
        report.breaks
    );
    assert_eq!(
        report.rows,
        (APPROVAL_KINDS.len() * 2),
        "claim-6 LIVE cross-tenant: expected {} verified rows ({} kinds × 2 tenants); got {}",
        APPROVAL_KINDS.len() * 2,
        APPROVAL_KINDS.len(),
        report.rows
    );
    assert!(
        report.breaks.is_empty(),
        "claim-6 LIVE cross-tenant: mixed-tenant rows must produce zero chain breaks; got {:?}",
        report.breaks
    );
}

#[tokio::test]
async fn claim_6_live_cross_tenant_blanket_export_isolation() {
    // Defensive — the canonical audit-export SQL shape used by the
    // gateway is `SELECT WHERE tenant_id = ?`. Pin that this filter
    // is the ONLY thing standing between tenants — there's no
    // session_id, no kind, no payload field that would leak across.
    let (store, session_id) = fresh_store_and_session().await;
    let sink = AuditSink::new_with_tenant(Arc::clone(&store), TENANT_A_LABEL.into());

    // Seed 4 rows under tenant_a + 1 row under tenant_b. Use a
    // non-approval kind for the tenant_a rows (ToolCall) and an
    // approval kind for the tenant_b row, to demonstrate the tenant
    // filter is kind-agnostic.
    for i in 0..4 {
        sink.write(
            Some(session_id),
            AuditKind::ToolCall,
            json!({"i": i, "tool": "echo"}),
            None,
        )
        .await
        .expect("write tenant_a ToolCall");
    }
    sink.write_with_tenant(
        Some(session_id),
        AuditKind::ApprovalAsked,
        json!({"action_id": "act-b-blanket", "risk_level": "high"}),
        None,
        TENANT_B_LABEL,
    )
    .await
    .expect("write tenant_b ApprovalAsked");
    sink.flush_pending().await.expect("flush");

    // tenant_a export: exactly 4 rows, all ToolCall, all act-a-side.
    let a_rows: Vec<(String, Vec<u8>)> = sqlx::query_as(
        "SELECT kind, tenant_id FROM audit_record WHERE tenant_id = ? ORDER BY seq ASC",
    )
    .bind(TENANT_A_LABEL)
    .fetch_all(store.pool())
    .await
    .expect("tenant_a export SELECT");
    assert_eq!(
        a_rows.len(),
        4,
        "claim-6 LIVE cross-tenant blanket: tenant_a export must return 4 rows; got {}",
        a_rows.len()
    );
    let tool_call_label = AuditKind::ToolCall.label();
    for (kind, t) in &a_rows {
        assert_eq!(
            kind, tool_call_label,
            "tenant_a export row kind must match ToolCall label; got {kind}"
        );
        assert_eq!(
            t.as_slice(),
            TENANT_A_LABEL.as_bytes(),
            "tenant_a export row's tenant_id must match"
        );
    }

    // tenant_b export: exactly 1 row, the ApprovalAsked.
    let b_rows: Vec<(String, Vec<u8>)> = sqlx::query_as(
        "SELECT kind, tenant_id FROM audit_record WHERE tenant_id = ? ORDER BY seq ASC",
    )
    .bind(TENANT_B_LABEL)
    .fetch_all(store.pool())
    .await
    .expect("tenant_b export SELECT");
    assert_eq!(
        b_rows.len(),
        1,
        "claim-6 LIVE cross-tenant blanket: tenant_b export must return its 1 row; got {}",
        b_rows.len()
    );
    assert_eq!(b_rows[0].0, "approval.asked");
    assert_eq!(b_rows[0].1.as_slice(), TENANT_B_LABEL.as_bytes());

    // A blanket-without-filter SELECT sees all 5 rows — proves the
    // filter is necessary (not just incidentally always-matching).
    let total: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM audit_record WHERE session_id = ?")
            .bind(session_id.0.as_bytes().as_slice())
            .fetch_one(store.pool())
            .await
            .expect("total count");
    assert_eq!(
        total, 5,
        "claim-6 LIVE cross-tenant blanket: substrate must hold 5 total rows so the filter is genuinely \
         the load-bearing isolation primitive"
    );

    // Chain integrity holds.
    let report = verify_session(store.as_ref(), session_id)
        .await
        .expect("verify_session");
    assert!(
        report.ok,
        "claim-6 LIVE cross-tenant blanket: chain must verify; breaks={:?}",
        report.breaks
    );
}
