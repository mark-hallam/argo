//! Asset-only checks for the GTM Engineer vertical (#692). Locks
//! the manifest's structural invariants so a later edit can't
//! silently break the safety boundary or the 8-skill bundle.
//!
//! Pure asset tests — no agent runs, no LLM calls. Just:
//! - the loader sees gtm-engineer
//! - the manifest validates + cross-refs resolve
//! - the draft-only deny list contains every state-mutating tool
//!   AND every external-sending tool we know of
//! - all 8 spec'd skills are present in the bundle
//! - CLI + gateway discovery surfaces both pick the vertical up
//!
//! Eval suite, memory schema, role section, and isolation /
//! cross-vertical leak checks live in the sibling test file
//! `vertical_gtm_engineer_assets_eval.rs` (split off to keep both
//! under the 500-LOC ceiling).

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
    // Agent H / Phase 3 §14). #722 also denied `web_search` and
    // `web_extract` because they were stubs at the time; #723
    // shipped real HTTP backends, so those two come back into
    // the allow list and only `memory` remains in the
    // stub-denied set.
    let mutating = [
        "file_write",
        "file_patch",
        "terminal",
        "delegate_task",
        "memory",
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
    // Allow-list must be the read-only / research-only tools
    // and ONLY those. A surprise addition here is suspicious.
    let allowed: HashSet<&str> = tools.allowed.iter().map(String::as_str).collect();
    let expected: HashSet<&str> = ["file_read", "web_search", "web_extract"]
        .into_iter()
        .collect();
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
