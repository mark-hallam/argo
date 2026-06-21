//! UF-A2: typed success-criteria → real, scored `[[kpi]]` targets.
//!
//! The materializer must emit the operator's CAPTURED success-criteria (A1)
//! as real `[[kpi]]` rows carrying their actual `kind` / `target` /
//! `comparator` (gate=Warn so dashboards score against them), NOT the generic
//! SLA skeleton. Two teammates built from DIFFERENT criteria therefore emit
//! DIFFERENT KPI targets, and each round-trips through the real loader.

use crate::materialize::{materialize_spec, write_capability_vertical};
use crate::spec::{CapabilityDecl, CapabilitySpec, EscalationSpec, ExecutorIntent, SuccessCriterion};
use crate::{Archetype, BroadRoleDraft};
use argo_vertical::{ActionRiskLevel, KpiComparator, KpiGate, KpiKind};

/// A from-scratch spec carrying explicit typed `success_criteria` — the richer
/// A1 intake path (`from_broad_draft` leaves `success_criteria` empty).
fn criteria_spec(persona: &str, criteria: Vec<SuccessCriterion>) -> CapabilitySpec {
    CapabilitySpec {
        charter: "a support lead who answers tickets fast".into(),
        persona: persona.into(),
        persona_description: "Owns the support queue.".into(),
        responsibilities: vec![],
        capabilities: vec![CapabilityDecl {
            action_name: "answer_ticket".into(),
            intent: "Answer a support ticket".into(),
            risk: ActionRiskLevel::Low,
            reversible: true,
            requires_approval: false,
            proposed_executor: ExecutorIntent::Human,
        }],
        success_metrics: vec!["tickets answered".into()],
        onboarding_script: "Hi.".into(),
        escalation: EscalationSpec::default(),
        nearest_archetype: Archetype::CustomerSupportAgent,
        from_scratch: true,
        cadence: Some("every weekday at 9am".into()),
        exceptions: vec![],
        success_criteria: criteria,
        access_integrations: vec![],
    }
}

#[test]
fn typed_success_criteria_emit_real_kpi_targets_gated_warn() {
    let spec = criteria_spec(
        "portia",
        vec![
            SuccessCriterion {
                name: "first response time".into(),
                kind: KpiKind::DurationMs,
                target: 60_000.0,
                comparator: KpiComparator::Lte,
            },
            SuccessCriterion {
                name: "csat".into(),
                kind: KpiKind::Score,
                target: 4.5,
                comparator: KpiComparator::Gte,
            },
        ],
    );
    let plan = materialize_spec(&spec).expect("materialize a criteria-driven teammate");
    let kpis = &plan.manifest.kpis;
    assert_eq!(kpis.len(), 2, "one [[kpi]] per captured criterion");

    // The operator's chosen targets ride through verbatim — NOT the SLA default.
    assert_eq!(kpis[0].name, "first_response_time");
    assert_eq!(kpis[0].kind, KpiKind::DurationMs);
    assert_eq!(kpis[0].threshold, 60_000.0);
    assert_eq!(kpis[0].comparator, KpiComparator::Lte);
    assert_eq!(kpis[1].threshold, 4.5);
    assert_eq!(kpis[1].comparator, KpiComparator::Gte);

    // gate=Warn → dashboards score against them (non-blocking).
    assert!(kpis.iter().all(|k| k.gate == KpiGate::Warn), "criteria are Warn-gated");

    // Typed criteria suppress the generic SLA skeleton entirely.
    assert!(
        plan.manifest.success_metrics.is_none(),
        "no generic SLA skeleton when real criteria were captured"
    );
    // The unified read path returns the typed rows.
    assert_eq!(plan.manifest.all_kpis().len(), 2);
}

