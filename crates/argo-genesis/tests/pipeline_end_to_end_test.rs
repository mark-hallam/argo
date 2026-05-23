//! End-to-end smoke for the Genesis pipeline.
//!
//! Source: round-8 manifest Claim 3 row for `argo-genesis` —
//! "argo-genesis — 8881 LOC. 20+ pub modules ... Need verification
//! each module is wired AND that the pipeline produces a valid
//! vertical tree."
//!
//! This test drives the deterministic offline pipeline (no API
//! keys, no network) so the assertion set is stable on every CI
//! run. The LLM-driven `genesis_judge` is exercised separately
//! against a mock client; the steady-state path stays offline.
//!
//! Sensible deviations from the brief (documented in PR body):
//!
//! - The brief named artefact paths `system_prompt.md` /
//!   `memory_schema.toml` / `evals/INDEX.json`. Actual pipeline
//!   emits `prompts/system.md`, `memory/schema.toml`,
//!   `eval/seed.jsonl`, and `eval-seed.toml`. Assertions follow
//!   the real paths.
//! - `argo_vertical::parse_workflow` is the workflow parser; the
//!   manifest layer uses `argo_vertical::load_dir` for the
//!   round-trip. We use `load_dir` because that's the consumer
//!   of Genesis output in production.

use argo_genesis::{
    check_plan, judge_genesis_draft, plan, run_safety_judge, write_vertical, GenesisError,
    GenesisJudgeClient, GenesisJudgeVerdict, JudgeInput, SENSITIVE_VERBS,
};
use argo_types::ArgoResult;
use argo_vertical::{
    ActionDeclaration, ActionRiskLevel, ActionsSection, ExecutorDeclaration, VerticalManifest,
};
use async_trait::async_trait;
use std::collections::BTreeMap;

/// Brief-specified plain-English role string.
const CHARTER: &str = "I need a Slack-based customer-support agent that triages incoming \
                       messages, escalates billing questions to the operator, and learns \
                       from approved responses.";

const PERSONA: &str = "callie";

