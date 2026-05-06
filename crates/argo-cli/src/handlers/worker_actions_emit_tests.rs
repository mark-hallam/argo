//! Tests for the action-proposal extractor + emission
//! orchestrator (#729, #769). Loaded via
//! `#[path = "worker_actions_emit_tests.rs"] mod tests;`
//! at the bottom of `worker_actions_emit.rs`.

use super::*;
use std::collections::HashSet;

/// Empty sold-set: emission tests that pre-date the #769 gate
/// pass this so behaviour matches the original signature.
fn no_sold() -> HashSet<String> {
    HashSet::new()
}

fn fixture_vertical() -> argo_vertical::VerticalManifest {
    argo_vertical::parse(
        r#"
schema_version = "1"

[vertical]
id      = "fix"
name    = "fixture"
version = "0.0.1"

[actions.verify_price]
risk              = "low"
requires_approval = false
description       = "Cross-check price."
executor          = "human"

[actions.post_listing]
risk              = "medium"
requires_approval = true
description       = "Public marketplace post."
executor          = "human"
platforms         = ["facebook_marketplace", "ebay"]
"#,
    )
    .expect("manifest parses")
}

#[test]
fn extract_finds_a_single_action_block() {
    let text = "Here's what I'll do:\n\
        \n\
        ```action\n\
        {\"action_type\": \"verify_price\", \"payload\": {\"target\": 250}}\n\
        ```\n\
        \n\
        Done.";
    let proposals = extract_action_proposals(text);
    assert_eq!(proposals.len(), 1);
    assert_eq!(proposals[0].action_type, "verify_price");
    assert_eq!(proposals[0].payload["target"], 250);
}

#[test]
fn extract_finds_multiple_action_blocks_in_one_message() {
    let text = "\
```action
{\"action_type\": \"verify_price\", \"payload\": {}}
```

Some prose in between.

```action
{\"action_type\": \"post_listing\", \"payload\": {\"title\": \"x\"}, \"platform\": \"ebay\"}
```
";
    let proposals = extract_action_proposals(text);
    assert_eq!(proposals.len(), 2);
    assert_eq!(proposals[0].action_type, "verify_price");
    assert_eq!(proposals[1].action_type, "post_listing");
    assert_eq!(proposals[1].platform.as_deref(), Some("ebay"));
}

#[test]
fn extract_skips_malformed_blocks_without_panicking() {
    let text = "\
```action
not actually json
```

```action
{\"action_type\": \"verify_price\", \"payload\": {}}
```

```action
{\"missing_action_type\": true}
```
";
    let proposals = extract_action_proposals(text);
    assert_eq!(
        proposals.len(),
        1,
        "only the well-formed block with action_type should land"
    );
    assert_eq!(proposals[0].action_type, "verify_price");
}

#[test]
fn extract_handles_optional_fields_with_defaults() {
    let text = r#"
```action
{
  "action_type": "post_listing",
  "payload": {"title": "Pro Skates"},
  "platform": "facebook_marketplace",
  "item_id": "skates-110",
  "evidence": ["https://example.com/comp1", "https://example.com/comp2"]
}
```
"#;
    let proposals = extract_action_proposals(text);
    assert_eq!(proposals.len(), 1);
    let p = &proposals[0];
    assert_eq!(p.platform.as_deref(), Some("facebook_marketplace"));
    assert_eq!(p.item_id.as_deref(), Some("skates-110"));
    assert_eq!(p.evidence.len(), 2);
}

#[test]
fn extract_returns_empty_on_text_with_no_action_blocks() {
    assert!(extract_action_proposals("just prose").is_empty());
    assert!(extract_action_proposals("```ruby\nputs 'hi'\n```").is_empty());
    assert!(extract_action_proposals("").is_empty());
}

#[test]
fn extract_does_not_match_inline_action_word() {
    // "action" appearing inline as text MUST NOT be treated as
    // a fence opener — only the literal `\`\`\`action` at code-fence
    // start counts.
    let text = "I'll take action: do this. ```bash\ncode\n```";
    assert!(extract_action_proposals(text).is_empty());
}

// ───────────────── orchestrator + evidence gate ──────────

#[test]
fn emit_orchestrator_lands_safe_action_pending_when_evidence_ok() {
    let v = fixture_vertical();
    let text = "```action\n\
        {\"action_type\": \"verify_price\", \"payload\": {\"target\": 250}}\n\
        ```";
    let records = emit_actions_from_text(&v, "job-1", text, true, &no_sold());
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].state, "pending");
    assert!(!records[0].requires_approval);
    assert_eq!(records[0].risk_level, "low");
}

#[test]
fn emit_orchestrator_lands_approval_action_pending_when_evidence_ok() {
    let v = fixture_vertical();
    let text = "```action\n\
        {\"action_type\": \"post_listing\", \"payload\": {\"title\":\"x\"}}\n\
        ```";
    let records = emit_actions_from_text(&v, "job-1", text, true, &no_sold());
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].state, "pending");
    assert!(records[0].requires_approval);
    assert_eq!(records[0].risk_level, "medium");
}

