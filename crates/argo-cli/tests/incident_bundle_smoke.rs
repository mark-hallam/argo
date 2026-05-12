//! Smoke test for `argo incident bundle` (Stream
//! `mission-incident-response-runbook`). Verifies that the CLI
//! produces a valid tar.gz with `manifest.json` + the expected
//! section files, that credential fields in `.argo/argo.toml` are
//! redacted, and that the bundle is openable without Argo installed.

mod common;

use common::argo_bin;
use flate2::read::GzDecoder;
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::io::Read;
use std::path::Path;
use std::process::Command;

const EXPECTED_SECTIONS: &[&str] = &[
    "doctor.json",
    "audit-tail.json",
    "worker-restarts.json",
    "gateway-metrics.json",
    "audit-archives.json",
    "build-identity.txt",
    "argo-toml.redacted.toml",
    "log-tail.txt",
];

#[test]
fn incident_bundle_produces_redacted_targz_with_all_sections() {
    let home = tempfile::tempdir().unwrap();
    let argo_dir = home.path().join(".argo");
    fs::create_dir_all(&argo_dir).unwrap();
    fs::write(
        argo_dir.join("argo.toml"),
        "[provider]\nmodel = \"opus\"\napi_key = \"sk-AAA\"\nbearer_token = \"BBB\"\nnote = \"public\"\n",
    )
    .unwrap();

    let log_path = home.path().join("argo.log");
    fs::write(&log_path, "line 1\nline 2\nline 3\n").unwrap();

    let workspace = tempfile::tempdir().unwrap();
    let bundle = workspace.path().join("out.tar.gz");
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .current_dir(workspace.path())
        .args([
            "incident",
            "bundle",
            "--since",
            "1h",
            "--output",
            bundle.to_str().unwrap(),
            "--log-tail",
            log_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "argo incident bundle failed: stdout={}\nstderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(bundle.is_file(), "bundle not written: {}", bundle.display());

    let entries = read_archive(&bundle);
    let names: BTreeSet<String> = entries.keys().cloned().collect();
    assert!(names.contains("manifest.json"), "missing manifest.json; got {names:?}");
    for expected in EXPECTED_SECTIONS {
        assert!(
            names.contains(*expected),
            "missing section {expected}; got {names:?}"
        );
    }

    let manifest_raw = entries.get("manifest.json").unwrap();
    let manifest: Value = serde_json::from_slice(manifest_raw).unwrap();
    assert_eq!(manifest["schema_version"], 1);
    assert!(manifest["generated_at"].as_str().unwrap().contains("T"));
    assert_eq!(manifest["since"], "1h");
    let sections = manifest["sections"].as_array().unwrap();
    let manifest_paths: BTreeSet<String> = sections
        .iter()
        .map(|s| s.get("path").unwrap().as_str().unwrap().to_string())
        .collect();
    for expected in EXPECTED_SECTIONS {
        assert!(
            manifest_paths.contains(*expected),
            "manifest missing section {expected}"
        );
    }

    let redacted = std::str::from_utf8(entries.get("argo-toml.redacted.toml").unwrap()).unwrap();
    assert!(redacted.contains("api_key = \"<REDACTED>\""), "api_key not redacted: {redacted}");
    assert!(redacted.contains("bearer_token = \"<REDACTED>\""), "bearer_token not redacted");
    assert!(redacted.contains("model = \"opus\""), "non-sensitive stripped: {redacted}");
    assert!(redacted.contains("note = \"public\""), "note stripped");
    assert!(!redacted.contains("sk-AAA"), "raw api_key leaked");
    assert!(!redacted.contains("BBB"), "raw bearer leaked");

    let log_tail = std::str::from_utf8(entries.get("log-tail.txt").unwrap()).unwrap();
    assert!(log_tail.contains("line 1"));
    assert!(log_tail.contains("line 3"));

    let build_id = std::str::from_utf8(entries.get("build-identity.txt").unwrap()).unwrap();
    assert!(build_id.starts_with("argo_version="), "build identity: {build_id}");
}

fn read_archive(path: &Path) -> std::collections::BTreeMap<String, Vec<u8>> {
    let file = fs::File::open(path).unwrap();
    let decoder = GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);
    let mut out = std::collections::BTreeMap::new();
    for entry in archive.entries().unwrap() {
        let mut entry = entry.unwrap();
        let name = entry.path().unwrap().display().to_string();
        let mut buf = Vec::new();
        entry.read_to_end(&mut buf).unwrap();
        out.insert(name, buf);
    }
    out
}
