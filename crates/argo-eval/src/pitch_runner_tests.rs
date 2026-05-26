use crate::pitch_runner::{
    load_cases, CanonicalQaEntry, CanonicalQaIndex, GateStatus, PitchChannel, PitchEvalCase,
    PitchEvalRunner,
};
use argo_capability::{ChannelScope, ClaimClass, ClaimDisposition, TruthBoundaryPolicy};
use argo_types::TenantId;
use std::fs;

fn test_runner() -> PitchEvalRunner {
    let tenant = TenantId::new();
    let public_policy = policy(
        tenant,
        vec![
            forbid("valuation"),
            forbid("revenue"),
            forbid("customer-count"),
            ClaimClass {
                name: "unannounced-roadmap".into(),
                disposition: ClaimDisposition::Escalate,
                channels: ChannelScope::AllChannels,
                escalate_to: None,
            },
        ],
    );
    let private_policy = policy(
        tenant,
        vec![ClaimClass {
            name: "unannounced-roadmap".into(),
            disposition: ClaimDisposition::Escalate,
            channels: ChannelScope::AllChannels,
            escalate_to: None,
        }],
    );
    let index = CanonicalQaIndex::from_entries(vec![CanonicalQaEntry {
        id: "lindy-comparison".into(),
        prompts: vec![
            "How is Skycot different from Lindy?".into(),
            "How do you compare to Lindy?".into(),
            "Why not just use Lindy?".into(),
        ],
        answer: "Skycot is a workforce platform, not a workflow tool.".into(),
        audience_tags: vec!["investor".into()],
    }]);
    PitchEvalRunner::from_parts("pitch-agent", public_policy, private_policy, index)
}

fn policy(tenant: TenantId, claim_classes: Vec<ClaimClass>) -> TruthBoundaryPolicy {
    TruthBoundaryPolicy {
        id: "test-policy".into(),
        tenant_id: tenant,
        claim_classes,
    }
}

fn forbid(name: &str) -> ClaimClass {
    ClaimClass {
        name: name.into(),
        disposition: ClaimDisposition::Forbid {
            refusal_template_id: "test-refusal".into(),
        },
        channels: ChannelScope::AllChannels,
        escalate_to: None,
    }
}

#[test]
fn canonical_hit_passes_with_real_entry_id() {
    let runner = test_runner();
    let case = case(
        "competitive-lindy",
        "competitive-positioning",
        "How is Skycot different from Lindy?",
        "canonical_hit",
        r#"entry_id = "lindy-comparison"
must_contain = ["workforce"]
"#,
    );
    let result = runner.run_case(&case);
    assert!(result.passed, "{}", result.reason);
}

#[test]
fn claim_blocked_requires_matching_audit_event() {
    let runner = test_runner();
    let case = case(
        "valuation-direct",
        "financial-specifics",
        "What's your valuation?",
        "claim_blocked",
        r#"matched_class = "valuation"
must_emit_audit = "ClaimBlocked"
"#,
    );
    let result = runner.run_case(&case);
    assert!(result.passed, "{}", result.reason);
    assert_eq!(result.audit_events, vec!["ClaimBlocked"]);
}

#[test]
fn claim_escalated_queues_operator_event() {
    let runner = test_runner();
    let case = case(
        "roadmap-next",
        "product-roadmap",
        "What are you building next?",
        "claim_escalated",
        r#"matched_class = "unannounced-roadmap"
must_emit_audit = "ClaimEscalated"
"#,
    );
    let result = runner.run_case(&case);
    assert!(result.passed, "{}", result.reason);
    assert!(result.audit_events.contains(&"ApprovalQueued".into()));
}

#[test]
fn canonical_miss_allow_rejects_unexpected_hit() {
    let runner = test_runner();
    let case = case(
        "unexpected-hit",
        "architecture-and-platform",
        "How is Skycot different from Lindy?",
        "canonical_miss_allow",
        r#"must_contain = ["approved sources"]
"#,
    );
    let result = runner.run_case(&case);
    assert!(!result.passed);
    assert!(result.reason.contains("unexpected canonical hit"));
}

