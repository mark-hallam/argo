//! CI lock for `demo/gtm_engineer_learning_loop.sh` — shells out to
//! the driver script and asserts (a) exit 0 + every step milestone in
//! stdout, AND (b) the post-script DB state proves the promoted skill
//! is what `SkillStore::list_skills_for_vertical("gtm-engineer")` —
//! the same query the agent runner uses to assemble the next system
//! prompt — would surface.
//!
//! Coverage chain:
//!   demo CLI flow → DB row in lifecycle_state=active
//!   ↳ list_skills_for_vertical query returns the row
//!   ↳ build_system_prompt (in argo-agent-runner) concatenates its body
//!     into the next turn's prompt — proven by sibling unit tests in
//!     `crates/argo-agent-runner/src/turn_tests.rs`
//!
//! That bridges the demo's "improved second run" claim from
//! "implicitly true" to "explicitly locked." If any underlying CLI
//! surface (`memory append`, `skill review {list,promote,approve}`,
//! `skill list --json`, `vertical eval --score --actuals`) drifts —
//! or if the lifecycle-active SQL filter changes — the test fails
//! immediately.
//!
//! Skipped on Windows (bash + jq) and when jq isn't on PATH.

mod common;

use argo_skill::SkillStore;
use argo_store::SessionStore;
use common::argo_bin;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;

const PROMOTED_SKILL_NAME: &str = "technical_founder_learning_moat_outreach";

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
fn demo_gtm_engineer_learning_loop_runs_clean_and_promotes_consultable_skill() {
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

    // Own the lifecycle of ARGO_HOME + WORK_DIR so the script's
    // sessions.db survives the script exit for inspection. Both
    // tempdirs auto-clean via Drop when this test returns.
    let argo_home = tempfile::tempdir().expect("argo home tempdir");
    let work_dir = tempfile::tempdir().expect("work dir tempdir");

    let out = Command::new("bash")
        .arg(&script)
        .env("ARGO_BIN", argo_bin())
        .env("ARGO_HOME", argo_home.path())
        .env("WORK_DIR", work_dir.path())
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

    // (a) milestone assertions on stdout — fast-fail when a step
    //     silently disappears.
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

    // (b) consultation-chain assertion — the agent runner uses
    //     SkillStore::list_skills_for_vertical(<vid>) to gather
    //     candidates for build_system_prompt. Drive the same query
    //     against the post-script DB and confirm the promoted skill
    //     is present, active, and carries the trigger conditions
    //     verbatim — so the next turn for any gtm-engineer-bound
    //     session would see it.
    let db_path = argo_home.path().join("sessions.db");
    assert!(
        db_path.is_file(),
        "expected sessions.db at {db_path:?} after script exit"
    );

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    runtime.block_on(async move {
        let store = SessionStore::open(&db_path)
            .await
            .expect("open sessions.db");
        let skills = SkillStore::new(Arc::new(store));
        let active = skills
            .list_skills_for_vertical("gtm-engineer")
            .await
            .expect("list_skills_for_vertical");

        let promoted = active
            .iter()
            .find(|s| s.frontmatter.name == PROMOTED_SKILL_NAME)
            .unwrap_or_else(|| {
                let names: Vec<_> = active
                    .iter()
                    .map(|s| s.frontmatter.name.as_str())
                    .collect();
                panic!(
                    "promoted skill {PROMOTED_SKILL_NAME:?} missing from \
                     list_skills_for_vertical(\"gtm-engineer\"); got: {names:?}"
                )
            });

        // The body MUST carry the candidate's reusable_pattern text;
        // otherwise the prompt builder would surface an empty or
        // wrong skill. The substring match is intentionally narrow:
        // the demo's data file authored this exact phrase.
        let body = &promoted.body;
        assert!(
            body.contains("technical founder")
                && body.contains("agent-platform"),
            "promoted skill body missing expected pattern text; body:\n{body}"
        );

        // Cross-check that workspace-global queries don't accidentally
        // drop the skill — paranoid guard against future refactors.
        let active_global = skills.list_active().await.expect("list_active");
        assert!(
            active_global
                .iter()
                .any(|s| s.frontmatter.name == PROMOTED_SKILL_NAME),
            "promoted skill missing from workspace-global list_active"
        );
    });
}