/// The headline #729 quality-gate test: when research evidence
/// is insufficient (#727), an approval-gated action MUST land
/// `blocked`, not `pending`. The model never gets to claim
/// approval-readiness on top of weak research.
#[test]
fn emit_orchestrator_blocks_approval_action_when_evidence_insufficient() {
    let v = fixture_vertical();
    let text = "```action\n\
        {\"action_type\": \"post_listing\", \"payload\": {\"title\":\"x\"}}\n\
        ```";
    let records = emit_actions_from_text(&v, "job-1", text, false, &no_sold());
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].state, "blocked");
    assert!(records[0]
        .blocked_reason
        .as_deref()
        .unwrap_or("")
        .contains("research evidence insufficient"));
}

/// Symmetric to the test above: safe-autonomous actions are
/// NOT downgraded by the evidence gate. Verifying prices and
/// drafting are autonomous-safe regardless of search quality.
#[test]
fn emit_orchestrator_does_not_downgrade_safe_action_when_evidence_insufficient() {
    let v = fixture_vertical();
    let text = "```action\n\
        {\"action_type\": \"verify_price\", \"payload\": {\"target\": 250}}\n\
        ```";
    let records = emit_actions_from_text(&v, "job-1", text, false, &no_sold());
    assert_eq!(records[0].state, "pending");
    assert!(records[0].blocked_reason.is_none());
}

/// Locks a multi-action emission: one safe + one approval-gated
/// in the same assistant message, evidence insufficient — only
/// the approval-gated one downgrades.
#[test]
fn emit_orchestrator_per_action_evidence_gate_decision() {
    let v = fixture_vertical();
    let text = "\
```action
{\"action_type\": \"verify_price\", \"payload\": {\"target\": 250}}
```

```action
{\"action_type\": \"post_listing\", \"payload\": {\"title\":\"x\"}}
```
";
    let records = emit_actions_from_text(&v, "job-1", text, false, &no_sold());
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].action_type, "verify_price");
    assert_eq!(records[0].state, "pending"); // safe-autonomous, untouched
    assert_eq!(records[1].action_type, "post_listing");
    assert_eq!(records[1].state, "blocked"); // gated, downgraded
}

#[test]
fn emit_orchestrator_preserves_undeclared_action_blocking_reason() {
    let v = fixture_vertical();
    let text = "```action\n\
        {\"action_type\": \"drain_bank_account\", \"payload\": {\"amount\": 99999}}\n\
        ```";
    // Even with evidence sufficient, undeclared types stay blocked.
    let records = emit_actions_from_text(&v, "job-1", text, true, &no_sold());
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].state, "blocked");
    let reason = records[0].blocked_reason.as_deref().unwrap_or("");
    assert!(reason.contains("drain_bank_account"));
    assert!(reason.contains("fix"), "reason names the vertical id");
}

// ───────────────── execution_plan.md renderer ────────────

#[test]
fn render_execution_plan_empty_state_placeholder() {
    let plan = render_execution_plan(&[]);
    assert!(plan.starts_with("# Execution plan"));
    assert!(plan.contains("(no actions emitted this run)"));
}

#[test]
fn render_execution_plan_groups_by_state_and_approval_class() {
    let v = fixture_vertical();
    let text = "\
```action
{\"action_type\": \"verify_price\", \"payload\": {\"target\": 250}}
```

```action
{\"action_type\": \"post_listing\", \"payload\": {\"title\":\"x\"}, \"platform\":\"ebay\"}
```

```action
{\"action_type\": \"drain_bank_account\", \"payload\": {}}
```
";
    let records = emit_actions_from_text(&v, "job-1", text, true, &no_sold());
    assert_eq!(records.len(), 3);
    let plan = render_execution_plan(&records);
    // Header + section headings present.
    assert!(plan.starts_with("# Execution plan"));
    assert!(plan.contains("Emitted 3 action(s)"));
    assert!(plan.contains("## Pending approval"));
    assert!(plan.contains("## Pending (autonomous-safe)"));
    assert!(plan.contains("## Blocked"));
    // Per-action lines: action_type + risk + platform.
    assert!(plan.contains("`verify_price`"));
    assert!(plan.contains("`post_listing`"));
    assert!(plan.contains("`drain_bank_account`"));
    assert!(plan.contains("platform `ebay`"));
    // Blocked emission carries its rejection reason inline.
    assert!(plan.contains("no capability grant"));
}

#[test]
fn render_execution_plan_omits_empty_sections() {
    let v = fixture_vertical();
    let text = "```action\n\
        {\"action_type\": \"verify_price\", \"payload\": {\"target\": 250}}\n\
        ```";
    let records = emit_actions_from_text(&v, "job-1", text, true, &no_sold());
    let plan = render_execution_plan(&records);
    // Only the autonomous-safe section should be present.
    assert!(plan.contains("## Pending (autonomous-safe)"));
    assert!(!plan.contains("## Pending approval"));
    assert!(!plan.contains("## Blocked"));
}

#[test]
fn render_execution_plan_evidence_gated_approval_lands_in_blocked_section() {
    let v = fixture_vertical();
    let text = "```action\n\
        {\"action_type\": \"post_listing\", \"payload\": {\"title\":\"x\"}}\n\
        ```";
    // Evidence insufficient → approval-gated emission blocks.
    let records = emit_actions_from_text(&v, "job-1", text, false, &no_sold());
    let plan = render_execution_plan(&records);
    assert!(plan.contains("## Blocked"));
    assert!(!plan.contains("## Pending approval"));
    assert!(plan.contains("research evidence insufficient"));
}

// ───────────────── #769: sold-item emission gate ─────────

/// Build a sold-set fixture for the gate tests.
fn sold(items: &[&str]) -> HashSet<String> {
    items.iter().map(|s| (*s).to_string()).collect()
}

/// Headline #769 contract: an emitted approval-gated action whose
/// `item_id` is in the sold set must land `blocked`, not
/// `pending`. The worker should not draft new listings for items
/// the operator has already closed.
#[test]
fn emit_orchestrator_blocks_approval_action_for_sold_item() {
    let v = fixture_vertical();
    let text = "```action\n\
        {\"action_type\": \"post_listing\", \"payload\": {\"title\":\"x\"}, \"item_id\": \"skates-A\"}\n\
        ```";
    let records = emit_actions_from_text(&v, "job-1", text, true, &sold(&["skates-A"]));
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].state, "blocked");
    let reason = records[0].blocked_reason.as_deref().unwrap_or("");
    assert!(
        reason.contains("already marked sold") && reason.contains("skates-A"),
        "blocked_reason should name the sold item; got: {reason}"
    );
}

/// Symmetric to the evidence gate: the sold-item gate also
/// blocks autonomous-safe actions for sold items. A
/// long-running worker should not even verify_price against an
/// item that's already been sold — there's no inventory to
/// price-check.
#[test]
fn emit_orchestrator_blocks_autonomous_action_for_sold_item() {
    let v = fixture_vertical();
    let text = "```action\n\
        {\"action_type\": \"verify_price\", \"payload\": {\"target\": 250}, \"item_id\": \"skates-A\"}\n\
        ```";
    let records = emit_actions_from_text(&v, "job-1", text, true, &sold(&["skates-A"]));
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].state, "blocked");
    assert!(records[0]
        .blocked_reason
        .as_deref()
        .unwrap_or("")
        .contains("already marked sold"));
}

/// Idempotency: if the worker re-emits a `mark_item_sold` for an
/// already-sold item, the gate must NOT block it (so a redundant
/// confirmation still flows through the normal pipeline).
#[test]
fn emit_orchestrator_does_not_block_redundant_mark_item_sold() {
    let v = argo_vertical::parse(
        r#"
schema_version = "1"
[vertical]
id = "fix"
name = "fixture"
version = "0.0.1"
[actions.mark_item_sold]
risk = "medium"
requires_approval = true
description = "Close the inventory item."
executor = "human"
"#,
    )
    .expect("manifest parses");
    let text = "```action\n\
        {\"action_type\": \"mark_item_sold\", \"payload\": {}, \"item_id\": \"skates-A\"}\n\
        ```";
    let records = emit_actions_from_text(&v, "job-1", text, true, &sold(&["skates-A"]));
    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0].state, "pending",
        "mark_item_sold must pass through the gate even when the item is already sold (idempotent)"
    );
}

