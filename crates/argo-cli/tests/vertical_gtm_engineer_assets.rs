//! Asset-only checks for the GTM Engineer vertical (#692). Locks
//! the manifest's structural invariants so a later edit can't
//! silently break the eval suite, the safety boundary, or the
//! 8-skill bundle.
//!
//! Pure asset tests — no agent runs, no LLM calls. Just:
//! - the loader sees gtm-engineer
//! - the manifest validates + cross-refs resolve
//! - the draft-only deny list contains every state-mutating tool
//!   AND every external-sending tool we know of
//! - all 8 spec'd skills are present in the bundle
//! - the eval suite has the 10 fixtures the spec called for, each
//!   with the required JSON shape
//! - the memory schema parses + carries the 11 entries the agent
//!   relies on
//! - none of the assets leak into another vertical's load (proves
//!   isolation by manifest path-confinement #676)
//! - CLI + gateway discovery surfaces both pick the vertical up

mod common;

use common::{argo_bin, body_of, http_get, pick_free_port};
use std::collections::HashSet;
use std::io::BufReader;
use std::path::Path;
use std::process::{Command, Stdio};

fn repo_verticals_dir() -> &'static Path {
    // Tests run from the crate root; verticals/ is two levels up
    // from crates/argo-cli — the workspace root. Mirror the
    // pr-checker / receptionist asset tests.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(|p| Box::leak(Box::new(p.join("verticals"))).as_path())
        .expect("workspace verticals dir resolves")
}

#[test]
fn gtm_engineer_loads_via_argo_vertical() {
    let dir = repo_verticals_dir();
    let (registry, errors) = argo_vertical::load_dir(dir).expect("load_dir");
    assert!(
        errors.is_empty(),
        "loader produced errors against the shipped verticals: {errors:?}"
    );
    let v = registry
        .get("gtm-engineer")
        .expect("gtm-engineer not in registry");
    assert_eq!(v.manifest.vertical.name, "GTM Engineer");
    assert_eq!(v.manifest.vertical.version, "0.1.0");
    assert!(
        v.manifest.vertical.tags.iter().any(|t| t == "draft-only"),
        "draft-only tag must be set so consumers can filter; got: {:?}",
        v.manifest.vertical.tags
    );
}

#[test]
fn gtm_engineer_validates_clean() {
    let dir = repo_verticals_dir();
    let (registry, _errors) = argo_vertical::load_dir(dir).expect("load_dir");
    let v = registry.get("gtm-engineer").unwrap();
    argo_vertical::validate(&v.manifest, &v.root).expect("gtm-engineer manifest must validate");
}

#[test]
fn gtm_engineer_is_draft_only_by_manifest() {
    // Hard requirement: the GTM Engineer drafts; it never sends.
    // Every state-mutating in-core tool AND every channel that
    // could be misused for sending must be denied. If a future
    // PR adds a new mutating tool to argo-tool, this list should
    // grow accordingly — failure here is the signal.
    let dir = repo_verticals_dir();
    let (registry, _) = argo_vertical::load_dir(dir).unwrap();
    let v = registry.get("gtm-engineer").unwrap();
    let tools = v.manifest.tools.as_ref().expect("[tools] section");
    // #721 added `memory` to the deny list because the in-tree
    // builtin is a stub ("memory store not wired" — lands with
    // Agent H / Phase 3 §14). #722 added `web_search` and
    // `web_extract` for the same reason (the in-tree tools
    // return a clear "configure an MCP backend" error). Once an
    // MCP web backend ships these come off the deny list.
    let mutating = [
        "file_write",
        "file_patch",
        "terminal",
        "delegate_task",
        "memory",
        "web_search",
        "web_extract",
    ];
    for t in &mutating {
        assert!(
            tools.denied.iter().any(|d| d == t),
            "gtm-engineer manifest must deny `{t}` — currently denied: {:?}",
            tools.denied
        );
        assert!(
            !tools.allowed.iter().any(|a| a == t),
            "gtm-engineer manifest must NOT allow `{t}` (draft-only)"
        );
    }
    // Allow-list must be the read-only / draft-only tools
    // and ONLY those. A surprise addition here is suspicious.
    let allowed: HashSet<&str> = tools.allowed.iter().map(String::as_str).collect();
    let expected: HashSet<&str> = ["file_read"].into_iter().collect();
    assert_eq!(
        allowed, expected,
        "gtm-engineer allow-list drift: expected {expected:?}, got {allowed:?}"
    );
}

