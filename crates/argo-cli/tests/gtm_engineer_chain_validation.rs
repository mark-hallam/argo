//! Manifest assertions for gtm-engineer's `on_complete_emit`
//! configuration — split out of `gtm_engineer_chain.rs` to keep
//! individual files below the 500-LOC ceiling.
//!
//! These tests parse the shipped
//! `verticals/gtm-engineer/vertical.toml` and assert the GTM
//! chain wiring + approval-gating contracts. The end-to-end
//! integration that drives the chain through the actual `argo`
//! binary lives in the sibling `gtm_engineer_chain.rs`.
//!
//! The GTM Engineer's action vocabulary is GTM-only (the
//! marketplace skate-selling vocabulary from #772 was replaced).
//! The two declarative chains:
//!
//!   research/draft loop (all autonomous-safe except the SEND):
//!     define_icp → find_target_accounts → research_company →
//!     research_person → draft_outreach → send_outreach
//!     (send_outreach is approval-gated + TERMINAL)
//!
//!   learning loop (all autonomous-safe):
//!     analyze_campaign → detect_gtm_pattern → propose_gtm_skill
//!     (propose_gtm_skill is TERMINAL — operator-decided)
//!
//! Mock provider only — no API key required.

use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

fn load_gtm_actions() -> argo_vertical::ActionsSection {
    let manifest_path = workspace_root().join("verticals/gtm-engineer/vertical.toml");
    let body = std::fs::read_to_string(&manifest_path).expect("read gtm-engineer manifest");
    let manifest = argo_vertical::parse(&body).expect("manifest parses");
    manifest.actions.expect("[actions] section present")
}

fn assert_chain(actions: &argo_vertical::ActionsSection, source: &str, expected: &[&str]) {
    let decl = actions
        .actions
        .get(source)
        .unwrap_or_else(|| panic!("missing [actions.{source}] declaration"));
    let expected_owned: Vec<String> = expected.iter().map(|s| (*s).to_string()).collect();
    assert_eq!(
        decl.on_complete_emit, expected_owned,
        "on_complete_emit for `{source}` should be {expected:?}"
    );
}

#[test]
fn research_draft_chain_walks_icp_to_send() {
    // The research/draft chain. Each link's classification is
    // exercised in the e2e test in the sibling file; here we just
    // lock the manifest declarations.
    let actions = load_gtm_actions();
    assert_chain(&actions, "define_icp", &["find_target_accounts"]);
    assert_chain(&actions, "find_target_accounts", &["research_company"]);
    assert_chain(&actions, "research_company", &["research_person"]);
    assert_chain(&actions, "research_person", &["draft_outreach"]);
    assert_chain(&actions, "draft_outreach", &["send_outreach"]);
}

#[test]
fn learning_loop_chain_walks_analyze_to_propose() {
    let actions = load_gtm_actions();
    assert_chain(&actions, "analyze_campaign", &["detect_gtm_pattern"]);
    assert_chain(&actions, "detect_gtm_pattern", &["propose_gtm_skill"]);
}

#[test]
fn send_outreach_does_not_auto_emit_anything() {
    // send_outreach is the single external-mutation action — it
    // touches another human and is irreversible. It must NOT
    // auto-emit a follow-up (no auto re-send, no auto "boost").
    // Any follow-up send is a fresh operator-initiated decision.
    // Mirrors the old marketplace post_listing "no auto-boost"
    // contract. Locks that send_outreach stays terminal.
    let actions = load_gtm_actions();
    let send = actions
        .actions
        .get("send_outreach")
        .expect("send_outreach declared");
    assert!(
        send.on_complete_emit.is_empty(),
        "send_outreach.on_complete_emit must stay empty — a follow-up send is \
         operator-decided. Got: {:?}",
        send.on_complete_emit
    );
}

#[test]
fn propose_gtm_skill_does_not_auto_emit_anything() {
    // propose_gtm_skill completing queues a learned-skill proposal
    // in pending_review; the operator decides whether to approve
    // it (`argo skill review`). There is no downstream action to
    // auto-emit — promotion is operator-gated by the lifecycle
    // gate (#681), not a chain. Mirrors the old marketplace
    // mark_item_sold "no auto-emit" contract. Locks this so a
    // future edit that wires propose_gtm_skill → anything (which
    // would bypass the operator's review) gets caught.
    let actions = load_gtm_actions();
    let propose = actions
        .actions
        .get("propose_gtm_skill")
        .expect("propose_gtm_skill declared");
    assert!(
        propose.on_complete_emit.is_empty(),
        "propose_gtm_skill.on_complete_emit must stay empty — promotion is \
         operator-gated, not auto-chained. Got: {:?}",
        propose.on_complete_emit
    );
}

