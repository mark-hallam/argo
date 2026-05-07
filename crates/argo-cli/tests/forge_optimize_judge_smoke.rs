//! Integration smoke for mission-wire-judge-on-forge-promote.
//!
//! After this brief, three new env vars influence `argo forge
//! optimize`: `ARGO_FORGE_PROMOTE_THRESHOLD`,
//! `ARGO_JUDGE_FORGE_FLOOR`, and `ARGO_JUDGE_FORGE_TIMEOUT_MS`.
//! This smoke drives the actual binary with those env vars set
//! against a missing skill, and asserts the binary still exits
//! with the expected error path — proving the gate's
//! env-parsing + module-wiring code path didn't break the
//! binary's CLI surface.
//!
//! Full mock-judge gate behavior (verdict→outcome mapping,
//! threshold gate firing, judge-unavailable fail-CLOSED, audit
//! row payload shapes) is covered by the unit tests in the
//! sibling file
//! `crates/argo-cli/src/handlers/forge_optimize_judge_tests.rs`.
//! That sibling test file holds the substantive coverage
//! because `argo-cli` is a binary-only crate (no `[lib]`
//! target), so integration tests in `tests/` cannot import the
//! gate module directly to construct stub `Judge` instances.
mod common;

use common::argo_bin;
use std::process::Command;

#[test]
fn forge_optimize_recognizes_new_gate_env_vars() {
    let home = tempfile::tempdir().expect("tempdir for ARGO_HOME");
    let output = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_FORGE_PROMOTE_THRESHOLD", "0.95")
        .env("ARGO_JUDGE_FORGE_FLOOR", "0.8")
        .env("ARGO_JUDGE_FORGE_TIMEOUT_MS", "5000")
        .args([
            "forge",
            "optimize",
            "missing-skill",
            "--examples",
            "missing.jsonl",
            "--iterations",
            "1",
            "--budget",
            "0.01",
        ])
        .output()
        .expect("spawn argo binary");
    assert!(
        !output.status.success(),
        "expected non-zero exit for missing skill; got success. \
         stderr={}",
        String::from_utf8_lossy(&output.stderr),
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{stderr}{stdout}");
    assert!(
        combined.contains("missing-skill") || combined.contains("no active skill"),
        "expected 'missing-skill' or 'no active skill' diagnostic; \
         got combined output: {combined}",
    );
}
