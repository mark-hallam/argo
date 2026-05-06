//! End-to-end smoke test for `argo genesis interactive
//! --non-interactive`. Drives the cli binary against a fixture
//! YAML and asserts a vertical lands at `--output` with the
//! expected files.
use std::process::Command;

fn argo_bin() -> &'static str {
    env!("CARGO_BIN_EXE_argo")
}

#[test]
fn non_interactive_yaml_produces_full_vertical() {
    let dir = tempfile::tempdir().expect("tempdir");
    let yaml_path = dir.path().join("dialogue.yaml");
    std::fs::write(
        &yaml_path,
        "
role: marketing intern
name: Maya
industry: B2B SaaS
duties:
  - Track competitor product launches
  - Draft tweets and LinkedIn posts
  - Respond to inbound demo requests
channels:
  - slack-dm
  - email
spend_cap_per_day_usd: 50
",
    )
    .expect("write yaml");
    let output_dir = dir.path().join("vertical");
    let out = Command::new(argo_bin())
        .args([
            "genesis",
            "interactive",
            "--non-interactive",
            yaml_path.to_str().expect("yaml path"),
            "--output",
            output_dir.to_str().expect("output path"),
            "--json",
        ])
        .output()
        .expect("spawn argo genesis interactive");
    assert!(
        out.status.success(),
        "exited non-zero: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("dialogue_captured"),
        "missing dialogue_captured envelope in stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("\"persona_slug\": \"maya\""),
        "persona slug not derived in stdout:\n{stdout}"
    );
    for relpath in [
        "vertical.toml",
        "worker.toml",
        "prompts/system.md",
        "prompts/persona.md",
        "memory/schema.toml",
        "eval/seed.jsonl",
    ] {
        assert!(
            output_dir.join(relpath).exists(),
            "missing generated file: {relpath}"
        );
    }
}

#[test]
fn missing_yaml_path_returns_clear_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    let output_dir = dir.path().join("vertical");
    let out = Command::new(argo_bin())
        .args([
            "genesis",
            "interactive",
            "--non-interactive",
            "/tmp/argo-test-does-not-exist.yaml",
            "--output",
            output_dir.to_str().expect("output path"),
        ])
        .output()
        .expect("spawn argo");
    assert!(!out.status.success(), "should have failed");
    let stderr = String::from_utf8_lossy(&out.stderr);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let combined = format!("{stderr}{stdout}");
    assert!(
        combined.contains("genesis interactive"),
        "missing context in error: stdout={stdout}, stderr={stderr}"
    );
}
