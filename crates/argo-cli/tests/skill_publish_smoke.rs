mod common;

use common::argo_bin;
use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};

#[test]
fn skill_publish_tags_and_unpublish_round_trip_mock_registry() {
    let home = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    let registry = tempfile::tempdir().unwrap();
    let registry_ref = format!("mock://{}", registry.path().display());

    assert_success(&run(
        home.path(),
        workspace.path(),
        &[
            "skill",
            "create",
            "publish-me",
            "--description",
            "Publish me",
            "--body",
            "# Body\n\nUseful marketplace behavior.",
            "--new-version",
            "0.1.0",
        ],
    ));

    let dry = run(
        home.path(),
        workspace.path(),
        &[
            "skill",
            "publish",
            "publish-me",
            "--registry",
            &registry_ref,
            "--tag",
            "dry",
            "--dry-run",
            "--json",
        ],
    );
    assert_success(&dry);
    assert!(!registry.path().join("oci/publish-me/dry").exists());

    let published = run(
        home.path(),
        workspace.path(),
        &[
            "skill",
            "publish",
            "publish-me",
            "--registry",
            &registry_ref,
            "--tag",
            "0.1.0",
            "--json",
        ],
    );
    assert_success(&published);
    let body: Value = serde_json::from_slice(&published.stdout).unwrap();
    let digest = body["record"]["digest"].as_str().unwrap();
    assert!(registry
        .path()
        .join("oci/publish-me/0.1.0/manifest.json")
        .exists());
    assert!(registry
        .path()
        .join("oci/publish-me/0.1.0/bundle.tar")
        .exists());
    assert!(registry
        .path()
        .join(format!("oci/blobs/sha256/{digest}.tar"))
        .exists());

    let tags = run(
        home.path(),
        workspace.path(),
        &[
            "skill",
            "tags",
            "publish-me",
            "--registry",
            &registry_ref,
            "--json",
        ],
    );
    assert_success(&tags);
    let body: Value = serde_json::from_slice(&tags.stdout).unwrap();
    assert_eq!(body["tags"][0]["tag"], "0.1.0");

    let unpublished = run(
        home.path(),
        workspace.path(),
        &[
            "skill",
            "unpublish",
            "publish-me:0.1.0",
            "--registry",
            &registry_ref,
            "--json",
        ],
    );
    assert_success(&unpublished);
    let body: Value = serde_json::from_slice(&unpublished.stdout).unwrap();
    assert_eq!(body["record"]["blob_preserved"], true);
    assert!(!registry.path().join("oci/publish-me/0.1.0").exists());
    assert!(registry
        .path()
        .join(format!("oci/blobs/sha256/{digest}.tar"))
        .exists());
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
