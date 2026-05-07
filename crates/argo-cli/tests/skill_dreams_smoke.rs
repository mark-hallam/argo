//! Integration smoke for `argo skill distill-now` (V2.0
//! Integration B, `claude-dreams-skill-distillation`).
//!
//! Verifies the binary surface end-to-end without requiring
//! `ANTHROPIC_API_KEY`: the handler short-circuits when stdin
//! produces zero qualifying cohorts, so we feed:
//! - empty stdin → `cohorts_total=0`
//! - undersized stdin (one trajectory) → `cohorts_total=0`
//!
//! The actual Dreams API call path is exercised by unit tests
//! in `argo-skill::dreams_distillation::tests` via a stub
//! caller — those tests own the cohort + body-source
//! assertions without a network dep.
use std::io::Write;
use std::process::{Command, Stdio};

fn argo_bin() -> &'static str {
    env!("CARGO_BIN_EXE_argo")
}

fn run_with_stdin(args: &[&str], stdin: &str) -> (bool, String, String) {
    let mut child = Command::new(argo_bin())
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn argo");
    child
        .stdin
        .as_mut()
        .expect("child stdin")
        .write_all(stdin.as_bytes())
        .expect("write stdin");
    let out = child.wait_with_output().expect("wait child");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
    )
}

#[test]
fn distill_now_help_is_exposed() {
    let out = Command::new(argo_bin())
        .args(["skill", "distill-now", "--help"])
        .output()
        .expect("spawn help");
    assert!(out.status.success(), "help exit: {:?}", out.status);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("--worker") && stdout.contains("--json"),
        "expected --worker and --json in help, got: {stdout}"
    );
}

#[test]
fn distill_now_empty_stdin_emits_zero_cohorts_json() {
    let (ok, stdout, stderr) =
        run_with_stdin(&["skill", "distill-now", "--json"], "");
    assert!(ok, "exit non-zero: stderr={stderr}");
    assert!(
        stdout.contains("\"cohorts_total\":0"),
        "expected cohorts_total=0 in JSON, got: {stdout}"
    );
    assert!(
        stdout.contains("\"distilled\":0"),
        "expected distilled=0 in JSON, got: {stdout}"
    );
}

#[test]
fn distill_now_undersized_input_yields_zero_cohorts() {
    // One trajectory < MIN_COHORT_SIZE → group_into_cohorts
    // returns empty → handler short-circuits before requiring
    // ANTHROPIC_API_KEY.
    let stdin = r#"[
        {
            "session_id":"s1",
            "action_type":"post_listing",
            "trajectory":["a","b","c"],
            "final_text":"done",
            "success_metric":0.9,
            "vertical_id":"marketplace"
        }
    ]"#;
    let (ok, stdout, stderr) =
        run_with_stdin(&["skill", "distill-now", "--json"], stdin);
    assert!(ok, "exit non-zero: stderr={stderr}");
    assert!(
        stdout.contains("\"cohorts_total\":0"),
        "expected cohorts_total=0 (one trajectory below MIN_COHORT_SIZE), got: {stdout}"
    );
}
