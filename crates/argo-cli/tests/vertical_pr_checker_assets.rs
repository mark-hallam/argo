//! Asset-only checks for the PR-Checker vertical (#652). Locks
//! the manifest's structural invariants so a later edit can't
//! silently break the eval suite or the system-prompt
//! reference.
//!
//! Pure asset tests — no agent runs, no LLM calls. Just:
//! - the loader sees pr-checker
//! - the manifest is structurally valid + cross-refs resolve
//! - the read-only deny list contains every state-mutating tool
//! - the eval suite has at least one fixture

mod common;

use common::{argo_bin, body_of, http_get, pick_free_port};
use std::io::BufReader;
use std::path::Path;
use std::process::{Command, Stdio};

fn repo_verticals_dir() -> &'static Path {
    // Tests run from the crate root; verticals/ is two levels up
    // from crates/argo-cli — the workspace root.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(|p| Box::leak(Box::new(p.join("verticals"))).as_path())
        .expect("workspace verticals dir resolves")
}

#[test]
fn pr_checker_loads_via_argo_vertical() {
    let dir = repo_verticals_dir();
    let (registry, errors) = argo_vertical::load_dir(dir).expect("load_dir");
    assert!(
        errors.is_empty(),
        "loader produced errors against the shipped verticals: {errors:?}"
    );
    let pr = registry
        .get("pr-checker")
        .expect("pr-checker not in registry");
    assert_eq!(pr.manifest.vertical.name, "Pull Request Checker");
    assert_eq!(pr.manifest.vertical.version, "0.1.0");
}

#[test]
fn pr_checker_validates_clean() {
    let dir = repo_verticals_dir();
    let (registry, _errors) = argo_vertical::load_dir(dir).expect("load_dir");
    let pr = registry.get("pr-checker").unwrap();
    argo_vertical::validate(&pr.manifest, &pr.root).expect("pr-checker manifest must validate");
}

#[test]
fn pr_checker_is_read_only_by_manifest() {
    let dir = repo_verticals_dir();
    let (registry, _) = argo_vertical::load_dir(dir).expect("load_dir");
    let pr = registry.get("pr-checker").unwrap();
    let tools = pr.manifest.tools.as_ref().expect("[tools] section");
    // Hard requirement: every state-mutating in-core tool must be
    // explicitly denied. If a future PR adds a new mutating tool
    // to argo-tool, this list should grow accordingly — failure
    // here is the signal.
    let mutating = ["file_write", "file_patch", "terminal", "delegate_task"];
    for t in &mutating {
        assert!(
            tools.denied.iter().any(|d| d == t),
            "pr-checker manifest must deny `{t}` — currently denied: {:?}",
            tools.denied
        );
        assert!(
            !tools.allowed.iter().any(|a| a == t),
            "pr-checker manifest must NOT allow `{t}`"
        );
    }
}

#[test]
fn pr_checker_escalation_lists_delegate_task_for_human_approval() {
    // Defence-in-depth: even though delegate_task is in the
    // deny list (deny-wins), the manifest also lists it under
    // [escalation].human_approval_required so a future edit
    // that accidentally removes it from `denied` still fails
    // closed (#660). Locks the operator-facing intent.
    let dir = repo_verticals_dir();
    let (registry, _) = argo_vertical::load_dir(dir).unwrap();
    let pr = registry.get("pr-checker").unwrap();
    let esc = pr
        .manifest
        .escalation
        .as_ref()
        .expect("[escalation] section");
    assert!(
        esc.human_approval_required.iter().any(|t| t == "delegate_task"),
        "pr-checker manifest must list delegate_task under [escalation].human_approval_required; got: {:?}",
        esc.human_approval_required
    );
}

#[test]
fn pr_checker_eval_suite_is_non_empty() {
    let dir = repo_verticals_dir();
    let (registry, _) = argo_vertical::load_dir(dir).expect("load_dir");
    let pr = registry.get("pr-checker").unwrap();
    let eval = pr.manifest.evaluation.as_ref().expect("[evaluation]");
    assert!(!eval.suites.is_empty(), "pr-checker needs at least one eval suite");

    for rel in &eval.suites {
        let p = pr.root.join(rel);
        let body = std::fs::read_to_string(&p).unwrap_or_else(|e| {
            panic!("eval suite {rel} unreadable at {}: {e}", p.display())
        });
        let lines: Vec<&str> = body.lines().filter(|l| !l.trim().is_empty()).collect();
        assert!(
            lines.len() >= 5,
            "eval suite {rel} should have at least 5 fixtures (has {})",
            lines.len()
        );
        // Each line must parse as JSON with id + input + expected.
        for (i, line) in lines.iter().enumerate() {
            let v: serde_json::Value = serde_json::from_str(line)
                .unwrap_or_else(|e| panic!("eval suite {rel} line {} bad JSON: {e}", i + 1));
            for key in ["id", "input", "expected"] {
                assert!(
                    v.get(key).is_some(),
                    "eval suite {rel} line {} missing key '{key}'",
                    i + 1
                );
            }
        }
    }
}

// ---- end-to-end: discovery surface from #650 sees the new vertical -

#[test]
fn cli_vertical_list_picks_up_repo_pr_checker() {
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
        loaded.iter().any(|e| e["id"] == "pr-checker"),
        "pr-checker not in CLI list: {loaded:?}"
    );
}

#[test]
fn gateway_get_verticals_includes_pr_checker() {
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
        loaded.iter().any(|e| e["id"] == "pr-checker"),
        "pr-checker not in gateway list"
    );

    // Hold stderr alive until the child exits so the gateway
    // doesn't SIGPIPE on its next stderr write.
    let _ = child.wait();
    drop(stderr);
}
