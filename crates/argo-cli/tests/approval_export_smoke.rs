mod common;

use chrono::{Duration, Utc};
use common::argo_bin;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
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
fn approval_export_writes_csv_and_refuses_overwrite() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    assert_success(&run(home.path(), work.path(), &["workspace", "init"]));
    let csv_path = work.path().join("approval-export.csv");
    seed_approval(
        home.path(),
        work.path(),
        "act-export",
        "finance-bot",
        "refund_customer",
        "refund, \"quoted\"\nline",
    );

    let out = run(
        home.path(),
        work.path(),
        &[
            "approval",
            "export",
            "--since",
            "30d",
            "--format",
            "csv",
            "--out",
            csv_path.to_str().unwrap(),
        ],
    );
    assert_success(&out);
    let bytes = fs::read(&csv_path).unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    let digest = hex::encode(Sha256::digest(&bytes));
    assert!(stderr.contains("wrote 1 rows"));
    assert!(stderr.contains(&format!("sha256={digest}")));
    let mut rdr = csv::Reader::from_reader(bytes.as_slice());
    let records: Vec<_> = rdr.records().collect::<Result<_, _>>().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].get(0), Some("act-export"));
    let excerpt = records[0].get(8).unwrap();
    assert!(excerpt.contains("refund,"));
    assert!(excerpt.contains("quoted"));
    assert!(excerpt.contains("\\nline"));

    let overwrite = run(
        home.path(),
        work.path(),
        &[
            "approval",
            "export",
            "--format",
            "csv",
            "--out",
            csv_path.to_str().unwrap(),
        ],
    );
    assert!(!overwrite.status.success());
    assert!(String::from_utf8_lossy(&overwrite.stderr).contains("refusing to overwrite"));
}

#[test]
fn approval_export_stdout_json_and_jsonl_are_valid() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    assert_success(&run(home.path(), work.path(), &["workspace", "init"]));
    seed_approval(
        home.path(),
        work.path(),
        "act-json",
        "finance-bot",
        "refund_customer",
        "plain",
    );
    let json = run(
        home.path(),
        work.path(),
        &["approval", "export", "--format", "json", "--out", "-"],
    );
    assert_success(&json);
    let rows: Value = serde_json::from_slice(&json.stdout).unwrap();
    assert_eq!(rows.as_array().unwrap().len(), 1);
    assert_eq!(rows[0]["id"], "act-json");
    assert!(String::from_utf8_lossy(&json.stderr).contains("sha256="));

    let jsonl = run(
        home.path(),
        work.path(),
        &["approval", "export", "--format", "jsonl", "--out", "-"],
    );
    assert_success(&jsonl);
    let lines: Vec<_> = String::from_utf8_lossy(&jsonl.stdout)
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .collect();
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0]["id"], "act-json");
}

#[test]
fn approval_export_help_pins_flags() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let out = run(home.path(), work.path(), &["approval", "export", "--help"]);
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
        "--out",
        "--force",
    ] {
        assert!(stdout.contains(flag), "missing {flag}\n{stdout}");
    }
}

fn seed_approval(home: &Path, work: &Path, id: &str, worker: &str, kind: &str, note: &str) {
    let requested_at = Utc::now() - Duration::days(1);
    append_action(
        home,
        work,
        ActionSeed {
            id,
            worker,
            kind,
            note,
            state: "pending",
            created_at: requested_at,
            decision: None,
        },
    );
    append_action(
        home,
        work,
        ActionSeed {
            id,
            worker,
            kind,
            note,
            state: "approved",
            created_at: requested_at + Duration::minutes(5),
            decision: Some(("approve", "operator")),
        },
    );
}

struct ActionSeed<'a> {
    id: &'a str,
    worker: &'a str,
    kind: &'a str,
    note: &'a str,
    state: &'a str,
    created_at: chrono::DateTime<Utc>,
    decision: Option<(&'a str, &'a str)>,
}

fn append_action(home: &Path, work: &Path, seed: ActionSeed<'_>) {
    let mut body = json!({
        "schema_key": "worker_action",
        "action_id": seed.id,
        "job_id": "job-approval-export",
        "worker_id": uuid::Uuid::nil(),
        "action_type": seed.kind,
        "risk_level": "high",
        "requires_approval": true,
        "state": seed.state,
        "payload": {"worker_id": seed.worker, "note": seed.note},
        "created_at": seed.created_at.to_rfc3339(),
    });
    if let Some((decision, by)) = seed.decision {
        body["approved_by"] = json!(by);
        body["approved_at"] = json!(seed.created_at.to_rfc3339());
        body["result"] = json!({ "decision": decision });
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

fn run(home: &Path, work: &Path, args: &[&str]) -> Output {
    let mut cmd = Command::new(argo_bin());
    cmd.current_dir(work)
        .env("ARGO_HOME", home)
        .env("ARGO_WORKSPACE_ROOT", work)
        .env("ARGO_VERTICALS_DIR", workspace_root().join("verticals"))
        .args(args);
    cmd.output().unwrap()
}

fn assert_success(out: &Output) {
    assert!(
        out.status.success(),
        "status={:?}\nstdout={}\nstderr={}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}
