//! P2 CLAIM-3 coverage: Genesis output round-trips through
//! `argo-vertical`'s loader for FIVE distinct role descriptions.
//!
//! Source: `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md`
//! Claim 3 row for `argo-vertical` — *"Loads `.workflow.toml`
//! files. Authoritative source for v1 per `workflow_definition.rs:1-10`
//! docstring. Mission claim 3 says Genesis-generated verticals must
//! work; need to verify Genesis output round-trips through
//! argo-vertical's loader."*
//!
//! Existing integration tests (`marketing_intern.rs`,
//! `pipeline_end_to_end_test.rs`, `eval_seed_generation.rs`) prove
//! the round-trip for the marketing-intern + customer-support
//! charters. This test extends the proof to a five-archetype
//! spread, catching any archetype-specific manifest defects that
//! would only surface under coverage.
//!
//! The deterministic offline pipeline is used end-to-end — no API
//! keys, no network, no provider transcripts.

use argo_genesis::{plan, write_vertical, Archetype, GenesisPlan};
use std::collections::{BTreeSet, HashSet};

/// Five role descriptions that span five distinct archetypes. Each
/// pairs with a unique persona so the generated vertical ids are
/// guaranteed distinct under the
/// `<archetype-slug>-<persona>` id rule in `pipeline::assemble_plan`.
const FIXTURES: &[(&str, &str, Archetype)] = &[
    (
        "be my marketing intern, run weekly campaigns to cold leads",
        "maya",
        Archetype::MarketingIntern,
    ),
    (
        "I need a Slack-based customer-support agent that triages \
         incoming messages, escalates billing questions to the \
         operator, and learns from approved responses.",
        "callie",
        Archetype::CustomerSupportAgent,
    ),
    (
        "be my sales SDR doing outbound prospecting; cold call \
         qualified leads and qualify each prospect against our ICP.",
        "sam",
        Archetype::SalesSdr,
    ),
    (
        "recruiter screener that screens inbound candidates, \
         coordinates interviews with hiring managers, and shortlists \
         talent for the recruit pipeline.",
        "rita",
        Archetype::RecruiterScreener,
    ),
    (
        "on-call ops engineer: triage incident alerts, page the \
         operator on severity-1 incidents, and walk the runbook for \
         each ops alert.",
        "ollie",
        Archetype::OpsOnCall,
    ),
];

/// Literal brief acceptance: drive Genesis on each charter, write
/// to its OWN tempdir, `argo_vertical::load_dir` each, assert no
/// parse errors.
#[test]
fn each_of_five_charters_round_trips_through_its_own_tempdir() {
    for (charter, persona, expected) in FIXTURES {
        let p = plan(charter, persona)
            .unwrap_or_else(|e| panic!("plan failed for charter {charter:?}: {e}"));
        assert_eq!(
            p.archetype, *expected,
            "charter {charter:?} classified as {:?}; expected {expected:?}",
            p.archetype
        );

        let dir = tempfile::tempdir().expect("tempdir");
        let out = dir.path().join(&p.vertical_id);
        write_vertical(&p, &out)
            .unwrap_or_else(|e| panic!("write_vertical failed for {}: {e}", p.vertical_id));

        let (registry, errors) = argo_vertical::load_dir(dir.path())
            .unwrap_or_else(|e| panic!("load_dir failed for {}: {e}", p.vertical_id));
        assert!(
            errors.is_empty(),
            "load_dir surfaced errors for charter {charter:?}: {errors:?}"
        );
        let loaded = registry
            .get(&p.vertical_id)
            .unwrap_or_else(|| panic!("generated vertical {} not in registry", p.vertical_id));
        assert_eq!(loaded.manifest.vertical.id, p.vertical_id);

        // The role's system-prompt ref must resolve to a real file
        // on disk after the round trip — proves the loader honoured
        // the cross-reference validator on EVERY archetype.
        let role = loaded
            .manifest
            .role
            .as_ref()
            .unwrap_or_else(|| panic!("role section missing for {}", p.vertical_id));
        let prompt_path = out.join(&role.system_prompt);
        assert!(
            prompt_path.is_file(),
            "system_prompt {} for {} must resolve on disk",
            role.system_prompt,
            p.vertical_id
        );
    }
}

/// Stronger variant: write ALL five generated verticals into the
/// SAME parent dir, then call `load_dir` once. This catches any
/// id-collision / cross-vertical state leakage that a per-tempdir
/// test would miss, since `verticals/` in production hosts every
/// catalogue vertical side-by-side.
#[test]
fn five_charters_coexist_under_one_verticals_dir() {
    let parent = tempfile::tempdir().expect("tempdir");
    let mut plans: Vec<GenesisPlan> = Vec::new();
    for (charter, persona, expected) in FIXTURES {
        let p = plan(charter, persona).expect("plan");
        assert_eq!(p.archetype, *expected);
        let out = parent.path().join(&p.vertical_id);
        write_vertical(&p, &out).expect("write_vertical");
        plans.push(p);
    }

    let (registry, errors) = argo_vertical::load_dir(parent.path()).expect("load_dir");
    assert!(
        errors.is_empty(),
        "load_dir over a 5-vertical catalogue surfaced errors: {errors:?}"
    );
    let loaded_ids: BTreeSet<&str> = registry.list().map(|(id, _)| id).collect();
    for p in &plans {
        assert!(
            loaded_ids.contains(p.vertical_id.as_str()),
            "vertical {} missing from registry; got: {loaded_ids:?}",
            p.vertical_id
        );
    }

    // Coverage assertion: 5 distinct archetypes proves the test
    // exercises real archetype breadth, not five aliases of the
    // same one.
    let archetypes: HashSet<Archetype> = plans.iter().map(|p| p.archetype).collect();
    assert_eq!(
        archetypes.len(),
        FIXTURES.len(),
        "fixtures must span {} distinct archetypes; got {}",
        FIXTURES.len(),
        archetypes.len()
    );
}
