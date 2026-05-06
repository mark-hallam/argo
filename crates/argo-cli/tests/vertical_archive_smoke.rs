mod common;

use common::argo_bin;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

const VERTICAL_TOML: &str = r#"schema_version = "1"

[vertical]
id = "{id}"
name = "Demo {id}"
version = "0.1.0"
"#;

#[test]
fn vertical_archive_commands_work_end_to_end() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let verticals = work.path().join("verticals");
    write_vertical(&verticals, "alpha");
    write_vertical(&verticals, "beta");

    let archived = run(
        home.path(),
        work.path(),
        &verticals,
        &["vertical", "archive", "alpha", "--json"],
    );
    assert_success(&archived);
    let archived: Value = serde_json::from_slice(&archived.stdout).unwrap();
    assert_eq!(archived["ok"], true);
    assert_eq!(archived["vertical_archive"]["id"], "alpha");
    assert!(archived["vertical_archive"]["archived_at"].is_i64());

    let visible = run(
        home.path(),
        work.path(),
        &verticals,
        &["vertical", "list", "--source", "workspace", "--json"],
    );
    assert_success(&visible);
    let visible: Value = serde_json::from_slice(&visible.stdout).unwrap();
    let visible_ids = ids(&visible);
    assert_eq!(visible_ids, vec!["beta"]);

    let with_archived = run(
        home.path(),
        work.path(),
        &verticals,
        &[
            "vertical",
            "list",
            "--source",
            "workspace",
            "--include-archived",
            "--json",
        ],
    );
    assert_success(&with_archived);
    let with_archived: Value = serde_json::from_slice(&with_archived.stdout).unwrap();
    assert_eq!(ids(&with_archived), vec!["alpha", "beta"]);
    let alpha = with_archived["loaded"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == "alpha")
        .unwrap();
    assert!(alpha["archived_at"].is_i64());

    let unarchived = run(
        home.path(),
        work.path(),
        &verticals,
        &["vertical", "unarchive", "alpha", "--json"],
    );
    assert_success(&unarchived);
    let unarchived: Value = serde_json::from_slice(&unarchived.stdout).unwrap();
    assert_eq!(unarchived["vertical_unarchive"]["archived_at"], Value::Null);

    let active_delete = run(
        home.path(),
        work.path(),
        &verticals,
        &["vertical", "delete", "beta", "--force", "--json"],
    );
    assert!(!active_delete.status.success());
    assert!(String::from_utf8_lossy(&active_delete.stderr).contains("archive vertical before"));

    assert_success(&run(
        home.path(),
        work.path(),
        &verticals,
        &["vertical", "archive", "alpha"],
    ));
    let deleted = run(
        home.path(),
        work.path(),
        &verticals,
        &["vertical", "delete", "alpha", "--force", "--json"],
    );
    assert_success(&deleted);
    let deleted: Value = serde_json::from_slice(&deleted.stdout).unwrap();
    assert_eq!(deleted["ok"], true);
    assert!(!verticals.join("alpha").exists());
}

fn write_vertical(verticals: &Path, id: &str) {
    let root = verticals.join(id);
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("vertical.toml"),
        VERTICAL_TOML.replace("{id}", id),
    )
    .unwrap();
}

fn run(home: &Path, cwd: &Path, verticals: &Path, args: &[&str]) -> Output {
    Command::new(argo_bin())
        .env("ARGO_HOME", home)
        .env("ARGO_VERTICALS_DIR", verticals)
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

fn ids(body: &Value) -> Vec<&str> {
    body["loaded"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["id"].as_str().unwrap())
        .collect()
}
