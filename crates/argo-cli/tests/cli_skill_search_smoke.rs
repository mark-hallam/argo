mod common;

use common::argo_bin;
use serde_json::Value;
use std::process::{Command, Output};

#[test]
fn skill_search_filters_and_json_output_work() {
    let home = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    let output = run(
        home.path(),
        workspace.path(),
        &[
            "skill",
            "search",
            "listing",
            "--tenant",
            "acme",
            "--trust-min",
            "published-community",
            "--limit",
            "1",
            "--json",
        ],
    );
    assert_success(&output);
    let rows: Value = serde_json::from_slice(&output.stdout).unwrap();
    let rows = rows.as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["tenant"], "acme");
    assert_eq!(rows[0]["name"], "marketplace-listing-draft");
    assert_eq!(rows[0]["trust_grade"], "published_verified");
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
