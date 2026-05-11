//! Integration smoke for mission
//! `mission-learned-plan-template-trust-ladder`.
//!
//! End-to-end: seed a learned-plan-template row, run
//! `argo plan template promote <id> --json`, then run
//! `argo skill list --json --name-contains learned:` and assert
//! a `learned:<id>` row exists. (`--learned` filter flag is
//! deferred to a follow-up brief — the namespace prefix is the
//! load-bearing convention and `--name-contains` exercises the
//! same code path the future flag would.)
use argo_store::learned_plan_template::{insert, NewLearnedPlanTemplate};
use argo_store::SessionStore;
use std::path::Path;
use std::process::{Command, Output};

fn argo_bin() -> &'static str {
    env!("CARGO_BIN_EXE_argo")
}

fn run(workspace: &Path, home: &Path, args: &[&str]) -> Output {
    Command::new(argo_bin())
        .current_dir(workspace)
        .env("ARGO_HOME", home)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("spawn argo {args:?}: {e}"))
}

fn assert_ok(out: &Output, label: &str) {
    assert!(
        out.status.success(),
        "{label} failed: stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
}

async fn seed_template(home: &Path) -> String {
    let store = SessionStore::open(home.join("sessions.db")).await.unwrap();
    store.migrate().await.unwrap();
    insert(
        store.pool(),
        &NewLearnedPlanTemplate {
            name: "marketing-launch-distilled",
            description: "Distilled from a cohort of 3 marketing-launch decompositions.",
            parent_goal: "promote our blog post launch",
            keywords_csv: "promote,launch,announce",
            yaml: "id: marketing-launch-distilled\ntasks:\n  - id: draft\n    worker: \"\"\n",
            cohort_size: 3,
        },
    )
    .await
    .expect("insert seed row")
}

#[tokio::test]
async fn promote_then_skill_list_shows_learned_namespace_row() {
    let workspace = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    assert_ok(
        &run(workspace.path(), home.path(), &["workspace", "init"]),
        "workspace init",
    );

    let id = seed_template(home.path()).await;

    // Pre-promote: skill list has no `learned:` rows.
    let pre = run(
        workspace.path(),
        home.path(),
        &["skill", "list", "--json", "--name-contains", "learned:"],
    );
    assert_ok(&pre, "pre-promote skill list");
    let pre_rows: Vec<serde_json::Value> = String::from_utf8_lossy(&pre.stdout)
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| serde_json::from_str(l).expect("ndjson row"))
        .collect();
    assert!(
        pre_rows.is_empty(),
        "expected no learned: rows pre-promote, got {pre_rows:?}"
    );

    // Promote — the new sink ingests a skill row.
    let promote_out = run(
        workspace.path(),
        home.path(),
        &["plan", "template", "promote", &id, "--json"],
    );
    assert_ok(&promote_out, "plan template promote --json");

    // Post-promote: a `learned:<id>` skill row is visible in the
    // catalog.
    let post = run(
        workspace.path(),
        home.path(),
        &["skill", "list", "--json", "--name-contains", "learned:"],
    );
    assert_ok(&post, "post-promote skill list");
    let post_rows: Vec<serde_json::Value> = String::from_utf8_lossy(&post.stdout)
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| serde_json::from_str(l).expect("ndjson row"))
        .collect();
    assert_eq!(post_rows.len(), 1, "exactly one learned skill expected");
    let expected_name = format!("learned:{id}");
    assert_eq!(post_rows[0]["name"], expected_name);
    assert_eq!(
        post_rows[0]["trust_grade"], "tenant",
        "default ingest grade should surface as `tenant` in the JSON envelope"
    );
}
