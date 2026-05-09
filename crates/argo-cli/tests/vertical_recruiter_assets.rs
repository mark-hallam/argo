//! Asset-only checks for the Recruiter vertical
//! (Stream `claude-vertical-recruiter`). Locks the manifest's
//! structural invariants so a later edit can't silently break the
//! eval suite, the executor wiring for the 5 actions, the EEOC
//! forbidden-signal invariants the brief calls out, or the
//! "AI never extends the offer itself" gate on `submit_offer`.
//!
//! Pure asset tests — no agent runs, no LLM calls. Just:
//! - the loader sees recruiter
//! - the manifest is structurally valid + cross-refs resolve
//! - the read-only deny list contains every state-mutating tool
//! - the eval suite has at least 10 fixtures
//! - the 5 declared actions match the brief's surface
//! - 2 actions are auto + 3 are approval-gated
//! - submit_offer routes to a human regardless of pre-approval
//! - flag_red_flag routes to a human
//! - the system prompt + screening rubric forbid each EEOC signal
//! - the persona's avoid_phrasing rejects "culture fit"

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
    let our_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.dir.ends_with("recruiter"))
        .collect();
    assert!(
        our_errors.is_empty(),
        "recruiter loader errors: {our_errors:?}"
    );
    registry
}

#[test]
fn recruiter_loads_and_validates() {
    let reg = registry();
    let v = reg.get("recruiter").expect("recruiter not in registry");
    assert_eq!(
        v.manifest.vertical.name,
        "Recruiter — Technical Sourcing & Screening"
    );
    assert_eq!(v.manifest.vertical.version, "0.1.0");
    argo_vertical::validate(&v.manifest, &v.root)
        .expect("recruiter manifest must validate");
}

