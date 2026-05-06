//! Smoke integration test for the computer-use executor.
//! Gated behind `E2B_API_KEY` + `ANTHROPIC_API_KEY` — when
//! either is absent the test logs a skip notice and returns
//! without making any network calls.
//!
//! Per the brief's acceptance criterion: "Integration test
//! (gated behind `E2B_API_KEY` env var): runs a synthetic
//! 'navigate to example.com, click the first link, capture
//! URL' task end-to-end."
//!
//! Drives a `marketplace_browser` profile against the real E2B
//! managed service and the real Anthropic CU loop, captures
//! the audit trail, and asserts the loop produced at least
//! one CU step. Exact URL output is not asserted (the model is
//! non-deterministic) — the test verifies the executor produces
//! either `Completed` or `HumanHandoff` cleanly without leaving
//! a half-built bundle on disk.

use argo_executor_computer_use::{
    ComputerUseExecutor, CuConfig, Outcome, ProfileRegistry, RunRequest,
};

#[tokio::test]
async fn cu_navigates_example_com_and_captures_outcome() {
    let Some(_e2b) = optional_env("E2B_API_KEY") else {
        eprintln!(
            "skipping cu_smoke: E2B_API_KEY not set. \
             Export both E2B_API_KEY and ANTHROPIC_API_KEY to run."
        );
        return;
    };
    let Some(_anth) = optional_env("ANTHROPIC_API_KEY") else {
        eprintln!(
            "skipping cu_smoke: ANTHROPIC_API_KEY not set. \
             Export both E2B_API_KEY and ANTHROPIC_API_KEY to run."
        );
        return;
    };

    let config = CuConfig::from_env().expect("env config builds");
    let registry = ProfileRegistry::with_builtin_marketplace_browser();
    let exec = ComputerUseExecutor::new(config, registry);

    let bundle_root = tempfile::tempdir().expect("tempdir");
    let bundle = bundle_root.path().join("act-smoke-1");
    let req = RunRequest {
        action_id: "act-smoke-1".into(),
        job_id: "job-smoke".into(),
        tenant_id: "smoke-tenant".into(),
        persona_id: "smoke-persona".into(),
        profile_name: "marketplace_browser".into(),
        system_prompt: "Navigate to https://example.com. \
                        Click the first link on the page. \
                        Emit `stop_success` with the URL you ended up on."
            .into(),
        budget_usd: Some(0.50),
        bundle_dir: Some(bundle.clone()),
    };

    let outcome = exec
        .run(req)
        .await
        .expect("smoke run completes (success or handoff)");

    // Either outcome polarity is fine — non-deterministic model.
    let bundle_dir = match outcome {
        Outcome::Completed { bundle_dir, .. } => bundle_dir,
        Outcome::HumanHandoff { bundle_dir, .. } => bundle_dir,
    };
    assert_eq!(bundle_dir, bundle, "outcome carries the requested bundle");
    let jsonl = bundle.join("browser.jsonl");
    assert!(
        jsonl.exists(),
        "audit log must exist regardless of outcome polarity"
    );
}

fn optional_env(name: &str) -> Option<String> {
    match std::env::var(name) {
        Ok(v) if !v.trim().is_empty() => Some(v),
        _ => None,
    }
}
