//! Eval / memory / role / isolation asset checks for the GTM
//! Engineer vertical (#692). Sibling to
//! `vertical_gtm_engineer_assets.rs`; together they lock the
//! manifest's structural invariants. Split from the parent test
//! file to keep both under the 500-LOC ceiling.
//!
//! Pure asset tests — no agent runs, no LLM calls, no spawned
//! `argo` binary. Just:
//! - the eval suite has the 10 fixtures the spec called for, each
//!   with the required JSON shape
//! - the memory schema parses + carries the 11 entries the agent
//!   relies on
//! - the role section points at extant files
//! - none of the assets leak into another vertical's load (proves
//!   isolation by manifest path-confinement #676)
//! - pr-checker's memory schema does not pick up GTM-only keys

use std::collections::HashSet;
use std::path::Path;

fn repo_verticals_dir() -> &'static Path {
    // Tests run from the crate root; verticals/ is two levels up
    // from crates/argo-cli — the workspace root. Mirror the
    // sibling asset test file.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(|p| Box::leak(Box::new(p.join("verticals"))).as_path())
        .expect("workspace verticals dir resolves")
}

#[test]
fn gtm_engineer_eval_suite_has_ten_required_fixtures() {
    // The spec called for exactly these 10 fixture ids covering
    // ICP definition, abstention, hallucination avoidance,
    // personalization, role-only personalization, low-reply
    // analysis, objection patterns, strong-evidence skill
    // proposals, weak-evidence refusals, and safety (no send).
    // A future edit that drops or renames a fixture changes the
    // contract — the test is the lock.
    let dir = repo_verticals_dir();
    let (registry, _) = argo_vertical::load_dir(dir).unwrap();
    let v = registry.get("gtm-engineer").unwrap();
    let eval = v.manifest.evaluation.as_ref().expect("[evaluation]");
    // The original spec called for exactly gtm_engineer_core.jsonl.
    // Worker v3 (#759) added a second suite alongside
    // (worker_v3_emission.jsonl) — that's allowed; this test
    // still locks the core suite being present with the original
    // 10 fixtures.
    assert!(
        !eval.suites.is_empty(),
        "[evaluation].suites must declare at least gtm_engineer_core.jsonl"
    );
    let core_suite = eval
        .suites
        .iter()
        .find(|s| s.ends_with("gtm_engineer_core.jsonl"))
        .unwrap_or_else(|| {
            panic!(
                "gtm_engineer_core.jsonl must be in the declared suites; got: {:?}",
                eval.suites
            )
        });
    let suite_path = v.root.join(core_suite);
    let body = std::fs::read_to_string(&suite_path)
        .unwrap_or_else(|e| panic!("eval suite unreadable at {}: {e}", suite_path.display()));
    let lines: Vec<&str> = body.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(
        lines.len(),
        10,
        "spec calls for exactly 10 fixtures; got {}",
        lines.len()
    );
    let required_ids = [
        "gtm_icp_skycot",
        "gtm_abstain_weak_context",
        "gtm_account_research_missing_evidence",
        "gtm_company_personalization",
        "gtm_role_based_personalization",
        "gtm_low_reply_analysis",
        "gtm_objection_pattern",
        "gtm_propose_skill_strong_evidence",
        "gtm_no_skill_from_weak_evidence",
        "gtm_safety_no_send",
    ];
    let mut got_ids: HashSet<String> = HashSet::new();
    for (i, line) in lines.iter().enumerate() {
        let v: serde_json::Value = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("eval suite line {} bad JSON: {e}", i + 1));
        for key in ["id", "input", "expected"] {
            assert!(
                v.get(key).is_some(),
                "eval suite line {} missing key '{key}'",
                i + 1
            );
        }
        got_ids.insert(v["id"].as_str().unwrap().to_string());
    }
    for id in &required_ids {
        assert!(
            got_ids.contains(*id),
            "spec-required fixture id `{id}` is missing"
        );
    }
}

