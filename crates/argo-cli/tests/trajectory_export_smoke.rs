//! `argo trajectory export` end-to-end smoke. Stages a fake action
//! bundle in a temp workspace, then runs the binary with `--json
//! --output <file>` and checks the envelope + the export file.

use std::path::PathBuf;
use std::process::Command;

fn argo_bin() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("../../target/debug/argo");
    p
}

fn write_action_json(bundle: &std::path::Path, action_id: &str, state: &str) {
    std::fs::create_dir_all(bundle).unwrap();
    let body = serde_json::json!({
        "schema_key": "worker_action",
        "action_id": action_id,
        "job_id": "job-1",
        "action_type": "verify_price",
        "risk_level": "low",
        "requires_approval": false,
        "state": state,
        "payload": {"item_id": "skate-1"},
        "created_at": "2025-06-01T00:00:00Z",
    });
    std::fs::write(
        bundle.join("action.json"),
        serde_json::to_string_pretty(&body).unwrap(),
    )
    .unwrap();
}

#[test]
fn trajectory_export_json_envelope_lists_collected_rows() {
    let bin = argo_bin();
    if !bin.exists() {
        eprintln!("skipping: {} not built", bin.display());
        return;
    }
    let workspace = tempfile::tempdir().unwrap();
    let bundles_root = workspace.path().join(".skycot").join("outputs");
    write_action_json(
        &bundles_root.join("job-1").join("actions").join("a-1"),
        "a-1",
        "completed",
    );
    write_action_json(
        &bundles_root.join("job-1").join("actions").join("a-2"),
        "a-2",
        "failed",
    );
    let output_path = workspace.path().join("export.json");
    let out = Command::new(&bin)
        .args([
            "trajectory",
            "export",
            "--format",
            "anthropic",
            "--filter",
            "passed",
            "--output",
        ])
        .arg(&output_path)
        .arg("--json")
        .current_dir(workspace.path())
        .output()
        .expect("spawn argo");
    assert!(
        out.status.success(),
        "argo failed: stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let envelope: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("valid json envelope");
    assert_eq!(envelope["rows"].as_u64().unwrap(), 1, "filter=passed should drop the failed action");
    assert_eq!(envelope["format"].as_str().unwrap(), "anthropic");
    let export = std::fs::read_to_string(&output_path).expect("export written");
    let arr: serde_json::Value = serde_json::from_str(&export).expect("valid array");
    assert_eq!(arr.as_array().unwrap().len(), 1);
    assert_eq!(arr[0]["action_id"].as_str().unwrap(), "a-1");
}
