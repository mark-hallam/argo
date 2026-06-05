//! Unit tests for the archetype-free companion-file writers.

use super::*;
use crate::spec::{CapabilityDecl, CapabilitySpec, EscalationSpec};
use crate::Archetype;
use argo_vertical::ActionRiskLevel;

fn spec() -> CapabilitySpec {
    CapabilitySpec {
        charter: "a clinical-intake coordinator who books patient assessments".into(),
        persona: "clara".into(),
        persona_description: "Coordinates clinical intake and books patient assessments.".into(),
        responsibilities: vec![
            "Triage inbound patient requests".into(),
            "Book assessment slots".into(),
        ],
        capabilities: vec![
            CapabilityDecl {
                action_name: "triage_request".into(),
                intent: "Triage an inbound patient request".into(),
                risk: ActionRiskLevel::Low,
                reversible: true,
                proposed_executor: crate::ExecutorIntent::Builtin,
                requires_approval: false,
            },
            CapabilityDecl {
                action_name: "send_booking_confirmation".into(),
                intent: "Send a booking confirmation".into(),
                risk: ActionRiskLevel::High,
                reversible: true,
                proposed_executor: crate::ExecutorIntent::Human,
                requires_approval: true,
            },
        ],
        success_metrics: vec!["no double-booked slots".into()],
        onboarding_script: "Hi, I keep intake moving.".into(),
        escalation: EscalationSpec::default(),
        nearest_archetype: Archetype::CalendarCoordinator,
        from_scratch: true,
        cadence: None,
        exceptions: vec![],
        success_criteria: vec![],
    }
}

#[test]
fn system_prompt_is_sourced_from_spec_not_archetype_label() {
    let prompt = system_prompt_from_spec(&spec());
    // Identity comes from the persona + description, NOT "Calendar Coordinator".
    assert!(prompt.contains("clara"));
    assert!(prompt.contains("Coordinates clinical intake"));
    assert!(!prompt.contains("Calendar Coordinator"), "must not leak archetype label");
    // Charter + responsibilities + action vocabulary are present.
    assert!(prompt.contains("clinical-intake coordinator"));
    assert!(prompt.contains("Triage inbound patient requests"));
    assert!(prompt.contains("`triage_request`"));
    // Safety + no-live-integration rules carry over.
    assert!(prompt.contains("ALWAYS require operator approval"));
    assert!(prompt.contains("NO live integrations at birth"));
    assert!(prompt.contains("escalate rather than guess"));
}

#[test]
fn persona_prompt_signs_off_with_persona_and_omits_archetype() {
    let prompt = persona_prompt_from_spec(&spec());
    assert!(prompt.contains("— clara"));
    assert!(prompt.contains("Coordinates clinical intake"));
    assert!(prompt.contains("synthesized from scratch"));
}

#[test]
fn write_spec_artefacts_lays_out_all_companion_files() {
    let dir = tempfile::tempdir().expect("tempdir");
    write_spec_artefacts(dir.path(), &spec()).expect("write artefacts");
    for rel in [
        "prompts/system.md",
        "prompts/persona.md",
        "memory/schema.toml",
        "eval/seed.jsonl",
    ] {
        assert!(dir.path().join(rel).exists(), "missing {rel}");
    }
    // The eval seed jsonl is valid JSON, one object per line.
    let jsonl = std::fs::read_to_string(dir.path().join("eval/seed.jsonl")).expect("read");
    assert!(!jsonl.trim().is_empty(), "eval seed must not be empty");
    for line in jsonl.lines() {
        let v: serde_json::Value = serde_json::from_str(line).expect("valid jsonl line");
        assert!(v.get("id").is_some(), "line missing id: {line}");
        assert!(v.get("prompt").is_some(), "line missing prompt: {line}");
    }
}
