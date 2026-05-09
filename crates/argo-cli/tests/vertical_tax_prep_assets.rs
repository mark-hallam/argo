//! Asset-only checks for the Tax Prep vertical
//! (Stream `claude-vertical-tax-prep`). Locks the manifest's
//! structural invariants so a later edit can't silently break the
//! eval suite, the executor wiring for the 5 actions, the
//! "no real SSNs" guarantee on the W-2 inventory, the standing
//! disclaimer in the system prompt, or the file_return human-gate
//! invariant the brief lists.
//!
//! Pure asset tests — no agent runs, no LLM calls. Just:
//! - the loader sees tax-prep
//! - the manifest is structurally valid + cross-refs resolve
//! - the read-only deny list contains every state-mutating tool
//! - the eval suite has at least 10 fixtures (per brief)
//! - the 5 declared actions match the brief's surface
//! - 2 actions are auto + 3 are approval-gated (file_return is one)
//! - the 5 actions exercise 3 distinct executor channels
//!   (mcp/gcal, document_creation, human)
//! - the system prompt carries the "licensed preparer must sign and
//!   file" disclaimer
//! - sample W-2s (5) are present + every SSN matches the synthetic
//!   `000-00-XXXX` pattern (no real-looking SSNs)

use std::path::Path;

fn repo_verticals_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(|p| Box::leak(Box::new(p.join("verticals"))).as_path())
        .expect("workspace verticals dir resolves")
}

#[test]
fn tax_prep_loads_via_argo_vertical() {
    let dir = repo_verticals_dir();
    let (registry, errors) = argo_vertical::load_dir(dir).expect("load_dir");
    let our_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.dir.ends_with("tax-prep"))
        .collect();
    assert!(
        our_errors.is_empty(),
        "tax-prep loader errors: {our_errors:?}"
    );
    let v = registry
        .get("tax-prep")
        .expect("tax-prep not in registry");
    assert_eq!(v.manifest.vertical.name, "Tax Preparation Assistant");
    assert_eq!(v.manifest.vertical.version, "0.1.0");
}

#[test]
fn tax_prep_validates_clean() {
    let dir = repo_verticals_dir();
    let (registry, _errors) = argo_vertical::load_dir(dir).expect("load_dir");
    let v = registry.get("tax-prep").unwrap();
    argo_vertical::validate(&v.manifest, &v.root)
        .expect("tax-prep manifest must validate");
}

#[test]
fn tax_prep_is_read_only_by_manifest() {
    let dir = repo_verticals_dir();
    let (registry, _) = argo_vertical::load_dir(dir).expect("load_dir");
    let v = registry.get("tax-prep").unwrap();
    let tools = v.manifest.tools.as_ref().expect("[tools] section");
    let mutating = ["file_write", "file_patch", "terminal", "delegate_task"];
    for t in &mutating {
        assert!(
            tools.denied.iter().any(|d| d == t),
            "tax-prep manifest must deny `{t}`; currently denied: {:?}",
            tools.denied
        );
        assert!(
            !tools.allowed.iter().any(|a| a == t),
            "tax-prep manifest must NOT allow `{t}`"
        );
    }
}

