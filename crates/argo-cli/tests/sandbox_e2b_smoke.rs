//! Env-gated smoke test for `argo sandbox e2b reap-idle`
//! (Stream `mission-e2b-reap-idle`).
//!
//! Skipped by default. Set `ARGO_E2B_API_KEY=<key>` to opt in;
//! the test creates a single sandbox via the real E2B service,
//! waits 1 s for it to register, then runs `reap-idle --max-age 0`
//! and asserts the sandbox is in the reap report. Costs roughly
//! the price of one sandbox-second.
//!
//! Run manually:
//!
//! ```sh
//! ARGO_E2B_API_KEY=<key> \
//!   cargo test -p argo-cli --test sandbox_e2b_smoke -- --ignored
//! ```

use argo_executor_computer_use::e2b::{E2bCall, E2bClient};
use argo_executor_computer_use::e2b_reap_cron::{ARGO_E2B_API_KEY_ENV, ARGO_E2B_BASE_URL_ENV};

const DEFAULT_BASE_URL: &str = "https://api.e2b.dev";
const TEST_TEMPLATE: &str = "browser";
const REQUEST_TIMEOUT_SECS: u32 = 30;

fn read_env() -> Option<(String, String)> {
    let key = std::env::var(ARGO_E2B_API_KEY_ENV).ok().filter(|s| !s.is_empty())?;
    let base_url =
        std::env::var(ARGO_E2B_BASE_URL_ENV).unwrap_or_else(|_| DEFAULT_BASE_URL.to_owned());
    Some((key, base_url))
}

#[tokio::test]
#[ignore = "real E2B call — set ARGO_E2B_API_KEY to opt in"]
async fn reap_idle_with_max_age_zero_closes_freshly_opened_sandbox() {
    let Some((api_key, base_url)) = read_env() else {
        eprintln!(
            "skipping: {ARGO_E2B_API_KEY_ENV} unset; see test docstring for how to enable"
        );
        return;
    };

    let client = E2bClient::new(base_url, api_key, REQUEST_TIMEOUT_SECS)
        .expect("E2bClient::new must succeed with valid args");

    let opened = client
        .open_sandbox(TEST_TEMPLATE)
        .await
        .expect("smoke test: open_sandbox must succeed when API key is valid");

    // Give the E2B control plane a beat to register the new
    // sandbox in its list-endpoint view before we list it.
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    let report = client
        .reap_idle(0)
        .await
        .expect("smoke test: reap_idle must succeed against the real service");

    assert!(
        report.scanned >= 1,
        "smoke test: expected at least one scanned sandbox, got {scanned}",
        scanned = report.scanned
    );
    assert!(
        report.reaped.iter().any(|id| id == &opened.id),
        "smoke test: expected the just-opened sandbox {id:?} to be in the reaped list \
         (reaped: {reaped:?}; errors: {errors:?})",
        id = opened.id,
        reaped = report.reaped,
        errors = report.errors,
    );
}
