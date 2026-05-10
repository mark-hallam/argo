//! Manifest assertions for gtm-engineer's `on_complete_emit`
//! configuration (#772) — split out of
//! `gtm_engineer_chain.rs` to keep individual files below the
//! 500-LOC ceiling.
//!
//! These tests parse the shipped
//! `verticals/gtm-engineer/vertical.toml` and assert the chain
//! wiring + approval-gating contracts. The end-to-end
//! integration that drives the chain through the actual `argo`
//! binary lives in the sibling `gtm_engineer_chain.rs`.
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
fn pre_sale_chain_is_verify_to_draft_to_photos_to_post() {
    // The four-step pre-sale chain. Each link's classification
    // is exercised in the e2e test in the sibling file; here we
    // just lock the manifest declarations.
    let actions = load_gtm_actions();
    assert_chain(&actions, "verify_price", &["create_listing_draft"]);
    assert_chain(&actions, "create_listing_draft", &["request_photos"]);
    assert_chain(&actions, "request_photos", &["post_listing"]);
}

#[test]
fn update_price_auto_emits_relist_item() {
    let actions = load_gtm_actions();
    assert_chain(&actions, "update_price", &["relist_item"]);
}

#[test]
fn post_listing_does_not_auto_emit_boost_listing() {
    // Boosting spends operator funds — auto-emitting it would
    // make the worker spend money on the operator's behalf
    // without an explicit per-item decision. The operator must
    // queue boost_listing manually when there's evidence of
    // weak organic traction. Locks that contract.
    let actions = load_gtm_actions();
    let post_listing = actions
        .actions
        .get("post_listing")
        .expect("post_listing declared");
    assert!(
        post_listing.on_complete_emit.is_empty(),
        "post_listing.on_complete_emit must stay empty — boost is operator-decided. \
         Got: {:?}",
        post_listing.on_complete_emit
    );
}

#[test]
fn mark_item_sold_does_not_auto_emit_anything() {
    // mark_item_sold completing closes the inventory item; the
    // sold-item gate (#769) handles the downstream stop, so
    // there's no on_complete_emit needed. Locks this contract
    // so a future edit that wires `mark_item_sold` →
    // `follow_up_buyer` (which would race with the sold-gate)
    // gets caught.
    //
    // The sold-gate logic itself is covered by the unit tests
    // in `worker_actions_emit_tests.rs` (#769) and
    // `worker_actions_followup_tests.rs` (#771): once a
    // mark_item_sold is `completed` for `item_id = X`, every
    // future emission with `item_id = X` (except a redundant
    // mark_item_sold) gets downgraded to `blocked`. Those
    // tests apply uniformly to the gtm-engineer chain because
    // the gate is enforced at the OS level, not per-vertical.
    let actions = load_gtm_actions();
    let mark_sold = actions
        .actions
        .get("mark_item_sold")
        .expect("mark_item_sold declared");
    assert!(
        mark_sold.on_complete_emit.is_empty(),
        "mark_item_sold.on_complete_emit must stay empty — sold-gate handles closure. \
         Got: {:?}",
        mark_sold.on_complete_emit
    );
}

#[test]
fn buyer_touching_actions_have_no_auto_emit() {
    // reply_to_buyer + follow_up_buyer touch another human and
    // are HIGH RISK. They MUST be operator-initiated, not
    // auto-emitted as part of a chain. Locks that contract
    // independently from the manifest's `requires_approval`
    // stamp so a future "convenience" edit can't sneak through.
    let actions = load_gtm_actions();
    for buyer_action in ["reply_to_buyer", "follow_up_buyer"] {
        let decl = actions
            .actions
            .get(buyer_action)
            .unwrap_or_else(|| panic!("{buyer_action} declared"));
        assert!(
            decl.on_complete_emit.is_empty(),
            "{buyer_action}.on_complete_emit must stay empty — buyer-touching actions \
             are operator-initiated only. Got: {:?}",
            decl.on_complete_emit
        );
    }
}

#[test]
fn every_chained_target_is_actually_declared() {
    // Cross-check: each action_type listed in any
    // on_complete_emit MUST itself be a declared action. A typo
    // (`request_phots`) would otherwise silently emit a
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
fn every_approval_gated_action_stays_approval_gated() {
    // The proof's load-bearing safety contract: every action
    // that touches money / public surfaces / buyers must
    // require approval. Lock the existing classifications so a
    // future "let's relax this" edit gets caught by CI.
    let actions = load_gtm_actions();
    let must_require_approval = [
        "post_listing",
        "boost_listing",
        "update_price",
        "relist_item",
        "mark_item_sold",
        "reply_to_buyer",
        "follow_up_buyer",
    ];
    for action_id in must_require_approval {
        let decl = actions
            .actions
            .get(action_id)
            .unwrap_or_else(|| panic!("{action_id} declared"));
        assert!(
            decl.requires_approval,
            "`{action_id}` MUST be approval-gated; manifest has requires_approval=false"
        );
    }
}
