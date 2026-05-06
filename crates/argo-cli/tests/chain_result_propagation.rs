//! End-to-end test for #778 — chain emitters now use the
//! COMPLETED record (with the operator's `result` populated),
//! not the stale pre-completion snapshot. Pre-#778 the
//! emitter saw `result: None`, so `--result-json` keys never
//! reached the next chain link's payload — the proof-run
//! `post_listing` template rendered a blank `{{ payload.price }}`.
//!
//! Drives the full path through the binary: queue a job, run
//! the mock worker once to emit verify_price, then complete
//! it WITH a result-json containing a marker key. Asserts that
//! the chain-emitted `create_listing_draft` carries the marker
//! in its payload.

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

fn fresh_workspace() -> (tempfile::TempDir, tempfile::TempDir) {
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

#[test]
fn complete_result_keys_propagate_into_next_chain_link_payload() {
    let (home, work) = fresh_workspace();
    let add = run_argo(
        home.path(),
        work.path(),
        &[
            "work",
            "add",
            "--vertical",
            "gtm-engineer",
            "--prompt",
            "result-key propagation smoke",
            "--json",
        ],
    );
    assert!(add.status.success());
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
    assert!(serve.status.success());

    // Complete verify_price WITH a result-json. The chain
    // should emit create_listing_draft with payload carrying
    // both the verify_price source payload AND the operator's
    // result keys.
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
            "verify_price",
            "--json",
        ],
    );
    assert!(list.status.success());
    let stdout = String::from_utf8(list.stdout).expect("utf8");
    let row: Value =
        serde_json::from_str(stdout.lines().next().expect("verify_price row"))
            .expect("json row");
    let verify_id = row["action_id"].as_str().expect("action_id").to_string();

    // execute → complete with marker keys in result-json
    let exec = run_argo(
        home.path(),
        work.path(),
        &["worker", "actions", "execute", &verify_id, "--json"],
    );
    assert!(
        exec.status.success(),
        "execute: {}",
        String::from_utf8_lossy(&exec.stderr)
    );
    let complete = run_argo(
        home.path(),
        work.path(),
        &[
            "worker",
            "actions",
            "complete",
            &verify_id,
            "--note",
            "operator accepts proposal",
            "--result-json",
            r#"{"accepted_price_aud": 549, "verdict": "accepted"}"#,
            "--json",
        ],
    );
    assert!(
        complete.status.success(),
        "complete: {}",
        String::from_utf8_lossy(&complete.stderr)
    );

    // Find the chain-emitted create_listing_draft and assert
    // both result-keys are visible in its payload.
    let draft_list = run_argo(
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
            "create_listing_draft",
            "--json",
        ],
    );
    assert!(draft_list.status.success());
    let draft_stdout = String::from_utf8(draft_list.stdout).expect("utf8");
    let draft_row: Value =
        serde_json::from_str(draft_stdout.lines().next().expect("draft row"))
            .expect("json row");

    let payload = &draft_row["payload"];
    assert_eq!(
        payload["accepted_price_aud"], 549,
        "result-key from verify_price.complete must propagate into create_listing_draft.payload — \
         pre-#778 the chain emitter saw the pre-completion snapshot and silently dropped result keys"
    );
    assert_eq!(
        payload["verdict"], "accepted",
        "string result-key must propagate too"
    );
}
