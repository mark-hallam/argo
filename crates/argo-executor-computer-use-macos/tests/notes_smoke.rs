//! End-to-end smoke test driving macOS Notes through the V1.5
//! PoC executor.
//!
//! Gating (any failure → test exits early with println, not
//! assertion fail):
//! - `cfg(target_os = "macos")`
//! - `ARGO_NATIVE_MACOS_INTEGRATION=1`
//! - `MacosCuExecutor::preflight()` reports all permissions
//!   granted
//!
//! On default-permissioned CI runners the test prints why it
//! skipped and returns successfully. Local validation: grant the
//! four permissions in System Settings, set
//! `ARGO_NATIVE_MACOS_INTEGRATION=1`, then `cargo test -p
//! argo-executor-computer-use-macos --test notes_smoke`.

use argo_executor_computer_use_macos::{
    a11y, events, notes, screenshot, MacosCuConfig, MacosCuExecutor,
};
use argo_executor_computer_use::e2b::CuAction;

#[tokio::test]
async fn notes_create_type_readback() {
    if std::env::var("ARGO_NATIVE_MACOS_INTEGRATION").ok().as_deref() != Some("1") {
        println!("ARGO_NATIVE_MACOS_INTEGRATION not set — skipping native macOS smoke test");
        return;
    }
    if !cfg!(target_os = "macos") {
        println!("not on macOS — skipping native macOS smoke test");
        return;
    }

    let exec = MacosCuExecutor::new(MacosCuConfig::for_notes_app())
        .expect("default config should build");
    let report = exec.preflight().await;
    if !report.all_granted() {
        println!(
            "preflight reports missing permissions, skipping:\n{}",
            report.actionable_summary()
        );
        return;
    }

    // Step 1: open Notes + capture a baseline screenshot.
    notes::ensure_notes_open()
        .await
        .expect("ensure Notes open");
    a11y::activate_app(notes::NOTES_BUNDLE_ID, exec.config())
        .await
        .expect("activate Notes");

    let tmp = tempfile::tempdir().expect("tempdir");
    let baseline = tmp.path().join("baseline.png");
    screenshot::capture_front_window(&baseline).expect("baseline screenshot");
    let png = std::fs::read(&baseline).expect("read baseline");
    assert!(png.len() > 1024, "baseline PNG suspiciously small");

    // Step 2: create a fresh note via osascript so we have a
    // stable target id for verification.
    let probe_body = format!(
        "argo-cu-macos-poc-test-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    );
    let note_id = notes::create_note(&probe_body)
        .await
        .expect("create note");
    assert!(!note_id.is_empty(), "Notes returned empty id");

    // Step 3: post a benign Type action through the executor's
    // event path. We append a marker to the existing note via
    // Cmd+V is too tightly coupled to clipboard; instead we
    // verify the executor can post a `Click` action without
    // panicking. The actual model-loop integration is exercised
    // by an upcoming brief that wires this executor into the
    // worker_actions dispatcher with a fixture Anthropic client.
    let click = CuAction::Click { x: 100, y: 100 };
    let result = exec
        .execute_step(&click)
        .await
        .expect("execute one step");
    assert!(result.ok, "step result should be ok");

    // Step 4: readback the body via osascript to prove the note
    // we created is still findable + has the body we wrote.
    let body = notes::body_of(&note_id).await.expect("readback body");
    assert!(
        body.contains(&probe_body),
        "expected `{probe_body}` in note body, got `{body}`"
    );

    // Step 5: clean up the test note so the operator's Notes
    // store doesn't accumulate fixtures.
    notes::delete_note(&note_id).await.expect("delete note");

    // Step 6: front-app should still be Notes (no drift).
    let front = events::active_app().await.expect("active_app probe");
    assert_eq!(
        front,
        notes::NOTES_BUNDLE_ID,
        "expected Notes still in front after step"
    );
}

#[tokio::test]
async fn config_validate_smoke() {
    // Always-runs guard: confirms the default config is sound
    // even when the integration test is skipped (so the test
    // file always contributes one passing assertion).
    let cfg = MacosCuConfig::for_notes_app();
    cfg.validate().expect("default Notes config validates");
    let _exec = MacosCuExecutor::new(cfg).expect("default config builds executor");
}
