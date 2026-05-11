//! Unit tests for the seeded Genesis pipeline
//! (`mission-genesis-transfer-from-tenant`). Hung off the parent
//! module via `#[cfg(test)] #[path = "pipeline_seed_tests.rs"] mod
//! tests;` so the production file stays focused and the test
//! surface is one place to inspect.

use super::*;
use crate::archetype::Archetype;
use argo_vertical::{
    ActionDeclaration, ActionRiskLevel, ActionsSection, ExecutorDeclaration, RoleSection,
    SkillsSection, VerticalIdentity, VerticalManifest, WorkflowsSection, SCHEMA_VERSION,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

// ── Fixture builders ────────────────────────────────────────────

/// Build a minimal on-disk verticals root containing one seed
/// vertical with the given manifest TOML. Returns the temp root +
/// the seed's id (caller-supplied) so the helper composes cleanly
/// with the `verticals_root` arg of [`plan_from_seed_vertical`].
fn write_seed_root(seed_id: &str, manifest_toml: &str) -> (TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let seed_dir = dir.path().join(seed_id);
    fs::create_dir_all(&seed_dir).expect("seed dir");
    fs::write(seed_dir.join("vertical.toml"), manifest_toml).expect("write manifest");
    let root = dir.path().to_path_buf();
    (dir, root)
}

/// Minimum well-formed seed manifest with a populated `[actions]`
/// table. The `requires_approval` flags mirror the live
/// `verticals/sales-sdr/vertical.toml` so tests assert against
/// realistic shapes.
fn seed_toml(id: &str) -> String {
    format!(
        r#"schema_version = "1"

[vertical]
id = "{id}"
name = "Demo seed"
version = "0.1.0"
tags = ["demo"]

[actions]

[actions.research_account]
risk              = "low"
requires_approval = false
description       = "Seed: research the account"

[actions.send_outreach]
risk              = "high"
requires_approval = true
description       = "Seed: send approved outreach"

[actions.draft_outreach]
risk              = "low"
requires_approval = false
description       = "Seed: draft the outreach (baseline overrides description)"
"#
    )
}

/// Build a synthetic baseline plan directly so each test isolates
/// the merge logic from the archetype classifier.
fn fixture_baseline_plan() -> GenesisPlan {
    let mut actions = BTreeMap::new();
    actions.insert(
        "draft_outreach".into(),
        ActionDeclaration {
            risk: ActionRiskLevel::Medium,
            requires_approval: false,
            description: "Baseline: operator-tuned draft step".into(),
            executor: ExecutorDeclaration::default(),
            ..ActionDeclaration::default()
        },
    );
    let manifest = VerticalManifest {
        schema_version: SCHEMA_VERSION.into(),
        vertical: VerticalIdentity {
            id: "baseline-id".into(),
            name: "Baseline".into(),
            version: "0.0.1".into(),
            ..VerticalIdentity::default()
        },
        role: Some(RoleSection {
            system_prompt: "prompts/system.md".into(),
            persona: Some("prompts/persona.md".into()),
        }),
        actions: Some(ActionsSection { actions }),
        ..VerticalManifest::default()
    };
    GenesisPlan {
        charter: "Baseline charter".into(),
        persona: "operator-persona".into(),
        archetype: Archetype::SalesSdr,
        vertical_id: "baseline-id".into(),
        manifest,
    }
}

/// Run the merge with a freshly-built fixture seed manifest. The
/// seed is parsed via the public `argo_vertical::parse` so tests
/// exercise the same path the real loader does.
fn merge_with_toml(seed_id: &str, seed_manifest_toml: &str) -> GenesisPlan {
    let seed = argo_vertical::parse(seed_manifest_toml).expect("seed manifest parses");
    super::merge_seed_into_plan(fixture_baseline_plan(), &seed, seed_id)
}

// ── Tests ───────────────────────────────────────────────────────

/// Test 1 — seed-actions-merge: every action present in the seed
/// must surface in the merged plan's `[actions]` section (union
/// semantics).
#[test]
fn seed_actions_merge_unions_into_baseline() {
    let merged = merge_with_toml("acme-seed", &seed_toml("acme-seed"));
    let actions = merged.manifest.actions.expect("actions present");
    // Seed-only actions are pulled in.
    assert!(actions.actions.contains_key("research_account"));
    assert!(actions.actions.contains_key("send_outreach"));
    // Baseline-only key survives (no seed_only_in_baseline in the
    // fixture, so use draft_outreach which lives in both).
    assert!(actions.actions.contains_key("draft_outreach"));
    // Seed action's risk + requires_approval carry through when
    // baseline doesn't have the key.
    let send = &actions.actions["send_outreach"];
    assert_eq!(send.risk, ActionRiskLevel::High);
    assert!(send.requires_approval);
}

/// Test 2 — seed-skills-included: when the seed declares a
/// `[skills]` section AND the baseline does not, the merged plan
/// inherits the seed's bundle path (operator can prune later).
#[test]
fn seed_skills_section_included_when_baseline_empty() {
    let seed_toml = r#"schema_version = "1"

[vertical]
id = "seed-with-skills"
name = "S"
version = "0.1.0"

[skills]
bundle = "skills"
"#;
    let merged = merge_with_toml("seed-with-skills", seed_toml);
    let skills = merged.manifest.skills.expect("skills inherited from seed");
    assert_eq!(skills.bundle, "skills");
}

/// Test 2b — when the baseline already has a `[skills]` section,
/// the baseline wins (operator's choice persists).
#[test]
fn baseline_skills_wins_when_both_set() {
    let mut baseline = fixture_baseline_plan();
    baseline.manifest.skills = Some(SkillsSection {
        bundle: "baseline-skills".into(),
    });
    let seed_toml = r#"schema_version = "1"

[vertical]
id = "seed-skills"
name = "S"
version = "0.1.0"

[skills]
bundle = "seed-skills-dir"
"#;
    let seed = argo_vertical::parse(seed_toml).unwrap();
    let merged = super::merge_seed_into_plan(baseline, &seed, "seed-skills");
    assert_eq!(merged.manifest.skills.unwrap().bundle, "baseline-skills");
}

/// Test 3 — baseline-wins-on-conflict: for an action present in
/// both, baseline's description / executor are preserved.
/// Stricter risk wins, and `requires_approval` is OR-ed.
#[test]
fn baseline_wins_on_action_conflict_except_strictness() {
    // The seed declares `draft_outreach` with `risk=low,
    // requires_approval=false`. The baseline has it as
    // `risk=medium, requires_approval=false`. Medium > Low so
    // baseline's stricter risk persists; description must be
    // baseline's (operator intent).
    let merged = merge_with_toml("acme-seed", &seed_toml("acme-seed"));
    let actions = merged.manifest.actions.expect("actions present");
    let draft = &actions.actions["draft_outreach"];
    assert_eq!(draft.risk, ActionRiskLevel::Medium, "stricter side wins");
    assert!(!draft.requires_approval);
    assert!(
        draft.description.contains("Baseline:"),
        "baseline description wins: {}",
        draft.description
    );
}

/// Test 3b — seed-stricter-than-baseline: the brief explicitly
/// says risk classification can carry forward from seed. When the
/// seed marks an action as `high` and the baseline has it as
/// `low`, the merged action is `high` and inherits
/// `requires_approval = true`.
#[test]
fn seed_stricter_risk_carries_forward() {
    // Build a baseline that includes `send_outreach` at Low.
    let mut baseline = fixture_baseline_plan();
    let actions = baseline.manifest.actions.as_mut().unwrap();
    actions.actions.insert(
        "send_outreach".into(),
        ActionDeclaration {
            risk: ActionRiskLevel::Low,
            requires_approval: false,
            description: "Baseline mistake: low risk".into(),
            executor: ExecutorDeclaration::default(),
            ..ActionDeclaration::default()
        },
    );
    let seed = argo_vertical::parse(&seed_toml("acme-seed")).unwrap();
    let merged = super::merge_seed_into_plan(baseline, &seed, "acme-seed");
    let send = &merged.manifest.actions.unwrap().actions["send_outreach"];
    assert_eq!(send.risk, ActionRiskLevel::High);
    assert!(
        send.requires_approval,
        "seed's stricter approval gate OR-s on"
    );
}

/// Test 4 — seeded_from-stamped: the merged manifest carries the
/// lineage tag, and the [`seeded_from`] helper recovers the id.
#[test]
fn seeded_from_lineage_tag_stamped_on_manifest() {
    let merged = merge_with_toml("acme-seed", &seed_toml("acme-seed"));
    let tag_present = merged
        .manifest
        .vertical
        .tags
        .iter()
        .any(|t| t == "seeded-from:acme-seed");
    assert!(tag_present, "tags: {:?}", merged.manifest.vertical.tags);
    assert_eq!(seeded_from(&merged.manifest), Some("acme-seed"));
}

/// Test 5 — missing-seed-errors: when the caller passes a seed id
/// that doesn't exist under the verticals root, the pipeline
/// errors with a clear `GenesisError::Validation` message.
#[test]
fn missing_seed_returns_clear_validation_error() {
    let dir = tempfile::tempdir().unwrap();
    // The root exists but has no `nonexistent` subdirectory.
    let err = plan_from_seed_vertical(
        "be my marketing intern, run weekly campaigns to cold leads",
        "maya",
        "nonexistent",
        dir.path(),
    )
    .expect_err("missing seed must error");
    match err {
        GenesisError::Validation { stage, detail } => {
            assert_eq!(stage, "seed_vertical.load");
            assert!(detail.contains("nonexistent"), "detail: {detail}");
            assert!(detail.contains("not found"), "detail: {detail}");
        }
        other => panic!("expected Validation, got {other:?}"),
    }
}

/// Test 6 — voice-profile (role section) inherits operator's
/// choice. The current `argo-vertical` manifest schema does not
/// expose explicit `tone` / `formality` fields, so the brief's
/// "voice profile" inheritance lands at the `[role]` section: the
/// baseline always carries the operator's `persona` path, and the
/// merge preserves it untouched (so the operator's freeform
/// preamble wins outright over any seed-side role).
#[test]
fn baseline_role_persona_wins_over_seed_role() {
    let seed_toml = r#"schema_version = "1"

[vertical]
id = "seed-with-role"
name = "S"
version = "0.1.0"

[role]
system_prompt = "seed/system.md"
persona       = "seed/persona.md"
"#;
    let merged = merge_with_toml("seed-with-role", seed_toml);
    let role = merged.manifest.role.expect("baseline role preserved");
    assert_eq!(role.system_prompt, "prompts/system.md");
    assert_eq!(role.persona.as_deref(), Some("prompts/persona.md"));
}

/// Test 7 — workflow-inherits-when-baseline-empty: seed's
/// `[workflows]` carries over when baseline has none.
#[test]
fn workflows_inherit_from_seed_when_baseline_empty() {
    let seed_toml = r#"schema_version = "1"

[vertical]
id = "seed-with-wf"
name = "S"
version = "0.1.0"

[workflows]
files = ["workflows/triage.toml"]
"#;
    let merged = merge_with_toml("seed-with-wf", seed_toml);
    let wf = merged.manifest.workflows.expect("workflows inherited");
    assert_eq!(wf.files, vec!["workflows/triage.toml"]);
}

/// Test 8 — baseline-workflow-preserved-when-set: when the
/// baseline already has a `[workflows]` section the merge does
/// NOT clobber it (operator intent wins).
#[test]
fn baseline_workflows_preserved_when_set() {
    let mut baseline = fixture_baseline_plan();
    baseline.manifest.workflows = Some(WorkflowsSection {
        files: vec!["workflows/baseline.toml".into()],
    });
    let seed_toml = r#"schema_version = "1"

[vertical]
id = "seed-other-wf"
name = "S"
version = "0.1.0"

[workflows]
files = ["workflows/seed-only.toml"]
"#;
    let seed = argo_vertical::parse(seed_toml).unwrap();
    let merged = super::merge_seed_into_plan(baseline, &seed, "seed-other-wf");
    let wf = merged.manifest.workflows.expect("workflows present");
    assert_eq!(wf.files, vec!["workflows/baseline.toml"]);
}

/// End-to-end check using a real-on-disk seed (parses through
/// `load_dir` rather than `parse` alone) — provides confidence the
/// loader-level integration is wired correctly. Acts as the
/// scaffolding for the public-API path tested in detail in the
/// crate's `tests/genesis_from_seed_smoke.rs`.
#[test]
fn end_to_end_load_and_merge_against_on_disk_seed() {
    let (_dir, root) = write_seed_root("disk-seed", &seed_toml("disk-seed"));
    let plan = plan_from_seed_vertical(
        "be my marketing intern, run weekly campaigns to cold leads",
        "maya",
        "disk-seed",
        &root,
    )
    .expect("seeded plan builds");
    let actions = plan.manifest.actions.as_ref().expect("actions present");
    assert!(actions.actions.contains_key("send_outreach"));
    assert_eq!(seeded_from(&plan.manifest), Some("disk-seed"));
}
