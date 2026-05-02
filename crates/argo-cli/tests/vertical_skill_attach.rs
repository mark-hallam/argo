//! Integration tests for the skill auto-attach behavior added in
//! #653. When a session is created bound to a vertical, the
//! vertical's `[skills].bundle` directory should be walked, every
//! `*.skill.md` installed (or reused if already present), and
//! linked to the new session via `session_skill`.
//!
//! Hermetic via `ARGO_VERTICALS_DIR` + `ARGO_HOME` tempdirs.

mod common;

use common::{argo_bin, await_listening, body_of, http_post, pick_free_port};
use std::io::BufReader;
use std::path::Path;
use std::process::{Command, Stdio};

/// Minimal valid skill markdown — argo-skill's parser requires
/// `name`, `version`, `description`, and a `provenance` block.
fn skill_md(name: &str) -> String {
    format!(
        "---\n\
         name: {name}\n\
         version: \"0.1.0\"\n\
         description: \"test fixture skill — {name}\"\n\
         provenance:\n  source: vertical/test\n  hash: sha256-test\n\
         ---\n\
         # {name}\n\
         body content for the skill.\n"
    )
}

fn write_demo_vertical_with_skills(verticals_dir: &Path, skill_names: &[&str]) {
    let demo = verticals_dir.join("demo");
    let skills = demo.join("skills");
    std::fs::create_dir_all(&skills).unwrap();
    std::fs::write(
        demo.join("vertical.toml"),
        "schema_version = \"1\"\n\n\
         [vertical]\n\
         id = \"demo\"\n\
         name = \"Demo\"\n\
         version = \"0.1.0\"\n\n\
         [skills]\n\
         bundle = \"skills/\"\n",
    )
    .unwrap();
    for name in skill_names {
        std::fs::write(skills.join(format!("{name}.skill.md")), skill_md(name)).unwrap();
    }
}

fn write_vertical_no_skills(verticals_dir: &Path) {
    let demo = verticals_dir.join("nobundle");
    std::fs::create_dir_all(&demo).unwrap();
    std::fs::write(
        demo.join("vertical.toml"),
        "schema_version = \"1\"\n\n\
         [vertical]\n\
         id = \"nobundle\"\n\
         name = \"NoBundle\"\n\
         version = \"0.1.0\"\n",
    )
    .unwrap();
}

fn write_vertical_with_broken_skill(verticals_dir: &Path) {
    let demo = verticals_dir.join("partial");
    let skills = demo.join("skills");
    std::fs::create_dir_all(&skills).unwrap();
    std::fs::write(
        demo.join("vertical.toml"),
        "schema_version = \"1\"\n\n\
         [vertical]\n\
         id = \"partial\"\n\
         name = \"Partial\"\n\
         version = \"0.1.0\"\n\n\
         [skills]\n\
         bundle = \"skills/\"\n",
    )
    .unwrap();
    std::fs::write(skills.join("good.skill.md"), skill_md("good")).unwrap();
    // Broken: no frontmatter at all.
    std::fs::write(skills.join("bad.skill.md"), "no frontmatter here\n").unwrap();
}

// ---- CLI ----------------------------------------------------------

#[test]
fn cli_new_with_vertical_attaches_bundle_skills() {
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    write_demo_vertical_with_skills(verticals.path(), &["alpha_skill", "beta_skill"]);

    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .args([
            "new", "--provider", "anthropic", "--model", "claude-haiku",
            "--vertical", "demo",
        ])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        stdout.contains("skills:    2 attached from vertical bundle"),
        "expected attach line, got: {stdout}"
    );
}

#[test]
fn cli_new_with_vertical_no_skills_block_attaches_zero() {
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    write_vertical_no_skills(verticals.path());

    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .args([
            "new", "--provider", "p", "--model", "m",
            "--vertical", "nobundle",
        ])
        .output()
        .expect("spawn");
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    // No skills line printed when zero attached.
    assert!(
        !stdout.contains("skills:"),
        "should NOT print skills line: {stdout}"
    );
}

#[test]
fn cli_new_with_vertical_partial_bundle_still_creates_session() {
    // One good skill + one broken skill in the bundle. Per-skill
    // errors are non-fatal: session is created, the good skill
    // attaches, the bad one is logged + skipped.
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    write_vertical_with_broken_skill(verticals.path());

    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .args([
            "new", "--provider", "p", "--model", "m",
            "--vertical", "partial",
        ])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "session creation must succeed even with one broken skill: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        stdout.contains("skills:    1 attached"),
        "expected 1 skill attached, got: {stdout}"
    );
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.contains("bad.skill.md") && stderr.contains("failed to attach"),
        "expected per-skill error in stderr, got: {stderr}"
    );
}

#[test]
fn cli_new_unbound_session_attaches_nothing() {
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    write_demo_vertical_with_skills(verticals.path(), &["alpha_skill"]);

    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .args(["new", "--provider", "p", "--model", "m"])
        .output()
        .expect("spawn");
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(!stdout.contains("vertical:"));
    assert!(!stdout.contains("skills:"));
}

// ---- Gateway ------------------------------------------------------

fn spawn_gateway(
    port: u16,
    home: &Path,
    verticals_dir: &Path,
    max_requests: u32,
) -> (std::process::Child, BufReader<std::process::ChildStderr>) {
    let mut child = Command::new(argo_bin())
        .env("ARGO_HOME", home)
        .env("ARGO_VERTICALS_DIR", verticals_dir)
        .args([
            "gateway", "start", "--port", &port.to_string(),
            "--max-requests", &max_requests.to_string(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    let mut stderr = BufReader::new(child.stderr.take().unwrap());
    await_listening(&mut stderr);
    (child, stderr)
}

#[test]
fn gateway_post_vertical_session_response_carries_skills_attached() {
    let port = pick_free_port();
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    write_demo_vertical_with_skills(verticals.path(), &["one_skill", "two_skill"]);
    let (mut child, _stderr) = spawn_gateway(port, home.path(), verticals.path(), 1);

    let resp = http_post(
        port,
        "/verticals/demo/sessions",
        r#"{"model":"m","provider":"p","title":"gw"}"#,
    );
    assert!(resp.contains("HTTP/1.1 201 Created"), "resp={resp}");
    let v: serde_json::Value = serde_json::from_str(body_of(&resp)).unwrap();
    assert_eq!(v["skills_attached"], 2);
    assert_eq!(v["vertical_id"], "demo");
    let _ = child.wait();
}

#[test]
fn gateway_post_no_vertical_response_has_skills_attached_zero() {
    let port = pick_free_port();
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    let (mut child, _stderr) = spawn_gateway(port, home.path(), verticals.path(), 1);

    let resp = http_post(
        port,
        "/sessions",
        r#"{"model":"m","provider":"p"}"#,
    );
    assert!(resp.contains("HTTP/1.1 201 Created"), "resp={resp}");
    let v: serde_json::Value = serde_json::from_str(body_of(&resp)).unwrap();
    assert_eq!(v["skills_attached"], 0);
    let _ = child.wait();
}
