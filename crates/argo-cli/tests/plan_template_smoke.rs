//! End-to-end smoke for the Dreams-distilled learned-plan-template
//! flow (Stream `claude-dreams-plan-templates`).
//!
//! Drives: seed a row directly into the SQLite store, then exercise
//! the full CLI surface — `argo plan template list/show/promote` —
//! and check the JSON envelope shape.
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
async fn list_show_promote_round_trip() {
    let workspace = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    assert_ok(
        &run(workspace.path(), home.path(), &["workspace", "init"]),
        "workspace init",
    );

    let id = seed_template(home.path()).await;

    // list (no filter) returns the seeded row
    let list_out = run(
        workspace.path(),
        home.path(),
        &["plan", "template", "list", "--json"],
    );
    assert_ok(&list_out, "plan template list --json");
    let env: serde_json::Value = serde_json::from_slice(&list_out.stdout).unwrap();
    assert_eq!(env["ok"], true);
    assert_eq!(env["count"], 1);
    let rows = env["rows"].as_array().unwrap();
    assert_eq!(rows[0]["id"], id);
    assert_eq!(rows[0]["trust_grade"], "pending");

    // list --grade promoted returns nothing yet
    let promoted_pre = run(
        workspace.path(),
        home.path(),
        &["plan", "template", "list", "--grade", "promoted", "--json"],
    );
    assert_ok(&promoted_pre, "plan template list --grade promoted (pre)");
    let env_pre: serde_json::Value = serde_json::from_slice(&promoted_pre.stdout).unwrap();
    assert_eq!(env_pre["count"], 0);

    // show fetches row + yaml
    let show_out = run(
        workspace.path(),
        home.path(),
        &["plan", "template", "show", &id, "--json"],
    );
    assert_ok(&show_out, "plan template show --json");
    let env_show: serde_json::Value = serde_json::from_slice(&show_out.stdout).unwrap();
    assert_eq!(env_show["ok"], true);
    assert_eq!(env_show["row"]["id"], id);
    assert_eq!(env_show["row"]["name"], "marketing-launch-distilled");
    assert_eq!(env_show["row"]["trust_grade"], "pending");
    assert!(
        env_show["row"]["yaml"]
            .as_str()
            .unwrap()
            .contains("marketing-launch-distilled"),
        "yaml body missing seed marker"
    );

    // promote flips the trust_grade
    let promote_out = run(
        workspace.path(),
        home.path(),
        &["plan", "template", "promote", &id, "--json"],
    );
    assert_ok(&promote_out, "plan template promote --json");
    let env_promote: serde_json::Value = serde_json::from_slice(&promote_out.stdout).unwrap();
    assert_eq!(env_promote["ok"], true);
    assert_eq!(env_promote["id"], id);
    assert_eq!(env_promote["row"]["trust_grade"], "promoted");
    assert!(
        env_promote["row"]["promoted_at"].is_number(),
        "promoted_at should be set after promote"
    );

    // list --grade promoted now returns the row
    let promoted_post = run(
        workspace.path(),
        home.path(),
        &["plan", "template", "list", "--grade", "promoted", "--json"],
    );
    assert_ok(&promoted_post, "plan template list --grade promoted (post)");
    let env_post: serde_json::Value = serde_json::from_slice(&promoted_post.stdout).unwrap();
    assert_eq!(env_post["count"], 1);
    assert_eq!(env_post["rows"][0]["id"], id);

    // promoting again is idempotent
    let promote_again = run(
        workspace.path(),
        home.path(),
        &["plan", "template", "promote", &id, "--json"],
    );
    assert_ok(&promote_again, "plan template promote (idempotent)");
    let env_again: serde_json::Value = serde_json::from_slice(&promote_again.stdout).unwrap();
    assert_eq!(env_again["already_promoted"], true);
}

#[test]
fn show_unknown_id_errors_cleanly() {
    let workspace = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    assert_ok(
        &run(workspace.path(), home.path(), &["workspace", "init"]),
        "workspace init",
    );
    let bad = run(
        workspace.path(),
        home.path(),
        &["plan", "template", "show", "no-such-id", "--json"],
    );
    assert!(
        !bad.status.success(),
        "expected non-zero on missing template"
    );
    let stderr = String::from_utf8_lossy(&bad.stderr);
    assert!(
        stderr.contains("no learned plan template"),
        "expected helpful error, got: {stderr}",
    );
}

#[test]
fn list_with_unknown_grade_errors() {
    let workspace = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    assert_ok(
        &run(workspace.path(), home.path(), &["workspace", "init"]),
        "workspace init",
    );
    let bad = run(
        workspace.path(),
        home.path(),
        &["plan", "template", "list", "--grade", "garbage"],
    );
    assert!(!bad.status.success(), "expected non-zero on bad --grade");
    let stderr = String::from_utf8_lossy(&bad.stderr);
    assert!(
        stderr.contains("unknown --grade"),
        "expected helpful error, got: {stderr}",
    );
}
