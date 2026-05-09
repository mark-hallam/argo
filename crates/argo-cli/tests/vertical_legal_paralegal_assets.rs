//! Asset-only checks for the Legal Paralegal vertical
//! (Stream `claude-vertical-legal-paralegal`). Locks the manifest's
//! structural invariants so a later edit can't silently break the
//! eval suite, the executor wiring for the 5 actions, the deny-list
//! invariants, or the cardinal-rule disclaimer.
//!
//! Pure asset tests — no agent runs, no LLM calls.

use std::path::Path;

fn repo_verticals_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(|p| Box::leak(Box::new(p.join("verticals"))).as_path())
        .expect("workspace verticals dir resolves")
}

fn registry() -> argo_vertical::Registry {
    let (registry, errors) =
        argo_vertical::load_dir(repo_verticals_dir()).expect("load_dir");
    let lp_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.dir.ends_with("legal-paralegal"))
        .collect();
    assert!(
        lp_errors.is_empty(),
        "legal-paralegal loader errors: {lp_errors:?}"
    );
    registry
}

#[test]
fn legal_paralegal_loads_and_validates() {
    let reg = registry();
    let v = reg
        .get("legal-paralegal")
        .expect("legal-paralegal not in registry");
    assert_eq!(v.manifest.vertical.name, "Legal — Paralegal");
    assert_eq!(v.manifest.vertical.version, "0.1.0");
    argo_vertical::validate(&v.manifest, &v.root)
        .expect("legal-paralegal manifest must validate");
}

#[test]
fn legal_paralegal_is_read_only_by_manifest() {
    let reg = registry();
    let v = reg.get("legal-paralegal").unwrap();
    let tools = v.manifest.tools.as_ref().expect("[tools] section");
    for t in ["file_write", "file_patch", "terminal", "delegate_task"] {
        assert!(
            tools.denied.iter().any(|d| d == t),
            "legal-paralegal must deny `{t}`; denied: {:?}",
            tools.denied
        );
        assert!(
            !tools.allowed.iter().any(|a| a == t),
            "legal-paralegal must NOT allow `{t}`"
        );
    }
    let esc = v
        .manifest
        .escalation
        .as_ref()
        .expect("[escalation] section");
    assert!(
        esc.human_approval_required
            .iter()
            .any(|t| t == "delegate_task"),
        "delegate_task must be in [escalation].human_approval_required"
    );
}

#[test]
fn legal_paralegal_eval_suite_has_ten_fixtures() {
    let reg = registry();
    let v = reg.get("legal-paralegal").unwrap();
    let eval = v.manifest.evaluation.as_ref().expect("[evaluation]");
    let rel = eval.suites.first().expect("at least one eval suite");
    let body =
        std::fs::read_to_string(v.root.join(rel)).expect("eval suite readable");
    let lines: Vec<&str> = body.lines().filter(|l| !l.trim().is_empty()).collect();
    assert!(
        lines.len() >= 10,
        "eval suite should have at least 10 fixtures (has {})",
        lines.len()
    );
    for (i, line) in lines.iter().enumerate() {
        let parsed: serde_json::Value = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("eval line {} bad JSON: {e}", i + 1));
        for key in ["id", "input", "expected"] {
            assert!(
                parsed.get(key).is_some(),
                "eval line {} missing key '{key}'",
                i + 1
            );
        }
    }
}

#[test]
fn legal_paralegal_declares_five_actions_with_correct_routing() {
    // 5 named actions, each wired to the right executor + approval
    // gate per the brief. flag_clause_concern + file_with_court
    // route to a human; draft_amendment uses document_creation;
    // review_contract + generate_summary are autonomous-safe.
    let reg = registry();
    let v = reg.get("legal-paralegal").unwrap();
    let actions = v.manifest.actions.as_ref().expect("[actions] section");
    let cases: &[(&str, bool, &str)] = &[
        ("review_contract", false, "human"),
        ("flag_clause_concern", true, "human"),
        ("draft_amendment", true, "document_creation"),
        ("file_with_court", true, "human"),
        ("generate_summary", false, "human"),
    ];
    for (name, requires_approval, executor_kind) in cases {
        let a = actions
            .actions
            .get(*name)
            .unwrap_or_else(|| panic!("{name} declared"));
        assert_eq!(
            a.requires_approval, *requires_approval,
            "{name} requires_approval mismatch"
        );
        assert_eq!(
            a.executor.kind_str(),
            *executor_kind,
            "{name} executor kind mismatch"
        );
    }
}

#[test]
fn legal_paralegal_persona_refuses_legal_advice() {
    // The brief is explicit: "Persona MUST decline to give legal
    // advice; system_prompt enforces 'I can summarize but not
    // advise — attorney required.'" Lock the disclaimer text into
    // both the system prompt and the persona so a later edit can't
    // silently soften the boundary.
    let reg = registry();
    let v = reg.get("legal-paralegal").unwrap();
    let role = v.manifest.role.as_ref().expect("[role] section");
    let sys = std::fs::read_to_string(v.root.join(&role.system_prompt))
        .expect("system prompt readable");
    assert!(
        sys.contains("not advise") && sys.contains("attorney"),
        "system prompt must carry the 'not advise — attorney' disclaimer"
    );
    let persona_rel = role.persona.as_ref().expect("persona path required");
    let persona = std::fs::read_to_string(v.root.join(persona_rel))
        .expect("persona readable");
    assert!(
        persona.contains("not advise") && persona.contains("attorney"),
        "persona must reinforce the 'not advise' boundary"
    );
}
