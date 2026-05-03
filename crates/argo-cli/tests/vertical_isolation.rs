//! Integration tests proving the static vertical-isolation
//! boundary (#676). These exercise the load-time path-traversal
//! validator end-to-end against the binary, plus a synthetic
//! two-vertical setup to prove neither can reference the
//! other's assets.
//!
//! Static isolation only — runtime layers (memory namespacing
//! per session.vertical_id, learned skill scoping) ship in
//! follow-up PRs.

mod common;

use common::argo_bin;
use std::path::Path;
use std::process::Command;

fn write_minimal_vertical(verticals_dir: &Path, id: &str) {
    let v = verticals_dir.join(id);
    std::fs::create_dir_all(v.join("prompts")).unwrap();
    std::fs::write(
        v.join("vertical.toml"),
        format!(
            "schema_version = \"1\"\n\
             [vertical]\nid = \"{id}\"\nname = \"X\"\nversion = \"0.1.0\"\n\
             [role]\nsystem_prompt = \"prompts/system.md\"\n"
        ),
    )
    .unwrap();
    std::fs::write(v.join("prompts/system.md"), format!("you are {id}")).unwrap();
}

#[test]
fn pr_checker_cannot_reference_receptionist_assets_in_its_manifest() {
    // The shipped pr-checker manifest must not declare any
    // path that, when resolved, escapes the pr-checker root
    // into receptionist's territory. The validator (which
    // runs isolation as its first check) catches this at
    // load time.
    let repo_verticals = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .join("verticals");
    let home = tempfile::tempdir().unwrap();
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", &repo_verticals)
        .args(["vertical", "validate", "pr-checker", "--json"])
        .output()
        .expect("spawn validate");
    assert!(
        out.status.success(),
        "shipped pr-checker must validate cleanly under isolation: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn receptionist_cannot_reference_pr_checker_assets_in_its_manifest() {
    let repo_verticals = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .join("verticals");
    let home = tempfile::tempdir().unwrap();
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", &repo_verticals)
        .args(["vertical", "validate", "receptionist", "--json"])
        .output()
        .expect("spawn validate");
    assert!(out.status.success());
}

#[test]
fn synthetic_two_verticals_in_same_dir_are_isolated() {
    // Two clean verticals next to each other in the same
    // discovery dir. Neither references the other → both
    // validate cleanly. Locks the baseline.
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    write_minimal_vertical(verticals.path(), "alpha");
    write_minimal_vertical(verticals.path(), "beta");

    for id in ["alpha", "beta"] {
        let out = Command::new(argo_bin())
            .env("ARGO_HOME", home.path())
            .env("ARGO_VERTICALS_DIR", verticals.path())
            .args(["vertical", "validate", id, "--json"])
            .output()
            .expect("spawn");
        assert!(
            out.status.success(),
            "{id} must validate cleanly: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

#[test]
fn vertical_with_role_pointing_at_sibling_vertical_fails_validate() {
    // alpha tries to reference beta's prompts via `..`.
    // Validator must reject before any file existence check.
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    write_minimal_vertical(verticals.path(), "beta"); // legitimate sibling
    let alpha = verticals.path().join("alpha");
    std::fs::create_dir_all(&alpha).unwrap();
    std::fs::write(
        alpha.join("vertical.toml"),
        "schema_version = \"1\"\n\
         [vertical]\nid = \"alpha\"\nname = \"A\"\nversion = \"0.1.0\"\n\
         [role]\nsystem_prompt = \"../beta/prompts/system.md\"\n",
    )
    .unwrap();

    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .args(["vertical", "validate", "alpha", "--json"])
        .output()
        .expect("spawn");
    assert!(!out.status.success(), "cross-vertical reference must fail");
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.contains("`..`") && stderr.contains("[role].system_prompt"),
        "expected isolation error, got: {stderr}"
    );
}

#[test]
fn vertical_with_workflow_file_outside_root_fails_validate() {
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    let alpha = verticals.path().join("alpha");
    std::fs::create_dir_all(&alpha).unwrap();
    std::fs::write(
        alpha.join("vertical.toml"),
        "schema_version = \"1\"\n\
         [vertical]\nid = \"alpha\"\nname = \"A\"\nversion = \"0.1.0\"\n\
         [workflows]\nfiles = [\"../escape.workflow.toml\"]\n",
    )
    .unwrap();
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .args(["vertical", "validate", "alpha", "--json"])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.contains("[workflows].files"),
        "expected workflows escape error, got: {stderr}"
    );
}

#[test]
fn vertical_with_skill_bundle_outside_root_fails_validate() {
    // Skill bundles are the runtime's path to attaching skills
    // to a session. Cross-vertical skill borrowing must be
    // blocked at the manifest level.
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    write_minimal_vertical(verticals.path(), "beta");
    let alpha = verticals.path().join("alpha");
    std::fs::create_dir_all(&alpha).unwrap();
    std::fs::write(
        alpha.join("vertical.toml"),
        "schema_version = \"1\"\n\
         [vertical]\nid = \"alpha\"\nname = \"A\"\nversion = \"0.1.0\"\n\
         [skills]\nbundle = \"../beta/skills/\"\n",
    )
    .unwrap();
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .args(["vertical", "validate", "alpha", "--json"])
        .output()
        .expect("spawn");
    assert!(!out.status.success(), "cross-vertical skill bundle must be rejected");
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("[skills].bundle"), "got: {stderr}");
}

#[test]
fn vertical_with_eval_suite_outside_root_fails_validate() {
    // Eval suites scoped per vertical — no cross-vertical
    // borrowing allowed.
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    write_minimal_vertical(verticals.path(), "beta");
    let alpha = verticals.path().join("alpha");
    std::fs::create_dir_all(&alpha).unwrap();
    std::fs::write(
        alpha.join("vertical.toml"),
        "schema_version = \"1\"\n\
         [vertical]\nid = \"alpha\"\nname = \"A\"\nversion = \"0.1.0\"\n\
         [evaluation]\nsuites = [\"../beta/eval/main.jsonl\"]\n",
    )
    .unwrap();
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .args(["vertical", "validate", "alpha", "--json"])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("[evaluation].suites"), "got: {stderr}");
}

