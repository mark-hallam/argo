//! Asset-only checks for the Finance AR vertical
//! (Stream `claude-vertical-finance-ar`). Locks the manifest's
//! structural invariants so a later edit can't silently break the
//! eval suite, the executor wiring for the 5 actions, or the
//! deny-list invariants the brief lists.
//!
//! Pure asset tests — no agent runs, no LLM calls. Just:
//! - the loader sees finance-ar
//! - the manifest is structurally valid + cross-refs resolve
//! - the read-only deny list contains every state-mutating tool
//! - the eval suite has at least 5 fixtures
//! - the 5 declared actions match the brief's surface

use std::path::Path;

fn repo_verticals_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(|p| Box::leak(Box::new(p.join("verticals"))).as_path())
        .expect("workspace verticals dir resolves")
}

#[test]
fn finance_ar_loads_via_argo_vertical() {
    let dir = repo_verticals_dir();
    // load_dir returns ([registry], [errors]) — pre-existing flakes
    // in unrelated verticals (e.g. the documented schema mismatch
    // on customer-support / marketing-intern / sales-sdr) shouldn't
    // gate this brief. Assert specifically that finance-ar loaded
    // without error.
    let (registry, errors) = argo_vertical::load_dir(dir).expect("load_dir");
    let finance_ar_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.dir.ends_with("finance-ar"))
        .collect();
    assert!(
        finance_ar_errors.is_empty(),
        "finance-ar loader errors: {finance_ar_errors:?}"
    );
    let v = registry.get("finance-ar").expect("finance-ar not in registry");
    assert_eq!(v.manifest.vertical.name, "Finance — Accounts Receivable");
    assert_eq!(v.manifest.vertical.version, "0.1.0");
}

#[test]
fn finance_ar_validates_clean() {
    let dir = repo_verticals_dir();
    let (registry, _errors) = argo_vertical::load_dir(dir).expect("load_dir");
    let v = registry.get("finance-ar").unwrap();
    argo_vertical::validate(&v.manifest, &v.root)
        .expect("finance-ar manifest must validate");
}

#[test]
fn finance_ar_is_read_only_by_manifest() {
    let dir = repo_verticals_dir();
    let (registry, _) = argo_vertical::load_dir(dir).expect("load_dir");
    let v = registry.get("finance-ar").unwrap();
    let tools = v.manifest.tools.as_ref().expect("[tools] section");
    let mutating = ["file_write", "file_patch", "terminal", "delegate_task"];
    for t in &mutating {
        assert!(
            tools.denied.iter().any(|d| d == t),
            "finance-ar manifest must deny `{t}`; currently denied: {:?}",
            tools.denied
        );
        assert!(
            !tools.allowed.iter().any(|a| a == t),
            "finance-ar manifest must NOT allow `{t}`"
        );
    }
}

#[test]
fn finance_ar_escalation_lists_delegate_task_for_human_approval() {
    let dir = repo_verticals_dir();
    let (registry, _) = argo_vertical::load_dir(dir).unwrap();
    let v = registry.get("finance-ar").unwrap();
    let esc = v.manifest.escalation.as_ref().expect("[escalation] section");
    assert!(
        esc.human_approval_required.iter().any(|t| t == "delegate_task"),
        "finance-ar manifest must list delegate_task under [escalation].human_approval_required; got: {:?}",
        esc.human_approval_required
    );
}

#[test]
fn finance_ar_eval_suite_has_five_fixtures() {
    let dir = repo_verticals_dir();
    let (registry, _) = argo_vertical::load_dir(dir).expect("load_dir");
    let v = registry.get("finance-ar").unwrap();
    let eval = v.manifest.evaluation.as_ref().expect("[evaluation]");
    assert!(!eval.suites.is_empty(), "finance-ar needs at least one eval suite");
    for rel in &eval.suites {
        let p = v.root.join(rel);
        let body = std::fs::read_to_string(&p)
            .unwrap_or_else(|e| panic!("eval suite {rel} unreadable at {}: {e}", p.display()));
        let lines: Vec<&str> = body.lines().filter(|l| !l.trim().is_empty()).collect();
        assert!(
            lines.len() >= 5,
            "finance-ar eval suite {rel} should have at least 5 fixtures (has {})",
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
fn finance_ar_declares_five_named_actions() {
    let dir = repo_verticals_dir();
    let (registry, _) = argo_vertical::load_dir(dir).expect("load_dir");
    let v = registry.get("finance-ar").unwrap();
    let actions = v.manifest.actions.as_ref().expect("[actions] section");
    let expected = [
        "review_invoices",
        "send_reminder",
        "escalate_overdue",
        "log_payment_receipt",
        "generate_aging_report",
    ];
    for name in expected {
        assert!(
            actions.actions.contains_key(name),
            "finance-ar manifest must declare action `{name}`; got: {:?}",
            actions.actions.keys().collect::<Vec<_>>()
        );
    }
}

#[test]
fn finance_ar_send_reminder_is_gmail_mcp_and_approval_gated() {
    let dir = repo_verticals_dir();
    let (registry, _) = argo_vertical::load_dir(dir).expect("load_dir");
    let v = registry.get("finance-ar").unwrap();
    let actions = v.manifest.actions.as_ref().expect("[actions] section");
    let send = actions.actions.get("send_reminder").expect("send_reminder declared");
    assert!(send.requires_approval, "send_reminder must require approval");
    assert_eq!(send.executor.kind_str(), "mcp", "send_reminder executor must be mcp");
    assert_eq!(send.executor.mcp_server(), Some("gmail"));
    assert_eq!(send.executor.mcp_tool(), Some("gmail.send"));
}
