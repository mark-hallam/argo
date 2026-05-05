//! End-to-end tests for `argo session transcript <id>` (#720).
//!
//! Drives the binary so the surface is exercised at the same
//! layer operators use it. Mock-mode only — no API key required.
//!
//! Coverage:
//!   - transcript --json emits the canonical self-describing
//!     payload with full message bodies (NO 80-char snippet
//!     truncation, NO ~2000-char completed_jobs truncation)
//!   - transcript handles work-created mock sessions including
//!     vertical_id, provider, model
//!   - transcript includes multiple messages (user + assistant)
//!   - transcript text-mode renders bodies in full
//!   - transcript on an unknown session id fails clearly
//!   - argo session show is unchanged (still prints snippets)

mod common;

use common::argo_bin;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

fn fresh_workspace() -> (tempfile::TempDir, tempfile::TempDir) {
    let home = tempfile::tempdir().expect("home tempdir");
    let work = tempfile::tempdir().expect("work tempdir");
    let init = Command::new(argo_bin())
        .arg("workspace")
        .arg("init")
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", workspace_root().join("verticals"))
        .current_dir(work.path())
        .output()
        .expect("workspace init");
    assert!(
        init.status.success(),
        "workspace init failed: {}",
        String::from_utf8_lossy(&init.stderr)
    );
    (home, work)
}

fn run_argo(home: &Path, work: &Path, args: &[&str]) -> std::process::Output {
    Command::new(argo_bin())
        .args(args)
        .env("ARGO_HOME", home)
        .env("ARGO_VERTICALS_DIR", workspace_root().join("verticals"))
        .current_dir(work)
        .output()
        .expect("spawn argo")
}

/// Drive the work-queue end-to-end, returning the resulting
/// session id along with the prompt we used.
fn process_one_mock_job(home: &Path, work: &Path, prompt: &str) -> String {
    let add = run_argo(
        home,
        work,
        &[
            "work",
            "add",
            "--vertical",
            "gtm-engineer",
            "--prompt",
            prompt,
            "--json",
        ],
    );
    assert!(
        add.status.success(),
        "{}",
        String::from_utf8_lossy(&add.stderr)
    );
    let serve = run_argo(
        home,
        work,
        &[
            "work",
            "serve",
            "--vertical",
            "gtm-engineer",
            "--once",
            // #722 — single-step keeps these transcript tests on
            // the v1 single-turn mock path so the existing
            // assertions about message ordering / count still hold.
            "--max-steps",
            "1",
            "--json",
        ],
    );
    assert!(
        serve.status.success(),
        "{}",
        String::from_utf8_lossy(&serve.stderr)
    );
    let processed: Value = serde_json::from_slice(&serve.stdout).expect("json");
    processed["session_id"]
        .as_str()
        .expect("session_id")
        .to_string()
}

// ── tests ─────────────────────────────────────────────────

