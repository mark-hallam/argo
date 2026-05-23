//! # Mission Claim 6 MVP — approval audit *kinds* contract pin
//!
//! Sibling pin to `claim_6_approval_audit_row_shapes.rs`. That file
//! pins the **row shapes** for the kinds that *currently* emit in
//! production. This file pins the **kind contract** for the four
//! kinds referenced by the row-4 mission-dashboard cell:
//!
//! > `ApprovalGranted` audit row on approve; `ApprovalDeclined` on
//! > decline (mission wording — actual kinds: `ApprovalReplied`
//! > carries approve/reject in `reply_kind`; CLI flow uses
//! > `ApprovalGranted` + `ApprovalRevoked`).
//!
//! The original `mission-claim-6-row-4-approval-audit-kinds-pin`
//! brief was blocked because `AuditKind::ApprovalDeclined` did not
//! exist. `mission-add-approval-audit-kinds` (this brief) adds the
//! missing variant + label so the row-4 pin can run.
//!
//! ## What this test proves
//!
//! For all four kinds named in the dashboard cell —
//! `ApprovalGranted`, `ApprovalDeclined`, `ApprovalReplied`,
//! `ApprovalRevoked` — the following stays true at the type level:
//!
//! 1. Each variant has a stable wire label.
//! 2. `from_label(label())` round-trips.
//! 3. `all()` lists every variant exactly once (no kind silently
//!    dropped from the iteration the chain-verifier walks).
//! 4. `serde_json` serialises each variant to a stable snake_case
//!    string (the persisted `kind` column on disk).
//!
//! Production emission of `ApprovalDeclined` is deferred: the
//! canonical decide path lives at
//! `argo-store::approvals::decide_request`, but `argo-store` cannot
//! depend on `argo-audit` (the dependency already runs the other
//! way) and the gateway approve route currently uses the
//! `worker_actions_transitions` flow, not `decide_request`. The
//! follow-up wire-up brief will route emission through whichever
//! caller composes the store + audit-sink dependencies (gateway
//! `routes_approvals_json::decide_response` is the leading
//! candidate).

use argo_audit::AuditKind;

const DASHBOARD_ROW_4_KINDS: &[(AuditKind, &str, &str)] = &[
    (AuditKind::ApprovalGranted, "approval.granted", "approval_granted"),
    (AuditKind::ApprovalDeclined, "approval.declined", "approval_declined"),
    (AuditKind::ApprovalReplied, "approval.replied", "approval_replied"),
    (AuditKind::ApprovalRevoked, "approval.revoked", "approval_revoked"),
];

#[test]
fn dashboard_row_4_kinds_round_trip() {
    for &(kind, label, _) in DASHBOARD_ROW_4_KINDS {
        assert_eq!(kind.label(), label, "label() for {kind:?}");
        assert_eq!(
            AuditKind::from_label(label),
            Some(kind),
            "from_label({label}) for {kind:?}",
        );
    }
}

#[test]
fn dashboard_row_4_kinds_listed_exactly_once() {
    let all: Vec<AuditKind> = AuditKind::all();
    for &(kind, _, _) in DASHBOARD_ROW_4_KINDS {
        let count = all.iter().filter(|k| **k == kind).count();
        assert_eq!(count, 1, "{kind:?} appears {count}× in AuditKind::all()");
    }
}

#[test]
fn dashboard_row_4_kinds_serde_shape_is_snake_case() {
    for &(kind, _, serde_label) in DASHBOARD_ROW_4_KINDS {
        let json = serde_json::to_string(&kind).expect("serialise");
        assert_eq!(json, format!("\"{serde_label}\""), "serde for {kind:?}");
        let back: AuditKind = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(back, kind, "serde round-trip for {kind:?}");
    }
}