#[test]
fn every_chained_target_is_actually_declared() {
    // Cross-check: each action_type listed in any
    // on_complete_emit MUST itself be a declared action. A typo
    // (`research_compny`) would otherwise silently emit a
    // `blocked` row at runtime — at lint-time we can catch it.
    let actions = load_gtm_actions();
    for (source_id, decl) in &actions.actions {
        for target in &decl.on_complete_emit {
            assert!(
                actions.actions.contains_key(target),
                "on_complete_emit on `{source_id}` references undeclared action_type `{target}`"
            );
        }
    }
}

#[test]
fn autonomous_safe_actions_stay_autonomous_safe() {
    // The eight research/draft/analysis steps are draft-only
    // artifacts — autonomous-safe (no approval, no external
    // mutation). Lock the existing classifications so a future
    // "let's gate this too" or "let's not gate this" edit gets
    // caught by CI. requires_approval=false AND risk=Low.
    let actions = load_gtm_actions();
    let must_be_autonomous_safe = [
        "define_icp",
        "find_target_accounts",
        "research_company",
        "research_person",
        "draft_outreach",
        "analyze_campaign",
        "detect_gtm_pattern",
        "propose_gtm_skill",
    ];
    for action_id in must_be_autonomous_safe {
        let decl = actions
            .actions
            .get(action_id)
            .unwrap_or_else(|| panic!("{action_id} declared"));
        assert!(
            !decl.requires_approval,
            "`{action_id}` MUST be autonomous-safe; manifest has requires_approval=true"
        );
        assert_eq!(
            decl.risk,
            argo_vertical::ActionRiskLevel::Low,
            "`{action_id}` MUST be risk=low (draft-only artifact)"
        );
    }
}

#[test]
fn send_outreach_is_approval_gated_high_risk() {
    // The proof's load-bearing safety contract: the ONE action
    // that touches another human / sends externally must require
    // approval AND be classified high-risk. Lock it so a future
    // "let's relax this" edit gets caught by CI.
    let actions = load_gtm_actions();
    let send = actions
        .actions
        .get("send_outreach")
        .expect("send_outreach declared");
    assert!(
        send.requires_approval,
        "send_outreach MUST be approval-gated; manifest has requires_approval=false"
    );
    assert_eq!(
        send.risk,
        argo_vertical::ActionRiskLevel::High,
        "send_outreach MUST be risk=high — it touches another human and is irreversible"
    );
}

#[test]
fn no_marketplace_action_names_remain() {
    // The vertical was de-marketplaced: the skate-selling action
    // vocabulary (verify_price / create_listing_draft /
    // request_photos / post_listing / boost_listing /
    // update_price / relist_item / mark_item_sold /
    // reply_to_buyer / follow_up_buyer) MUST NOT reappear. The
    // action vocabulary is GTM-only. Locks the refactor so a
    // future copy-paste from marketplace-seller gets caught.
    let actions = load_gtm_actions();
    let forbidden = [
        "verify_price",
        "create_listing_draft",
        "request_photos",
        "post_listing",
        "boost_listing",
        "update_price",
        "relist_item",
        "mark_item_sold",
        "reply_to_buyer",
        "follow_up_buyer",
    ];
    for name in forbidden {
        assert!(
            !actions.actions.contains_key(name),
            "marketplace action `{name}` leaked back into the GTM Engineer vocabulary"
        );
    }
    // And the exact GTM-only set (9 actions) is what's declared.
    let expected: std::collections::HashSet<&str> = [
        "define_icp",
        "find_target_accounts",
        "research_company",
        "research_person",
        "draft_outreach",
        "analyze_campaign",
        "detect_gtm_pattern",
        "propose_gtm_skill",
        "send_outreach",
    ]
    .into_iter()
    .collect();
    let got: std::collections::HashSet<&str> =
        actions.actions.keys().map(String::as_str).collect();
    assert_eq!(
        got, expected,
        "GTM Engineer action vocabulary drift: expected exactly the 9 GTM actions"
    );
}

#[test]
fn research_steps_declare_retry_policy() {
    // The web-research steps (find_target_accounts /
    // research_company / research_person / analyze_campaign)
    // declare max_attempts=3 so transient backend errors don't
    // burn the safety budget. The single high-risk send stays
    // tight at max 2. Locks the retry policy.
    let actions = load_gtm_actions();
    for action_id in [
        "find_target_accounts",
        "research_company",
        "research_person",
        "analyze_campaign",
    ] {
        let decl = actions
            .actions
            .get(action_id)
            .unwrap_or_else(|| panic!("{action_id} declared"));
        let retry = decl
            .retry
            .as_ref()
            .unwrap_or_else(|| panic!("{action_id} must declare a [retry] policy"));
        assert_eq!(
            retry.max_attempts, 3,
            "{action_id} research retry should allow 3 attempts"
        );
    }
    let send = actions.actions.get("send_outreach").expect("declared");
    let send_retry = send.retry.as_ref().expect("send_outreach retry");
    assert_eq!(
        send_retry.max_attempts, 2,
        "send_outreach retry must stay tight (max 2) to avoid double-sending to a human"
    );
}
