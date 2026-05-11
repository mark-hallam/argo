//! End-to-end smoke test against a real signal-cli daemon.
//!
//! Gated by three env vars — the test silently skips unless ALL are set:
//!
//! - `ARGO_SIGNAL_DAEMON_URL`  — e.g. `http://127.0.0.1:8080`
//! - `ARGO_SIGNAL_ACCOUNT`     — the signal-cli-registered account (E.164)
//! - `ARGO_SIGNAL_TEST_RECIPIENT` — destination phone number (E.164)
//!
//! Operator runbook (see `README.md`):
//!
//! ```text
//! signal-cli -a +15551234567 daemon --http 0.0.0.0:8080
//! ```
//!
//! The test sends a one-shot message, polls `receive` once, and asserts the
//! send round-trips a positive timestamp. It does NOT assert receipt
//! end-to-end (recipient acknowledgement would require a second registered
//! account); the receive call is a smoke that the GET path is wired and
//! returns a deserializable response, not a content assertion.

use argo_mcp_connector_signal::SignalConnector;

fn env_var(name: &str) -> Option<String> {
    match std::env::var(name) {
        Ok(v) if !v.trim().is_empty() => Some(v),
        _ => None,
    }
}

#[tokio::test]
async fn smoke_send_and_receive_roundtrip() {
    let Some(daemon_url) = env_var("ARGO_SIGNAL_DAEMON_URL") else {
        eprintln!("smoke skipped: ARGO_SIGNAL_DAEMON_URL not set");
        return;
    };
    let Some(account) = env_var("ARGO_SIGNAL_ACCOUNT") else {
        eprintln!("smoke skipped: ARGO_SIGNAL_ACCOUNT not set");
        return;
    };
    let Some(recipient) = env_var("ARGO_SIGNAL_TEST_RECIPIENT") else {
        eprintln!("smoke skipped: ARGO_SIGNAL_TEST_RECIPIENT not set");
        return;
    };

    let signal = SignalConnector::new(daemon_url, account);
    let body = format!(
        "argo-mcp-connector-signal smoke @ {}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    );

    let receipt = signal
        .send_message(&recipient, &body)
        .await
        .expect("send_message must succeed against a live daemon");
    assert!(
        receipt.timestamp > 0,
        "live daemon must return a positive envelope timestamp, got {}",
        receipt.timestamp
    );

    // Single poll of receive — proves the GET path is wired. We do not assert
    // any specific envelope because the daemon may have nothing queued for our
    // account, which is the common case for a fresh smoke run.
    let _inbox = signal
        .receive()
        .await
        .expect("receive must succeed against a live daemon");
}
