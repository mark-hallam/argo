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
fn approval_history_filters_mixed_decisions() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    assert_success(&run(home.path(), work.path(), &["workspace", "init"]));

    let now = Utc::now();
    seed_approval(
        home.path(),
        work.path(),
        "act-approve",
        "payments-bot",
        "refund_customer",
        now - Duration::days(1),
        Some(("approved", "approve", "op-a")),
    );
    seed_approval(
        home.path(),
        work.path(),
        "act-deny",
        "payments-bot",
        "refund_customer",
        now - Duration::days(1),
        Some(("blocked", "reject", "op-b")),
    );
    seed_approval(
        home.path(),
        work.path(),
        "act-timeout",
        "payments-bot",
        "refund_customer",
        now - Duration::hours(2),
        Some(("blocked", "timeout", "system")),
    );
    seed_approval(
        home.path(),
        work.path(),
        "act-other-worker",
        "support-bot",
        "refund_customer",
        now - Duration::hours(1),
        Some(("blocked", "reject", "op-c")),
    );
    seed_approval(
        home.path(),
        work.path(),
        "act-old",
        "payments-bot",
        "refund_customer",
        now - Duration::days(10),
        Some(("blocked", "reject", "op-d")),
    );

    let out = run(
        home.path(),
        work.path(),
        &[
            "approval",
            "history",
            "--since",
            "7d",
            "--decision",
            "deny",
            "--decision",
            "timeout",
            "--worker",
            "payments-bot",
            "--action-kind",
            "refund_customer",
            "--format",
            "json",
        ],
    );
    assert_success(&out);
    let rows: Vec<Value> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().any(|row| row["decision"] == "deny"));
    assert!(rows.iter().any(|row| row["decision"] == "timeout"));
}

#[test]
fn approval_history_help_pins_flags() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let out = run(home.path(), work.path(), &["approval", "history", "--help"]);
    assert_success(&out);
    let stdout = String::from_utf8_lossy(&out.stdout);
    for flag in [
        "--since",
        "--until",
        "--decision",
        "--worker",
        "--action-kind",
        "--limit",
        "--format",
    ] {
        assert!(stdout.contains(flag), "missing {flag}\n{stdout}");
    }
}

#[test]
fn approval_history_markdown_emits_table() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    assert_success(&run(home.path(), work.path(), &["workspace", "init"]));
    let now = Utc::now();
    seed_approval(
        home.path(),
        work.path(),
        "act-md",
        "payments-bot",
        "refund_customer",
        now - Duration::hours(3),
        Some(("approved", "approve", "op-md")),
    );
    let out = run(
        home.path(),
        work.path(),
        &[
            "approval",
            "history",
            "--since",
            "7d",
            "--format",
            "markdown",
        ],
    );
    assert_success(&out);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.starts_with("| id |"), "expected markdown header: {stdout}");
    assert!(stdout.contains("| --- |"));
    assert!(stdout.contains("act-md"));
}

fn seed_approval(
    home: &Path,
    work: &Path,
    id: &str,
    worker: &str,
    kind: &str,
    requested_at: chrono::DateTime<Utc>,
    decision: Option<(&str, &str, &str)>,
) {
    append_action(
        home,
        work,
        ActionSeed {
            id,
            worker,
            kind,
            state: "pending",
            created_at: requested_at,
            decision: None,
        },
    );
    if let Some((state, decision, by)) = decision {
        append_action(
            home,
            work,
            ActionSeed {
                id,
                worker,
                kind,
                state,
                created_at: requested_at + Duration::minutes(5),
                decision: Some((decision, by)),
            },
        );
    }
}

struct ActionSeed<'a> {
    id: &'a str,
    worker: &'a str,
    kind: &'a str,
    state: &'a str,
    created_at: chrono::DateTime<Utc>,
    decision: Option<(&'a str, &'a str)>,
}

fn append_action(home: &Path, work: &Path, seed: ActionSeed<'_>) {
    let mut body = json!({
        "schema_key": "worker_action",
        "action_id": seed.id,
        "job_id": "job-approval-history",
        "worker_id": uuid::Uuid::nil(),
        "action_type": seed.kind,
        "risk_level": "high",
        "requires_approval": true,
        "state": seed.state,
        "payload": {"worker_id": seed.worker},
        "created_at": seed.created_at.to_rfc3339(),
    });
    if let Some((decision, by)) = seed.decision {
        body["approved_by"] = json!(by);
        body["approved_at"] = json!(seed.created_at.to_rfc3339());
        body["result"] = json!({"decision": decision});
    }
    assert_success(&run(
        home,
        work,
        &[
            "memory",
            "append",
            "--vertical",
            "gtm-engineer",
            "memory",
            &body.to_string(),
        ],
    ));
}

fn run(home: &Path, cwd: &Path, args: &[&str]) -> Output {
    Command::new(argo_bin())
        .env("ARGO_HOME", home)
        .env("ARGO_WORKSPACE_ROOT", cwd)
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
