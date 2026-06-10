//! Public-surface contract pins for `argo-types`.
//!
//! Authored by `pin-surface-contract-argo-types-store-audit`
//! (test-health audit 2026-06-10, F4). `argo-types` is the
//! canonical-types home with **zero dependencies on other Argo
//! crates** — the whole point of the `AuditRow` / `SessionId` / `Role`
//! canonicalization. Every other crate consumes these symbols, so a
//! silent re-export drift, renamed enum variant, or changed
//! constructor signature here is the worst-case data-integrity hazard.
//! These imports + type-level fences make such a drift fail loud at
//! compile time rather than ripple silently through 50+ downstream
//! crates.

#![allow(dead_code)]

// 1. Re-export inventory fence. This `use` block fails to COMPILE if
//    any canonical symbol is renamed or dropped from `lib.rs`. The
//    grouping mirrors the `pub use` blocks in `lib.rs`.
use argo_types::{
    AgentId, AgentName, ApprovalDecision, ArgoError, ArgoResult, Budget, BudgetExhausted,
    BudgetKind, BudgetNode, BudgetWindow, Capability, CapabilityRole, CapabilitySet, CapScope,
    ContentBlock, ForgeRunId, JudgeRecord, JudgeRecordId, JudgeSubject, JudgeVerdict, Lock, Message,
    MessageId, MessageRole, PlanProvenance, Principal, ProviderModel, Redacted, RequestId,
    RoleBinding, RoutingDecision, RoutingDecisionId, RoutingPolicy, SessionId, SkillId, SpawnKind,
    TenantId, TieredRouting, ToolCallId, ToolCallRequest, ToolCallResult, WorkspaceId,
};
use serde_json::json;

// ---------------------------------------------------------------------------
// 2. Canonical identifiers: constructor signatures + tuple-field shape.
//    These are plumbed through every primitive; a `new()` arity change
//    or a `.0` type change breaks the workspace-wide id contract.
// ---------------------------------------------------------------------------

#[test]
fn id_constructors_and_field_shapes_are_stable() {
    // Constructor fences — `fn() -> Self` for every canonical id.
    let _: fn() -> SessionId = SessionId::new;
    let _: fn() -> TenantId = TenantId::new;
    let _: fn() -> WorkspaceId = WorkspaceId::new;
    let _: fn() -> AgentId = AgentId::new;
    let _: fn() -> MessageId = MessageId::new;
    let _: fn() -> ToolCallId = ToolCallId::new;
    let _: fn() -> SkillId = SkillId::new;
    let _: fn() -> RequestId = RequestId::new;
    let _: fn() -> JudgeRecordId = JudgeRecordId::new;
    let _: fn() -> RoutingDecisionId = RoutingDecisionId::new;
    let _: fn() -> ForgeRunId = ForgeRunId::new;

    // The `.0` field stays a `uuid::Uuid` (v7). Downstream blob/round
    // -trip code reads `id.0` directly.
    let session = SessionId::new();
    let uuid: uuid::Uuid = session.0;
    assert_eq!(uuid.get_version_num(), 7);

    // `AgentName` is the lone string-newtype id; `.0` is a `String`.
    let name = AgentName("riley-ar".into());
    let inner: String = name.0;
    assert_eq!(inner, "riley-ar");
}

// ---------------------------------------------------------------------------
// 3. Capability model — the authorization unit. Pin the `covers`
//    signature, the load-bearing enum variants, and `CapabilityRole`
//    (the canonical "Role" home).
// ---------------------------------------------------------------------------

#[test]
fn capability_set_covers_signature_and_semantics_are_stable() {
    let _: fn(&CapabilitySet, &Capability) -> bool = CapabilitySet::covers;
    let _: fn() -> CapabilitySet = CapabilitySet::empty;

    let granted = CapabilitySet(vec![Capability::FsRead {
        path_glob: "/tmp/**".into(),
    }]);
    assert!(granted.covers(&Capability::FsRead {
        path_glob: "/tmp/foo/bar.txt".into(),
    }));
    assert!(!granted.covers(&Capability::FsWrite {
        path_glob: "/tmp/foo".into(),
    }));

    // Spawn coverage compares `SpawnKind` for equality (strictly
    // stronger variants must not leak).
    let spawn = CapabilitySet(vec![Capability::Spawn {
        spawn_kind: SpawnKind::Subprocess,
    }]);
    assert!(spawn.covers(&Capability::Spawn {
        spawn_kind: SpawnKind::Subprocess,
    }));
    assert!(!spawn.covers(&Capability::Spawn {
        spawn_kind: SpawnKind::BackgroundProcess,
    }));
}

#[test]
fn capability_serde_tag_and_unit_variants_are_stable() {
    // Tagged-enum wire shape: `{ "kind": "net_egress", ... }`.
    let cap = Capability::NetEgress {
        host_glob: "*.example.com".into(),
        port: Some(443),
    };
    assert_eq!(serde_json::to_value(&cap).unwrap()["kind"], "net_egress");

    // Unit variants the runtime gates on by exact match.
    for unit in [
        Capability::AuditWrite,
        Capability::MemoryWrite,
        Capability::MemoryWriteTeam,
        Capability::SkillWrite,
        Capability::SkillJudge,
        Capability::ForgeRun,
        Capability::WorkerFreeze,
        Capability::ImageGenerate,
        Capability::LocalCodeExecution,
    ] {
        // Each unit variant covers itself and nothing else trivially.
        let set = CapabilitySet(vec![unit.clone()]);
        assert!(set.covers(&unit));
    }
}

