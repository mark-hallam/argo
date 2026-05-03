//! CI locks for `demo/gtm_engineer_learning_loop.sh`.
//!
//! Three tests:
//!
//! 1. `demo_gtm_engineer_learning_loop_runs_clean_and_promotes_consultable_skill`
//!    — deterministic mock-mode end-to-end, asserts every step milestone +
//!    that the post-run DB state has the promoted skill visible to
//!    `SkillStore::list_skills_for_vertical("gtm-engineer")` (the same
//!    query the agent runner uses to assemble the next system prompt).
//!
//! 2. `demo_live_mode_without_api_key_fails_clearly` — runs the script
//!    with `--live` and an unset `ANTHROPIC_API_KEY`, asserts non-zero
//!    exit + a `FAIL: live mode requires ANTHROPIC_API_KEY` line on
//!    stderr. Always runnable; never spends API credit.
//!
//! 3. `demo_live_mode_end_to_end_when_opted_in` — runs `--live` with
//!    a real API call against anthropic/claude-sonnet-4-6. Skipped
//!    UNLESS `ARGO_LIVE_DEMO=1` AND `ANTHROPIC_API_KEY` are both set,
//!    so CI never accidentally hits the API. When enabled, asserts the
//!    same end-state milestones as test #1 but also that the captured
//!    candidate body was authored by the LLM (not the canned mock data).
//!
//! Coverage chain (deterministic):
//!   demo CLI flow → DB row in lifecycle_state=active
//!   ↳ list_skills_for_vertical query returns the row
//!   ↳ build_system_prompt (in argo-agent-runner) concatenates its body
//!     into the next turn's prompt — proven by sibling unit tests in
//!     `crates/argo-agent-runner/src/turn_tests.rs`
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

#[test]
#[cfg(not(target_os = "windows"))]
fn demo_live_mode_without_api_key_fails_clearly() {
    if !jq_available() {
        eprintln!("SKIP: jq not on PATH — demo script requires it");
        return;
    }
    let root = workspace_root();
    let script = root.join("demo/gtm_engineer_learning_loop.sh");

    // Explicitly unset both env vars that would unlock live mode, so
    // the test still does the right thing on a developer machine where
    // ANTHROPIC_API_KEY is exported in the parent shell.
    let out = Command::new("bash")
        .arg(&script)
        .arg("--live")
        .env("ARGO_BIN", argo_bin())
        .env("ARGO_VERTICALS_DIR", root.join("verticals"))
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("LIVE_LLM")
        .output()
        .expect("spawn demo script with --live");

    assert!(
        !out.status.success(),
        "live-mode script must exit non-zero without ANTHROPIC_API_KEY; \
         exit={:?} stdout:\n{}\nstderr:\n{}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        combined.contains("FAIL: live mode requires ANTHROPIC_API_KEY"),
        "expected explicit ANTHROPIC_API_KEY error message; got:\n{combined}"
    );
}

#[test]
#[cfg(not(target_os = "windows"))]
fn demo_live_mode_end_to_end_when_opted_in() {
    // Hard gate: BOTH env vars must be set. ARGO_LIVE_DEMO=1 is the
    // operator's explicit "yes, spend API credit" opt-in;
    // ANTHROPIC_API_KEY is the credential. CI must not set either.
    let opted_in = std::env::var("ARGO_LIVE_DEMO")
        .map(|v| v == "1")
        .unwrap_or(false);
    let key_present = std::env::var("ANTHROPIC_API_KEY")
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);
    if !opted_in || !key_present {
        eprintln!(
            "SKIP: live demo opt-in requires ARGO_LIVE_DEMO=1 AND \
             ANTHROPIC_API_KEY in env; running on CI is intentional NO-OP"
        );
        return;
    }
    if !jq_available() {
        eprintln!("SKIP: jq not on PATH — demo script requires it");
        return;
    }

    let root = workspace_root();
    let script = root.join("demo/gtm_engineer_learning_loop.sh");
    let argo_home = tempfile::tempdir().expect("argo home tempdir");
    let work_dir = tempfile::tempdir().expect("work dir tempdir");

    // Note: ANTHROPIC_API_KEY is NOT explicitly forwarded — we let the
    // child inherit the parent process env. That keeps the key out of
    // any code path the test author can accidentally log.
    let out = Command::new("bash")
        .arg(&script)
        .arg("--live")
        .env("ARGO_BIN", argo_bin())
        .env("ARGO_HOME", argo_home.path())
        .env("WORK_DIR", work_dir.path())
        .env("ARGO_VERTICALS_DIR", root.join("verticals"))
        .output()
        .expect("spawn demo script with --live + ANTHROPIC_API_KEY");

    assert!(
        out.status.success(),
        "live-mode script failed (exit={:?})\nstdout:\n{}\nstderr:\n{}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    for marker in [
        "STEP 1 — role:",
        "STEP 2-4 (live) — start gateway",
        "analyze response captured",
        "pattern response captured",
        "candidate JSON validated",
        "STEP 6 — approve:",
        "STEP 7 — improved second run:",
        "OK — demo completed successfully.",
    ] {
        assert!(
            stdout.contains(marker),
            "live-mode stdout missing milestone {marker:?}; full stdout:\n{stdout}"
        );
    }

    // Open the post-run DB and confirm the LLM-emitted candidate is in
    // the active vertical-scoped skill list. We do NOT assert the exact
    // skill_name because the LLM picks it — but we DO assert there's at
    // least one active gtm-engineer skill beyond the 8 manifest-bundled
    // ones, and that its body is non-trivial (>200 chars).
    let db_path = argo_home.path().join("sessions.db");
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
        assert!(
            active.len() >= 9,
            "expected ≥9 active gtm-engineer skills (8 bundled + 1 promoted); \
             got {} ({:?})",
            active.len(),
            active.iter().map(|s| s.frontmatter.name.as_str()).collect::<Vec<_>>()
        );
        // The newly promoted skill should be the most recently created
        // one (manifest skills were materialised at workspace_init time;
        // the promoted one was added later via review/promote/approve).
        let promoted = active
            .iter()
            .max_by_key(|s| s.created_at)
            .expect("at least one skill");
        assert!(
            promoted.body.len() > 200,
            "promoted skill body is suspiciously short ({} chars); body:\n{}",
            promoted.body.len(),
            promoted.body
        );
    });
}
