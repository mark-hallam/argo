//! End-to-end test for Worker v3 action emission (#729).
//!
//! Drives the same `argo work serve --once` path the production
//! worker uses, verifies that the `action`-fenced JSON blocks
//! the mock step writes get extracted, classified against the
//! gtm-engineer manifest, persisted as `worker_action` memory
//! rows, AND rendered into `execution_plan.md` alongside the
//! rest of the artifact bundle.
//!
//! Mock-mode only — no API key required. Uses the production
//! `verticals/gtm-engineer/vertical.toml` so the assertions are
//! anchored on its real `[actions]` declarations:
//!   * `find_target_accounts`  — autonomous-safe (low risk, no approval)
//!   * `send_outreach` — approval-gated (high risk + approval)
//!
//! Mock mode never calls `web_search` / `web_extract`, so the
//! research-evidence gate trips for any approval-gated action:
//!   * `find_target_accounts` lands `pending` (autonomous, gate bypassed)
//!   * `send_outreach` lands `blocked` with a "research evidence
//!     insufficient" reason (gate downgrade)

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

fn fresh_workspace() -> (tempfile::TempDir, tempfile::TempDir) {
    let home = tempfile::tempdir().expect("home tempdir");
    let work = tempfile::tempdir().expect("work tempdir");
    let init = Command::new(argo_bin())
        .arg("workspace")
        .arg("init")
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", workspace_root().join("verticals"))
        .current_dir(work.path())
        .output()
        .expect("workspace init");
    assert!(
        init.status.success(),
        "workspace init failed: {}",
        String::from_utf8_lossy(&init.stderr)
    );
    (home, work)
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

#[test]
fn mock_run_emits_actions_and_writes_execution_plan() {
    let (home, work) = fresh_workspace();

    // Enqueue a job for gtm-engineer (which now declares
    // `find_target_accounts` + `send_outreach` in its [actions] section).
    let add = run_argo(
        home.path(),
        work.path(),
        &[
            "work",
            "add",
            "--vertical",
            "gtm-engineer",
            "--prompt",
            "Run the first GTM outreach experiment.",
            "--json",
        ],
    );
    assert!(
        add.status.success(),
        "{}",
        String::from_utf8_lossy(&add.stderr)
    );

    // --max-steps 1 + mock provider → run_mock_step emits both
    // a find_target_accounts (autonomous) + a send_outreach (approval-
    // gated) action block in the assistant text.
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
        "{}",
        String::from_utf8_lossy(&serve.stderr)
    );
    let processed: Value = serde_json::from_slice(&serve.stdout).expect("json");
    assert_eq!(processed["ok"], true);
    // Both action blocks should land as memory rows.
    assert_eq!(
        processed["actions_emitted"], 2,
        "expected 2 worker_action rows persisted; got {processed}",
    );

    // execution_plan.md lands alongside the rest of the bundle.
    let output_dir = PathBuf::from(
        processed["output_dir"]
            .as_str()
            .expect("output_dir on serve --json"),
    );
    let plan_path = output_dir.join("execution_plan.md");
    assert!(
        plan_path.exists(),
        "execution_plan.md missing in {}",
        output_dir.display()
    );
    let plan = std::fs::read_to_string(&plan_path).expect("read execution_plan.md");
    assert!(
        plan.starts_with("# Execution plan"),
        "execution_plan.md must start with the H1; got:\n{plan}"
    );
    assert!(
        plan.contains("Emitted 2 action(s)"),
        "execution_plan.md should summarise the emission count; got:\n{plan}"
    );
    // find_target_accounts is autonomous-safe → in the autonomous section.
    assert!(
        plan.contains("## Pending (autonomous-safe)"),
        "find_target_accounts (autonomous-safe) should land in the safe section; got:\n{plan}"
    );
    assert!(
        plan.contains("`find_target_accounts`"),
        "execution_plan.md should mention find_target_accounts; got:\n{plan}"
    );
    // Mock mode runs no web tools → research evidence is
    // insufficient → send_outreach (approval-gated) downgrades to
    // blocked, NOT pending-approval.
    assert!(
        plan.contains("## Blocked"),
        "send_outreach should fall into the Blocked section under mock evidence; got:\n{plan}"
    );
    assert!(
        plan.contains("`send_outreach`"),
        "execution_plan.md should mention send_outreach; got:\n{plan}"
    );
    assert!(
        plan.contains("research evidence insufficient"),
        "blocked-reason text from the evidence-gate downgrade should appear; got:\n{plan}"
    );

    // Persistent ledger view: `argo worker actions list --state all`
    // should surface both rows, with the right state assignments.
    let list = run_argo(
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
            "--json",
        ],
    );
    assert!(
        list.status.success(),
        "actions list failed: {}",
        String::from_utf8_lossy(&list.stderr)
    );
    let rows: Vec<Value> = String::from_utf8_lossy(&list.stdout)
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();
    assert_eq!(rows.len(), 2, "expected 2 action rows; got {rows:?}");

    // find_target_accounts is pending; send_outreach is blocked w/ reason.
    let by_type: std::collections::HashMap<&str, &Value> = rows
        .iter()
        .filter_map(|r| {
            r.get("action_type")
                .and_then(|v| v.as_str())
                .map(|t| (t, r))
        })
        .collect();
    let vp = by_type.get("find_target_accounts").expect("find_target_accounts row");
    assert_eq!(vp["state"], "pending");
    assert_eq!(vp["risk_level"], "low");
    assert_eq!(vp["requires_approval"], false);

    let pl = by_type.get("send_outreach").expect("send_outreach row");
    assert_eq!(pl["state"], "blocked");
    assert_eq!(pl["risk_level"], "high");
    assert_eq!(pl["requires_approval"], true);
    let reason = pl["blocked_reason"].as_str().unwrap_or("");
    assert!(
        reason.contains("research evidence insufficient"),
        "send_outreach blocked_reason missing the gate text; got: {reason}"
    );

    // actions.json is the machine-readable mirror of
    // execution_plan.md — operators and tooling consume this
    // instead of parsing the markdown.
    let actions_json_path = output_dir.join("actions.json");
    assert!(
        actions_json_path.exists(),
        "actions.json missing in {}",
        output_dir.display()
    );
    let actions_json: Value =
        serde_json::from_slice(&std::fs::read(&actions_json_path).expect("read actions.json"))
            .expect("parse actions.json");
    assert_eq!(actions_json["schema_version"], "1");
    assert_eq!(actions_json["vertical_id"], "gtm-engineer");
    // Mock mode runs no web tools → evidence_sufficient is false.
    assert_eq!(actions_json["evidence_sufficient"], false);
    assert_eq!(actions_json["counts"]["total"], 2);
    assert_eq!(actions_json["counts"]["pending_autonomous"], 1);
    assert_eq!(actions_json["counts"]["pending_approval"], 0);
    assert_eq!(actions_json["counts"]["blocked"], 1);
    let arr = actions_json["actions"]
        .as_array()
        .expect("actions array in actions.json");
    assert_eq!(arr.len(), 2);
    let types: Vec<&str> = arr
        .iter()
        .filter_map(|r| r["action_type"].as_str())
        .collect();
    assert!(types.contains(&"find_target_accounts"));
    assert!(types.contains(&"send_outreach"));

    // result.json now carries the per-job action bundle pointers
    // so operators can find both the markdown + the JSON via
    // the same canonical index file.
    let result_json: Value =
        serde_json::from_slice(&std::fs::read(output_dir.join("result.json")).unwrap())
            .expect("parse result.json");
    let plan_path_in_result = result_json["execution_plan_path"]
        .as_str()
        .expect("execution_plan_path on result.json");
    assert!(
        plan_path_in_result.ends_with("execution_plan.md"),
        "execution_plan_path should point at execution_plan.md; got {plan_path_in_result}"
    );
    let actions_path_in_result = result_json["actions_path"]
        .as_str()
        .expect("actions_path on result.json");
    assert!(
        actions_path_in_result.ends_with("actions.json"),
        "actions_path should point at actions.json; got {actions_path_in_result}"
    );
    assert_eq!(result_json["actions"]["total"], 2);
    assert_eq!(result_json["actions"]["pending_autonomous"], 1);
    assert_eq!(result_json["actions"]["pending_approval"], 0);
    assert_eq!(result_json["actions"]["blocked"], 1);
}