#[test]
fn capability_role_field_shape_is_stable() {
    // The canonical "Role" home: name + capabilities + single
    // -inheritance parent.
    let role = CapabilityRole {
        name: "owner".into(),
        capabilities: CapabilitySet(vec![Capability::AuditWrite]),
        inherits_from: Some("admin".into()),
    };
    assert_eq!(role.name, "owner");
    assert_eq!(role.inherits_from.as_deref(), Some("admin"));

    let binding = RoleBinding {
        principal: Principal::Forge,
        role_name: "builder".into(),
    };
    assert_eq!(binding.role_name, "builder");
}

// ---------------------------------------------------------------------------
// 4. Judge verdict — the audit-recorded adjudication enum.
// ---------------------------------------------------------------------------

#[test]
fn judge_verdict_serde_tag_is_stable() {
    assert_eq!(
        serde_json::to_value(JudgeVerdict::Success).unwrap()["verdict"],
        "success"
    );
    assert_eq!(
        serde_json::to_value(JudgeVerdict::Failure {
            reason: "no coverage".into(),
        })
        .unwrap()["verdict"],
        "failure"
    );
    // `JudgeSubject` tag fence.
    let subject = JudgeSubject::DelegateOutcome {
        delegate_session: SessionId::new(),
    };
    assert_eq!(
        serde_json::to_value(&subject).unwrap()["kind"],
        "delegate_outcome"
    );
    // `JudgeRecord` carries the verdict + provider model; field fence.
    let _record_field: fn(&JudgeRecord) -> &JudgeVerdict = |r| &r.verdict;
}

// ---------------------------------------------------------------------------
// 5. Error contract — variant → failure-mode mapping is load-bearing
//    for the gateway HTTP-status triage. Pin the variant shapes and
//    the `ArgoResult` / `BudgetWindow` aliases.
// ---------------------------------------------------------------------------

#[test]
fn argo_error_variants_and_aliases_are_stable() {
    // `ArgoResult<T>` resolves to `Result<T, ArgoError>` — typed-binding
    // fence (the error type is exercised by the variant asserts below).
    let _ok: ArgoResult<u8> = Ok(1);

    // `BudgetWindow` is a type alias of `CapScope`.
    let _window: BudgetWindow = CapScope::Day;

    // Display contracts the gateway/audit layers parse against.
    assert!(format!("{}", ArgoError::Interrupted).contains("interrupted"));
    assert!(format!(
        "{}",
        ArgoError::CapabilityDenied(Capability::AuditWrite)
    )
    .starts_with("capability denied"));
    assert!(format!(
        "{}",
        ArgoError::BudgetExhausted {
            kind: BudgetKind::Iterations,
        }
    )
    .starts_with("budget exhausted"));
    assert!(format!("{}", ArgoError::NotFound("session: 1".into())).starts_with("not found"));

    // `From<std::io::Error>` auto-conversion stays wired.
    let io: ArgoError = std::io::Error::other("boom").into();
    assert!(matches!(io, ArgoError::Io(_)));
}

// ---------------------------------------------------------------------------
// 6. Approval decision — channel-token + store-label string contract.
//    A drift here corrupts signed approval tokens AND stored rows.
// ---------------------------------------------------------------------------

#[test]
fn approval_decision_string_contracts_are_stable() {
    let _: fn(ApprovalDecision) -> &'static str = ApprovalDecision::as_token_str;
    let _: fn(ApprovalDecision) -> &'static str = ApprovalDecision::as_store_str;
    let _: fn(&str) -> Option<ApprovalDecision> = ApprovalDecision::parse_token_str;

    assert_eq!(ApprovalDecision::Approve.as_token_str(), "approve");
    assert_eq!(ApprovalDecision::Reject.as_token_str(), "reject");
    assert_eq!(ApprovalDecision::Approve.as_store_str(), "granted");
    assert_eq!(ApprovalDecision::Reject.as_store_str(), "denied");
    assert_eq!(
        ApprovalDecision::parse_token_str("approve"),
        Some(ApprovalDecision::Approve)
    );
    assert_eq!(ApprovalDecision::parse_token_str("nope"), None);

    // Serde wire form for the decision itself.
    assert_eq!(json!(ApprovalDecision::Approve), json!("approve"));
}

// ---------------------------------------------------------------------------
// 7. Canonical message / routing / budget type inventory. These are
//    consumed workspace-wide; naming each here makes a rename or a
//    removal from the re-export set a compile error.
// ---------------------------------------------------------------------------

#[test]
fn canonical_message_routing_budget_types_are_named() {
    let _: Option<Message> = None;
    let _: Option<MessageRole> = None;
    let _: Option<ContentBlock> = None;
    let _: Option<ToolCallRequest> = None;
    let _: Option<ToolCallResult> = None;
    let _: Option<ProviderModel> = None;
    let _: Option<RoutingDecision> = None;
    let _: Option<RoutingPolicy> = None;
    let _: Option<TieredRouting> = None;
    let _: Option<PlanProvenance> = None;
    let _: Option<Lock> = None;
    let _: Option<Budget> = None;
    let _: Option<BudgetNode> = None;
    let _: Option<BudgetExhausted> = None;
    let _: Option<Redacted<String>> = None;
}