#[test]
fn gtm_engineer_memory_schema_parses_with_eleven_entries() {
    // The spec calls for 11 role-scoped memory namespaces. Lock
    // every key so a future edit that drops or renames one changes
    // the agent's contract loudly.
    let dir = repo_verticals_dir();
    let (registry, _) = argo_vertical::load_dir(dir).unwrap();
    let v = registry.get("gtm-engineer").unwrap();
    let mem = v.manifest.memory.as_ref().expect("[memory] section");
    let schema_rel = mem.schema.as_ref().expect("[memory].schema set");
    let body = std::fs::read_to_string(v.root.join(schema_rel))
        .unwrap_or_else(|e| panic!("memory schema unreadable: {e}"));
    let schema = argo_vertical::parse_memory_schema(&body).expect("memory schema must parse");
    assert_eq!(
        schema.entries.len(),
        11,
        "spec calls for 11 memory entries; got {}",
        schema.entries.len()
    );
    let required_keys = [
        "icp",
        "target_segments",
        "account_research",
        "persona_research",
        "winning_messages",
        "failed_messages",
        "objections",
        "campaign_results",
        "patterns",
        "winning_plays",
        "learned_skill_candidates",
    ];
    let got: HashSet<&str> = schema.entries.iter().map(|e| e.key.as_str()).collect();
    for k in &required_keys {
        assert!(
            got.contains(*k),
            "spec-required memory entry `{k}` missing — got: {got:?}"
        );
    }
}

#[test]
fn gtm_engineer_role_section_points_at_extant_files() {
    let dir = repo_verticals_dir();
    let (registry, _) = argo_vertical::load_dir(dir).unwrap();
    let v = registry.get("gtm-engineer").unwrap();
    let role = v.manifest.role.as_ref().expect("[role] section");
    let sys = v.root.join(&role.system_prompt);
    assert!(
        sys.is_file(),
        "system_prompt path must resolve to a file: {}",
        sys.display()
    );
    let body = std::fs::read_to_string(&sys).expect("system prompt must read");
    assert!(
        body.contains("GTM Engineer"),
        "system prompt should self-identify as GTM Engineer"
    );
    if let Some(persona_rel) = role.persona.as_deref() {
        let persona = v.root.join(persona_rel);
        assert!(
            persona.is_file(),
            "persona path must resolve to a file: {}",
            persona.display()
        );
    }
}

#[test]
fn gtm_engineer_assets_do_not_leak_into_pr_checker() {
    // Isolation invariant (#676 + #678): a vertical's assets must
    // be reachable only through its own root. If pr-checker were
    // ever resolved to gtm-engineer's skill bundle (e.g. via a
    // path-traversal regression), it would silently inherit the
    // 8 GTM skills. The manifest path-confinement check rejects
    // that at load time; this test confirms the static result.
    let dir = repo_verticals_dir();
    let (registry, _) = argo_vertical::load_dir(dir).unwrap();
    let pr = registry.get("pr-checker").expect("pr-checker present");
    let pr_skills = pr.manifest.skills.as_ref().unwrap();
    let pr_bundle = pr.root.join(&pr_skills.bundle);
    let mut pr_skill_names: HashSet<String> = HashSet::new();
    for entry in std::fs::read_dir(&pr_bundle).unwrap() {
        let path = entry.unwrap().path();
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.ends_with(".skill.md") {
                pr_skill_names.insert(name.to_string());
            }
        }
    }
    let leaked = [
        "define_icp.skill.md",
        "find_target_accounts.skill.md",
        "generate_outreach.skill.md",
        "propose_gtm_skill.skill.md",
    ];
    for name in &leaked {
        assert!(
            !pr_skill_names.contains(*name),
            "GTM Engineer skill `{name}` leaked into pr-checker bundle — isolation broken"
        );
    }
    // Symmetric check: gtm-engineer's root path is distinct from
    // pr-checker's root path. A registry that returns the same root
    // for both ids would be a smoking gun.
    let gtm = registry.get("gtm-engineer").unwrap();
    assert_ne!(gtm.root, pr.root, "vertical roots must be distinct");
}

#[test]
fn gtm_engineer_memory_schema_does_not_leak_keys_into_pr_checker() {
    // Symmetric isolation check on the memory schema side (#680
    // namespacing): pr-checker's schema must not pick up the
    // GTM-specific keys.
    let dir = repo_verticals_dir();
    let (registry, _) = argo_vertical::load_dir(dir).unwrap();
    let pr = registry.get("pr-checker").unwrap();
    let pr_mem = pr.manifest.memory.as_ref().expect("[memory] section");
    let pr_schema_rel = pr_mem.schema.as_ref().expect("[memory].schema set");
    let pr_body = std::fs::read_to_string(pr.root.join(pr_schema_rel)).unwrap();
    let pr_schema = argo_vertical::parse_memory_schema(&pr_body).unwrap();
    let pr_keys: HashSet<&str> = pr_schema.entries.iter().map(|e| e.key.as_str()).collect();
    let gtm_only = [
        "icp",
        "account_research",
        "campaign_results",
        "learned_skill_candidates",
    ];
    for key in &gtm_only {
        assert!(
            !pr_keys.contains(*key),
            "GTM Engineer memory key `{key}` leaked into pr-checker schema"
        );
    }
}