#[test]
fn tax_prep_eval_suite_has_at_least_ten_fixtures() {
    let dir = repo_verticals_dir();
    let (registry, _) = argo_vertical::load_dir(dir).expect("load_dir");
    let v = registry.get("tax-prep").unwrap();
    let eval = v.manifest.evaluation.as_ref().expect("[evaluation]");
    assert!(!eval.suites.is_empty(), "needs at least one eval suite");
    for rel in &eval.suites {
        let p = v.root.join(rel);
        let body = std::fs::read_to_string(&p)
            .unwrap_or_else(|e| panic!("eval suite {rel} unreadable at {}: {e}", p.display()));
        let lines: Vec<&str> = body.lines().filter(|l| !l.trim().is_empty()).collect();
        assert!(
            lines.len() >= 10,
            "tax-prep eval suite {rel} should have at least 10 fixtures (has {})",
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
fn tax_prep_declares_five_named_actions() {
    let dir = repo_verticals_dir();
    let (registry, _) = argo_vertical::load_dir(dir).expect("load_dir");
    let v = registry.get("tax-prep").unwrap();
    let actions = v.manifest.actions.as_ref().expect("[actions] section");
    let expected = [
        "review_w2",
        "flag_deduction_eligibility",
        "draft_return",
        "schedule_review",
        "file_return",
    ];
    for name in expected {
        assert!(
            actions.actions.contains_key(name),
            "tax-prep manifest must declare action `{name}`; got: {:?}",
            actions.actions.keys().collect::<Vec<_>>()
        );
    }
}

#[test]
fn tax_prep_action_gates_split_two_auto_three_approval() {
    let dir = repo_verticals_dir();
    let (registry, _) = argo_vertical::load_dir(dir).expect("load_dir");
    let v = registry.get("tax-prep").unwrap();
    let actions = v.manifest.actions.as_ref().expect("[actions]");

    let auto = ["review_w2", "schedule_review"];
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
    let gated = [
        "flag_deduction_eligibility",
        "draft_return",
        "file_return",
    ];
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
fn tax_prep_actions_exercise_three_distinct_executor_channels() {
    // Brief acceptance: 5 actions exercise gcal, document_creation,
    // and human channels (auto + 3 approval-gated).
    let dir = repo_verticals_dir();
    let (registry, _) = argo_vertical::load_dir(dir).expect("load_dir");
    let v = registry.get("tax-prep").unwrap();
    let actions = v.manifest.actions.as_ref().expect("[actions]");

    let sched = actions.actions.get("schedule_review").unwrap();
    assert_eq!(sched.executor.kind_str(), "mcp");
    assert_eq!(sched.executor.mcp_server(), Some("gcal"));
    assert!(!sched.requires_approval);

    let draft = actions.actions.get("draft_return").unwrap();
    assert_eq!(draft.executor.kind_str(), "document_creation");

    let flag = actions.actions.get("flag_deduction_eligibility").unwrap();
    assert_eq!(flag.executor.kind_str(), "human");

    let file = actions.actions.get("file_return").unwrap();
    assert_eq!(file.executor.kind_str(), "human");
}

#[test]
fn tax_prep_file_return_is_always_human_gated() {
    // The file_return action is the load-bearing human-approval gate
    // mapped onto the e-file 8879 authorization. It must require
    // approval AND it must route to the human executor — never to
    // an automated MCP / api / code_execution channel.
    let dir = repo_verticals_dir();
    let (registry, _) = argo_vertical::load_dir(dir).expect("load_dir");
    let v = registry.get("tax-prep").unwrap();
    let actions = v.manifest.actions.as_ref().expect("[actions]");
    let file = actions
        .actions
        .get("file_return")
        .expect("file_return must be declared");
    assert!(
        file.requires_approval,
        "file_return MUST be approval-gated; the brief makes this non-negotiable"
    );
    assert_eq!(
        file.executor.kind_str(),
        "human",
        "file_return MUST route to the human executor — no MCP/api/code_execution \
         shortcut is acceptable for IRS submission"
    );
    let template = file.instruction_template.as_deref().unwrap_or("");
    let lower = template.to_lowercase();
    assert!(
        lower.contains("8879"),
        "file_return instruction_template MUST reference Form 8879 (e-file authorization)"
    );
}

#[test]
fn tax_prep_system_prompt_carries_signing_disclaimer() {
    let dir = repo_verticals_dir();
    let (registry, _) = argo_vertical::load_dir(dir).expect("load_dir");
    let v = registry.get("tax-prep").unwrap();
    let role = v.manifest.role.as_ref().expect("[role] section");
    let path = v.root.join(&role.system_prompt);
    let body = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("system prompt unreadable at {}: {e}", path.display()));
    let lower = body.to_lowercase();
    assert!(
        lower.contains("licensed preparer"),
        "system prompt MUST reference the licensed preparer"
    );
    assert!(
        lower.contains("sign and file"),
        "system prompt MUST carry the 'sign and file' disclaimer language"
    );
}

#[test]
fn tax_prep_sample_w2s_has_five_entries() {
    let dir = repo_verticals_dir();
    let (registry, _) = argo_vertical::load_dir(dir).expect("load_dir");
    let v = registry.get("tax-prep").unwrap();
    let path = v.root.join("inventory/sample_w2s.json");
    let body = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("sample_w2s unreadable at {}: {e}", path.display()));
    let parsed: serde_json::Value =
        serde_json::from_str(&body).expect("sample_w2s.json must be valid JSON");
    let w2s = parsed
        .get("w2s")
        .and_then(|v| v.as_array())
        .expect("w2s array");
    assert_eq!(
        w2s.len(),
        5,
        "brief calls for 5 sample W-2s; got {}",
        w2s.len()
    );
}

#[test]
fn tax_prep_sample_w2s_use_only_synthetic_ssns() {
    // Brief acceptance criterion: "All sample SSNs MUST be
    // 000-00-0000 or similar invalid patterns". We assert two
    // things on the W-2 inventory file's raw text:
    //   1. Every SSN-shaped substring (NNN-NN-NNNN) starts with
    //      the explicitly-invalid `000-00-` SSA prefix.
    //   2. No SSN-shaped substring leaks a real-looking area
    //      number (anything other than `000-` for our synthetic
    //      data).
    // Belt-and-braces: a future contributor who replaces a fake
    // SSN with a real one trips this test at compile time.
    let dir = repo_verticals_dir();
    let (registry, _) = argo_vertical::load_dir(dir).expect("load_dir");
    let v = registry.get("tax-prep").unwrap();
    let path = v.root.join("inventory/sample_w2s.json");
    let body = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("sample_w2s unreadable at {}: {e}", path.display()));

    // Walk the raw bytes and find every NNN-NN-NNNN substring.
    let bytes = body.as_bytes();
    let mut matches: Vec<&str> = Vec::new();
    let mut i = 0usize;
    while i + 11 <= bytes.len() {
        let window = &bytes[i..i + 11];
        let is_ssn_shape = window[0].is_ascii_digit()
            && window[1].is_ascii_digit()
            && window[2].is_ascii_digit()
            && window[3] == b'-'
            && window[4].is_ascii_digit()
            && window[5].is_ascii_digit()
            && window[6] == b'-'
            && window[7].is_ascii_digit()
            && window[8].is_ascii_digit()
            && window[9].is_ascii_digit()
            && window[10].is_ascii_digit();
        if is_ssn_shape {
            // Reject if neighbouring chars extend the digit run on
            // either side (e.g. an EIN like 12-3456789 prefixed by
            // a stray digit). We only want bona-fide SSN shapes.
            let left_ok = i == 0 || !bytes[i - 1].is_ascii_digit();
            let right_ok = i + 11 == bytes.len() || !bytes[i + 11].is_ascii_digit();
            if left_ok && right_ok {
                matches.push(std::str::from_utf8(window).unwrap());
                i += 11;
                continue;
            }
        }
        i += 1;
    }

    assert!(
        !matches.is_empty(),
        "expected at least one SSN-shaped string in sample_w2s.json; found none"
    );
    for m in &matches {
        assert!(
            m.starts_with("000-00-"),
            "SSN `{m}` in sample_w2s.json is not synthetic; brief requires `000-00-XXXX` pattern"
        );
    }
}
