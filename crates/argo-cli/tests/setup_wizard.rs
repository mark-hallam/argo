mod common;

use common::argo_bin;
use serde_json::Value;
use std::process::{Command, Output};

#[test]
fn setup_wizard_non_interactive_json_has_seven_steps() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let out = run(home.path(), work.path(), &["setup", "--non-interactive", "--json"]);
    assert_success(&out);
    let value: Value = serde_json::from_slice(&out.stdout).unwrap();
    let steps = value["steps"].as_array().unwrap();
    assert_eq!(steps.len(), 7);
    assert!(value["ready"].as_bool().is_some());
    assert!(steps.iter().any(|step| step["name"] == "Audit query"));
}

#[test]
fn setup_wizard_table_prints_status_markers() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let out = run(home.path(), work.path(), &["setup", "--non-interactive"]);
    assert_success(&out);
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("argo setup wizard"));
    assert!(stdout.contains("OK ✓") || stdout.contains("FAIL ✗"));
    assert!(stdout.contains("Genesis archetype suggestions"));
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