#[test]
fn transcript_emits_full_messages_for_work_created_mock_session() {
    let (home, work) = fresh_workspace();
    // Long-ish prompt so we can prove no snippet/2000-char truncation.
    let prompt = format!(
        "Analyze this weak campaign: 100 emails, 2 replies, 0 positive. \
         Recommend the next GTM move. {}",
        "x".repeat(400),
    );
    let session_id = process_one_mock_job(home.path(), work.path(), &prompt);

    let out = run_argo(
        home.path(),
        work.path(),
        &["session", "transcript", &session_id, "--json"],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let payload: Value = serde_json::from_slice(&out.stdout).expect("json");

    // Header
    assert_eq!(payload["session_id"], session_id);
    assert_eq!(payload["vertical_id"], "gtm-engineer");
    assert_eq!(payload["provider"], "mock");
    assert_eq!(payload["model"], "mock");
    assert!(payload["started_at"].as_str().is_some());

    // Two messages: user prompt + assistant response (mock-mode)
    let messages = payload["messages"].as_array().expect("messages array");
    assert_eq!(messages.len(), 2);

    // [1] user has the full prompt verbatim — no truncation
    assert_eq!(messages[0]["role"], "user");
    let user_blocks = messages[0]["blocks"].as_array().expect("user blocks");
    let user_text = user_blocks
        .iter()
        .find_map(|b| (b["type"] == "text").then(|| b["text"].as_str()).flatten())
        .expect("user text block");
    assert_eq!(
        user_text, prompt,
        "user prompt must be verbatim full-length, not snippet/truncated"
    );

    // [2] assistant has the full mock response (which includes the
    // entire prompt by our work-handler convention). Pre-#720 the
    // user only saw an 80-char snippet from `argo session show`;
    // here we assert the full text is reachable.
    assert_eq!(messages[1]["role"], "assistant");
    let asst_blocks = messages[1]["blocks"].as_array().expect("assistant blocks");
    let asst_text = asst_blocks
        .iter()
        .find_map(|b| (b["type"] == "text").then(|| b["text"].as_str()).flatten())
        .expect("assistant text block");
    assert!(asst_text.starts_with("[mock]"));
    assert!(
        asst_text.contains(&prompt),
        "assistant text must include the full prompt verbatim (work mock convention); \
         got {} chars (no mid-prompt truncation)",
        asst_text.len()
    );
    // Length exceeds both the 80-char snippet show-truncates to AND
    // any preview cap, since contains(&prompt) above already proves
    // the full prompt body roundtrips.
    assert!(asst_text.len() > prompt.len() + 80);
}

#[test]
fn transcript_text_mode_renders_full_bodies_and_header() {
    let (home, work) = fresh_workspace();
    let prompt = "lorem ipsum dolor sit amet — consectetur adipiscing.";
    let session_id = process_one_mock_job(home.path(), work.path(), prompt);

    let out = run_argo(
        home.path(),
        work.path(),
        &["session", "transcript", &session_id],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);

    // Header lines
    assert!(stdout.contains("vertical: gtm-engineer"));
    assert!(stdout.contains("provider: mock / mock"));
    assert!(stdout.contains("messages: 2"));

    // Both messages present in order
    assert!(stdout.contains("[01] user"));
    assert!(stdout.contains("[02] assistant"));

    // Full body of each is in the output
    assert!(
        stdout.contains(prompt),
        "user prompt body must appear in text-mode transcript; full output:\n{stdout}"
    );
    assert!(
        stdout.contains("[mock]"),
        "assistant body must appear in text-mode transcript"
    );
}

#[test]
fn transcript_includes_multiple_message_roles_in_sequence() {
    let (home, work) = fresh_workspace();
    let session_id = process_one_mock_job(home.path(), work.path(), "first job");

    // Add a second user message manually so we can assert the
    // transcript surfaces messages in seq order.
    let extra = run_argo(
        home.path(),
        work.path(),
        &[
            "chat",
            "--session-id",
            &session_id,
            "--user",
            "follow-up question",
        ],
    );
    // chat may exit non-zero against mock provider (no transport);
    // we only need the user message to land. Tolerate either.
    let _ = extra;

    let out = run_argo(
        home.path(),
        work.path(),
        &["session", "transcript", &session_id, "--json"],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let payload: Value = serde_json::from_slice(&out.stdout).expect("json");
    let messages = payload["messages"].as_array().expect("messages");

    // We expect at least the two work-driven messages present in order.
    assert!(messages.len() >= 2);
    assert_eq!(messages[0]["role"], "user");
    assert_eq!(messages[1]["role"], "assistant");
    // seq numbers are monotonic
    let seqs: Vec<i64> = messages
        .iter()
        .map(|m| m["seq"].as_i64().unwrap_or(0))
        .collect();
    let sorted: Vec<i64> = {
        let mut v = seqs.clone();
        v.sort();
        v
    };
    assert_eq!(seqs, sorted, "messages must be in seq order");
}

#[test]
fn transcript_unknown_session_fails_clearly() {
    let (home, work) = fresh_workspace();
    let out = run_argo(
        home.path(),
        work.path(),
        &[
            "session",
            "transcript",
            "00000000-0000-7000-8000-000000000000",
            "--json",
        ],
    );
    assert!(
        !out.status.success(),
        "expected non-zero exit for unknown session"
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        combined.contains("no session")
            || combined.contains("not found")
            || combined.contains("00000000"),
        "expected clear error referencing the unknown id; got:\n{combined}"
    );
}

#[test]
fn argo_session_show_remains_snippet_only_post_transcript() {
    // Regression guard: the new `transcript` command must not have
    // changed `session show`'s existing snippet behaviour.
    let (home, work) = fresh_workspace();
    let prompt = "verifying that show stays unchanged";
    let session_id = process_one_mock_job(home.path(), work.path(), prompt);

    let out = run_argo(
        home.path(),
        work.path(),
        &["session", "show", &session_id, "--json"],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let payload: Value = serde_json::from_slice(&out.stdout).expect("json");

    // show keeps its snippet shape — no `messages` array, only first/last.
    assert!(
        payload.get("messages").is_none(),
        "show must NOT carry messages array"
    );
    assert!(
        payload["first_message"].is_object() || payload["first_message"].is_null(),
        "show must still surface first_message"
    );
    assert_eq!(payload["message_count"], 2);
}
