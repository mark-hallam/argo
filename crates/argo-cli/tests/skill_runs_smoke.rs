mod common;

use chrono::{Duration, Utc};
use common::argo_bin;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .to_path_buf()
}

#[test]
fn skill_runs_cli_smoke() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    assert_success(&run(home.path(), work.path(), &["workspace", "init"]));

    let now = Utc::now();
    let append = |action_id: &str,
                  worker_id: &str,
                  tenant_id: &str,
                  skill: &str,
                  state: &str,
                  created_at| {
        append_action(
            home.path(),
            work.path(),
            ActionSeed {
                action_id,
                worker_id,
                tenant_id,
                skill,
                state,
                created_at,
            },
        )
    };
    for i in 0..12 {
        append(
            &format!("act-{i:02}"),
            "w-1",
            "tenant-a",
            "my-skill",
            "completed",
            now - Duration::minutes(i),
        );
    }
    append(
        "act-w2",
        "w-2",
        "tenant-a",
        "my-skill",
        "completed",
        now - Duration::minutes(1),
    );
    append(
        "act-old",
        "w-1",
        "tenant-a",
        "my-skill",
        "completed",
        now - Duration::hours(2),
    );
    append(
        "act-failed",
        "w-1",
        "tenant-a",
        "my-skill",
        "failed",
        now - Duration::minutes(2),
    );
    append(
        "act-skipped",
        "w-1",
        "tenant-a",
        "my-skill",
        "blocked",
        now - Duration::minutes(3),
    );
    append(
        "act-other-skill",
        "w-3",
        "tenant-a",
        "other-skill",
        "completed",
        now - Duration::minutes(1),
    );
    append(
        "act-other-tenant",
        "w-4",
        "tenant-b",
        "my-skill",
        "completed",
        now - Duration::minutes(1),
    );

    let limited = json_cmd(
        home.path(),
        work.path(),
        &["skill", "runs", "my-skill", "--limit", "10", "--json"],
    );
    let rows = limited["skill_runs"]["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 10);
    assert_eq!(rows[0]["action_id"], "act-00");
    assert_eq!(rows[0]["decision"], "success");

    let since = json_cmd(
        home.path(),
        work.path(),
        &[
            "skill", "runs", "my-skill", "--since", "1h", "--limit", "50", "--json",
        ],
    );
    let since_rows = since["skill_runs"]["rows"].as_array().unwrap();
    assert!(since_rows.iter().all(|row| row["action_id"] != "act-old"));

    let worker = json_cmd(
        home.path(),
        work.path(),
        &["skill", "runs", "my-skill", "--worker", "w-2", "--json"],
    );
    let worker_rows = worker["skill_runs"]["rows"].as_array().unwrap();
    assert_eq!(worker_rows.len(), 1);
    assert_eq!(worker_rows[0]["worker_id"], "w-2");

    let failed = json_cmd(
        home.path(),
        work.path(),
        &[
            "skill",
            "runs",
            "my-skill",
            "--include-failed",
            "--limit",
            "50",
            "--json",
        ],
    );
    let failed_rows = failed["skill_runs"]["rows"].as_array().unwrap();
    assert!(failed_rows.iter().any(|row| row["decision"] == "fail"));
    assert!(failed_rows.iter().any(|row| row["decision"] == "skipped"));

    let tenant = json_cmd(
        home.path(),
        work.path(),
        &[
            "skill", "runs", "--tenant", "tenant-a", "--limit", "50", "--json",
        ],
    );
    let tenant_rows = tenant["skill_runs"]["rows"].as_array().unwrap();
    assert!(tenant_rows.iter().all(|row| row["tenant_id"] == "tenant-a"));
    assert!(tenant_rows
        .iter()
        .any(|row| row["skill_name"] == "other-skill"));
    assert_eq!(tenant["ok"], true);
}

struct ActionSeed<'a> {
    action_id: &'a str,
    worker_id: &'a str,
    tenant_id: &'a str,
    skill: &'a str,
    state: &'a str,
    created_at: chrono::DateTime<Utc>,
}

fn append_action(home: &Path, work: &Path, action: ActionSeed<'_>) {
    let body = json!({
        "schema_key": "worker_action",
        "action_id": action.action_id,
        "job_id": "job-skill-runs",
        "action_type": "post_listing",
        "risk_level": "low",
        "requires_approval": false,
        "state": action.state,
        "payload": {
            "worker_id": action.worker_id,
            "tenant_id": action.tenant_id,
            "skill_used": action.skill,
            "latency_ms": 125
        },
        "created_at": action.created_at.to_rfc3339(),
    })
    .to_string();
    assert_success(&run(
        home,
        work,
        &[
            "memory",
            "append",
            "--vertical",
            "gtm-engineer",
            "memory",
            &body,
        ],
    ));
}

fn json_cmd(home: &Path, work: &Path, args: &[&str]) -> Value {
    let out = run(home, work, args);
    assert_success(&out);
    serde_json::from_slice(&out.stdout).unwrap()
}

fn run(home: &Path, cwd: &Path, args: &[&str]) -> Output {
    Command::new(argo_bin())
        .env("ARGO_HOME", home)
        .env("ARGO_VERTICALS_DIR", workspace_root().join("verticals"))
        .current_dir(cwd)
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
