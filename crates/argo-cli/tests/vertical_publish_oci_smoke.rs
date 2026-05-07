//! Gated live smoke for `argo vertical publish/pull` against an
//! OCI registry such as ttl.sh.

mod common;

use common::argo_bin;
use serde_json::Value;
use std::path::Path;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn ttl_sh_publish_pull_round_trip_when_enabled() {
    let Ok(registry) = std::env::var("ARGO_OCI_TEST_REGISTRY") else {
        eprintln!("SKIP: set ARGO_OCI_TEST_REGISTRY=https://ttl.sh to run live OCI smoke");
        return;
    };
    let registry = normalize_registry(&registry);
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let verticals = work.path().join("verticals");
    write_vertical(&verticals, "oci-smoke");
    let repo_org = format!("argo-oci-smoke-{}", unique_suffix());

    let published = run(
        work.path(),
        home.path(),
        &verticals,
        &[
            "vertical",
            "publish",
            "oci-smoke",
            "--registry",
            &registry,
            "--repo-org",
            &repo_org,
            "--tag",
            "1h",
            "--json",
        ],
    );
    assert_success(&published, "publish");
    let body: Value = serde_json::from_slice(&published.stdout).unwrap();
    let reference = body["reference"].as_str().unwrap().to_string();
    let digest = body["digest"].as_str().unwrap().to_string();

    let pulled = run(
        work.path(),
        home.path(),
        &verticals,
        &[
            "vertical",
            "pull",
            &reference,
            "--registry",
            &registry,
            "--rename",
            "oci-smoke-pulled",
            "--json",
        ],
    );
    assert_success(&pulled, "pull");
    let body: Value = serde_json::from_slice(&pulled.stdout).unwrap();
    assert_eq!(body["digest"].as_str(), Some(digest.as_str()));
    assert!(verticals
        .join("oci-smoke-pulled")
        .join("vertical.toml")
        .exists());
}

fn run(workspace: &Path, home: &Path, verticals: &Path, args: &[&str]) -> Output {
    Command::new(argo_bin())
        .current_dir(workspace)
        .env("ARGO_HOME", home)
        .env("ARGO_VERTICALS_DIR", verticals)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("spawn argo {args:?}: {e}"))
}

fn assert_success(out: &Output, label: &str) {
    assert!(
        out.status.success(),
        "{label} failed: stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn write_vertical(root: &Path, id: &str) {
    let dir = root.join(id);
    std::fs::create_dir_all(dir.join("prompts")).unwrap();
    std::fs::write(
        dir.join("vertical.toml"),
        format!(
            "schema_version = \"1\"\n\n[vertical]\nid = \"{id}\"\nname = \"OCI smoke\"\nversion = \"0.1.0\"\n"
        ),
    )
    .unwrap();
    std::fs::write(dir.join("prompts/system.md"), "publish and pull smoke").unwrap();
}

fn normalize_registry(raw: &str) -> String {
    let trimmed = raw.trim().trim_end_matches('/');
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        trimmed.to_string()
    } else {
        format!("https://{trimmed}")
    }
}

fn unique_suffix() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{}-{nanos}", std::process::id())
}
