//! Unit tests for `handoff_policy.rs` (Stream
//! `claude-worker-handoff-policy`). Covers parser shape, first-
//! match-wins precedence, `*` wildcard, numeric matchers,
//! missing-file fallback, and CLI smoke surfaces (`show` /
//! `validate`).

use super::*;

fn ctx(kind: &str) -> ActionContext {
    ActionContext {
        action_kind: kind.to_string(),
        ..Default::default()
    }
}

#[test]
fn parser_accepts_brief_example() {
    let body = r#"
[defaults]
inline_max_retries = 2
inline_max_latency_seconds = 30
escalate_on_low_confidence = 0.6

[[rules]]
match = { action_kind = "approval_request" }
policy = "always_escalate"

[[rules]]
match = { action_kind = "send_email", confidence_lt = 0.8 }
policy = "escalate_to_senior"

[[rules]]
match = { action_kind = "*", retry_count_gte = 3 }
policy = "escalate"
"#;
    let policy = HandoffPolicy::from_toml(body).expect("brief example must parse");
    assert_eq!(policy.rules.len(), 3);
    assert_eq!(policy.defaults.inline_max_retries, Some(2));
    assert_eq!(policy.defaults.inline_max_latency_seconds, Some(30));
    assert_eq!(policy.defaults.escalate_on_low_confidence, Some(0.6));
}

#[test]
fn parser_rejects_unknown_policy_tag() {
    let body = r#"
[[rules]]
match = { action_kind = "x" }
policy = "yeet_to_orbit"
"#;
    let err = HandoffPolicy::from_toml(body).expect_err("unknown policy must error");
    let msg = err.to_string();
    assert!(msg.contains("handoff-policy parse"), "expected parse-error prefix, got: {msg}");
}

#[test]
fn empty_input_yields_empty_policy() {
    let policy = HandoffPolicy::from_toml("").unwrap();
    assert!(policy.rules.is_empty());
    assert_eq!(policy.evaluate(&ctx("anything")), None);
}

#[test]
fn first_match_wins_order_matters() {
    // Specific approval_request rule precedes wildcard catch-all.
    let body = r#"
[[rules]]
match = { action_kind = "approval_request" }
policy = "always_escalate"

[[rules]]
match = { action_kind = "*" }
policy = "inline"
"#;
    let policy = HandoffPolicy::from_toml(body).unwrap();
    let (idx, decision) = policy.evaluate(&ctx("approval_request")).unwrap();
    assert_eq!(idx, 0);
    assert_eq!(decision, Decision::Escalate);

    // Same rules, different action — second rule wins.
    let (idx, decision) = policy.evaluate(&ctx("send_email")).unwrap();
    assert_eq!(idx, 1);
    assert_eq!(decision, Decision::Inline);
}

#[test]
fn wildcard_matches_any_action_kind() {
    let body = r#"
[[rules]]
match = { action_kind = "*" }
policy = "escalate"
"#;
    let policy = HandoffPolicy::from_toml(body).unwrap();
    assert_eq!(
        policy.evaluate(&ctx("anything-at-all")).unwrap().1,
        Decision::Escalate
    );
}

#[test]
fn confidence_lt_matcher_respects_threshold() {
    let body = r#"
[[rules]]
match = { action_kind = "send_email", confidence_lt = 0.8 }
policy = "escalate"
"#;
    let policy = HandoffPolicy::from_toml(body).unwrap();

    // Below threshold → match.
    let mut c = ctx("send_email");
    c.confidence = Some(0.5);
    assert!(policy.evaluate(&c).is_some());

    // At threshold → no match (strict less-than).
    let mut c = ctx("send_email");
    c.confidence = Some(0.8);
    assert!(policy.evaluate(&c).is_none());

    // Missing confidence → no match (we cannot prove the predicate).
    assert!(policy.evaluate(&ctx("send_email")).is_none());
}

#[test]
fn retry_count_gte_matcher_respects_threshold() {
    let body = r#"
[[rules]]
match = { action_kind = "*", retry_count_gte = 3 }
policy = "escalate"
"#;
    let policy = HandoffPolicy::from_toml(body).unwrap();

    let mut c = ctx("anything");
    c.retry_count = 2;
    assert!(policy.evaluate(&c).is_none());

    c.retry_count = 3;
    assert!(policy.evaluate(&c).is_some());

    c.retry_count = 10;
    assert!(policy.evaluate(&c).is_some());
}

