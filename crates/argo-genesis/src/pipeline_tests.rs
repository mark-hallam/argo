//! Tests for the top-level Genesis pipeline. Extracted from `pipeline.rs`
//! to keep that file under the per-file 500-LOC budget (the crate's
//! established `#[path = "..._tests.rs"]` pattern).

use super::*;

#[test]
fn plan_for_marketing_charter_resolves_archetype_and_passes_safety() {
    let p = plan(
        "be my marketing intern, run weekly campaigns to cold leads",
        "maya",
    )
    .expect("plan builds");
    assert_eq!(p.archetype, Archetype::MarketingIntern);
    assert_eq!(p.persona, "maya");
    assert!(p.vertical_id.starts_with("marketing-intern-"));
}

#[test]
fn plan_rejects_empty_charter() {
    let err = plan("   ", "maya").expect_err("empty charter");
    assert!(matches!(err, GenesisError::EmptyCharter));
}

#[test]
fn plan_rejects_invalid_persona_with_special_chars() {
    let err = plan("be my marketing intern", "maya@").expect_err("bad persona");
    assert!(matches!(err, GenesisError::InvalidPersona(_)));
}

#[test]
fn write_vertical_creates_full_layout_and_passes_round_trip() {
    let p = plan(
        "be my marketing intern, run weekly campaigns to cold leads",
        "maya",
    )
    .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join(&p.vertical_id);
    write_vertical(&p, &out).expect("write succeeds");
    for relpath in [
        "vertical.toml",
        "worker.toml",
        "eval-seed.toml",
        "prompts/system.md",
        "prompts/persona.md",
        "memory/schema.toml",
        "eval/seed.jsonl",
    ] {
        assert!(out.join(relpath).exists(), "missing: {relpath}");
    }
    let (registry, errors) = argo_vertical::load_dir(dir.path()).expect("load_dir");
    assert!(errors.is_empty(), "load errors: {errors:?}");
    let loaded = registry.get(&p.vertical_id).expect("vertical loaded by id");
    assert_eq!(loaded.manifest.vertical.id, p.vertical_id);
}

#[test]
fn write_vertical_refuses_to_clobber_existing_dir() {
    let p = plan("be my marketing intern", "maya").unwrap();
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join(&p.vertical_id);
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join("preexisting.txt"), "do not delete").unwrap();
    let err = write_vertical(&p, &out).expect_err("must refuse");
    assert!(matches!(err, GenesisError::OutputDirNotEmpty(_)));
}

#[test]
fn write_vertical_with_overwrite_false_refuses_existing_dir() {
    let p = plan("be my marketing intern", "maya").unwrap();
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join(&p.vertical_id);
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join("preexisting.txt"), "keep me").unwrap();
    let err = write_vertical_with_overwrite(&p, &out, false).expect_err("must refuse");
    assert!(matches!(err, GenesisError::OutputDirNotEmpty(_)));
    assert!(out.join("preexisting.txt").exists(), "must not delete");
}

#[test]
fn write_vertical_with_overwrite_true_wipes_then_writes() {
    let p = plan("be my marketing intern", "maya").unwrap();
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join(&p.vertical_id);
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join("stale.txt"), "stale").unwrap();
    write_vertical_with_overwrite(&p, &out, true).expect("overwrite succeeds");
    assert!(!out.join("stale.txt").exists(), "pre-existing file wiped");
    assert!(out.join("vertical.toml").exists(), "fresh manifest written");
}

#[test]
fn materialize_spec_lands_all_intake_slots_and_kpis() {
    // DEC-048 rule 2 round-trip (captured-equals-built): a spec carrying all
    // three intake slots → `materialize_spec` must (1) keep them on `plan.spec`,
    // (2) emit the typed success-criterion as a REAL manifest KPI (not the
    // default skeleton), and (3) carry all three onto the review contract via
    // `with_intake_slots`.
    use crate::spec::{CapabilityDecl, CapabilitySpec, EscalationSpec, SuccessCriterion};
    use crate::{materialize_spec, Archetype, ExecutorIntent, RoleContract};
    use argo_vertical::{ActionRiskLevel, KpiComparator, KpiKind};

    let spec = CapabilitySpec {
        charter: "a vineyard logistics coordinator who schedules harvests".into(),
        persona: "vinny".into(),
        persona_description: "Coordinates vineyard harvest scheduling.".into(),
        responsibilities: vec!["Schedule harvests".into()],
        capabilities: vec![CapabilityDecl {
            action_name: "schedule_harvest".into(),
            intent: "Schedule a harvest window".into(),
            risk: ActionRiskLevel::Low,
            reversible: true,
            proposed_executor: ExecutorIntent::Builtin,
            requires_approval: false,
        }],
        success_metrics: vec![],
        onboarding_script: "Hi, I keep the harvest calendar tight.".into(),
        escalation: EscalationSpec::default(),
        nearest_archetype: Archetype::CalendarCoordinator,
        from_scratch: true,
        cadence: Some("every weekday at 5pm".into()),
        exceptions: vec!["never touch legal escalations".into()],
        access_integrations: vec![],
        success_criteria: vec![SuccessCriterion {
            name: "on-time dispatch rate".into(),
            kind: KpiKind::Percentage,
            target: 95.0,
            comparator: KpiComparator::Gte,
        }],
    };

    let plan = materialize_spec(&spec).expect("materialize");

    // (1) the plan keeps the spec with all three slots intact.
    let s = plan.spec.as_ref().expect("plan carries spec");
    assert_eq!(s.cadence.as_deref(), Some("every weekday at 5pm"));
    assert_eq!(s.exceptions, vec!["never touch legal escalations".to_string()]);
    assert_eq!(s.success_criteria.len(), 1);

    // (2) the operator's typed goal reached the manifest as a REAL KPI.
    assert!(
        plan.manifest.kpis.iter().any(|k| k.kind == KpiKind::Percentage
            && (k.threshold - 95.0).abs() < f64::EPSILON
            && k.comparator == KpiComparator::Gte),
        "typed success-criterion must emit a manifest KPI, not the default skeleton"
    );

    // (3) the review contract carries all three for the pre-hire display.
    let contract = RoleContract::derive_from_manifest(&plan.manifest).with_intake_slots(s);
    assert_eq!(contract.cadence.as_deref(), Some("every weekday at 5pm"));
    assert_eq!(contract.exceptions.len(), 1);
    assert_eq!(contract.success_criteria.len(), 1);
}

