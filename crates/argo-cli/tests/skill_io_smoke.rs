mod common;

use common::argo_bin;
use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};

const BODY: &str = "First line.\n\nSecond line survives round trip.\n";

#[test]
fn skill_export_import_round_trip_and_rename() {
    let source_home = tempfile::tempdir().unwrap();
    let import_home = tempfile::tempdir().unwrap();
    let dry_home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();

    let create = argo(
        source_home.path(),
        work.path(),
        &[
            "skill",
            "create",
            "offline-skill",
            "--description",
            "Offline Skill",
            "--body",
            BODY,
        ],
    );
    assert!(create.status.success(), "{}", stderr(&create));

    let artifact = work.path().join("offline.skill.tar.gz");
    let export = argo(
        source_home.path(),
        work.path(),
        &[
            "skill",
            "export",
            "offline-skill",
            "--out",
            artifact.to_str().unwrap(),
            "--json",
        ],
    );
    assert!(export.status.success(), "{}", stderr(&export));
    let export_json = json_stdout(&export);
    assert_eq!(export_json["ok"], true);
    assert!(artifact.is_file());
    assert_eq!(
        artifact_entries(&artifact),
        vec!["body.md", "manifest.json", "sha256.txt"]
    );
    assert!(export_json["artifact"]["body_sha256"]
        .as_str()
        .unwrap()
        .starts_with("sha256-"));

    let dry_run = argo(
        dry_home.path(),
        work.path(),
        &[
            "skill",
            "import",
            artifact.to_str().unwrap(),
            "--dry-run",
            "--json",
        ],
    );
    assert!(dry_run.status.success(), "{}", stderr(&dry_run));
    let dry_json = json_stdout(&dry_run);
    assert_eq!(dry_json["dry_run"], true);
    let dry_count = argo(dry_home.path(), work.path(), &["skill", "count", "--json"]);
    assert_eq!(json_stdout(&dry_count)["count"], 0);

    let import = argo(
        import_home.path(),
        work.path(),
        &["skill", "import", artifact.to_str().unwrap(), "--json"],
    );
    assert!(import.status.success(), "{}", stderr(&import));
    let import_json = json_stdout(&import);
    assert_eq!(import_json["skill"]["name"], "offline-skill");

    let view = argo(
        import_home.path(),
        work.path(),
        &["skill", "view", "offline-skill", "--json"],
    );
    assert!(view.status.success(), "{}", stderr(&view));
    let viewed = json_stdout(&view);
    assert_eq!(viewed["description"], "Offline Skill");
    assert_eq!(viewed["body"], BODY);

    let duplicate = argo(
        import_home.path(),
        work.path(),
        &["skill", "import", artifact.to_str().unwrap()],
    );
    assert!(!duplicate.status.success());
    assert!(stderr(&duplicate).contains("already installed"));

    let renamed = argo(
        import_home.path(),
        work.path(),
        &[
            "skill",
            "import",
            artifact.to_str().unwrap(),
            "--name",
            "offline-copy",
            "--json",
        ],
    );
    assert!(renamed.status.success(), "{}", stderr(&renamed));
    assert_eq!(json_stdout(&renamed)["skill"]["name"], "offline-copy");

    let batch_dir = work.path().join("batch");
    let export_all = argo(
        import_home.path(),
        work.path(),
        &[
            "skill",
            "export-all",
            "--out",
            batch_dir.to_str().unwrap(),
            "--json",
        ],
    );
    assert!(export_all.status.success(), "{}", stderr(&export_all));
    let batch = json_stdout(&export_all);
    assert_eq!(batch["ok"], true);
    assert_eq!(batch["count"], 2);
}

fn argo(home: &Path, cwd: &Path, args: &[&str]) -> Output {
    Command::new(argo_bin())
        .env("ARGO_HOME", home)
        .env_remove("OPENAI_API_KEY")
        .env_remove("ANTHROPIC_API_KEY")
        .current_dir(cwd)
        .args(args)
        .output()
        .unwrap()
}

fn json_stdout(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap()
}

fn artifact_entries(path: &Path) -> Vec<String> {
    let file = std::fs::File::open(path).unwrap();
    let decoder = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);
    let mut entries: Vec<String> = archive
        .entries()
        .unwrap()
        .map(|entry| entry.unwrap().path().unwrap().to_string_lossy().to_string())
        .collect();
    entries.sort();
    entries
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
