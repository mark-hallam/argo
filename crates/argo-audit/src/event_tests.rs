//! Unit tests for [`crate::event`]. Included via `#[path]` from
//! `event.rs` so the tests stay private to the crate while keeping
//! `event.rs` under the 500-LOC ceiling.
//!
// TODO(post-MVP): file at 512 LOC under the OVERNIGHT DIRECTIVE §10
// 550-LOC soft ceiling (window 2026-05-25→26). Split into variant-
// cluster sibling files per `fix-split-event-tests-rs-loc-budget-breach`
// (the original P1 brief, downgraded to P2 once the soft ceiling
// landed). Tracked at
// `docs/skycot/agents/heartbeats/loc-overage-tracking.md`.
use super::AuditKind;

#[test]
fn label_round_trip_for_every_kind() {
    for k in AuditKind::all() {
        assert_eq!(AuditKind::from_label(k.label()), Some(k), "{:?}", k);
    }
}

#[test]
fn all_kind_count_tracks_declared_variants() {
    assert_eq!(AuditKind::all().len(), 77);
}

#[test]
fn claim_truth_boundary_kinds_round_trip_and_listed() {
    for (label, kind) in [
        ("claim.allowed", AuditKind::ClaimAllowed),
        ("claim.blocked", AuditKind::ClaimBlocked),
        ("claim.escalated", AuditKind::ClaimEscalated),
    ] {
        assert_eq!(AuditKind::from_label(label), Some(kind));
        assert_eq!(kind.label(), label);
        assert!(AuditKind::all().contains(&kind));
    }
}

#[test]
fn hipaa_compliance_check_round_trips_and_listed() {
    let kind = AuditKind::HipaaComplianceCheck;
    assert_eq!(AuditKind::from_label("hipaa_compliance_check"), Some(kind));
    assert_eq!(kind.label(), "hipaa_compliance_check");
    let labels: Vec<&'static str> = AuditKind::all().iter().map(|k| k.label()).collect();
    assert!(labels.contains(&"hipaa_compliance_check"));
}

#[test]
fn trust_grade_changed_kind_round_trips_and_listed() {
    assert_eq!(
        AuditKind::from_label("trust_grade_changed"),
        Some(AuditKind::TrustGradeChanged)
    );
    assert_eq!(AuditKind::TrustGradeChanged.label(), "trust_grade_changed");
    let labels: Vec<&'static str> = AuditKind::all().iter().map(|k| k.label()).collect();
    assert!(labels.contains(&"trust_grade_changed"));
    assert_eq!(
        serde_json::to_string(&AuditKind::TrustGradeChanged).unwrap(),
        "\"trust_grade_changed\""
    );
}

#[test]
fn unknown_label_yields_none() {
    assert!(AuditKind::from_label("not_a_kind").is_none());
}

#[test]
fn escalation_required_is_listed() {
    // Locked: gateway rollups, CLI filters, and the verifier
    // all walk `AuditKind::all()` — escalation.required must
    // be enumerable there.
    let labels: Vec<&'static str> = AuditKind::all().iter().map(|k| k.label()).collect();
    assert!(labels.contains(&"escalation.required"), "got: {labels:?}");
    assert_eq!(
        AuditKind::from_label("escalation.required"),
        Some(AuditKind::EscalationRequired)
    );
}

#[test]
fn capability_gap_round_trips() {
    let kind = AuditKind::CapabilityGap;
    assert_eq!(AuditKind::from_label("capability.gap"), Some(kind));
    assert_eq!(kind.label(), "capability.gap");
    let labels: Vec<&'static str> = AuditKind::all().iter().map(|k| k.label()).collect();
    assert!(labels.contains(&"capability.gap"));
}