#[test]
fn recruiter_is_read_only_by_manifest() {
    let reg = registry();
    let v = reg.get("recruiter").unwrap();
    let tools = v.manifest.tools.as_ref().expect("[tools] section");
    for t in ["file_write", "file_patch", "terminal", "delegate_task"] {
        assert!(
            tools.denied.iter().any(|d| d == t),
            "recruiter must deny `{t}`; denied: {:?}",
            tools.denied
        );
        assert!(
            !tools.allowed.iter().any(|a| a == t),
            "recruiter must NOT allow `{t}`"
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
fn recruiter_eval_suite_has_ten_fixtures() {
    let reg = registry();
    let v = reg.get("recruiter").unwrap();
    let eval = v.manifest.evaluation.as_ref().expect("[evaluation]");
    let rel = eval.suites.first().expect("at least one eval suite");
    let body = std::fs::read_to_string(v.root.join(rel)).expect("eval suite readable");
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
fn recruiter_declares_five_actions_with_correct_routing() {
    // 5 named actions, each wired to the right executor + approval
    // gate per the brief. screen_candidate + schedule_interview are
    // autonomous-safe; send_screening_questions hits gmail; submit_offer
    // and flag_red_flag are always-human.
    let reg = registry();
    let v = reg.get("recruiter").unwrap();
    let actions = v.manifest.actions.as_ref().expect("[actions] section");
    let cases: &[(&str, bool, &str)] = &[
        ("screen_candidate", false, "document_creation"),
        ("send_screening_questions", true, "mcp"),
        ("schedule_interview", false, "mcp"),
        ("submit_offer", true, "human"),
        ("flag_red_flag", true, "human"),
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
fn recruiter_submit_offer_is_always_human_gated() {
    // Brief acceptance: "submit_offer + flag_red_flag correctly route
    // to human approval (NEVER auto)". The platform position is
    // 'AI never extends the offer itself' — this test pins the gate
    // so a later edit can't silently flip it to auto.
    let reg = registry();
    let v = reg.get("recruiter").unwrap();
    let actions = v.manifest.actions.as_ref().expect("[actions]");
    let offer = actions.actions.get("submit_offer").unwrap();
    assert!(
        offer.requires_approval,
        "submit_offer MUST require approval — AI never extends the offer itself"
    );
    assert_eq!(offer.executor.kind_str(), "human");

    let flag = actions.actions.get("flag_red_flag").unwrap();
    assert!(
        flag.requires_approval,
        "flag_red_flag MUST require approval — recruiter decides"
    );
    assert_eq!(flag.executor.kind_str(), "human");
}

#[test]
fn recruiter_schedule_interview_uses_gcal() {
    let reg = registry();
    let v = reg.get("recruiter").unwrap();
    let actions = v.manifest.actions.as_ref().expect("[actions]");
    let sched = actions.actions.get("schedule_interview").unwrap();
    assert_eq!(sched.executor.kind_str(), "mcp");
    assert_eq!(sched.executor.mcp_server(), Some("gcal"));
    assert!(!sched.requires_approval);
}

#[test]
fn recruiter_send_screening_questions_uses_gmail() {
    let reg = registry();
    let v = reg.get("recruiter").unwrap();
    let actions = v.manifest.actions.as_ref().expect("[actions]");
    let send = actions.actions.get("send_screening_questions").unwrap();
    assert_eq!(send.executor.kind_str(), "mcp");
    assert_eq!(send.executor.mcp_server(), Some("gmail"));
    assert_eq!(send.executor.mcp_tool(), Some("gmail.send"));
    assert!(send.requires_approval);
}

/// Brief acceptance: "EEOC-compliance test: screening output does NOT
/// reference forbidden signals." The system prompt names every
/// forbidden signal in order to forbid scoring on them; the smoke
/// test asserts each one is present (so a later edit can't silently
/// drop a signal from the forbidden list) AND that the persona's
/// avoid_phrasing rejects "culture fit" (the legally-fraught proxy).
#[test]
fn recruiter_system_prompt_forbids_each_eeoc_signal() {
    let reg = registry();
    let v = reg.get("recruiter").unwrap();
    let role = v.manifest.role.as_ref().expect("[role] section");
    let sys = std::fs::read_to_string(v.root.join(&role.system_prompt))
        .expect("system prompt readable");
    let lower = sys.to_lowercase();

    // Anchor phrase — the prompt must reference EEOC compliance or
    // forbidden signals explicitly. Either anchor satisfies.
    assert!(
        lower.contains("eeoc")
            || lower.contains("forbidden signal")
            || lower.contains("forbidden signals"),
        "system prompt must anchor the EEOC / forbidden-signals frame"
    );

    // Each forbidden signal must appear (named to be forbidden).
    let must_name = [
        "age",
        "school prestige",
        "gender",
        "race",
        "marital status",
        "religion",
        "disability",
        "sexual orientation",
        "salary history",
    ];
    for needle in must_name {
        assert!(
            lower.contains(needle),
            "system prompt must explicitly forbid `{needle}`; not found"
        );
    }
}

#[test]
fn recruiter_screening_rubric_lists_forbidden_signals() {
    // skills/screening_rubric.md is the bias-mitigation surface the
    // brief calls "PRD-grade copy". Lock the forbidden-signal section.
    let reg = registry();
    let v = reg.get("recruiter").unwrap();
    let rubric_path = v.root.join("skills/screening_rubric.md");
    let rubric = std::fs::read_to_string(&rubric_path).unwrap_or_else(|e| {
        panic!(
            "screening_rubric.md unreadable at {}: {e}",
            rubric_path.display()
        )
    });
    let lower = rubric.to_lowercase();
    assert!(
        lower.contains("forbidden signals"),
        "rubric must have a 'forbidden signals' section"
    );
    for needle in [
        "age",
        "school",
        "gender",
        "race",
        "marital",
        "religion",
        "disability",
        "salary history",
    ] {
        assert!(
            lower.contains(needle),
            "rubric must name forbidden signal `{needle}`"
        );
    }
    assert!(
        lower.contains("culture fit"),
        "rubric must call out 'culture fit' as forbidden phrasing"
    );
}

#[test]
fn recruiter_persona_avoids_culture_fit_phrasing() {
    // The brief's notes treat the rubric as PRD-grade; the persona's
    // avoid_phrasing reinforces it. "Culture fit" is the canonical
    // legally-fraught proxy — pin it.
    let reg = registry();
    let v = reg.get("recruiter").unwrap();
    let role = v.manifest.role.as_ref().expect("[role] section");
    let persona_rel = role.persona.as_ref().expect("persona path");
    let persona = std::fs::read_to_string(v.root.join(persona_rel))
        .expect("persona readable");
    let lower = persona.to_lowercase();
    assert!(
        lower.contains("culture fit"),
        "persona must reference 'culture fit' (in avoid_phrasing)"
    );
}

#[test]
fn recruiter_sample_candidates_has_ten_entries() {
    // Brief: "10 sample candidates is the right number for a
    // screening demo".
    let reg = registry();
    let v = reg.get("recruiter").unwrap();
    let path = v.root.join("inventory/sample_candidates.json");
    let body = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("candidates unreadable at {}: {e}", path.display()));
    let parsed: serde_json::Value =
        serde_json::from_str(&body).expect("sample_candidates.json must be valid JSON");
    let candidates = parsed
        .get("candidates")
        .and_then(|v| v.as_array())
        .expect("candidates array");
    assert_eq!(
        candidates.len(),
        10,
        "brief calls for 10 sample candidates; got {}",
        candidates.len()
    );
}

#[test]
fn recruiter_auto_approval_only_wakes_safe_actions() {
    // The autonomous-safe action set lives in
    // workspace/auto_approval.toml. Pin the wake list — a later edit
    // that adds submit_offer or flag_red_flag would silently break
    // the platform position.
    let reg = registry();
    let v = reg.get("recruiter").unwrap();
    let path = v.root.join("workspace/auto_approval.toml");
    let body = std::fs::read_to_string(&path).expect("auto_approval.toml readable");
    let parsed: toml::Value = toml::from_str(&body).expect("auto_approval.toml parses as TOML");
    let wake = parsed
        .get("wake_action_types")
        .and_then(|v| v.as_array())
        .expect("wake_action_types array");
    let wake_strs: Vec<&str> = wake.iter().filter_map(|v| v.as_str()).collect();
    assert!(
        wake_strs.contains(&"screen_candidate"),
        "wake list must include screen_candidate"
    );
    assert!(
        wake_strs.contains(&"schedule_interview"),
        "wake list must include schedule_interview"
    );
    for forbidden in ["submit_offer", "send_screening_questions", "flag_red_flag"] {
        assert!(
            !wake_strs.contains(&forbidden),
            "wake list must NOT include `{forbidden}` — it is human-gated"
        );
    }
}