#[test]
fn match_spec_combines_action_kind_with_numeric_matchers() {
    let body = r#"
[[rules]]
match = { action_kind = "send_email", confidence_lt = 0.5, retry_count_gte = 2 }
policy = "escalate"
"#;
    let policy = HandoffPolicy::from_toml(body).unwrap();

    let mut c = ctx("send_email");
    c.confidence = Some(0.4);
    c.retry_count = 2;
    assert!(policy.evaluate(&c).is_some());

    // Same kind but retry count below threshold → no match.
    c.retry_count = 1;
    assert!(policy.evaluate(&c).is_none());

    // Right kind + retries OK but confidence at threshold → no match.
    c.retry_count = 5;
    c.confidence = Some(0.5);
    assert!(policy.evaluate(&c).is_none());
}

#[test]
fn missing_workspace_file_yields_default_policy() {
    let dir = tempfile::tempdir().expect("tempdir");
    let policy = HandoffPolicy::load_from_workspace(dir.path()).unwrap();
    assert!(policy.rules.is_empty());
}

#[test]
fn workspace_file_round_trip() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dot = dir.path().join(".skycot");
    std::fs::create_dir_all(&dot).unwrap();
    let path = dot.join("handoff-policy.toml");
    std::fs::write(
        &path,
        r#"
[[rules]]
match = { action_kind = "approval_request" }
policy = "always_escalate"
"#,
    )
    .unwrap();
    let policy = HandoffPolicy::load_from_workspace(dir.path()).unwrap();
    assert_eq!(policy.rules.len(), 1);
    assert_eq!(
        policy.evaluate(&ctx("approval_request")).unwrap().1,
        Decision::Escalate
    );
}

#[test]
fn policy_hook_returns_none_for_inline_or_escalate() {
    // Without an `escalate_to:<id>` syntax in the rules DSL, the
    // hook always returns None today — the walker falls through
    // to its existing routing engine. This is the no-policy-file
    // graceful-absence guarantee in hook form.
    let body = r#"
[[rules]]
match = { action_kind = "*" }
policy = "escalate"
"#;
    let policy = HandoffPolicy::from_toml(body).unwrap();
    let hook = PolicyHook::new(policy);
    assert!(hook.override_target("anything").is_none());
}

#[tokio::test]
async fn cli_show_smoke_renders_text_then_json() {
    let dir = tempfile::tempdir().expect("tempdir");
    let dot = dir.path().join(".skycot");
    std::fs::create_dir_all(&dot).unwrap();
    std::fs::write(
        dot.join("handoff-policy.toml"),
        r#"
[[rules]]
match = { action_kind = "approval_request" }
policy = "always_escalate"
"#,
    )
    .unwrap();
    let restore = std::env::current_dir().ok();
    std::env::set_current_dir(dir.path()).unwrap();
    let result_text = run(HandoffPolicyCmd::Show {
        explain: Some("approval_request".into()),
        json: false,
    })
    .await;
    let result_json = run(HandoffPolicyCmd::Show {
        explain: None,
        json: true,
    })
    .await;
    if let Some(p) = restore {
        let _ = std::env::set_current_dir(p);
    }
    assert!(result_text.is_ok());
    assert!(result_json.is_ok());
}

#[tokio::test]
async fn cli_validate_smoke_ok_and_error_paths() {
    let dir = tempfile::tempdir().expect("tempdir");
    let good = dir.path().join("good.toml");
    std::fs::write(
        &good,
        r#"
[[rules]]
match = { action_kind = "*" }
policy = "escalate"
"#,
    )
    .unwrap();
    let bad = dir.path().join("bad.toml");
    std::fs::write(
        &bad,
        r#"
[[rules]]
match = { action_kind = "x" }
policy = "yeet_to_orbit"
"#,
    )
    .unwrap();

    let ok = run(HandoffPolicyCmd::Validate {
        path: Some(good),
        json: true,
    })
    .await;
    assert!(ok.is_ok());

    let err = run(HandoffPolicyCmd::Validate {
        path: Some(bad),
        json: true,
    })
    .await;
    assert!(err.is_err());

    let missing = run(HandoffPolicyCmd::Validate {
        path: Some(dir.path().join("nope.toml")),
        json: false,
    })
    .await;
    assert!(missing.is_err());
}