#[test]
fn worker_oom_restart_round_trips_and_listed() {
    let kind = AuditKind::WorkerOomRestart;
    assert_eq!(AuditKind::from_label("worker.oom_restart"), Some(kind));
    assert_eq!(kind.label(), "worker.oom_restart");
    let labels: Vec<&'static str> = AuditKind::all().iter().map(|k| k.label()).collect();
    assert!(labels.contains(&"worker.oom_restart"));
}

#[test]
fn worker_freeze_blocked_round_trips_and_serdes_as_snake_case() {
    let kind = AuditKind::WorkerFreezeBlocked;
    assert_eq!(AuditKind::from_label("worker_freeze_blocked"), Some(kind));
    assert_eq!(kind.label(), "worker_freeze_blocked");
    assert_eq!(
        serde_json::to_string(&kind).unwrap(),
        "\"worker_freeze_blocked\""
    );
    assert_eq!(
        serde_json::from_str::<AuditKind>("\"worker_freeze_blocked\"").unwrap(),
        kind
    );
    let labels: Vec<&'static str> = AuditKind::all().iter().map(|k| k.label()).collect();
    assert!(labels.contains(&"worker_freeze_blocked"));
}

#[test]
fn runtime_context_compressed_round_trips_and_listed() {
    let kind = AuditKind::RuntimeContextCompressed;
    assert_eq!(
        AuditKind::from_label("runtime.context_compressed"),
        Some(kind)
    );
    assert_eq!(kind.label(), "runtime.context_compressed");
    let labels: Vec<&'static str> = AuditKind::all().iter().map(|k| k.label()).collect();
    assert!(labels.contains(&"runtime.context_compressed"));
}

#[test]
fn retro_kinds_round_trip_and_listed() {
    for (label, kind) in [
        ("retro.created", AuditKind::RetroCreated),
        ("retro.finalized", AuditKind::RetroFinalized),
    ] {
        assert_eq!(AuditKind::from_label(label), Some(kind));
        assert_eq!(kind.label(), label);
    }
    let labels: Vec<&'static str> = AuditKind::all().iter().map(|k| k.label()).collect();
    assert!(labels.contains(&"retro.created"));
    assert!(labels.contains(&"retro.finalized"));
}

#[test]
fn security_key_rotated_round_trips_and_listed() {
    let kind = AuditKind::SecurityKeyRotated;
    assert_eq!(AuditKind::from_label("security.key_rotated"), Some(kind));
    assert_eq!(kind.label(), "security.key_rotated");
    let labels: Vec<&'static str> = AuditKind::all().iter().map(|k| k.label()).collect();
    assert!(labels.contains(&"security.key_rotated"));
}

#[test]
fn service_token_binding_expired_round_trips_and_listed() {
    let kind = AuditKind::ServiceTokenBindingExpired;
    assert_eq!(
        AuditKind::from_label("service_token_binding.expired"),
        Some(kind)
    );
    assert_eq!(kind.label(), "service_token_binding.expired");
    let labels: Vec<&'static str> = AuditKind::all().iter().map(|k| k.label()).collect();
    assert!(labels.contains(&"service_token_binding.expired"));
}

#[test]
fn tenant_provisioned_round_trips_and_listed() {
    let kind = AuditKind::TenantProvisioned;
    assert_eq!(AuditKind::from_label("tenant.provisioned"), Some(kind));
    assert_eq!(kind.label(), "tenant.provisioned");
    let labels: Vec<&'static str> = AuditKind::all().iter().map(|k| k.label()).collect();
    assert!(labels.contains(&"tenant.provisioned"));
}

#[test]
fn data_export_requested_round_trips_and_listed() {
    let kind = AuditKind::DataExportRequested;
    assert_eq!(AuditKind::from_label("data_export.requested"), Some(kind));
    assert_eq!(kind.label(), "data_export.requested");
    let labels: Vec<&'static str> = AuditKind::all().iter().map(|k| k.label()).collect();
    assert!(labels.contains(&"data_export.requested"));
}

