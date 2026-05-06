//! Skate-selling proof chain — locks gtm-engineer's
//! `on_complete_emit` configuration (#772).
//!
//! Two flavours of test in one file:
//!
//! 1. **Manifest assertions**: parse the shipped
//!    `verticals/gtm-engineer/vertical.toml` and assert the
//!    `on_complete_emit` lists match the conservative chain
//!    we want. A future edit that wires `boost_listing` after
//!    `post_listing` (or removes the verify→draft→photos→post
//!    chain) will fail one of these.
//!
//! 2. **End-to-end integration**: drive the actual chain
//!    through the `argo` binary, completing each step in turn
//!    and asserting the next follow-up appears in the ledger
//!    with the right classification (autonomous-safe vs.
//!    pending-approval).
//!
//! Mock provider only — no API key required.

mod common;

use common::argo_bin;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

fn run_argo(home: &Path, work: &Path, args: &[&str]) -> std::process::Output {
    Command::new(argo_bin())
        .args(args)
        .env("ARGO_HOME", home)
        .env("ARGO_VERTICALS_DIR", workspace_root().join("verticals"))
        .current_dir(work)
        .output()
        .expect("spawn argo")
}

// ────────────────── manifest assertions ──────────────────

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
    // is exercised in the integration test below; here we just
    // lock the manifest declarations.
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

// ────────────────── end-to-end chain (integration) ──────────────────

fn fresh_workspace_for_chain() -> (tempfile::TempDir, tempfile::TempDir) {
    let home = tempfile::tempdir().expect("home tempdir");
    let work = tempfile::tempdir().expect("work tempdir");
    let init = run_argo(home.path(), work.path(), &["workspace", "init"]);
    assert!(
        init.status.success(),
        "init: {}",
        String::from_utf8_lossy(&init.stderr)
    );
    (home, work)
}

/// Walk the next pending action of the given action_type
/// through `complete` (using `execute` first if the row is
/// still in `pending`). Returns the completed row's
/// `action_id` so the caller can chain assertions.
fn execute_and_complete_next(home: &Path, work: &Path, action_type: &str) -> String {
    let list = run_argo(
        home,
        work,
        &[
            "worker",
            "actions",
            "list",
            "--state",
            "pending",
            "--vertical",
            "gtm-engineer",
            "--action-type",
            action_type,
            "--json",
        ],
    );
    assert!(
        list.status.success(),
        "list pending {action_type}: {}",
        String::from_utf8_lossy(&list.stderr)
    );
    let stdout = String::from_utf8(list.stdout).expect("utf8");
    let line = stdout
        .lines()
        .next()
        .unwrap_or_else(|| panic!("no pending `{action_type}` row in ledger:\n{stdout}"));
    let row: Value = serde_json::from_str(line).expect("json row");
    let action_id = row["action_id"].as_str().expect("action_id").to_string();
    let exec = run_argo(
        home,
        work,
        &["worker", "actions", "execute", &action_id, "--json"],
    );
    assert!(
        exec.status.success(),
        "execute {action_id}: {}",
        String::from_utf8_lossy(&exec.stderr)
    );
    let complete = run_argo(
        home,
        work,
        &[
            "worker",
            "actions",
            "complete",
            &action_id,
            "--note",
            &format!("smoke complete {action_type}"),
            "--json",
        ],
    );
    assert!(
        complete.status.success(),
        "complete {action_id}: {}",
        String::from_utf8_lossy(&complete.stderr)
    );
    action_id
}

/// Find the most recent row of the given (action_type,
/// state) combination and return its `evidence` list. The
/// chain's follow-ups always reference the source action via
/// `argo://chain/<id>` in `evidence` (#774), so this lets the
/// test distinguish chain-emitted rows from any other rows
/// the mock provider produced (e.g. the mock initially emits
/// a `post_listing` that lands `blocked` under mock evidence).
fn latest_row_evidence(
    home: &Path,
    work: &Path,
    action_type: &str,
    state: &str,
) -> Vec<String> {
    let list = run_argo(
        home,
        work,
        &[
            "worker",
            "actions",
            "list",
            "--state",
            state,
            "--vertical",
            "gtm-engineer",
            "--action-type",
            action_type,
            "--limit",
            "1",
            "--json",
        ],
    );
    assert!(list.status.success());
    let stdout = String::from_utf8(list.stdout).expect("utf8");
    let line = stdout.lines().next().unwrap_or_else(|| {
        panic!("no `{action_type}` row in state `{state}`:\n{stdout}")
    });
    let row: Value = serde_json::from_str(line).expect("json");
    row["evidence"]
        .as_array()
        .expect("evidence array")
        .iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect()
}

