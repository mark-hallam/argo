//! Unit tests for the worker-bound least-privilege capability profile
//! and the deterministic irreversibility gate (DEC-025 Genesis fence).

use super::*;
use argo_types::Capability;

fn tool(name: &str) -> Capability {
    Capability::ToolDispatch {
        tool_name_glob: name.to_string(),
    }
}

#[test]
fn empty_profile_denies_everything_by_default() {
    let profile = WorkerCapabilityProfile::derive(
        Vec::<String>::new(),
        Vec::<String>::new(),
        Vec::<String>::new(),
        Vec::<String>::new(),
    );
    assert!(!profile.permits_tool("terminal"));
    assert!(!profile.permits_tool("send_message"));
    assert!(!profile.permits_tool("file_write"));
    // Even with an empty allow-set, the to_capability_set carries the
    // minimal always-safe tools but NOTHING side-effecting.
    let set = profile.to_capability_set();
    assert!(set.covers(&tool("clarify")));
    assert!(!set.covers(&tool("terminal")));
    assert!(!set.covers(&tool("send_message")));
}

#[test]
fn derive_grants_declared_actions_and_allowed_tools_only() {
    let profile = WorkerCapabilityProfile::derive(
        ["draft_reply".into(), "summarize_ticket".into()],
        ["web_search".into()],
        Vec::<String>::new(),
        Vec::<String>::new(),
    );
    assert!(profile.permits_tool("draft_reply"));
    assert!(profile.permits_tool("summarize_ticket"));
    assert!(profile.permits_tool("web_search"));
    // Not declared → denied.
    assert!(!profile.permits_tool("terminal"));
    assert!(!profile.permits_tool("file_write"));
}

#[test]
fn deny_wins_over_allow() {
    // A tool listed in both allowed + denied is denied.
    let profile = WorkerCapabilityProfile::derive(
        ["send_email".into()],
        ["send_email".into()],
        ["send_email".into()],
        Vec::<String>::new(),
    );
    assert!(!profile.permits_tool("send_email"));
    let set = profile.to_capability_set();
    assert!(!set.covers(&tool("send_email")));
}

#[test]
fn capability_set_never_grants_star_reach() {
    // The owner role grants ToolDispatch "*"; a derived profile never
    // does, even when the spec declares many tools.
    let profile = WorkerCapabilityProfile::derive(
        ["a".into(), "b".into(), "c".into()],
        ["web_search".into(), "web_extract".into()],
        Vec::<String>::new(),
        Vec::<String>::new(),
    );
    let set = profile.to_capability_set();
    // A wildcard request is NOT covered — the fence is concrete-tool only.
    assert!(!set.covers(&tool("*")));
    assert!(!set.covers(&tool("anything_undeclared")));
    // The concrete declared tools ARE covered.
    assert!(set.covers(&tool("a")));
    assert!(set.covers(&tool("web_search")));
}

#[test]
fn integrations_are_draft_only_not_in_allow_set() {
    // DEC-025 locked #6: MCP/connector servers are recorded as draft,
    // NOT promoted into the live dispatch allow-set at birth.
    let profile = WorkerCapabilityProfile::derive(
        ["draft_reply".into()],
        Vec::<String>::new(),
        Vec::<String>::new(),
        ["gmail".into(), "stripe".into()],
    );
    assert_eq!(profile.draft_integrations(), vec!["gmail", "stripe"]);
    // The integration server names are NOT dispatchable tools.
    assert!(!profile.permits_tool("gmail"));
    assert!(!profile.permits_tool("stripe"));
    let set = profile.to_capability_set();
    assert!(!set.covers(&tool("gmail")));
    assert!(!set.covers(&tool("stripe")));
}

#[test]
fn derive_trims_and_drops_blank_entries() {
    let profile = WorkerCapabilityProfile::derive(
        ["  draft_reply  ".into(), "   ".into(), String::new()],
        ["  web_search ".into()],
        Vec::<String>::new(),
        Vec::<String>::new(),
    );
    assert!(profile.permits_tool("draft_reply"));
    assert!(profile.permits_tool("web_search"));
    assert_eq!(profile.allowed_tools(), vec!["draft_reply", "web_search"]);
}

#[test]
fn irreversible_action_catches_money_data_identity_verbs() {
    assert!(is_irreversible_action("wire_funds"));
    assert!(is_irreversible_action("issue_refund"));
    assert!(is_irreversible_action("process_chargeback"));
    assert!(is_irreversible_action("drop_table"));
    assert!(is_irreversible_action("truncate_log"));
    assert!(is_irreversible_action("wipe_disk"));
    assert!(is_irreversible_action("purge_records"));
    assert!(is_irreversible_action("terminate_employee"));
    assert!(is_irreversible_action("revoke_access"));
    assert!(is_irreversible_action("cancel_subscription"));
    assert!(is_irreversible_action("delete_account"));
}

#[test]
fn irreversible_action_is_case_insensitive() {
    assert!(is_irreversible_action("WIRE_FUNDS"));
    assert!(is_irreversible_action("Drop_Table"));
}

#[test]
fn irreversible_action_does_not_false_positive_on_substrings() {
    // "rewire_config" contains "wire" as a substring but not as a token,
    // and carries no danger stem.
    assert!(!is_irreversible_action("rewire_config"));
    // Benign draft / read / summarize actions are reversible.
    assert!(!is_irreversible_action("draft_summary"));
    assert!(!is_irreversible_action("summarize_ticket"));
    assert!(!is_irreversible_action("read_inbox"));
    assert!(!is_irreversible_action("tag_inventory"));
    // Control: a legit support / customer-success vocabulary stays
    // reversible (no over-flagging of benign work) — the false-positive
    // guard the brief requires for the control role.
    assert!(!is_irreversible_action("triage_ticket"));
    assert!(!is_irreversible_action("draft_response"));
    assert!(!is_irreversible_action("escalate_ticket"));
    assert!(!is_irreversible_action("attach_screenshot"));
    assert!(!is_irreversible_action("display_dashboard"));
    assert!(!is_irreversible_action("reach_out_internally"));
}

#[test]
fn irreversible_action_catches_synonym_families_the_old_token_list_missed() {
    // The live stress-test false-negatives. Each carries a money / destroy /
    // egress / credential family that the original token-exact list let
    // through as AUTONOMOUS + low-risk.
    for name in [
        // money movement
        "process_payment",
        "transfer_funds",
        "remit_to_vendor",
        "disburse_payroll",
        "initiate_payout",
        "withdraw_balance",
        // destructive
        "remove_account_data",
        "erase_records",
        "empty_archived_storage",
        "truncate_audit_log",
        // egress / exfiltration
        "extract_customer_records",
        "export_customer_list",
        "upload_to_partner_system",
        "exfiltrate_pii",
        "download_full_database",
        "forward_to_external",
        // credential entry
        "authenticate_portal",
        "store_credential",
        "submit_password",
    ] {
        assert!(
            is_irreversible_action(name),
            "{name} carries a dangerous family and must be always-human"
        );
    }
}
