//! Integration tests for `argo vertical install <source>` and
//! `argo vertical uninstall <id>` (#661).
//!
//! These boot the real `argo` binary against tempdirs so we
//! exercise the clap parsing, dispatch wiring, target-resolution
//! policy, and exit codes — everything that the unit tests skip
//! because they call into the handler functions directly.

mod common;

use common::argo_bin;
use std::path::Path;
use std::process::Command;

fn write_minimal_vertical(dir: &Path, id: &str) {
    std::fs::create_dir_all(dir.join("prompts")).unwrap();
    std::fs::write(
        dir.join("vertical.toml"),
        format!(
            "schema_version = \"1\"\n\
             [vertical]\nid = \"{id}\"\nname = \"X\"\nversion = \"0.1.0\"\n\
             [role]\nsystem_prompt = \"prompts/system.md\"\n"
        ),
    )
    .unwrap();
    std::fs::write(dir.join("prompts/system.md"), "you are X").unwrap();
}

#[test]
fn cli_install_copies_source_into_target_and_lists_it() {
    let home = tempfile::tempdir().unwrap();
    let src = tempfile::tempdir().unwrap();
    let target = tempfile::tempdir().unwrap();
    write_minimal_vertical(src.path(), "demo");

    // install
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args([
            "vertical",
            "install",
            src.path().to_str().unwrap(),
            "--target",
            target.path().to_str().unwrap(),
            "--json",
        ])
        .output()
        .expect("spawn install");
    assert!(
        out.status.success(),
        "install stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
    assert_eq!(v["ok"], true);
    assert_eq!(v["id"], "demo");
    assert_eq!(v["version"], "0.1.0");

    // The discovery dir now contains demo
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", target.path())
        .args(["vertical", "list", "--json"])
        .output()
        .expect("spawn list");
    assert!(out.status.success());
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
    let loaded = v["loaded"].as_array().unwrap();
    assert!(loaded.iter().any(|e| e["id"] == "demo"), "got: {loaded:?}");
}

#[test]
fn cli_install_uses_argo_verticals_dir_when_target_omitted() {
    // When --target is not passed, install resolves the discovery
    // dir via `argo_vertical::default_dir()`, which honours
    // `$ARGO_VERTICALS_DIR`. This test locks that contract.
    let home = tempfile::tempdir().unwrap();
    let src = tempfile::tempdir().unwrap();
    let discovery = tempfile::tempdir().unwrap();
    write_minimal_vertical(src.path(), "implicit");

    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", discovery.path())
        .args(["vertical", "install", src.path().to_str().unwrap(), "--json"])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(discovery.path().join("implicit/vertical.toml").exists());
}

#[test]
fn cli_install_refuses_to_overwrite_without_force_and_succeeds_with_it() {
    let home = tempfile::tempdir().unwrap();
    let src = tempfile::tempdir().unwrap();
    let target = tempfile::tempdir().unwrap();
    write_minimal_vertical(src.path(), "demo");

    let first = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args([
            "vertical",
            "install",
            src.path().to_str().unwrap(),
            "--target",
            target.path().to_str().unwrap(),
        ])
        .output()
        .expect("spawn");
    assert!(first.status.success());

    let second = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args([
            "vertical",
            "install",
            src.path().to_str().unwrap(),
            "--target",
            target.path().to_str().unwrap(),
        ])
        .output()
        .expect("spawn");
    assert!(!second.status.success(), "second install must fail");
    let stderr = String::from_utf8(second.stderr).unwrap();
    assert!(stderr.contains("already installed"), "got: {stderr}");

    // Add a marker, install with --force, confirm marker survives
    // (=> source was copied) AND old contents were removed.
    std::fs::write(src.path().join("V2_MARK"), "v2").unwrap();
    let third = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args([
            "vertical",
            "install",
            src.path().to_str().unwrap(),
            "--target",
            target.path().to_str().unwrap(),
            "--force",
        ])
        .output()
        .expect("spawn");
    assert!(
        third.status.success(),
        "force install stderr: {}",
        String::from_utf8_lossy(&third.stderr)
    );
    assert!(target.path().join("demo/V2_MARK").exists());
}

#[test]
fn cli_install_then_uninstall_round_trip() {
    let home = tempfile::tempdir().unwrap();
    let src = tempfile::tempdir().unwrap();
    let target = tempfile::tempdir().unwrap();
    write_minimal_vertical(src.path(), "demo");

    Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args([
            "vertical",
            "install",
            src.path().to_str().unwrap(),
            "--target",
            target.path().to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(target.path().join("demo").exists());

    // uninstall without --yes refuses (non-zero exit, dir stays)
    let dry = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args([
            "vertical",
            "uninstall",
            "demo",
            "--target",
            target.path().to_str().unwrap(),
        ])
        .output()
        .expect("spawn");
    assert!(!dry.status.success());
    assert!(target.path().join("demo").exists());

    // with --yes the dir is removed
    let real = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args([
            "vertical",
            "uninstall",
            "demo",
            "--target",
            target.path().to_str().unwrap(),
            "--yes",
            "--json",
        ])
        .output()
        .expect("spawn");
    assert!(
        real.status.success(),
        "uninstall stderr: {}",
        String::from_utf8_lossy(&real.stderr)
    );
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(real.stdout).unwrap()).unwrap();
    assert_eq!(v["ok"], true);
    assert_eq!(v["id"], "demo");
    assert!(!target.path().join("demo").exists());
}

#[test]
fn cli_install_then_workflow_run_round_trip() {
    // End-to-end: install a vertical that has a tool_call
    // workflow, then run it. Locks the install path against the
    // workflow runner's expected layout.
    let home = tempfile::tempdir().unwrap();
    let src = tempfile::tempdir().unwrap();
    let target = tempfile::tempdir().unwrap();
    let wd = tempfile::tempdir().unwrap();
    std::fs::write(wd.path().join("greet.txt"), "hi from install round-trip").unwrap();

    // Source vertical with a one-step workflow
    std::fs::create_dir_all(src.path().join("workflows")).unwrap();
    std::fs::write(
        src.path().join("vertical.toml"),
        "schema_version = \"1\"\n\
         [vertical]\nid = \"greeter\"\nname = \"G\"\nversion = \"0.1.0\"\n\
         [tools]\nallowed = [\"file_read\"]\ndenied = []\n\
         [workflows]\nfiles = [\"workflows/say.workflow.toml\"]\n",
    )
    .unwrap();
    std::fs::write(
        src.path().join("workflows/say.workflow.toml"),
        "schema_version = \"1\"\n\
         [workflow]\nname = \"go\"\n\
         [[step]]\nname = \"r\"\nkind = \"tool_call\"\ntool = \"file_read\"\n\
         args = { path = \"greet.txt\" }\n",
    )
    .unwrap();

    Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args([
            "vertical",
            "install",
            src.path().to_str().unwrap(),
            "--target",
            target.path().to_str().unwrap(),
        ])
        .status()
        .unwrap();

    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", target.path())
        .current_dir(wd.path())
        .args(["vertical", "workflow", "greeter", "go", "--json"])
        .output()
        .expect("spawn workflow");
    assert!(
        out.status.success(),
        "workflow stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
    assert_eq!(v["ok"], true);
    let steps = v["steps"].as_array().unwrap();
    assert_eq!(steps[0]["status"], "ok");
    assert!(steps[0]["message"]
        .as_str()
        .unwrap()
        .contains("hi from install round-trip"));
}