// ---- LLM-classifier dispatch (mission-genesis-llm-classifier) ----

use argo_types::ArgoResult;
use async_trait::async_trait;

struct StubProvider {
    slug: &'static str,
}

#[async_trait]
impl ProviderClient for StubProvider {
    async fn classify(&self, _prompt: &str) -> ArgoResult<String> {
        Ok(self.slug.to_string())
    }
}

#[tokio::test]
async fn plan_with_uses_llm_classifier_when_enabled() {
    // Charter would classify to MarketingIntern via keyword; the
    // LLM stub forces a different archetype to prove the LLM path
    // ran instead of the keyword fallback.
    let cfg = GenesisConfig {
        use_llm_classifier: true,
    };
    let llm = StubProvider {
        slug: "ops_on_call",
    };
    let p = plan_with(
        "be my marketing intern, run weekly campaigns to cold leads",
        "maya",
        &cfg,
        Some(&llm),
    )
    .await
    .expect("plan_with builds");
    assert_eq!(p.archetype, Archetype::OpsOnCall);
}

#[tokio::test]
async fn plan_with_falls_back_to_keyword_when_llm_disabled() {
    let cfg = GenesisConfig::default(); // use_llm_classifier=false
    let llm = StubProvider {
        slug: "ops_on_call",
    };
    let p = plan_with(
        "be my marketing intern, run weekly campaigns to cold leads",
        "maya",
        &cfg,
        Some(&llm),
    )
    .await
    .expect("plan_with builds");
    assert_eq!(p.archetype, Archetype::MarketingIntern);
}

#[tokio::test]
async fn plan_with_falls_back_to_keyword_when_no_provider() {
    let cfg = GenesisConfig {
        use_llm_classifier: true,
    };
    let p = plan_with(
        "be my marketing intern, run weekly campaigns to cold leads",
        "maya",
        &cfg,
        None,
    )
    .await
    .expect("plan_with builds");
    assert_eq!(p.archetype, Archetype::MarketingIntern);
}

#[test]
fn sync_action_descriptions_plumbs_authored_intent_into_the_manifest() {
    // The live `/genesis` finding: the from-scratch path materializes the
    // manifest from the spec's *templated* fallback descriptions, then the LLM
    // author enriches `spec.capabilities[].intent` — but the review panel reads
    // the manifest, so without this sync the operator only ever saw the
    // fallback. Assert: (a) authored intent reaches the manifest + the derived
    // RoleContract, and (b) an un-authored action keeps the grammatical
    // fallback (no "as the …" tail).
    use crate::{apply_authored, AuthoredTeammate, BroadRoleDraft, CapabilitySpec, RoleContract};

    let draft = BroadRoleDraft {
        charter: "research assistant who reads and summarises articles".into(),
        persona: "remy".into(),
        persona_description: "Reads and summarises.".into(),
        action_vocabulary: vec!["archive_article".into(), "extract_takeaways".into()],
        success_metrics: vec![],
        onboarding_script: String::new(),
    };
    let spec = CapabilitySpec::from_broad_draft(draft, Archetype::ResearchAnalyst);
    let mut plan = crate::materialize_spec(&spec).expect("materialize broad spec");

    // Before authoring: the manifest carries the grammatical fallback floor.
    let actions = plan.manifest.actions.as_ref().expect("actions block");
    assert_eq!(actions.actions["archive_article"].description, "Archive article.");

    // Simulate the LLM author providing ONE specific per-action intent.
    let authored_intent = "File the source article to the research library for citation";
    let authored = AuthoredTeammate {
        persona_description: "Reads and summarises sources for the team.".into(),
        action_intents: [("archive_article".to_string(), authored_intent.to_string())]
            .into_iter()
            .collect(),
        ..AuthoredTeammate::default()
    };
    plan.spec = Some(apply_authored(spec, authored));
    plan.sync_action_descriptions_from_spec();

    let actions = plan.manifest.actions.as_ref().expect("actions block");
    // (a) Authored intent reached the manifest.
    assert_eq!(actions.actions["archive_article"].description, authored_intent);
    // (b) Un-authored action kept the grammatical fallback — no "as the" tail.
    assert_eq!(actions.actions["extract_takeaways"].description, "Extract takeaways.");

    // The review panel (RoleContract derived from the manifest) shows the prose.
    let contract = RoleContract::derive_from_manifest(&plan.manifest);
    assert!(
        contract
            .actions
            .iter()
            .any(|a| a.action == "archive_article" && a.description == authored_intent),
        "review-panel contract must carry the authored description"
    );
}

#[test]
fn sync_action_descriptions_is_a_noop_on_the_archetype_path() {
    // The archetype path leaves `spec` None; the sync must not panic or mutate.
    let mut plan = plan("be my marketing intern, run weekly campaigns", "maya")
        .expect("archetype plan builds");
    let before = plan.manifest.clone();
    plan.sync_action_descriptions_from_spec();
    assert_eq!(plan.manifest, before, "archetype-path manifest is untouched");
}
