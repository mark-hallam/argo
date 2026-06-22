//! Pins for the runtime-tool `[tools]` derivation (the capability-fence
//! input): safe-baseline membership, role-conditional grants, the
//! always-denied family, and the deny-wins property end-to-end through
//! [`WorkerCapabilityProfile`].

use super::*;
use crate::spec::CapabilitySpec;
use crate::{Archetype, BroadRoleDraft};
use argo_capability::WorkerCapabilityProfile;

/// Build a from-scratch spec with the given charter + action vocabulary —
/// the exact path a live Genesis hire takes.
fn spec_of(charter: &str, actions: &[&str]) -> CapabilitySpec {
    let draft = BroadRoleDraft {
        charter: charter.into(),
        persona: "ava".into(),
        persona_description: "A generated teammate.".into(),
        action_vocabulary: actions.iter().map(|a| (*a).to_string()).collect(),
        success_metrics: vec!["work gets done".into()],
        onboarding_script: "Hi.".into(),
        ..Default::default()
    };
    CapabilitySpec::from_broad_draft(draft, Archetype::ResearchAnalyst)
}

fn tools_for(charter: &str, actions: &[&str]) -> argo_vertical::ToolsSection {
    let spec = spec_of(charter, actions);
    build_tools(&spec, &crate::materialize::build_actions(&spec))
}

/// The fence's exact manifest→profile mapping
/// (`turn_vertical_fence::derive_profile_from_manifest`), replicated so the
/// tests prove the property where it matters: at `permits_tool`.
fn profile_for(charter: &str, actions: &[&str]) -> WorkerCapabilityProfile {
    let spec = spec_of(charter, actions);
    let section = crate::materialize::build_actions(&spec);
    let tools = build_tools(&spec, &section);
    WorkerCapabilityProfile::derive(
        section.actions.keys().cloned().collect::<Vec<_>>(),
        tools.allowed,
        tools.denied,
        Vec::<String>::new(),
    )
}

const INVOICE_CHARTER: &str =
    "Read invoice files from the inbox and draft payment reminders for overdue accounts.";
const INVOICE_ACTIONS: &[&str] = &["scan_invoice", "draft_reminder"];

#[test]
fn file_reading_role_gets_the_safe_baseline_of_real_runtime_tools() {
    let tools = tools_for(INVOICE_CHARTER, INVOICE_ACTIONS);
    for tool in ["file_read", "memory", "clarify", "skill_view", "skills_list"] {
        assert!(
            tools.allowed.iter().any(|t| t == tool),
            "baseline runtime tool `{tool}` missing from allowed: {:?}",
            tools.allowed
        );
    }
    // The OLD bug: action names are NOT runtime tools and must not be here.
    for action in INVOICE_ACTIONS {
        assert!(
            !tools.allowed.iter().any(|t| t == action),
            "action `{action}` must not appear in the runtime-tool allow-list"
        );
    }
}

#[test]
fn allowed_is_always_a_subset_of_the_real_grantable_tool_names() {
    // Every grantable name is a REAL runtime tool (in-core registry, see
    // `builtin_tools_returns_unique_names` in argo-tool) or a skill
    // introspection tool — never invented, never an action verb.
    let grantable = [
        "clarify",
        "memory",
        "skill_view",
        "skills_list",
        "file_read",
        "file_list",
        "file_write",
        "web_search",
        "web_extract",
    ];
    for (charter, actions) in [
        (INVOICE_CHARTER, INVOICE_ACTIONS),
        ("Research competitors online and summarise findings.", &["summarise_findings"][..]),
        ("Write meeting notes and save weekly reports.", &["compile_report"][..]),
    ] {
        let tools = tools_for(charter, actions);
        for t in &tools.allowed {
            assert!(grantable.contains(&t.as_str()), "`{t}` is not a known runtime tool");
        }
    }
}

