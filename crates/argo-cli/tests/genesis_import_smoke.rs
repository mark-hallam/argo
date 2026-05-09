//! End-to-end smoke for `argo genesis import`. Exercises the
//! built binary against the markdown + langchain fixtures and
//! asserts the scaffold lands on disk with a parseable
//! `vertical.toml`.

mod common;

use common::argo_bin;
use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};

fn fixture(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn import_markdown_fixture_writes_complete_vertical_scaffold() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let out = work.path().join("verticals");

    let result = argo(
        home.path(),
        work.path(),
        &[
            "genesis",
            "import",
            "--source",
            fixture("import_markdown_sample.md").to_str().unwrap(),
            "--name",
            "cara-support",
            "--output",
            out.to_str().unwrap(),
            "--json",
        ],
    );
    assert!(
        result.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&result.stderr)
    );

    let envelope: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(envelope["vertical_id"], "cara-support");
    assert_eq!(envelope["format"], "markdown");

    let vert_dir = out.join("cara-support");
    let manifest = std::fs::read_to_string(vert_dir.join("vertical.toml")).unwrap();
    let parsed: argo_vertical::VerticalManifest = toml::from_str(&manifest).unwrap();
    assert_eq!(parsed.vertical.id, "cara-support");

    let prompt = std::fs::read_to_string(vert_dir.join("system-prompt.md")).unwrap();
    assert!(prompt.contains("CARA"), "prompt: {prompt}");
    assert!(prompt.contains("acknowledge the issue"), "prompt: {prompt}");

    assert!(vert_dir.join("memory-schema.toml").is_file());
    assert!(vert_dir.join("eval-seed.toml").is_file());
}

#[test]
fn import_langchain_fixture_extracts_template_string() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let out = work.path().join("verticals");

    let result = argo(
        home.path(),
        work.path(),
        &[
            "genesis",
            "import",
            "--source",
            fixture("import_langchain_sample.py").to_str().unwrap(),
            "--name",
            "rex-recep",
            "--output",
            out.to_str().unwrap(),
            "--json",
        ],
    );
    assert!(
        result.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&result.stderr)
    );

    let envelope: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(envelope["format"], "langchain");

    let prompt =
        std::fs::read_to_string(out.join("rex-recep/system-prompt.md")).unwrap();
    assert!(prompt.contains("REX"), "prompt: {prompt}");
    assert!(prompt.contains("Question: {question}"), "prompt: {prompt}");
}

#[test]
fn import_refuses_existing_vertical_dir() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let out = work.path().join("verticals");

    let first = argo(
        home.path(),
        work.path(),
        &[
            "genesis",
            "import",
            "--source",
            fixture("import_markdown_sample.md").to_str().unwrap(),
            "--name",
            "cara-support",
            "--output",
            out.to_str().unwrap(),
        ],
    );
    assert!(first.status.success());

    let second = argo(
        home.path(),
        work.path(),
        &[
            "genesis",
            "import",
            "--source",
            fixture("import_markdown_sample.md").to_str().unwrap(),
            "--name",
            "cara-support",
            "--output",
            out.to_str().unwrap(),
        ],
    );
    assert!(!second.status.success());
    let stderr = String::from_utf8_lossy(&second.stderr);
    assert!(stderr.contains("already exists"), "stderr: {stderr}");
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
