//! Integration tests for `argo handoff-rules` (Stream
//! `claude-handoff-routing-rules`). Exercises the
//! add/list/show/remove/test path end-to-end against a fresh
//! ARGO_HOME.

mod common;

use argo_store::SessionStore;
use argo_types::TenantId;
use common::argo_bin;
use serde_json::Value;
use std::process::{Command, Output};
use tempfile::TempDir;

struct World {
    home: TempDir,
    work: TempDir,
    tenant: TenantId,
}

#[tokio::test]
async fn add_list_show_test_remove_round_trip() {
    let world = seed_world().await;
    let tenant = world.tenant.to_string();
    let worker_id = uuid::Uuid::now_v7().to_string();
    let rule_path = world.work.path().join("rule.json");
    std::fs::write(
        &rule_path,
        format!(
            "{{\"name\": \"copy-review\", \
             \"when\": {{\"expected_outcome_contains\": \"copy review\"}}, \
             \"then\": {{\"specific_worker\": \"{worker_id}\"}}, \
             \"priority\": 5, \
             \"active\": true}}"
        ),
    )
    .unwrap();

    let added = run(
        &world,
        &[
            "handoff-rules",
            "add",
            "--tenant",
            &tenant,
            "--json",
            rule_path.to_str().unwrap(),
        ],
    );
    assert_success(&added);
    let added_payload: Value = serde_json::from_slice(&added.stdout).unwrap();
    assert_eq!(added_payload["name"], "copy-review");
    assert_eq!(added_payload["priority"], 5);

    let listed = run(
        &world,
        &["handoff-rules", "list", "--tenant", &tenant, "--json"],
    );
    assert_success(&listed);
    let rows: Value = serde_json::from_slice(&listed.stdout).unwrap();
    let names: Vec<&str> = rows
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"copy-review"));

    let shown = run(
        &world,
        &[
            "handoff-rules",
            "show",
            "--tenant",
            &tenant,
            "copy-review",
            "--json",
        ],
    );
    assert_success(&shown);
    let detail: Value = serde_json::from_slice(&shown.stdout).unwrap();
    assert_eq!(detail["name"], "copy-review");

    // Test path: rule should match an outcome containing "copy review".
    let tested = run(
        &world,
        &[
            "handoff-rules",
            "test",
            "--tenant",
            &tenant,
            "--expected-outcome",
            "please do a copy review for this draft",
            "--json",
        ],
    );
    assert_success(&tested);
    let test_payload: Value = serde_json::from_slice(&tested.stdout).unwrap();
    assert_eq!(test_payload["matched"], true);
    assert_eq!(test_payload["target_worker_id"], worker_id);

    // Outcome that doesn't match → no rule fires.
    let no_match = run(
        &world,
        &[
            "handoff-rules",
            "test",
            "--tenant",
            &tenant,
            "--expected-outcome",
            "ship the build",
            "--json",
        ],
    );
    assert_success(&no_match);
    let no_match_payload: Value = serde_json::from_slice(&no_match.stdout).unwrap();
    assert_eq!(no_match_payload["matched"], false);

    let removed = run(
        &world,
        &[
            "handoff-rules",
            "remove",
            "--tenant",
            &tenant,
            "copy-review",
            "--json",
        ],
    );
    assert_success(&removed);

    let listed_after = run(
        &world,
        &["handoff-rules", "list", "--tenant", &tenant, "--json"],
    );
    assert_success(&listed_after);
    let rows: Value = serde_json::from_slice(&listed_after.stdout).unwrap();
    assert!(rows.as_array().unwrap().is_empty());
}

async fn seed_world() -> World {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(work.path().join(".skycot")).unwrap();
    let store = SessionStore::open(home.path().join("sessions.db"))
        .await
        .unwrap();
    store.migrate().await.unwrap();
    let (tenant, _ws) = store
        .ensure_workspace("handoff-rules-cli", &work.path().to_string_lossy())
        .await
        .unwrap();
    World {
        home,
        work,
        tenant,
    }
}

fn run(world: &World, args: &[&str]) -> Output {
    Command::new(argo_bin())
        .env("ARGO_HOME", world.home.path())
        .current_dir(world.work.path())
        .args(args)
        .output()
        .unwrap()
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "status: {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
