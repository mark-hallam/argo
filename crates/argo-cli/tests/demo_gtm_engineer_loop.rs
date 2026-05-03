//! CI lock for `demo/gtm_engineer_learning_loop.sh` — shells
//! out to the driver script and asserts exit 0. Breakage in
//! any CLI surface the demo touches (`memory append`,
//! `skill review {list,promote,approve}`, `vertical eval
//! --score --actuals`, `skill list --json`) shows up here
//! immediately rather than the next time the demo is manually
//! run.
//!
//! Skipped automatically on Windows — the script is bash and
//! relies on POSIX tooling (`mktemp -d`, `awk`, `jq`). On
//! Linux/macOS the test additionally skips itself if `jq` is
//! not available, since the script `require`s it for JSON
//! assertions.

mod common;

use common::argo_bin;
use std::path::Path;
use std::process::Command;

fn workspace_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

fn jq_available() -> bool {
    Command::new("jq")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[test]
#[cfg(not(target_os = "windows"))]
fn demo_gtm_engineer_learning_loop_runs_clean() {
    if !jq_available() {
        eprintln!("SKIP: jq not on PATH — demo script requires it");
        return;
    }
    let root = workspace_root();
    let script = root.join("demo/gtm_engineer_learning_loop.sh");
    assert!(
        script.is_file(),
        "demo script missing at {script:?} — has demo/ moved?"
    );

    let out = Command::new("bash")
        .arg(&script)
        .env("ARGO_BIN", argo_bin())
        .env("ARGO_VERTICALS_DIR", root.join("verticals"))
        .output()
        .expect("spawn demo script");
    assert!(
        out.status.success(),
        "demo script failed (exit={:?})\nstdout:\n{}\nstderr:\n{}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    // Sanity-check key milestones in the captured stdout so a
    // silent success that skipped a step also fails the test.
    let stdout = String::from_utf8_lossy(&out.stdout);
    for marker in [
        "STEP 1 — role:",
        "STEP 4 — candidate:",
        "STEP 5b — promote:",
        "STEP 6 — approve:",
        "STEP 7 — improved second run:",
        "OK — demo completed successfully.",
    ] {
        assert!(
            stdout.contains(marker),
            "demo stdout missing milestone {marker:?}; full stdout:\n{stdout}"
        );
    }
}