#[test]
fn two_specs_with_different_criteria_produce_different_kpi_targets() {
    // The brief's acceptance criterion: two teammates built from DIFFERENT
    // success-criteria emit DIFFERENT KPI targets, and each round-trips.
    let fast = criteria_spec(
        "fastlane",
        vec![SuccessCriterion {
            name: "first response time".into(),
            kind: KpiKind::DurationMs,
            target: 30_000.0,
            comparator: KpiComparator::Lte,
        }],
    );
    let slow = criteria_spec(
        "slowlane",
        vec![SuccessCriterion {
            name: "first response time".into(),
            kind: KpiKind::DurationMs,
            target: 120_000.0,
            comparator: KpiComparator::Lte,
        }],
    );
    let fast_plan = materialize_spec(&fast).expect("materialize fast");
    let slow_plan = materialize_spec(&slow).expect("materialize slow");
    assert_eq!(fast_plan.manifest.all_kpis()[0].threshold, 30_000.0);
    assert_eq!(slow_plan.manifest.all_kpis()[0].threshold, 120_000.0);
    assert_ne!(
        fast_plan.manifest.all_kpis()[0].threshold,
        slow_plan.manifest.all_kpis()[0].threshold,
        "different criteria must score against different targets"
    );

    // Both round-trip through the REAL loader with zero validation errors.
    for (spec, plan) in [(&fast, &fast_plan), (&slow, &slow_plan)] {
        let dir = tempfile::tempdir().expect("tempdir");
        let out = dir.path().join(&plan.vertical_id);
        write_capability_vertical(spec, &out).expect("write the vertical");
        let (registry, errors) = argo_vertical::load_dir(dir.path()).expect("load_dir");
        assert!(errors.is_empty(), "criteria vertical load errors: {errors:?}");
        let loaded = registry.get(&plan.vertical_id).expect("loaded by id");
        assert_eq!(
            loaded.manifest.all_kpis()[0].threshold,
            plan.manifest.all_kpis()[0].threshold,
            "the captured target survives the manifest round-trip"
        );
    }
}

#[test]
fn duplicate_criterion_names_are_uniquified_not_rejected() {
    // Two criteria sanitising to the same slug must NOT collide into a
    // duplicate KPI name (validation rejects dups → would fail the hire).
    let spec = criteria_spec(
        "dupe",
        vec![
            SuccessCriterion {
                name: "quality".into(),
                kind: KpiKind::Score,
                target: 4.0,
                comparator: KpiComparator::Gte,
            },
            SuccessCriterion {
                name: "quality".into(),
                kind: KpiKind::Score,
                target: 5.0,
                comparator: KpiComparator::Gte,
            },
        ],
    );
    let plan = materialize_spec(&spec).expect("colliding names must not fail the hire");
    let names: Vec<&str> = plan.manifest.kpis.iter().map(|k| k.name.as_str()).collect();
    assert_eq!(names.len(), 2);
    assert_ne!(names[0], names[1], "the collision must be uniquified");

    // And the uniquified manifest still passes the loader's validation.
    let dir = tempfile::tempdir().expect("tempdir");
    let out = dir.path().join(&plan.vertical_id);
    write_capability_vertical(&spec, &out).expect("write");
    let (_registry, errors) = argo_vertical::load_dir(dir.path()).expect("load_dir");
    assert!(errors.is_empty(), "dup-name uniquification must validate: {errors:?}");
}

#[test]
fn no_typed_criteria_keeps_the_sla_skeleton_and_free_text_hints() {
    // Back-compat: when no typed criteria were captured, the legacy free-text
    // `success_metrics` still become Inform hints AND the SLA skeleton is kept
    // as the fallback floor (`from_broad_draft` leaves criteria empty).
    let draft = BroadRoleDraft {
        charter: "a vineyard coordinator who schedules harvests".into(),
        persona: "vinny".into(),
        persona_description: "Coordinates harvest scheduling.".into(),
        action_vocabulary: vec!["schedule_harvest".into()],
        success_metrics: vec!["zero missed harvest windows".into()],
        onboarding_script: "Hi.".into(),
        ..Default::default()
    };
    let spec = CapabilitySpec::from_broad_draft(draft, Archetype::CalendarCoordinator);
    assert!(spec.success_criteria.is_empty(), "from_broad_draft leaves criteria empty");

    let plan = materialize_spec(&spec).expect("materialize");
    assert!(
        plan.manifest.success_metrics.is_some(),
        "SLA skeleton kept as the fallback floor when no typed criteria"
    );
    assert!(
        !plan.manifest.kpis.is_empty()
            && plan.manifest.kpis.iter().all(|k| k.gate == KpiGate::Inform),
        "free-text metrics map to Inform hints"
    );
    assert!(!plan.manifest.all_kpis().is_empty());
}
