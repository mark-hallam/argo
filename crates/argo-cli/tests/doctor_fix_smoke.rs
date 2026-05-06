mod common;

use common::argo_bin;
use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};

#[test]
fn doctor_fix_safe_json_and_unsafe_exit_code() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();

    let safe = run(home.path(), work.path(), &["doctor", "fix", "--json"]);
    assert_success(&safe);
    let body: Value = serde_json::from_slice(&safe.stdout).unwrap();
    assert_eq!(body["ok"], true);
    assert_eq!(body["fixes"].as_array().unwrap().len(), 4);
    assert!(home.path().join("sessions.db").is_file());
    assert!(work.path().join(".argo/manifests").is_dir());
    assert!(work.path().join(".skycot/auto_approval.toml").is_file());

    let unsafe_out = run(
        home.path(),
        work.path(),
        &["doctor", "fix", "--issue", "database_corruption", "--json"],
    );
    assert!(!unsafe_out.status.success());
    let body: Value = serde_json::from_slice(&unsafe_out.stdout).unwrap();
    assert_eq!(body["ok"], false);
    assert_eq!(body["fixes"][0]["status"], "unsafe");
    assert!(body["fixes"][0]["remediation"]
        .as_str()
        .unwrap()
        .contains("restore"));
}

fn run(home: &Path, cwd: &Path, args: &[&str]) -> Output {
    Command::new(argo_bin())
        .env("ARGO_HOME", home)
        .env_remove("OPENAI_API_KEY")
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("AWS_ACCESS_KEY_ID")
        .env_remove("ELEVENLABS_API_KEY")
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