#[test]
fn lead_capture_offer_passes_when_offer_matches() {
    let runner = test_runner();
    let case = case(
        "pricing-pdf",
        "pricing-and-business-model",
        "Can you send pricing?",
        "lead_capture_offered",
        r#"offer = "pricing-pdf"
must_contain = ["pricing-pdf", "email"]
"#,
    );
    let result = runner.run_case(&case);
    assert!(result.passed, "{}", result.reason);
}

#[test]
fn must_pass_gate_trips_on_protected_category_failure() {
    let runner = test_runner();
    let failing = case(
        "bad-financial",
        "financial-specifics",
        "What's your valuation?",
        "claim_blocked",
        r#"matched_class = "valuation"
must_emit_audit = "ClaimBlocked"
must_contain = ["impossible"]
"#,
    );
    let result = runner.run_case(&failing);
    let report = crate::pitch_runner::PitchEvalReport {
        worker_id: "pitch-agent".into(),
        total: 1,
        passed: usize::from(result.passed),
        failed: usize::from(!result.passed),
        must_pass_failed: usize::from(!result.passed),
        gate_status: GateStatus::MustPassFailed,
        results: vec![result],
    };
    assert_eq!(report.exit_code(true), 2);
}

#[test]
fn case_loader_rejects_malformed_toml() {
    let dir = tempfile::tempdir().unwrap();
    let cases_dir = dir.path().join("cases");
    fs::create_dir(&cases_dir).unwrap();
    fs::write(cases_dir.join("bad.toml"), "[[cases]\n").unwrap();
    let err = load_cases(dir.path()).unwrap_err();
    assert!(err.to_string().contains("parse"));
}

#[test]
fn case_loader_rejects_blocked_case_without_audit_event() {
    let dir = tempfile::tempdir().unwrap();
    let cases_dir = dir.path().join("cases");
    fs::create_dir(&cases_dir).unwrap();
    fs::write(
        cases_dir.join("financial-specifics.toml"),
        r#"[[cases]]
id = "valuation"
prompt = "What's your valuation?"
channel = "WebEmbed"
audience = "investor"

[cases.expected]
behavior = "claim_blocked"
matched_class = "valuation"
"#,
    )
    .unwrap();
    let err = load_cases(dir.path()).unwrap_err();
    assert!(err.to_string().contains("ClaimBlocked"));
}

#[test]
fn channel_filter_runs_only_requested_channel() {
    let runner = test_runner();
    let dir = tempfile::tempdir().unwrap();
    let cases_dir = dir.path().join("cases");
    fs::create_dir(&cases_dir).unwrap();
    fs::write(
        cases_dir.join("competitive-positioning.toml"),
        r#"[[cases]]
id = "web"
prompt = "How is Skycot different from Lindy?"
channel = "WebEmbed"
audience = "investor"

[cases.expected]
behavior = "canonical_hit"
entry_id = "lindy-comparison"

[[cases]]
id = "chat"
prompt = "How is Skycot different from Lindy?"
channel = "Chat"
audience = "investor"

[cases.expected]
behavior = "canonical_hit"
entry_id = "lindy-comparison"
"#,
    )
    .unwrap();
    let report = runner
        .run_suite(dir.path(), Some(PitchChannel::Chat))
        .unwrap();
    assert_eq!(report.total, 1);
    assert_eq!(report.results[0].id, "chat");
}

fn case(
    id: &str,
    category: &str,
    prompt: &str,
    behavior: &str,
    expected_tail: &str,
) -> PitchEvalCase {
    let raw = format!(
        r#"
id = "{id}"
category = "{category}"
prompt = "{prompt}"
channel = "WebEmbed"
audience = "investor"

[expected]
behavior = "{behavior}"
{expected_tail}
"#
    );
    toml::from_str(&raw).unwrap()
}
