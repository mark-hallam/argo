//! Unit tests for the approval-policy template resolver.
//!
//! Exercise the pure helpers (`resolve_at`, `parse_and_merge_at`,
//! `body_uses_templates`, the bundled-template registry). The
//! `init` / `list` / `show` arms of `run` are integration-tested
//! via `tests/help_smoke.rs` plus the round-trip checks below.

use super::{
    body_uses_templates, bundled_body, parse_and_merge_at, resolve_at,
    resolve_workspace_body_at, table_to_policy, BUNDLED_TEMPLATES,
};
use argo_identity::AutoApprovalPolicy;
use std::fs;
use tempfile::TempDir;

#[test]
fn registry_lists_five_bundled_templates() {
    let names: Vec<&str> = BUNDLED_TEMPLATES.iter().map(|(n, _)| *n).collect();
    assert_eq!(names.len(), 5, "expected 5 bundled templates, got {names:?}");
    for expected in [
        "auto-approve-low-risk",
        "require-human-for-finance",
        "deny-network-egress",
        "read-only-developer",
        "strict-production",
    ] {
        assert!(
            names.contains(&expected),
            "missing bundled template '{expected}' in {names:?}"
        );
    }
}

#[test]
fn every_bundled_template_parses_into_a_valid_policy() {
    for (name, body) in BUNDLED_TEMPLATES {
        let policy: AutoApprovalPolicy = toml::from_str(body)
            .unwrap_or_else(|e| panic!("bundled '{name}' did not parse: {e}\nbody:\n{body}"));
        // Every starter declares the boolean explicitly so a
        // future reader can read the file without context. If
        // any future template wants a default, fix the assertion
        // and document the why.
        assert!(
            body.contains("enabled"),
            "bundled '{name}' must declare `enabled` explicitly"
        );
        // Round-trip back to TOML — proves the deserialised shape
        // can be serialised cleanly for the `policy show` surface.
        let serialised = toml::to_string_pretty(&policy)
            .unwrap_or_else(|e| panic!("re-serialise '{name}': {e}"));
        assert!(!serialised.is_empty());
    }
}

#[test]
fn bundled_template_lookup_returns_some_for_known_name() {
    assert!(bundled_body("auto-approve-low-risk").is_some());
    assert!(bundled_body("not-a-real-template").is_none());
}

#[test]
fn body_uses_templates_detects_template_key() {
    assert!(body_uses_templates("template = \"foo\"\n"));
    assert!(body_uses_templates("extends = [\"a\"]\n"));
    assert!(!body_uses_templates("enabled = true\n"));
    assert!(!body_uses_templates(""));
}

#[test]
fn resolve_known_bundled_template_inlines_rules() {
    let dir = TempDir::new().unwrap();
    let policy = resolve_at("auto-approve-low-risk", dir.path()).unwrap();
    assert!(policy.enabled);
    assert_eq!(policy.max_usd_micros_per_action, Some(500_000));
    assert_eq!(policy.daily_budget_usd_micros, Some(5_000_000));
}

#[test]
fn resolve_unknown_template_errors() {
    let dir = TempDir::new().unwrap();
    let err = resolve_at("not-a-template", dir.path()).unwrap_err();
    let msg = format!("{err}");
    assert!(
        msg.contains("not-a-template"),
        "error should name the missing template, got: {msg}"
    );
}

#[test]
fn extends_chain_merges_in_declared_order_later_wins() {
    // Body that extends two bundled templates. The second one
    // (`strict-production`) sets `max_usd_micros_per_action =
    // 100_000`, so its value should win over the first
    // (`auto-approve-low-risk`'s 500_000).
    let dir = TempDir::new().unwrap();
    let body = r#"
extends = ["auto-approve-low-risk", "strict-production"]
"#;
    let policy = resolve_workspace_body_at(body, dir.path()).unwrap();
    assert!(policy.enabled);
    assert_eq!(policy.max_usd_micros_per_action, Some(100_000));
    assert_eq!(policy.daily_budget_usd_micros, Some(20_000_000));
    assert_eq!(
        policy.wake_action_types,
        vec!["page_oncall", "escalate_ticket", "escalate_to_human"]
    );
}