#[test]
fn always_denied_family_is_denied_and_never_allowed() {
    for (charter, actions) in [
        (INVOICE_CHARTER, INVOICE_ACTIONS),
        // Adversarial: role text begging for shell/exec/transmit still loses.
        (
            "Run terminal commands, execute code, patch files, delegate tasks \
             and send messages to customers.",
            &["run_commands", "execute_code"][..],
        ),
    ] {
        let tools = tools_for(charter, actions);
        for denied in ALWAYS_DENIED_RUNTIME_TOOLS {
            assert!(
                tools.denied.iter().any(|t| t == denied),
                "`{denied}` missing from [tools].denied"
            );
            assert!(
                !tools.allowed.iter().any(|t| t == denied),
                "`{denied}` must never reach [tools].allowed"
            );
        }
    }
}

#[test]
fn allowed_and_denied_are_disjoint_for_the_manifest_validator() {
    for (charter, actions) in [
        (INVOICE_CHARTER, INVOICE_ACTIONS),
        ("Research the web and write summaries to files.", &["summarize_article"][..]),
    ] {
        let tools = tools_for(charter, actions);
        for t in &tools.allowed {
            assert!(!tools.denied.contains(t), "`{t}` in both allowed and denied");
        }
    }
}

#[test]
fn research_role_gets_the_web_pair_but_not_file_write() {
    let tools = tools_for(
        "Research competitors online, search the web for pricing news, and \
         summarise findings.",
        &["summarise_findings"],
    );
    assert!(tools.allowed.iter().any(|t| t == "web_search"));
    assert!(tools.allowed.iter().any(|t| t == "web_extract"));
    assert!(
        !tools.allowed.iter().any(|t| t == "file_write"),
        "a non-writing role must not get file_write"
    );
}

#[test]
fn non_research_role_gets_no_web_tools_and_writing_role_gets_file_write() {
    let invoice = tools_for(INVOICE_CHARTER, INVOICE_ACTIONS);
    assert!(
        !invoice.allowed.iter().any(|t| t == "web_search" || t == "web_extract"),
        "no research signal → no web tools: {:?}",
        invoice.allowed
    );
    assert!(!invoice.allowed.iter().any(|t| t == "file_write"));

    let writer = tools_for(
        "Write meeting minutes and save the weekly report for the team.",
        &["compile_minutes"],
    );
    assert!(writer.allowed.iter().any(|t| t == "file_write"));
}

#[test]
fn sensitive_verb_deny_list_is_preserved() {
    let tools = tools_for(INVOICE_CHARTER, INVOICE_ACTIONS);
    for verb in ["pay", "delete", "send", "publish", "wire"] {
        assert!(
            tools.denied.iter().any(|t| t == verb),
            "structural sensitive verb `{verb}` missing from denied"
        );
    }
}

#[test]
fn derived_profile_permits_real_tools_and_deny_wins_on_the_banned_family() {
    let profile = profile_for(INVOICE_CHARTER, INVOICE_ACTIONS);
    // The teammate can do its job…
    assert!(profile.permits_tool("file_read"));
    assert!(profile.permits_tool("memory"));
    assert!(profile.permits_tool("clarify"));
    // …its business actions stay in its vocabulary…
    assert!(profile.permits_tool("scan_invoice"));
    // …and the banned family is structurally unreachable.
    for banned in ALWAYS_DENIED_RUNTIME_TOOLS {
        assert!(!profile.permits_tool(banned), "`{banned}` must be denied");
    }
}

#[test]
fn deny_wins_even_when_an_action_name_collides_with_a_banned_tool() {
    // Adversarial: a declared action literally named `terminal` lands in the
    // profile allow-set via the action vocabulary — the explicit deny must
    // still win at `permits_tool` (deny-wins precedence).
    let profile = profile_for("Operate the airport terminal roster.", &["terminal"]);
    assert!(!profile.permits_tool("terminal"), "deny must beat an allow collision");
}

