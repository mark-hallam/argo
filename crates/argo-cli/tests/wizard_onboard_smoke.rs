//! Integration smoke for `argo wizard onboard --non-interactive`
//! (Mission `mission-operator-onboarding-wizard`). Drives the
//! built binary against a temp workspace with a pre-populated
//! answers file and asserts the visible artefacts: an `argo.toml`
//! with an `[operator]` block, a credentials file with the API
//! key, and a scaffolded first-worker dir. `skip_key_test=true`
//! keeps the test offline.

mod common;

use common::argo_bin;
use std::process::{Command, Output};

#[test]
fn wizard_onboard_non_interactive_writes_workspace_artefacts() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();

    let vertical_dir = work.path().join("verticals/marketplace-seller-browser");
    std::fs::create_dir_all(&vertical_dir).unwrap();
    std::fs::write(
        vertical_dir.join("vertical.toml"),
        "[meta]\nid = \"marketplace-seller-browser\"\n",
    )
    .unwrap();

    let answers = work.path().join("answers.toml");
    let body = r#"
provider_api_key = "sk-ant-smoke"
operator_name = "Ada Lovelace"
operator_email = "ada@example.com"
vertical = "marketplace-seller-browser"
worker_name = "ada"
worker_persona = "first worker for the smoke test"
skip_key_test = true
"#;
    std::fs::write(&answers, body).unwrap();

    let out = run(
        home.path(),
        work.path(),
        &[
            "wizard",
            "onboard",
            "--non-interactive",
            "--answers",
            answers.to_str().unwrap(),
        ],
    );
    assert_success(&out);

    let argo_toml = std::fs::read_to_string(work.path().join("argo.toml")).unwrap();
    assert!(argo_toml.contains("[operator]"), "argo.toml: {argo_toml}");
    assert!(argo_toml.contains("ada@example.com"), "argo.toml: {argo_toml}");
    assert!(argo_toml.contains("Ada Lovelace"), "argo.toml: {argo_toml}");

    let worker_toml = std::fs::read_to_string(work.path().join("workers/ada/worker.toml")).unwrap();
    assert!(worker_toml.contains("name = \"ada\""), "worker: {worker_toml}");
    assert!(
        worker_toml.contains("marketplace-seller-browser"),
        "worker: {worker_toml}"
    );
    assert!(work.path().join(".argo/credentials.toml").exists());
}

#[test]
fn wizard_onboard_non_interactive_requires_answers_path() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let out = run(
        home.path(),
        work.path(),
        &["wizard", "onboard", "--non-interactive"],
    );
    assert!(
        !out.status.success(),
        "expected failure: stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("--answers"), "stderr: {stderr}");
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
