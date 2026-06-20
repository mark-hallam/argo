//! GTM research/draft proof chain — end-to-end integration that
//! drives the gtm-engineer manifest chain through the actual
//! `argo` binary, completing each step in turn and asserting the
//! next follow-up appears in the ledger with the right
//! classification (autonomous-safe vs. pending-approval).
//!
//! The vertical's action vocabulary is GTM-only (the marketplace
//! skate-selling vocabulary from #772 was replaced). The mock
//! worker (vertical-aware in `work_steps::run_mock_step`) emits
//! `find_target_accounts` (autonomous-safe → lands `pending`,
//! boots the chain; the GTM analog of the old verify_price) +
//! `send_outreach` (approval-gated → lands `blocked` under mock
//! evidence). Completing each autonomous-safe link auto-emits the
//! next per the manifest's `on_complete_emit`:
//!
//!   find_target_accounts → research_company → research_person →
//!   draft_outreach → send_outreach
//!
//! (The full manifest chain also has `define_icp →
//! find_target_accounts` upstream; the mock boots from
//! find_target_accounts so the chain test exercises the research
//! → draft → gated-send arc. The define_icp link is locked in the
//! sibling validation test.)
//!
//! send_outreach is approval-gated, so the final follow-up lands
//! `pending` awaiting an explicit operator `approve`.
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
/// a `send_outreach` that lands `blocked` under mock evidence).
fn latest_row_evidence(home: &Path, work: &Path, action_type: &str, state: &str) -> Vec<String> {
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
    let line = stdout
        .lines()
        .next()
        .unwrap_or_else(|| panic!("no `{action_type}` row in state `{state}`:\n{stdout}"));
    let row: Value = serde_json::from_str(line).expect("json");
    row["evidence"]
        .as_array()
        .expect("evidence array")
        .iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect()
}

#[test]
fn research_draft_chain_fires_through_the_binary() {
    let (home, work) = fresh_workspace_for_chain();

    // Boot the chain: the mock worker emits a find_target_accounts
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
            "find target accounts and run the first GTM experiment",
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

    // Step 1: complete find_target_accounts → research_company.
    // Find the find_target_accounts pending row, walk it through
    // execute → complete. The chain's evidence URL points back at
    // it. (find_target_accounts is the mock's chain root — the GTM
    // analog of the old marketplace verify_price.)
    let accounts_id =
        execute_and_complete_next(home.path(), work.path(), "find_target_accounts");
    let company_evidence =
        latest_row_evidence(home.path(), work.path(), "research_company", "pending");
    assert!(
        company_evidence.contains(&format!("argo://chain/{accounts_id}")),
        "research_company follow-up must reference find_target_accounts `{accounts_id}` in \
         evidence; got: {company_evidence:?}"
    );

    // Step 2: complete research_company → research_person.
    let company_id = execute_and_complete_next(home.path(), work.path(), "research_company");
    let person_evidence =
        latest_row_evidence(home.path(), work.path(), "research_person", "pending");
    assert!(
        person_evidence.contains(&format!("argo://chain/{company_id}")),
        "research_person follow-up must reference research_company `{company_id}`"
    );

    // Step 3: complete research_person → draft_outreach.
    let person_id = execute_and_complete_next(home.path(), work.path(), "research_person");
    let draft_evidence =
        latest_row_evidence(home.path(), work.path(), "draft_outreach", "pending");
    assert!(
        draft_evidence.contains(&format!("argo://chain/{person_id}")),
        "draft_outreach follow-up must reference research_person `{person_id}`"
    );

    // Step 4: complete draft_outreach → send_outreach.
    // send_outreach is approval-gated — the follow-up must land
    // `pending` awaiting an explicit operator `approve`. Note
    // the mock provider may have separately emitted a `blocked`
    // send_outreach earlier (evidence-insufficient under mock);
    // the chain's row is the one with `state=pending` AND
    // evidence pointing back at draft_outreach.
    let draft_id = execute_and_complete_next(home.path(), work.path(), "draft_outreach");
    let send_evidence = latest_row_evidence(home.path(), work.path(), "send_outreach", "pending");
    assert!(
        send_evidence.contains(&format!("argo://chain/{draft_id}")),
        "send_outreach chain row must reference draft_outreach `{draft_id}`; got: {send_evidence:?}"
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
            "send_outreach",
            "--json",
        ],
    );
    let stdout = String::from_utf8(list.stdout).expect("utf8");
    let row: Value =
        serde_json::from_str(stdout.lines().next().expect("send_outreach row")).unwrap();
    assert_eq!(
        row["requires_approval"].as_bool(),
        Some(true),
        "send_outreach follow-up must preserve approval gate"
    );

    // Step 5: send_outreach is TERMINAL — completing it must NOT
    // auto-emit any follow-up send. We can't `complete` the
    // approval-gated row without approving it first, but the
    // manifest contract (locked in the sibling validation test)
    // guarantees send_outreach.on_complete_emit is empty. Here we
    // assert the runtime never produced any SECOND-generation
    // chain row beyond send_outreach: there must be exactly one
    // chain-pending send_outreach and nothing chained off it.
    // (The "no auto follow-up" contract end-to-end.)
    let send_pending = run_argo(
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
            "send_outreach",
            "--json",
        ],
    );
    let send_stdout = String::from_utf8(send_pending.stdout).expect("utf8");
    let send_rows: Vec<&str> = send_stdout
        .lines()
        .filter(|l| !l.trim().is_empty())
        .collect();
    assert_eq!(
        send_rows.len(),
        1,
        "exactly one pending send_outreach chain row expected (no auto re-send); got: {send_rows:?}"
    );
}
