//! Skate-selling proof chain — end-to-end integration that
//! drives the gtm-engineer manifest chain (#772) through the
//! actual `argo` binary, completing each step in turn and
//! asserting the next follow-up appears in the ledger with the
//! right classification (autonomous-safe vs. pending-approval).
//!
//! The companion file `gtm_engineer_chain_validation.rs` holds
//! the manifest-only assertions (chain wiring, approval-gating
//! contracts) — they parse `verticals/gtm-engineer/vertical.toml`
//! directly and need no binary spawn.
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
/// through `execute` then `complete`, including the supplied
/// result-json if any. Returns the completed row's `action_id`
/// so the caller can chain assertions.
///
/// `result_json` lets a test assert end-to-end that the
/// operator's completion-time output flows into the next
/// chain link's payload (#778). When `None`, only `--note` is
/// recorded — the source action's `result` is just `{summary}`.
fn execute_and_complete_next_with(
    home: &Path,
    work: &Path,
    action_type: &str,
    result_json: Option<&str>,
) -> String {
    let action_id = execute_pending_returning_id(home, work, action_type);
    let mut args: Vec<String> = vec![
        "worker".into(),
        "actions".into(),
        "complete".into(),
        action_id.clone(),
        "--note".into(),
        format!("smoke complete {action_type}"),
        "--json".into(),
    ];
    if let Some(rj) = result_json {
        args.push("--result-json".into());
        args.push(rj.into());
    }
    let args_ref: Vec<&str> = args.iter().map(String::as_str).collect();
    let complete = run_argo(home, work, &args_ref);
    assert!(
        complete.status.success(),
        "complete {action_id}: {}",
        String::from_utf8_lossy(&complete.stderr)
    );
    action_id
}

fn execute_pending_returning_id(home: &Path, work: &Path, action_type: &str) -> String {
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
    action_id
}

/// Walk the next pending action through `execute → complete`
/// with no result-json (only `--note`). Thin wrapper around
/// `execute_and_complete_next_with` for readability in tests
/// that don't care about result-key propagation.
fn execute_and_complete_next(home: &Path, work: &Path, action_type: &str) -> String {
    execute_and_complete_next_with(home, work, action_type, None)
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
