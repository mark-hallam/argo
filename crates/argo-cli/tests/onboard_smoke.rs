mod common;

use common::argo_bin;
use serde_json::Value;
use std::process::{Command, Output};

#[test]
fn onboard_non_interactive_json_returns_seven_stages() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let out = run(
        home.path(),
        work.path(),
        &["onboard", "--non-interactive", "--json"],
    );
    assert_success(&out);
    let value: Value = serde_json::from_slice(&out.stdout).unwrap();
    let stages = value["stages"].as_array().unwrap();
    assert_eq!(stages.len(), 7);
    assert_eq!(value["count"].as_u64().unwrap(), 7);
    assert!(!value["any_fail"].as_bool().unwrap());
    assert_eq!(stages[0]["name"].as_str().unwrap(), "Welcome");
    assert_eq!(stages[6]["name"].as_str().unwrap(), "Done");
}

#[test]
fn onboard_text_prints_progress_summary() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let out = run(home.path(), work.path(), &["onboard", "--non-interactive"]);
    assert_success(&out);
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("argo onboard"));
    assert!(stdout.contains("Welcome"));
    assert!(stdout.contains("Workspace"));
    assert!(stdout.contains("Done"));
}

#[test]
fn onboard_with_config_drives_credentials_and_charter_stages() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let toml_path = work.path().join("onboarding.toml");
    let body = r#"
[credentials]
ANTHROPIC_API_KEY = "sk-test-onboard"

[charter]
text = "be my marketing intern, run weekly campaigns to cold leads"
persona = "maya"
output = "verticals/maya"
"#;
    std::fs::write(&toml_path, body).unwrap();
    let out = run(
        home.path(),
        work.path(),
        &[
            "onboard",
            "--non-interactive",
            "--json",
            "--config",
            toml_path.to_str().unwrap(),
        ],
    );
    assert_success(&out);
    let value: Value = serde_json::from_slice(&out.stdout).unwrap();
    let stages = value["stages"].as_array().unwrap();
    let creds = stages
        .iter()
        .find(|s| s["name"].as_str() == Some("Credentials"))
        .unwrap();
    assert_eq!(creds["status"].as_str().unwrap(), "ok");
    let worker = stages
        .iter()
        .find(|s| s["name"].as_str() == Some("First worker"))
        .unwrap();
    assert_eq!(worker["status"].as_str().unwrap(), "ok", "stage: {worker:?}");
    assert!(work.path().join(".argo/credentials.toml").exists());
    assert!(work.path().join("verticals/maya/vertical.toml").exists());
}

fn run(home: &std::path::Path, cwd: &std::path::Path, args: &[&str]) -> Output {
    Command::new(argo_bin())
        .env("ARGO_HOME", home)
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
