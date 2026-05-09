//! Tests for deterministic Genesis action suggestions.

use super::*;

fn has_source(actions: &[SuggestedAction], name: &str) -> bool {
    actions.iter().any(|action| {
        matches!(
            &action.source,
            ActionSource::Connector(connector) if connector == name
        )
    })
}

#[test]
fn sales_sdr_email_role_suggests_gmail_and_hubspot() {
    let actions = suggest(
        "sales SDR who emails prospects and tracks outbound deals",
        5,
    )
    .unwrap();
    assert!(has_source(&actions, "gmail"), "actions: {actions:?}");
    assert!(has_source(&actions, "hubspot"), "actions: {actions:?}");
}

#[test]
fn finance_ar_role_suggests_stripe_for_invoices() {
    let actions = suggest("AR clerk reconciling invoices and payment follow-up", 5).unwrap();
    assert!(has_source(&actions, "stripe"), "actions: {actions:?}");
}

#[test]
fn top_k_zero_returns_empty_vec() {
    let actions = suggest("sales SDR who emails prospects", 0).unwrap();
    assert!(actions.is_empty());
}

#[test]
fn unknown_role_returns_generic_actions() {
    let actions = suggest("frobnicate orbital widgets with purple taxonomy", 5).unwrap();
    let ids: Vec<&str> = actions.iter().map(|a| a.id.as_str()).collect();
    assert!(ids.contains(&"web_search"), "ids: {ids:?}");
    assert!(ids.contains(&"scratchpad_note"), "ids: {ids:?}");
}

#[test]
fn enumerate_connectors_finds_required_connector_crates() {
    let connectors = enumerate_connectors().unwrap();
    assert!(connectors.len() >= 5, "connectors: {connectors:?}");
    for id in ["gmail", "gcal", "hubspot", "slack", "notion"] {
        assert!(
            connectors.iter().any(|connector| connector.id == id),
            "missing {id}: {connectors:?}"
        );
    }
}

#[test]
fn enumerate_skills_finds_demo_vertical_skills() {
    let skills = enumerate_skills(&workspace_root().join("verticals")).unwrap();
    assert!(skills.len() >= 5, "skills: {skills:?}");
    assert!(skills.iter().any(|skill| skill.id == "draft_outreach"));
}

#[test]
fn enumerate_skills_skips_missing_skills_dir() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("demo-vertical")).unwrap();
    let skills = enumerate_skills(dir.path()).unwrap();
    assert!(skills.is_empty());
}

#[test]
fn demo_roles_get_three_non_empty_reason_actions() {
    for role in [
        "sales SDR who emails prospects",
        "customer support agent triaging tickets and drafting replies",
        "marketing intern drafting social campaigns",
        "AR clerk reconciling invoices",
        "marketplace seller posting listings and following up with buyers",
    ] {
        let actions = suggest(role, 5).unwrap();
        assert!(actions.len() >= 3, "{role}: {actions:?}");
        assert!(
            actions.iter().all(|action| !action.reason.is_empty()),
            "{role}: {actions:?}"
        );
    }
}
