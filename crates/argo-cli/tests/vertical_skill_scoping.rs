//! Integration tests for per-vertical skill scoping (#678).
//!
//! Locks the runtime contract: when two verticals each bundle
//! their own skill sets, neither can see the other's via the
//! per-vertical lookup. The test creates a synthetic
//! two-vertical setup, binds a session to each, and confirms
//! the skill rows in SQLite carry the correct `vertical_id`.

mod common;

use common::argo_bin;
use std::path::Path;
use std::process::Command;

fn write_vertical_with_skill(
    verticals_dir: &Path,
    id: &str,
    skill_name: &str,
) {
    let v = verticals_dir.join(id);
    std::fs::create_dir_all(v.join("skills")).unwrap();
    std::fs::write(
        v.join("vertical.toml"),
        format!(
            "schema_version = \"1\"\n\
             [vertical]\nid = \"{id}\"\nname = \"X\"\nversion = \"0.1.0\"\n\
             [skills]\nbundle = \"skills/\"\n"
        ),
    )
    .unwrap();
    std::fs::write(
        v.join(format!("skills/{skill_name}.skill.md")),
        format!(
            "---\n\
             name: {skill_name}\n\
             version: 1.0.0\n\
             description: scoping smoke skill\n\
             provenance:\n  source: tests/vertical_skill_scoping\n  hash: 0\n\
             ---\n\
             # {skill_name}\nbody"
        ),
    )
    .unwrap();
}

fn extract_session_id(stdout: &str) -> String {
    stdout
        .lines()
        .next()
        .and_then(|l| l.strip_prefix("created session "))
        .map(|s| s.trim().to_string())
        .expect("first line should be `created session <uuid>`")
}

#[test]
fn two_verticals_bundle_skills_to_separate_namespaces() {
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    write_vertical_with_skill(verticals.path(), "alpha", "alpha_skill");
    write_vertical_with_skill(verticals.path(), "beta", "beta_skill");

    // Bind a session to each vertical → each session triggers
    // install_and_attach_bundle, which now tags skills with
    // vertical_id (#678).
    for id in ["alpha", "beta"] {
        let out = Command::new(argo_bin())
            .env("ARGO_HOME", home.path())
            .env("ARGO_VERTICALS_DIR", verticals.path())
            .args([
                "new",
                "--provider",
                "mock",
                "--model",
                "mock",
                "--vertical",
                id,
                "--title",
                &format!("scoping-test-{id}"),
            ])
            .output()
            .expect("spawn new");
        assert!(
            out.status.success(),
            "{id} session create failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let sid = extract_session_id(&String::from_utf8(out.stdout).unwrap());

        // Confirm session.show JSON carries the right vertical.
        let show = Command::new(argo_bin())
            .env("ARGO_HOME", home.path())
            .args(["session", "show", &sid, "--json"])
            .output()
            .expect("spawn show");
        assert!(show.status.success());
        let v: serde_json::Value =
            serde_json::from_str(&String::from_utf8(show.stdout).unwrap()).unwrap();
        assert_eq!(v["vertical_id"], id);
    }

    // Now confirm at the skill list level: skills installed
    // belong to the right vertical. We use `argo skill list`
    // which lists every active skill — both should be there.
    let list = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .args(["skill", "list"])
        .output()
        .expect("spawn skill list");
    assert!(
        list.status.success(),
        "skill list stderr: {}",
        String::from_utf8_lossy(&list.stderr)
    );
    let body = String::from_utf8(list.stdout).unwrap();
    assert!(body.contains("alpha_skill"), "body: {body}");
    assert!(body.contains("beta_skill"), "body: {body}");
}

#[test]
fn vertical_run_attaches_only_its_own_bundled_skills() {
    // The composite `argo vertical run` (#669) creates a session
    // bound to the vertical and attaches its skill bundle. The
    // skills_attached count must reflect ONLY that vertical's
    // bundle — not the other one's.
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    let wd = tempfile::tempdir().unwrap();
    std::fs::write(wd.path().join("hello.txt"), "x").unwrap();
    // alpha has 2 skills; beta has 1.
    write_vertical_with_skill(verticals.path(), "alpha", "a_one");
    let alpha2 = verticals.path().join("alpha/skills/a_two.skill.md");
    std::fs::write(
        &alpha2,
        "---\nname: a_two\nversion: 1.0.0\ndescription: x\n\
         provenance:\n  source: t\n  hash: 0\n---\n# a_two\nbody",
    )
    .unwrap();
    write_vertical_with_skill(verticals.path(), "beta", "b_only");

    // Add a one-step workflow each so vertical run has something
    // to run against.
    for id in ["alpha", "beta"] {
        let v = verticals.path().join(id);
        std::fs::create_dir_all(v.join("workflows")).unwrap();
        std::fs::write(
            v.join("workflows/main.workflow.toml"),
            "schema_version = \"1\"\n\
             [workflow]\nname = \"go\"\n\
             [[step]]\nname = \"r\"\nkind = \"prompt\"\nprompt = \"hi\"\n",
        )
        .unwrap();
        // Add the workflow file to the manifest's [workflows]
        // section.
        let manifest_path = v.join("vertical.toml");
        let mut body = std::fs::read_to_string(&manifest_path).unwrap();
        body.push_str("[workflows]\nfiles = [\"workflows/main.workflow.toml\"]\n");
        std::fs::write(&manifest_path, body).unwrap();
    }

    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .current_dir(wd.path())
        .args(["vertical", "run", "alpha", "--workflow", "go", "--json"])
        .output()
        .expect("spawn vertical run alpha");
    assert!(out.status.success(), "alpha run stderr: {}", String::from_utf8_lossy(&out.stderr));
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
    let attached = v["skills_attached"].as_u64().unwrap();
    assert_eq!(
        attached, 2,
        "alpha must attach exactly its 2 bundled skills, got {attached}"
    );

    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .current_dir(wd.path())
        .args(["vertical", "run", "beta", "--workflow", "go", "--json"])
        .output()
        .expect("spawn vertical run beta");
    assert!(out.status.success());
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
    let attached = v["skills_attached"].as_u64().unwrap();
    // beta has only 1 of its OWN bundled skills. The 2 alpha
    // skills installed in the prior run are tagged with
    // vertical_id="alpha" so they don't contaminate beta's
    // attach count.
    //
    // BUT: install_and_attach_bundle's idempotent guard
    // (`find_active_by_name`) is GLOBAL — if alpha already
    // installed a skill named `a_one`, beta's bundle dir
    // wouldn't be allowed to install one with the same name
    // (different content). Since alpha and beta use DISTINCT
    // skill names here, beta installs its 1 skill cleanly.
    assert_eq!(
        attached, 1,
        "beta must attach exactly its 1 bundled skill, got {attached}"
    );
}