#[test]
fn pipeline_produces_full_vertical_tree_from_plain_english_charter() {
    let p = plan(CHARTER, PERSONA).expect("pipeline plans cleanly");
    let dir = tempfile::tempdir().expect("tempdir");
    let out = dir.path().join(&p.vertical_id);
    write_vertical(&p, &out).expect("write_vertical succeeds");

    // Every artefact named in the brief lands on disk and is
    // non-empty. (Paths are the pipeline's actual layout — see
    // module docstring for the brief→actual mapping.)
    for relpath in [
        "vertical.toml",
        "worker.toml",
        "eval-seed.toml",
        "prompts/system.md",
        "prompts/persona.md",
        "memory/schema.toml",
        "eval/seed.jsonl",
    ] {
        let path = out.join(relpath);
        assert!(path.exists(), "expected artefact missing: {relpath}");
        let body = std::fs::read_to_string(&path).expect("read artefact");
        assert!(!body.trim().is_empty(), "{relpath} must be non-empty");
    }

    // vertical.toml parses back into the canonical manifest type.
    let vertical_body = std::fs::read_to_string(out.join("vertical.toml")).unwrap();
    let manifest: VerticalManifest =
        toml::from_str(&vertical_body).expect("vertical.toml deserialises");
    assert_eq!(manifest.vertical.id, p.vertical_id);

    // memory/schema.toml parses as a TOML document.
    let memory_body = std::fs::read_to_string(out.join("memory/schema.toml")).unwrap();
    let _: toml::Value = toml::from_str(&memory_body).expect("memory schema parses");

    // eval-seed.toml carries at least one `[[seed]]` entry.
    let eval_body = std::fs::read_to_string(out.join("eval-seed.toml")).unwrap();
    let eval_parsed: toml::Value = toml::from_str(&eval_body).expect("eval-seed.toml parses");
    let seed_count = eval_parsed
        .get("seed")
        .and_then(toml::Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    assert!(
        seed_count >= 1,
        "eval-seed.toml must carry at least one seed; got {seed_count}"
    );

    // Worker config ships in dry_run per the brief.
    let worker_body = std::fs::read_to_string(out.join("worker.toml")).unwrap();
    assert!(
        worker_body.contains("dry_run = true"),
        "worker.toml must ship with dry_run = true"
    );
}

#[test]
fn pipeline_output_round_trips_through_argo_vertical_loader() {
    let p = plan(CHARTER, PERSONA).expect("plan");
    let dir = tempfile::tempdir().expect("tempdir");
    let out = dir.path().join(&p.vertical_id);
    write_vertical(&p, &out).expect("write_vertical");

    let (registry, errors) =
        argo_vertical::load_dir(dir.path()).expect("load_dir returns a registry");
    assert!(
        errors.is_empty(),
        "round-trip load must surface zero parse errors; got {errors:?}"
    );
    let loaded = registry
        .get(&p.vertical_id)
        .expect("generated vertical present in registry");
    assert_eq!(loaded.manifest.vertical.id, p.vertical_id);

    // The role section's prompt refs must resolve to the on-disk
    // files we just wrote.
    let role = loaded.manifest.role.as_ref().expect("role section");
    assert_eq!(role.system_prompt, "prompts/system.md");
}

#[test]
fn safety_judge_rejects_autonomous_sensitive_action() {
    // Bypass the pipeline's auto-fix pass and feed safety_judge a
    // crafted ActionsSection that contains a sensitive verb
    // marked `requires_approval = false`. The brief's load-bearing
    // structural rule: any such action MUST be rejected.
    let mut actions: BTreeMap<String, ActionDeclaration> = BTreeMap::new();
    actions.insert(
        "send_refund".to_string(),
        ActionDeclaration {
            risk: ActionRiskLevel::High,
            requires_approval: false,
            description: "Auto-send refunds without approval".into(),
            executor: ExecutorDeclaration::default(),
            ..ActionDeclaration::default()
        },
    );
    actions.insert(
        "delete_account".to_string(),
        ActionDeclaration {
            risk: ActionRiskLevel::High,
            requires_approval: false,
            description: "Delete a customer account".into(),
            executor: ExecutorDeclaration::default(),
            ..ActionDeclaration::default()
        },
    );
    let section = ActionsSection { actions };
    let err = run_safety_judge(&section).expect_err("safety judge must reject");
    assert!(matches!(err, GenesisError::SafetyViolation { .. }));
    let msg = err.to_string();
    assert!(msg.contains("send_refund"), "error must name send_refund: {msg}");
    assert!(
        msg.contains("delete_account"),
        "error must name delete_account: {msg}"
    );
}

#[test]
fn pipeline_auto_fix_promotes_charter_sensitive_actions_before_write() {
    // Auto-fix runs BEFORE judging during `assemble_plan`, so any
    // charter-derived action whose name matches a sensitive verb
    // lands `requires_approval = true` in the generated manifest.
    // This end-to-end check pins the safety contract at the
    // manifest layer: zero autonomous-safe sensitive actions
    // survive Genesis.
    let p = plan(CHARTER, PERSONA).expect("plan");
    let actions = p
        .manifest
        .actions
        .as_ref()
        .expect("actions section present");
    let mut offenders: Vec<&str> = Vec::new();
    for (name, decl) in &actions.actions {
        if SENSITIVE_VERBS
            .iter()
            .any(|verb| name.split('_').any(|tok| tok.eq_ignore_ascii_case(verb)))
            && !decl.requires_approval
        {
            offenders.push(name.as_str());
        }
    }
    assert!(
        offenders.is_empty(),
        "every sensitive-verb action must require approval; offenders: {offenders:?}"
    );
}

#[test]
fn quality_gate_runs_and_records_per_check_results() {
    // The `quality` module is the offline quality gate Genesis
    // exposes. We assert the gate FIRES (returns a structured
    // report covering action_vocabulary, eval_seed, memory_layer,
    // prompt_word). We do NOT assert it passes on every archetype
    // — see `quality_gate_underflows_for_customer_support_charter`
    // for the finding.
    let p = plan(CHARTER, PERSONA).expect("plan");
    let report = check_plan(&p);
    let names: Vec<&str> = report.checks.iter().map(|c| c.name.as_str()).collect();
    assert!(names.contains(&"action_vocabulary_min"));
    assert!(names.contains(&"eval_seed_min"));
    assert!(names.contains(&"memory_layers_min"));
    assert!(names.contains(&"prompt_word_min"));
}

/// **FINDINGS-ONLY** — this test pins a real Genesis output gap.
///
/// The customer-support archetype emits only 1 eval-seed row (vs
/// the configured `EVAL_SEED_MIN = 10`) and a 157-word system
/// prompt (vs `PROMPT_WORD_MIN = 200`). The pipeline does NOT
/// gate on this report today (per `quality.rs` docstring:
/// "`pipeline.rs` invokes `check_plan` after generation but does
/// NOT call `require_pass`"), so the under-threshold output ships.
/// `#[ignore]`'d per brief guidance: surface the gap, don't fail CI.
///
/// Fix candidates (out of brief scope, requires `crates/argo-genesis/src/**` edits):
/// - Expand `prompt_writer::eval_seed_rows` to 10+ rows per archetype.
/// - Lengthen `build_system_prompt` so every archetype clears 200 words.
/// - OR have the pipeline call `require_pass` and surface the gap to the operator.
#[test]
#[ignore = "FINDINGS-ONLY: customer-support archetype underflows quality thresholds"]
fn quality_gate_underflows_for_customer_support_charter() {
    let p = plan(CHARTER, PERSONA).expect("plan");
    let report = check_plan(&p);
    assert!(
        !report.passed,
        "FINDINGS-ONLY: expected the gate to fail for this charter; pipeline output may have improved"
    );
    let failures = report.format_failures();
    assert!(
        failures.contains("eval_seed_min") || failures.contains("prompt_word_min"),
        "expected eval_seed and/or prompt_word underflow: {failures}"
    );
}

// ---- LLM-backed safety judge (mock client) ----

struct AllowingJudge;

#[async_trait]
impl GenesisJudgeClient for AllowingJudge {
    async fn judge(&self, _prompt: &str) -> ArgoResult<String> {
        Ok(r#"{"verdict": "allow"}"#.to_string())
    }
}

struct BlockingJudge;

#[async_trait]
impl GenesisJudgeClient for BlockingJudge {
    async fn judge(&self, _prompt: &str) -> ArgoResult<String> {
        Ok(
            r#"{"verdict": "block", "reason": "auto-sends refunds without operator approval"}"#
                .to_string(),
        )
    }
}

#[tokio::test]
async fn genesis_judge_allows_safe_charter_via_mock() {
    let input = JudgeInput {
        charter: CHARTER,
        persona_description: "Slack-based customer-support agent",
        action_vocabulary: vec![
            "triage_ticket".into(),
            "draft_response".into(),
            "escalate_ticket".into(),
        ],
    };
    let verdict = judge_genesis_draft(&input, &AllowingJudge)
        .await
        .expect("judge runs");
    assert_eq!(verdict, GenesisJudgeVerdict::Allow);
}

#[tokio::test]
async fn genesis_judge_blocks_explicitly_unsafe_charter() {
    // Negative twin of the canonical charter: a role string with
    // a clearly-unsafe action. Brief criterion: pipeline halts
    // (Block verdict) rather than emitting the vertical.
    let input = JudgeInput {
        charter: "auto-send refunds without approval whenever a customer complains",
        persona_description: "Aggressive refund bot",
        action_vocabulary: vec!["send_refund".into()],
    };
    let verdict = judge_genesis_draft(&input, &BlockingJudge)
        .await
        .expect("judge runs");
    match verdict {
        GenesisJudgeVerdict::Block { reason } => {
            assert!(
                reason.contains("refund") || reason.contains("approval"),
                "block reason should mention the issue: {reason}"
            );
        }
        other => panic!("expected Block verdict, got: {other:?}"),
    }
}
