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
fn action_history_timeline_recent_smoke() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    assert_success(&run(home.path(), work.path(), &["workspace", "init"]));

    let now = Utc::now();
    for i in 0..12 {
        append_action(
            home.path(),
            work.path(),
            &format!("act-{i:02}"),
            "w-1",
            "tenant-a",
            "pricing",
            now - Duration::minutes(i),
        );
    }
    append_action(
        home.path(),
        work.path(),
        "act-old",
        "w-1",
        "tenant-a",
        "pricing",
        now - Duration::hours(2),
    );
    append_action(
        home.path(),
        work.path(),
        "act-other-worker",
        "w-2",
        "tenant-a",
        "copywriting",
        now - Duration::minutes(1),
    );
    append_action(
        home.path(),
        work.path(),
        "act-other-tenant",
        "w-3",
        "tenant-b",
        "support",
        now - Duration::minutes(1),
    );

    let history = json_cmd(
        home.path(),
        work.path(),
        &["action", "history", "w-1", "--limit", "10", "--json"],
    );
    let rows = history["action_history"]["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 10);
    assert_eq!(rows[0]["action_id"], "act-00");
    assert_eq!(rows[0]["spend_usd_micros"], 1_250_000);

    let with_skill = json_cmd(
        home.path(),
        work.path(),
        &[
            "action",
            "history",
            "w-1",
            "--limit",
            "1",
            "--include-skill",
            "--json",
        ],
    );
    assert_eq!(
        with_skill["action_history"]["rows"][0]["skill_used"],
        "pricing"
    );

    let recent = json_cmd(
        home.path(),
        work.path(),
        &["action", "recent", "--tenant", "tenant-a", "--limit", "50", "--json"],
    );
    let recent_rows = recent["action_recent"]["rows"].as_array().unwrap();
    assert_eq!(recent_rows.len(), 14);
    assert!(recent_rows.iter().all(|row| row["tenant_id"] == "tenant-a"));

    let since = json_cmd(
        home.path(),
        work.path(),
        &["action", "history", "w-1", "--since", "1h", "--limit", "20", "--json"],
    );
    let since_rows = since["action_history"]["rows"].as_array().unwrap();
    assert_eq!(since_rows.len(), 12);
    assert!(since_rows.iter().all(|row| row["action_id"] != "act-old"));

    let timeline = json_cmd(
        home.path(),
        work.path(),
        &[
            "action",
            "timeline",
            "w-1",
            "--bucket-mins",
            "5",
            "--since",
            "24h",
            "--json",
        ],
    );
    let histogram = timeline["action_timeline"]["histogram"].as_str().unwrap();
    assert_eq!(timeline["action_timeline"]["bucket_count"], 288);
    assert_eq!(histogram.chars().count(), 288);
    assert!(histogram.contains('█'));
}

fn append_action(
    home: &Path,
    work: &Path,
    action_id: &str,
    worker_id: &str,
    tenant_id: &str,
    skill: &str,
    created_at: chrono::DateTime<Utc>,
) {
    let body = json!({
        "schema_key": "worker_action",
        "action_id": action_id,
        "job_id": "job-history",
        "action_type": "post_listing",
        "risk_level": "low",
        "requires_approval": false,
        "state": "completed",
        "payload": {
            "worker_id": worker_id,
            "tenant_id": tenant_id,
            "skill_used": skill,
            "spend_usd_micros": 1_250_000
        },
        "created_at": created_at.to_rfc3339(),
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