#[test]
fn local_body_overlays_template_keys() {
    // The local body sets `enabled = false` — that should win
    // over the bundled template's `enabled = true`.
    let dir = TempDir::new().unwrap();
    let body = r#"
template = "auto-approve-low-risk"
enabled = false
max_usd_micros_per_action = 1
"#;
    let policy = resolve_workspace_body_at(body, dir.path()).unwrap();
    assert!(!policy.enabled, "local enabled=false should override template");
    assert_eq!(policy.max_usd_micros_per_action, Some(1));
    // Untouched fields fall through from the template.
    assert_eq!(policy.daily_budget_usd_micros, Some(5_000_000));
}

#[test]
fn local_template_dir_shadows_bundled() {
    // Drop a local override at
    // `.skycot/approval-policies/auto-approve-low-risk.toml`
    // and confirm `resolve_at` reads the override instead of
    // the bundled body.
    let dir = TempDir::new().unwrap();
    let local_dir = dir.path().join(super::LOCAL_TEMPLATE_DIR);
    fs::create_dir_all(&local_dir).unwrap();
    fs::write(
        local_dir.join("auto-approve-low-risk.toml"),
        "enabled = true\nmax_usd_micros_per_action = 999\n",
    )
    .unwrap();
    let policy = resolve_at("auto-approve-low-risk", dir.path()).unwrap();
    assert_eq!(
        policy.max_usd_micros_per_action,
        Some(999),
        "local override should shadow the bundled body"
    );
}

#[test]
fn extends_cycle_errors_with_max_depth() {
    // `a` extends `b`; `b` extends `a`. Should hit the depth
    // guard before stack-overflowing.
    let dir = TempDir::new().unwrap();
    let local_dir = dir.path().join(super::LOCAL_TEMPLATE_DIR);
    fs::create_dir_all(&local_dir).unwrap();
    fs::write(
        local_dir.join("a.toml"),
        "extends = [\"b\"]\nenabled = true\n",
    )
    .unwrap();
    fs::write(
        local_dir.join("b.toml"),
        "extends = [\"a\"]\nenabled = true\n",
    )
    .unwrap();
    let err = resolve_at("a", dir.path()).unwrap_err();
    let msg = format!("{err}");
    assert!(
        msg.contains("max extends depth"),
        "cycle should hit the depth guard, got: {msg}"
    );
}

#[test]
fn round_trip_bundled_to_policy_to_toml_is_equivalent() {
    // Acceptance criterion: bundled TOML → parse → re-serialise
    // produces an equivalent `AutoApprovalPolicy`.
    let dir = TempDir::new().unwrap();
    for (name, _) in BUNDLED_TEMPLATES {
        let policy = resolve_at(name, dir.path())
            .unwrap_or_else(|e| panic!("{name}: resolve failed: {e}"));
        let serialised = toml::to_string_pretty(&policy)
            .unwrap_or_else(|e| panic!("{name}: serialise failed: {e}"));
        let reparsed: AutoApprovalPolicy = toml::from_str(&serialised)
            .unwrap_or_else(|e| panic!("{name}: reparse failed: {e}"));
        assert_eq!(policy, reparsed, "{name}: round-trip diverged");
    }
}

#[test]
fn body_with_no_refs_resolves_to_itself() {
    let dir = TempDir::new().unwrap();
    let body = "enabled = true\nmax_usd_micros_per_action = 42\n";
    let policy = resolve_workspace_body_at(body, dir.path()).unwrap();
    assert!(policy.enabled);
    assert_eq!(policy.max_usd_micros_per_action, Some(42));
}

#[test]
fn parse_and_merge_strips_template_and_extends_keys() {
    // After resolution the merged TOML table must NOT contain
    // `template` or `extends` keys; otherwise
    // `AutoApprovalPolicy::deserialize` could see them and
    // (depending on serde flags) fail or silently absorb them.
    let dir = TempDir::new().unwrap();
    let body = r#"
template = "auto-approve-low-risk"
enabled = false
"#;
    let merged = parse_and_merge_at(body, dir.path(), 0).unwrap();
    assert!(!merged.contains_key("template"));
    assert!(!merged.contains_key("extends"));
    let policy = table_to_policy(merged).unwrap();
    assert!(!policy.enabled);
}
