//! Asset-only checks for the Real Estate Agent vertical
//! (Stream `claude-vertical-real-estate-agent`). Locks the manifest's
//! structural invariants so a later edit can't silently break the
//! eval suite, the executor wiring for the 5 actions, the Fair
//! Housing language in the system prompt, or the deny-list invariants
//! the brief lists.
//!
//! Pure asset tests — no agent runs, no LLM calls. Just:
//! - the loader sees real-estate-agent
//! - the manifest is structurally valid + cross-refs resolve
//! - the read-only deny list contains every state-mutating tool
//! - the eval suite has at least 10 fixtures (per brief)
//! - the 5 declared actions match the brief's surface
//! - 2 actions are auto + 3 are approval-gated, exercising 3 channels
//! - the system prompt carries Fair Housing language
//! - sample listings (8) are present

use std::path::Path;

fn repo_verticals_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(|p| Box::leak(Box::new(p.join("verticals"))).as_path())
        .expect("workspace verticals dir resolves")
}

#[test]
fn real_estate_agent_loads_via_argo_vertical() {
    let dir = repo_verticals_dir();
    let (registry, errors) = argo_vertical::load_dir(dir).expect("load_dir");
    let our_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.dir.ends_with("real-estate-agent"))
        .collect();
    assert!(
        our_errors.is_empty(),
        "real-estate-agent loader errors: {our_errors:?}"
    );
    let v = registry
        .get("real-estate-agent")
        .expect("real-estate-agent not in registry");
    assert_eq!(v.manifest.vertical.name, "Real Estate — Agent Assistant");
    assert_eq!(v.manifest.vertical.version, "0.1.0");
}

#[test]
fn real_estate_agent_validates_clean() {
    let dir = repo_verticals_dir();
    let (registry, _errors) = argo_vertical::load_dir(dir).expect("load_dir");
    let v = registry.get("real-estate-agent").unwrap();
    argo_vertical::validate(&v.manifest, &v.root)
        .expect("real-estate-agent manifest must validate");
}

#[test]
fn real_estate_agent_is_read_only_by_manifest() {
    let dir = repo_verticals_dir();
    let (registry, _) = argo_vertical::load_dir(dir).expect("load_dir");
    let v = registry.get("real-estate-agent").unwrap();
    let tools = v.manifest.tools.as_ref().expect("[tools] section");
    let mutating = ["file_write", "file_patch", "terminal", "delegate_task"];
    for t in &mutating {
        assert!(
            tools.denied.iter().any(|d| d == t),
            "real-estate-agent manifest must deny `{t}`; currently denied: {:?}",
            tools.denied
        );
        assert!(
            !tools.allowed.iter().any(|a| a == t),
            "real-estate-agent manifest must NOT allow `{t}`"
        );
    }
}

#[test]
fn real_estate_agent_eval_suite_has_at_least_ten_fixtures() {
    let dir = repo_verticals_dir();
    let (registry, _) = argo_vertical::load_dir(dir).expect("load_dir");
    let v = registry.get("real-estate-agent").unwrap();
    let eval = v.manifest.evaluation.as_ref().expect("[evaluation]");
    assert!(!eval.suites.is_empty(), "needs at least one eval suite");
    for rel in &eval.suites {
        let p = v.root.join(rel);
        let body = std::fs::read_to_string(&p)
            .unwrap_or_else(|e| panic!("eval suite {rel} unreadable at {}: {e}", p.display()));
        let lines: Vec<&str> = body.lines().filter(|l| !l.trim().is_empty()).collect();
        assert!(
            lines.len() >= 10,
            "real-estate-agent eval suite {rel} should have at least 10 fixtures (has {})",
            lines.len()
        );
        for (i, line) in lines.iter().enumerate() {
            let parsed: serde_json::Value = serde_json::from_str(line)
                .unwrap_or_else(|e| panic!("eval suite {rel} line {} bad JSON: {e}", i + 1));
            for key in ["id", "input", "expected"] {
                assert!(
                    parsed.get(key).is_some(),
                    "eval suite {rel} line {} missing key '{key}'",
                    i + 1
                );
            }
        }
    }
}

#[test]
fn real_estate_agent_declares_five_named_actions() {
    let dir = repo_verticals_dir();
    let (registry, _) = argo_vertical::load_dir(dir).expect("load_dir");
    let v = registry.get("real-estate-agent").unwrap();
    let actions = v.manifest.actions.as_ref().expect("[actions] section");
    let expected = [
        "schedule_viewing",
        "send_listing_packet",
        "draft_offer",
        "flag_inspection_finding",
        "follow_up_lead",
    ];
    for name in expected {
        assert!(
            actions.actions.contains_key(name),
            "real-estate-agent manifest must declare action `{name}`; got: {:?}",
            actions.actions.keys().collect::<Vec<_>>()
        );
    }
}