#[test]
fn pre_sale_chain_fires_through_the_binary() {
    let (home, work) = fresh_workspace_for_chain();

    // Boot the chain: the mock worker emits a verify_price
    // action when given any prompt. From there, every operator
    // `complete` triggers the next link in the manifest chain.
    let add = run_argo(
        home.path(),
        work.path(),
        &[
            "work",
            "add",
            "--vertical",
            "gtm-engineer",
            "--prompt",
            "verify the skate price",
            "--json",
        ],
    );
    assert!(
        add.status.success(),
        "add: {}",
        String::from_utf8_lossy(&add.stderr)
    );
    let serve = run_argo(
        home.path(),
        work.path(),
        &[
            "work",
            "serve",
            "--vertical",
            "gtm-engineer",
            "--once",
            "--max-steps",
            "1",
            "--json",
        ],
    );
    assert!(
        serve.status.success(),
        "serve: {}",
        String::from_utf8_lossy(&serve.stderr)
    );

    // Step 1: complete verify_price. Find the verify_price
    // pending row, walk it through execute → complete. The
    // chain's evidence URL points back at this verify_price.
    let verify_id = execute_and_complete_next(home.path(), work.path(), "verify_price");
    let draft_evidence =
        latest_row_evidence(home.path(), work.path(), "create_listing_draft", "pending");
    assert!(
        draft_evidence.contains(&format!("argo://chain/{verify_id}")),
        "create_listing_draft follow-up must reference verify_price `{verify_id}` in evidence; \
         got: {draft_evidence:?}"
    );

    // Step 2: complete create_listing_draft → request_photos.
    let draft_id = execute_and_complete_next(home.path(), work.path(), "create_listing_draft");
    let photos_evidence =
        latest_row_evidence(home.path(), work.path(), "request_photos", "pending");
    assert!(
        photos_evidence.contains(&format!("argo://chain/{draft_id}")),
        "request_photos follow-up must reference create_listing_draft `{draft_id}`"
    );

    // Step 3: complete request_photos → post_listing.
    // post_listing is approval-gated — the follow-up must land
    // `pending` awaiting an explicit operator `approve`. Note
    // the mock provider may have separately emitted a `blocked`
    // post_listing earlier (evidence-insufficient under mock);
    // the chain's row is the one with `state=pending` AND
    // evidence pointing back at request_photos.
    let photos_id = execute_and_complete_next(home.path(), work.path(), "request_photos");
    let post_evidence = latest_row_evidence(home.path(), work.path(), "post_listing", "pending");
    assert!(
        post_evidence.contains(&format!("argo://chain/{photos_id}")),
        "post_listing chain row must reference request_photos `{photos_id}`; got: {post_evidence:?}"
    );
    let list = run_argo(
        home.path(),
        work.path(),
        &[
            "worker",
            "actions",
            "list",
            "--state",
            "pending",
            "--vertical",
            "gtm-engineer",
            "--action-type",
            "post_listing",
            "--json",
        ],
    );
    let stdout = String::from_utf8(list.stdout).expect("utf8");
    let row: Value = serde_json::from_str(stdout.lines().next().expect("post_listing row")).unwrap();
    assert_eq!(
        row["requires_approval"].as_bool(),
        Some(true),
        "post_listing follow-up must preserve approval gate"
    );

    // Step 4: NO boost_listing should have appeared anywhere
    // in the ledger across the entire chain. Locks the "no
    // auto-spend" contract end-to-end.
    let boost_list = run_argo(
        home.path(),
        work.path(),
        &[
            "worker",
            "actions",
            "list",
            "--state",
            "all",
            "--vertical",
            "gtm-engineer",
            "--action-type",
            "boost_listing",
            "--json",
        ],
    );
    let boost_stdout = String::from_utf8(boost_list.stdout).expect("utf8");
    let boost_rows: Vec<&str> = boost_stdout
        .lines()
        .filter(|l| !l.trim().is_empty())
        .collect();
    assert!(
        boost_rows.is_empty(),
        "boost_listing must NOT auto-emit anywhere in the chain; got rows: {boost_rows:?}"
    );
}