#[test]
fn capability_mutation_kinds_round_trip_and_serde_as_snake_case() {
    for (label, kind) in [
        ("role_binding_changed", AuditKind::RoleBindingChanged),
        (
            "dynamic_capability_granted",
            AuditKind::DynamicCapabilityGranted,
        ),
    ] {
        assert_eq!(AuditKind::from_label(label), Some(kind));
        assert_eq!(kind.label(), label);
        assert_eq!(
            serde_json::to_string(&kind).unwrap(),
            format!("\"{label}\"")
        );
    }
    let labels: Vec<&'static str> = AuditKind::all().iter().map(|k| k.label()).collect();
    assert!(labels.contains(&"role_binding_changed"));
    assert!(labels.contains(&"dynamic_capability_granted"));
}

#[test]
fn gateway_insecure_bind_round_trips_and_listed() {
    let kind = AuditKind::GatewayInsecureBind;
    assert_eq!(AuditKind::from_label("gateway_insecure_bind"), Some(kind));
    assert_eq!(kind.label(), "gateway_insecure_bind");
    let labels: Vec<&'static str> = AuditKind::all().iter().map(|k| k.label()).collect();
    assert!(labels.contains(&"gateway_insecure_bind"));
}

#[test]
fn legacy_http_request_event_type_strings_still_parse_as_labels() {
    // Acceptance: the new variant labels were chosen to match the
    // historical `event_type` payload strings emitted while the
    // `HttpRequest + event_type` workaround was in place. Audit-feed
    // clients that filtered on those strings keep working unchanged
    // when they switch to the new top-level `kind` column.
    for label in [
        "runtime.context_compressed",
        "retro.created",
        "retro.finalized",
        "security.key_rotated",
        "approval.expired_by_timeout",
    ] {
        assert!(
            AuditKind::from_label(label).is_some(),
            "legacy label {label} must now parse as a first-class kind"
        );
    }
}

#[test]
fn workplace_knowledge_doc_kinds_round_trip_and_listed() {
    // Stream `mission-workplace-knowledge-audit-kinds` — wiki doc
    // lifecycle (create / update / delete) promoted out of the
    // generic `HttpRequest` bucket so design-doc §7 Pillar 1
    // (`SELECT kind, payload FROM audit_record WHERE kind =
    // 'workplace_knowledge.doc.updated'`) works against named rows.
    let kinds = [
        (
            "workplace_knowledge.doc.created",
            AuditKind::WorkplaceKnowledgeDocCreated,
        ),
        (
            "workplace_knowledge.doc.updated",
            AuditKind::WorkplaceKnowledgeDocUpdated,
        ),
        (
            "workplace_knowledge.doc.deleted",
            AuditKind::WorkplaceKnowledgeDocDeleted,
        ),
    ];
    for (label, kind) in kinds {
        assert_eq!(AuditKind::from_label(label), Some(kind));
        assert_eq!(kind.label(), label);
    }
    let labels: Vec<&'static str> = AuditKind::all().iter().map(|k| k.label()).collect();
    for (label, _) in kinds {
        assert!(labels.contains(&label), "missing {label}: {labels:?}");
    }
}

#[test]
fn skill_review_kinds_round_trip() {
    for (label, kind) in [
        ("skill.approved", AuditKind::SkillApproved),
        ("skill.rejected", AuditKind::SkillRejected),
        (
            "skill.candidate.promoted",
            AuditKind::SkillCandidatePromoted,
        ),
        ("skill.rollback", AuditKind::SkillRolledBack),
    ] {
        assert_eq!(AuditKind::from_label(label), Some(kind));
        assert_eq!(kind.label(), label);
    }
    let labels: Vec<&'static str> = AuditKind::all().iter().map(|k| k.label()).collect();
    assert!(labels.contains(&"skill.approved"));
    assert!(labels.contains(&"skill.rejected"));
    assert!(labels.contains(&"skill.candidate.promoted"));
    assert!(labels.contains(&"skill.rollback"));
}