#[test]
fn real_estate_agent_action_gates_split_two_auto_three_approval() {
    let dir = repo_verticals_dir();
    let (registry, _) = argo_vertical::load_dir(dir).expect("load_dir");
    let v = registry.get("real-estate-agent").unwrap();
    let actions = v.manifest.actions.as_ref().expect("[actions]");

    let auto = ["schedule_viewing", "follow_up_lead"];
    for name in &auto {
        let a = actions
            .actions
            .get(*name)
            .unwrap_or_else(|| panic!("missing action `{name}`"));
        assert!(
            !a.requires_approval,
            "`{name}` must be autonomous-safe (requires_approval = false)"
        );
    }
    let gated = ["send_listing_packet", "draft_offer", "flag_inspection_finding"];
    for name in &gated {
        let a = actions
            .actions
            .get(*name)
            .unwrap_or_else(|| panic!("missing action `{name}`"));
        assert!(
            a.requires_approval,
            "`{name}` must be approval-gated (requires_approval = true)"
        );
    }
}

#[test]
fn real_estate_agent_three_gated_actions_use_three_distinct_channels() {
    // Brief acceptance: send_listing_packet (gmail.send), draft_offer
    // (document_creation), flag_inspection_finding (human) — three
    // different NotifyChannel shapes.
    let dir = repo_verticals_dir();
    let (registry, _) = argo_vertical::load_dir(dir).expect("load_dir");
    let v = registry.get("real-estate-agent").unwrap();
    let actions = v.manifest.actions.as_ref().expect("[actions]");

    let packet = actions.actions.get("send_listing_packet").unwrap();
    assert_eq!(packet.executor.kind_str(), "mcp");
    assert_eq!(packet.executor.mcp_server(), Some("gmail"));
    assert_eq!(packet.executor.mcp_tool(), Some("gmail.send"));

    let offer = actions.actions.get("draft_offer").unwrap();
    assert_eq!(offer.executor.kind_str(), "document_creation");

    let flag = actions.actions.get("flag_inspection_finding").unwrap();
    assert_eq!(flag.executor.kind_str(), "human");
}

#[test]
fn real_estate_agent_schedule_viewing_uses_gcal() {
    let dir = repo_verticals_dir();
    let (registry, _) = argo_vertical::load_dir(dir).expect("load_dir");
    let v = registry.get("real-estate-agent").unwrap();
    let actions = v.manifest.actions.as_ref().expect("[actions]");
    let sched = actions.actions.get("schedule_viewing").unwrap();
    assert_eq!(sched.executor.kind_str(), "mcp");
    assert_eq!(sched.executor.mcp_server(), Some("gcal"));
    assert!(!sched.requires_approval);
}

#[test]
fn real_estate_agent_system_prompt_carries_fair_housing_language() {
    let dir = repo_verticals_dir();
    let (registry, _) = argo_vertical::load_dir(dir).expect("load_dir");
    let v = registry.get("real-estate-agent").unwrap();
    let role = v.manifest.role.as_ref().expect("[role] section");
    let path = v.root.join(&role.system_prompt);
    let body = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("system prompt unreadable at {}: {e}", path.display()));
    let lower = body.to_lowercase();
    assert!(
        lower.contains("fair housing"),
        "system prompt MUST mention Fair Housing"
    );
    assert!(
        lower.contains("protected"),
        "system prompt MUST reference protected classes"
    );
}

#[test]
fn real_estate_agent_sample_listings_has_eight_entries() {
    let dir = repo_verticals_dir();
    let (registry, _) = argo_vertical::load_dir(dir).expect("load_dir");
    let v = registry.get("real-estate-agent").unwrap();
    let path = v.root.join("inventory/sample_listings.json");
    let body = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("listings unreadable at {}: {e}", path.display()));
    let parsed: serde_json::Value =
        serde_json::from_str(&body).expect("sample_listings.json must be valid JSON");
    let listings = parsed
        .get("listings")
        .and_then(|v| v.as_array())
        .expect("listings array");
    assert_eq!(
        listings.len(),
        8,
        "brief calls for 8 sample listings; got {}",
        listings.len()
    );
}