#[test]
fn vertical_with_memory_schema_outside_root_fails_validate() {
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    write_minimal_vertical(verticals.path(), "beta");
    let alpha = verticals.path().join("alpha");
    std::fs::create_dir_all(&alpha).unwrap();
    std::fs::write(
        alpha.join("vertical.toml"),
        "schema_version = \"1\"\n\
         [vertical]\nid = \"alpha\"\nname = \"A\"\nversion = \"0.1.0\"\n\
         [memory]\nschema = \"../beta/memory.schema.toml\"\n",
    )
    .unwrap();
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .args(["vertical", "validate", "alpha", "--json"])
        .output()
        .expect("spawn");
    assert!(!out.status.success());
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("[memory].schema"), "got: {stderr}");
}

#[test]
fn doctor_surfaces_isolation_violations_through_validate_check() {
    // Doctor composes validate + eval + lint. An isolation
    // violation should surface as a validate failure in the
    // doctor's report.
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    let alpha = verticals.path().join("alpha");
    std::fs::create_dir_all(&alpha).unwrap();
    std::fs::write(
        alpha.join("vertical.toml"),
        "schema_version = \"1\"\n\
         [vertical]\nid = \"alpha\"\nname = \"A\"\nversion = \"0.1.0\"\n\
         [role]\nsystem_prompt = \"../escape.md\"\n",
    )
    .unwrap();
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .args(["vertical", "doctor", "alpha"])
        .output()
        .expect("spawn doctor");
    assert!(!out.status.success(), "doctor must surface isolation failures");
}

#[test]
fn install_refuses_a_vertical_with_path_traversal_in_manifest() {
    // Installing a tarball / source dir whose manifest
    // contains a path-traversal escape must fail during the
    // pre-copy validate. Locks the install path against
    // accidental security-boundary regressions.
    let home = tempfile::tempdir().unwrap();
    let src = tempfile::tempdir().unwrap();
    std::fs::write(
        src.path().join("vertical.toml"),
        "schema_version = \"1\"\n\
         [vertical]\nid = \"escape\"\nname = \"E\"\nversion = \"0.1.0\"\n\
         [role]\nsystem_prompt = \"../escape.md\"\n",
    )
    .unwrap();
    let target = tempfile::tempdir().unwrap();
    let out = Command::new(argo_bin())
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
    assert!(!out.status.success(), "install must reject traversal manifest");
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.contains("`..`") || stderr.contains("escape"),
        "expected isolation error from install, got: {stderr}"
    );
    // And the target dir must NOT contain the partial install.
    assert!(
        !target.path().join("escape").exists(),
        "install must not leave a partial copy on isolation failure"
    );
}
