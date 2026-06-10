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
        access_integrations: vec![],
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
fn system_prompt_lists_operator_exceptions() {
    // DEC-048 rule 2 (captured-equals-built): the carve-outs the operator typed
    // at intake must land in the minted pack's system prompt, not be display-only.
    let mut s = spec();
    s.exceptions = vec![
        "never touch legal escalations".into(),
        "hand off refunds over $5k".into(),
    ];
    let prompt = system_prompt_from_spec(&s);
    assert!(prompt.contains("What you will NOT do"));
    assert!(prompt.contains("never touch legal escalations"));
    assert!(prompt.contains("hand off refunds over $5k"));
}

#[test]
fn system_prompt_omits_exceptions_section_when_none() {
    // Back-compat: a spec with no captured carve-outs emits no carve-out
    // section (the text-only path is unchanged).
    let prompt = system_prompt_from_spec(&spec());
    assert!(!prompt.contains("What you will NOT do"));
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

#[test]
fn spec_workflow_files_emits_one_loadable_workflow_per_duty() {
    let s = spec();
    let files = spec_workflow_files(&s);
    // One workflow per declared capability ("primary duty").
    assert_eq!(files.len(), s.capabilities.len());
    for (rel, body) in &files {
        assert!(rel.starts_with("workflows/"), "bad path: {rel}");
        assert!(rel.ends_with(".workflow.toml"), "bad path: {rel}");
        // Every emitted TOML validates against the canonical loader.
        let wf = argo_vertical::parse_workflow(body)
            .unwrap_or_else(|e| panic!("{rel} must parse: {e}"));
        // Matches the receptionist reference shape: two ordered tool_call steps.
        assert_eq!(wf.steps.len(), 2, "{rel}");
        assert!(wf.steps.iter().all(|st| st.kind == "tool_call"), "{rel}");
        assert_eq!(wf.steps[1].depends_on, vec!["read_manifest"], "{rel}");
    }
    // Named after the duties (sanitised action names).
    assert!(files
        .iter()
        .any(|(rel, _)| rel == "workflows/triage_request.workflow.toml"));
    assert!(files
        .iter()
        .any(|(rel, _)| rel == "workflows/send_booking_confirmation.workflow.toml"));
}

#[test]
fn write_spec_artefacts_writes_validated_workflow_files() {
    let dir = tempfile::tempdir().expect("tempdir");
    write_spec_artefacts(dir.path(), &spec()).expect("write artefacts");
    for (rel, _) in spec_workflow_files(&spec()) {
        let path = dir.path().join(&rel);
        assert!(path.exists(), "missing {rel}");
        let body = std::fs::read_to_string(&path).expect("read workflow");
        argo_vertical::parse_workflow(&body).expect("on-disk workflow must validate");
    }
}

#[test]
fn workflow_description_with_quotes_stays_loadable() {
    // A free-text intent carrying a double-quote must not break the emitted TOML.
    let mut s = spec();
    s.capabilities[0].intent = "Triage a \"VIP\" patient request\nwith newline".into();
    let files = spec_workflow_files(&s);
    let (_, body) = files
        .iter()
        .find(|(rel, _)| rel.contains("triage_request"))
        .expect("triage workflow present");
    argo_vertical::parse_workflow(body).expect("escaped description must validate");
}