/// Item-less actions are unaffected by the gate. Verifies the
/// gate only triggers on `Some(item_id)`.
#[test]
fn emit_orchestrator_does_not_block_item_less_action_for_sold_set() {
    let v = fixture_vertical();
    let text = "```action\n\
        {\"action_type\": \"verify_price\", \"payload\": {\"target\": 250}}\n\
        ```";
    // Same sold set, but THIS action has no item_id — must not
    // be gated.
    let records = emit_actions_from_text(&v, "job-1", text, true, &sold(&["skates-A"]));
    assert_eq!(records[0].state, "pending");
    assert!(records[0].blocked_reason.is_none());
}

/// Mixed emission in one assistant text: one action for a sold
/// item (blocks), one for an unsold item (passes). Ensures the
/// gate is per-action, not per-batch.
#[test]
fn emit_orchestrator_per_action_sold_gate_decision() {
    let v = fixture_vertical();
    let text = "\
```action
{\"action_type\": \"verify_price\", \"payload\": {\"target\": 250}, \"item_id\": \"skates-A\"}
```

```action
{\"action_type\": \"verify_price\", \"payload\": {\"target\": 175}, \"item_id\": \"skates-B\"}
```
";
    let records = emit_actions_from_text(&v, "job-1", text, true, &sold(&["skates-A"]));
    assert_eq!(records.len(), 2);
    let by_item: std::collections::HashMap<String, &ActionRecord> = records
        .iter()
        .map(|r| (r.item_id.clone().unwrap_or_default(), r))
        .collect();
    assert_eq!(by_item["skates-A"].state, "blocked");
    assert_eq!(by_item["skates-B"].state, "pending");
}
