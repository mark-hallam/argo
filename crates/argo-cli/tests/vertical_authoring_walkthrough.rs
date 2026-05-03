//! Locks the "Day-1" walkthrough in `verticals/AUTHORING.md`
//! against the running binary (#662). If a future edit changes
//! the CLI surface in a way that breaks the documented sequence,
//! this test fails and the doc must be updated in the same PR.
//!
//! The walkthrough is a tempdir-scoped, no-LLM sequence:
//!
//! 1. scaffold a minimal vertical (manifest + system prompt +
//!    one workflow + one eval fixture)
//! 2. `argo vertical list` sees it
//! 3. `argo vertical validate` passes
//! 4. `argo vertical eval --json` returns ok
//! 5. `argo vertical workflow … --json` returns ok with one
//!    `tool_call` step that read the manifest back
//! 6. `argo vertical install … --target …` copies it under a
//!    second target dir
//! 7. `argo vertical uninstall … --yes` removes it

mod common;

use common::argo_bin;
use std::path::Path;
use std::process::Command;

fn scaffold_hello_vertical(parent: &Path) {
    let v = parent.join("hello-vertical");
    std::fs::create_dir_all(v.join("prompts")).unwrap();
    std::fs::create_dir_all(v.join("workflows")).unwrap();
    std::fs::create_dir_all(v.join("eval")).unwrap();
    std::fs::write(
        v.join("vertical.toml"),
        "schema_version = \"1\"\n\
         [vertical]\nid = \"hello\"\nname = \"Hello, world vertical\"\nversion = \"0.1.0\"\n\
         [role]\nsystem_prompt = \"prompts/system.md\"\n\
         [tools]\nallowed = [\"file_read\"]\ndenied = [\"file_write\", \"file_patch\", \"terminal\", \"delegate_task\"]\n\
         [workflows]\nfiles = [\"workflows/say.workflow.toml\"]\n\
         [evaluation]\nsuites = [\"eval/main.jsonl\"]\n",
    )
    .unwrap();
    std::fs::write(
        v.join("prompts/system.md"),
        "You are a friendly assistant. Reply in one sentence.",
    )
    .unwrap();
    std::fs::write(
        v.join("workflows/say.workflow.toml"),
        "schema_version = \"1\"\n\
         [workflow]\nname = \"say\"\n\
         [[step]]\nname = \"read_self\"\nkind = \"tool_call\"\ntool = \"file_read\"\n\
         args = { path = \"hello-vertical/vertical.toml\" }\n",
    )
    .unwrap();
    std::fs::write(
        v.join("eval/main.jsonl"),
        "{\"id\":\"a\",\"input\":\"hello\",\"expected\":{\"intent\":\"greeting\"}}\n",
    )
    .unwrap();
}

#[test]
fn day1_walkthrough_runs_clean_against_the_binary() {
    let home = tempfile::tempdir().unwrap();
    let wd = tempfile::tempdir().unwrap();
    let installed = tempfile::tempdir().unwrap();
    scaffold_hello_vertical(wd.path());

    // 2. `argo vertical list` sees it (discovery dir = wd)
    let list_out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", wd.path())
        .current_dir(wd.path())
        .args(["vertical", "list", "--json"])
        .output()
        .expect("spawn list");
    assert!(
        list_out.status.success(),
        "vertical list stderr: {}",
        String::from_utf8_lossy(&list_out.stderr)
    );
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(list_out.stdout).unwrap()).unwrap();
    assert_eq!(v["ok"], true);
    assert!(v["loaded"]
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["id"] == "hello"));

    // 3. validate
    let validate_out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", wd.path())
        .current_dir(wd.path())
        .args(["vertical", "validate", "hello", "--json"])
        .output()
        .expect("spawn validate");
    assert!(
        validate_out.status.success(),
        "validate stderr: {}",
        String::from_utf8_lossy(&validate_out.stderr)
    );
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(validate_out.stdout).unwrap()).unwrap();
    assert_eq!(v["valid"], true);

    // 4. eval (structural)
    let eval_out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", wd.path())
        .current_dir(wd.path())
        .args(["vertical", "eval", "hello", "--json"])
        .output()
        .expect("spawn eval");
    assert!(
        eval_out.status.success(),
        "eval stderr: {}",
        String::from_utf8_lossy(&eval_out.stderr)
    );
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(eval_out.stdout).unwrap()).unwrap();
    assert_eq!(v["ok"], true);

    // 5. workflow run (tool_call should land green)
    let wf_out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", wd.path())
        .current_dir(wd.path())
        .args(["vertical", "workflow", "hello", "say", "--json"])
        .output()
        .expect("spawn workflow");
    assert!(
        wf_out.status.success(),
        "workflow stderr: {}",
        String::from_utf8_lossy(&wf_out.stderr)
    );
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(wf_out.stdout).unwrap()).unwrap();
    assert_eq!(v["ok"], true);
    let steps = v["steps"].as_array().unwrap();
    assert_eq!(steps.len(), 1);
    assert_eq!(steps[0]["status"], "ok");

    // 6. install into a second target
    let install_out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .current_dir(wd.path())
        .args([
            "vertical",
            "install",
            "hello-vertical",
            "--target",
            installed.path().to_str().unwrap(),
            "--json",
        ])
        .output()
        .expect("spawn install");
    assert!(
        install_out.status.success(),
        "install stderr: {}",
        String::from_utf8_lossy(&install_out.stderr)
    );
    assert!(installed.path().join("hello/vertical.toml").exists());

    // 7. uninstall
    let uninst_out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args([
            "vertical",
            "uninstall",
            "hello",
            "--target",
            installed.path().to_str().unwrap(),
            "--yes",
            "--json",
        ])
        .output()
        .expect("spawn uninstall");
    assert!(
        uninst_out.status.success(),
        "uninstall stderr: {}",
        String::from_utf8_lossy(&uninst_out.stderr)
    );
    assert!(!installed.path().join("hello").exists());
}

#[test]
fn authoring_md_exists_at_canonical_path() {
    // Lock the path the README references. If the doc moves,
    // update both.
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .to_path_buf();
    let p = repo_root.join("verticals/AUTHORING.md");
    assert!(p.exists(), "AUTHORING.md missing at {}", p.display());
    let body = std::fs::read_to_string(&p).unwrap();
    // Sanity: the doc must mention every command the
    // walkthrough exercises so a future edit that drops one
    // also fails this test.
    for cmd in [
        "argo vertical list",
        "argo vertical validate",
        "argo vertical eval",
        "argo vertical workflow",
        "argo vertical install",
        "argo vertical uninstall",
    ] {
        assert!(body.contains(cmd), "AUTHORING.md missing mention of `{cmd}`");
    }
}