#[test]
fn gtm_engineer_escalation_lists_delegate_task() {
    // Defence-in-depth: even though delegate_task is in the
    // deny list, the manifest also lists it under
    // [escalation].human_approval_required so a future edit
    // that accidentally removes it from `denied` still fails
    // closed (#660). Same lock pattern as pr-checker.
    let dir = repo_verticals_dir();
    let (registry, _) = argo_vertical::load_dir(dir).unwrap();
    let v = registry.get("gtm-engineer").unwrap();
    let esc = v
        .manifest
        .escalation
        .as_ref()
        .expect("[escalation] section");
    assert!(
        esc.human_approval_required
            .iter()
            .any(|t| t == "delegate_task"),
        "gtm-engineer manifest must list delegate_task under [escalation].human_approval_required; got: {:?}",
        esc.human_approval_required
    );
}

#[test]
fn gtm_engineer_bundles_all_eight_spec_skills() {
    let dir = repo_verticals_dir();
    let (registry, _) = argo_vertical::load_dir(dir).unwrap();
    let v = registry.get("gtm-engineer").unwrap();
    let skills = v.manifest.skills.as_ref().expect("[skills] section");
    let bundle_dir = v.root.join(&skills.bundle);
    let mut found: HashSet<String> = HashSet::new();
    for entry in std::fs::read_dir(&bundle_dir)
        .unwrap_or_else(|e| panic!("read_dir {}: {e}", bundle_dir.display()))
    {
        let path = entry.unwrap().path();
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.ends_with(".skill.md") {
                found.insert(name.to_string());
            }
        }
    }
    let required = [
        "define_icp.skill.md",
        "find_target_accounts.skill.md",
        "research_company.skill.md",
        "research_person.skill.md",
        "generate_outreach.skill.md",
        "analyze_campaign.skill.md",
        "detect_gtm_pattern.skill.md",
        "propose_gtm_skill.skill.md",
    ];
    for name in &required {
        assert!(
            found.contains(*name),
            "missing required skill `{name}` in bundle — present: {found:?}"
        );
    }
    assert_eq!(
        found.len(),
        required.len(),
        "bundle drift — expected exactly {} skills, found {}: {found:?}",
        required.len(),
        found.len()
    );
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
    assert_eq!(
        eval.suites.len(),
        1,
        "spec calls for exactly one suite (gtm_engineer_core.jsonl); got: {:?}",
        eval.suites
    );
    let suite_path = v.root.join(&eval.suites[0]);
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

// ---- end-to-end: discovery surface from #650 sees the new vertical -

#[test]
fn cli_vertical_list_picks_up_repo_gtm_engineer() {
    let out = Command::new(argo_bin())
        .env("ARGO_VERTICALS_DIR", repo_verticals_dir())
        .args(["vertical", "list", "--json"])
        .output()
        .expect("spawn");
    assert!(out.status.success());
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
    assert_eq!(v["ok"], true);
    let loaded = v["loaded"].as_array().unwrap();
    assert!(
        loaded.iter().any(|e| e["id"] == "gtm-engineer"),
        "gtm-engineer not in CLI list: {loaded:?}"
    );
}

#[test]
fn cli_vertical_doctor_passes_for_gtm_engineer() {
    // The composite check (validate + lint + eval) is the
    // operator-facing health probe. If it fails, the vertical is
    // not shippable.
    let out = Command::new(argo_bin())
        .env("ARGO_VERTICALS_DIR", repo_verticals_dir())
        .args(["vertical", "doctor", "gtm-engineer", "--json"])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "doctor must exit 0; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
    assert_eq!(v["ok"], true, "doctor JSON must report ok=true: {v}");
}

#[test]
fn gateway_get_verticals_includes_gtm_engineer() {
    let port = pick_free_port();
    let home = tempfile::tempdir().unwrap();
    let mut child = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", repo_verticals_dir())
        .args([
            "gateway",
            "start",
            "--port",
            &port.to_string(),
            "--max-requests",
            "1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    let mut stderr = BufReader::new(child.stderr.take().unwrap());
    common::await_listening(&mut stderr);

    let resp = http_get(port, "/verticals");
    assert!(resp.contains("HTTP/1.1 200 OK"), "resp={resp}");
    let v: serde_json::Value = serde_json::from_str(body_of(&resp)).unwrap();
    let loaded = v["loaded"].as_array().unwrap();
    assert!(
        loaded.iter().any(|e| e["id"] == "gtm-engineer"),
        "gtm-engineer not in gateway list"
    );

    // Hold stderr alive until the child exits so the gateway
    // doesn't SIGPIPE on its next stderr write.
    let _ = child.wait();
    drop(stderr);
}
