mod common;

use common::argo_bin;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

#[test]
fn backup_restore_full_round_trip() {
    let home = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    let argo = workspace.path().join(".argo");
    fs::create_dir_all(argo.join("verticals/demo")).unwrap();
    fs::write(argo.join("sessions.db"), "session rows").unwrap();
    fs::write(argo.join("verticals/demo/vertical.toml"), "id = 'demo'").unwrap();
    fs::write(argo.join("credentials.toml"), "OPENAI_API_KEY='secret'").unwrap();

    let backup_path = workspace.path().join("backup.tar.gz");
    let created = run(
        home.path(),
        workspace.path(),
        &[
            "backup",
            "create",
            "--output",
            backup_path.to_str().unwrap(),
            "--json",
        ],
    );
    assert_success(&created);
    let created: Value = serde_json::from_slice(&created.stdout).unwrap();
    assert_eq!(created["ok"], true);
    assert!(created["backup_create"]["sha256"].as_str().unwrap().len() == 64);
    assert!(backup_path.is_file());

    let verified = run(
        home.path(),
        workspace.path(),
        &["backup", "verify", backup_path.to_str().unwrap(), "--json"],
    );
    assert_success(&verified);
    let verified: Value = serde_json::from_slice(&verified.stdout).unwrap();
    assert_eq!(verified["ok"], true);

    let dry_run_target = workspace.path().join("dry-run-target");
    let dry_run = run(
        home.path(),
        workspace.path(),
        &[
            "restore",
            backup_path.to_str().unwrap(),
            "--into",
            dry_run_target.to_str().unwrap(),
            "--dry-run",
            "--json",
        ],
    );
    assert_success(&dry_run);
    let dry_run: Value = serde_json::from_slice(&dry_run.stdout).unwrap();
    assert_eq!(dry_run["ok"], true);
    assert_eq!(dry_run["restore"]["dry_run"], true);
    assert!(!dry_run_target.exists());

    let restored = workspace.path().join("restored");
    let restored_out = run(
        home.path(),
        workspace.path(),
        &[
            "restore",
            backup_path.to_str().unwrap(),
            "--into",
            restored.to_str().unwrap(),
            "--json",
        ],
    );
    assert_success(&restored_out);
    assert_eq!(
        fs::read_to_string(restored.join(".argo/sessions.db")).unwrap(),
        "session rows"
    );
    assert!(!restored.join(".argo/credentials.toml").exists());

    let non_empty = workspace.path().join("non-empty");
    fs::create_dir_all(&non_empty).unwrap();
    fs::write(non_empty.join("marker"), "occupied").unwrap();
    let refused = run(
        home.path(),
        workspace.path(),
        &[
            "restore",
            backup_path.to_str().unwrap(),
            "--into",
            non_empty.to_str().unwrap(),
        ],
    );
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("not empty"));

    let with_creds_path = workspace.path().join("with-creds.tar.gz");
    assert_success(&run(
        home.path(),
        workspace.path(),
        &[
            "backup",
            "create",
            "--output",
            with_creds_path.to_str().unwrap(),
            "--include-credentials",
            "--json",
        ],
    ));
    let restored_with_creds = workspace.path().join("restored-with-creds");
    assert_success(&run(
        home.path(),
        workspace.path(),
        &[
            "restore",
            with_creds_path.to_str().unwrap(),
            "--into",
            restored_with_creds.to_str().unwrap(),
        ],
    ));
    assert_eq!(
        fs::read_to_string(restored_with_creds.join(".argo/credentials.toml")).unwrap(),
        "OPENAI_API_KEY='secret'"
    );

    assert_success(&run(
        home.path(),
        workspace.path(),
        &["backup", "create", "--json"],
    ));
    let listed = run(home.path(), workspace.path(), &["backup", "list", "--json"]);
    assert_success(&listed);
    let listed: Value = serde_json::from_slice(&listed.stdout).unwrap();
    assert_eq!(listed["ok"], true);
    assert_eq!(listed["backups"].as_array().unwrap().len(), 1);
}

fn run(home: &Path, cwd: &Path, args: &[&str]) -> Output {
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
