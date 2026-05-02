//! Asset checks for the AI Receptionist vertical (#657). Locks
//! the manifest's structural invariants so a later edit can't
//! silently break the eval suite or the deny list.

mod common;

use std::path::Path;

fn repo_verticals_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(|p| Box::leak(Box::new(p.join("verticals"))).as_path())
        .expect("workspace verticals dir resolves")
}

#[test]
fn receptionist_loads_via_argo_vertical() {
    let dir = repo_verticals_dir();
    let (registry, errors) = argo_vertical::load_dir(dir).expect("load_dir");
    assert!(errors.is_empty(), "shipped verticals must all load: {errors:?}");
    let r = registry.get("receptionist").expect("receptionist not in registry");
    assert_eq!(r.manifest.vertical.name, "AI Receptionist");
}

#[test]
fn receptionist_validates_clean() {
    let (registry, _) = argo_vertical::load_dir(repo_verticals_dir()).unwrap();
    let r = registry.get("receptionist").unwrap();
    argo_vertical::validate(&r.manifest, &r.root).expect("must validate");
}

#[test]
fn receptionist_is_conversation_only_by_manifest() {
    let (registry, _) = argo_vertical::load_dir(repo_verticals_dir()).unwrap();
    let r = registry.get("receptionist").unwrap();
    let tools = r.manifest.tools.as_ref().expect("[tools]");
    // Same security boundary as PR-Checker: every state-mutating
    // tool must be explicitly denied so capability enforcement
    // (#655) blocks them.
    let mutating = ["file_write", "file_patch", "terminal", "delegate_task"];
    for t in &mutating {
        assert!(
            tools.denied.iter().any(|d| d == t),
            "receptionist must deny `{t}`; current denied: {:?}",
            tools.denied
        );
        assert!(
            !tools.allowed.iter().any(|a| a == t),
            "receptionist must NOT allow `{t}`"
        );
    }
}

#[test]
fn receptionist_eval_suite_is_substantial() {
    let (registry, _) = argo_vertical::load_dir(repo_verticals_dir()).unwrap();
    let r = registry.get("receptionist").unwrap();
    let eval = r.manifest.evaluation.as_ref().expect("[evaluation]");
    assert!(!eval.suites.is_empty());
    for rel in &eval.suites {
        let p = r.root.join(rel);
        let body = std::fs::read_to_string(&p).expect("readable");
        let lines: Vec<&str> = body.lines().filter(|l| !l.trim().is_empty()).collect();
        assert!(
            lines.len() >= 10,
            "{rel} should have at least 10 fixtures (got {})",
            lines.len()
        );
    }
}

#[test]
fn receptionist_distinct_from_pr_checker() {
    // The whole point of the second vertical is to prove the
    // abstraction generalizes. Both should load through the
    // same registry; their tool surfaces should be DIFFERENT
    // (each declares its own allow/deny).
    let (registry, _) = argo_vertical::load_dir(repo_verticals_dir()).unwrap();
    let r = registry.get("receptionist").unwrap();
    let p = registry.get("pr-checker").unwrap();

    let r_allowed: std::collections::BTreeSet<&str> = r
        .manifest
        .tools
        .as_ref()
        .map(|t| t.allowed.iter().map(String::as_str).collect())
        .unwrap_or_default();
    let p_allowed: std::collections::BTreeSet<&str> = p
        .manifest
        .tools
        .as_ref()
        .map(|t| t.allowed.iter().map(String::as_str).collect())
        .unwrap_or_default();
    assert_ne!(r_allowed, p_allowed, "two verticals should have different tool surfaces");

    // Concrete distinguishing surface: PR-Checker uses
    // `web_extract` to fetch PR diffs; the receptionist
    // doesn't need it.
    assert!(p_allowed.contains("web_extract"));
    assert!(!r_allowed.contains("web_extract"));
    // The receptionist uses memory + file_read for FAQ lookup;
    // PR-Checker uses file_read + web_extract for diff
    // inspection. Both allow `memory` (the platform's general-
    // purpose key/value store), so that's not a distinguisher.
}
