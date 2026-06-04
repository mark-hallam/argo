//! Network-free unit tests for the live provider-key validation
//! decision logic. The actual one-token round-trip is exercised by the
//! gateway boot test in
//! `crates/argo-cli/tests/onboarding_provider_key_live_validation.rs`;
//! here we pin the pure pieces (gate, provider→manifest mapping, error
//! classification) so they stay deterministic and offline in CI.
//!
//! ## One env var, one test
//!
//! `ARGO_LIVE_KEY_VALIDATION` is process-global; the bin test binary
//! runs cases concurrently, so every assertion that reads or writes the
//! gate lives in the SINGLE [`gate_and_skip_behaviour`] test and runs
//! sequentially there. Splitting them into separate `#[test]` fns let
//! one case observe another's mid-flight `set_var` (the original flake).
//! The non-env tests below touch no shared state and stay parallel-safe.

use super::*;

#[tokio::test]
async fn gate_and_skip_behaviour() {
    let prev = std::env::var_os(LIVE_KEY_VALIDATION_ENV);

    // Unset → off, and a mapped provider still skips the network.
    set_gate(None);
    assert!(!live_validation_enabled(), "unset must read as off");
    assert_eq!(
        validate_key_live("anthropic", "sk-ant-whatever-key-value").await,
        LiveKeyOutcome::Undetermined,
        "gate off must skip the live call for a mapped provider"
    );

    // Truthy spellings enable the gate.
    for v in ["1", "true", "TRUE", "on", "Yes", " 1 "] {
        set_gate(Some(v));
        assert!(live_validation_enabled(), "{v:?} should enable the gate");
    }

    // Falsey / unknown spellings leave it off.
    for v in ["0", "false", "no", "off", "", "maybe"] {
        set_gate(Some(v));
        assert!(!live_validation_enabled(), "{v:?} should NOT enable the gate");
    }

    // Gate on but provider unmapped → short-circuit to Undetermined
    // before any dial-out, so this stays offline despite the gate.
    set_gate(Some("1"));
    assert_eq!(
        validate_key_live("custom", "some-opaque-custom-key-value").await,
        LiveKeyOutcome::Undetermined,
        "unmapped provider must skip the live call even with the gate on"
    );

    // Restore the prior value for any later test in this binary.
    restore_gate(prev);
}

/// Set or clear the gate env var. SAFETY: the whole gate surface is read
/// only from inside the single [`gate_and_skip_behaviour`] test, so no
/// other thread observes these writes.
fn set_gate(value: Option<&str>) {
    unsafe {
        match value {
            Some(v) => std::env::set_var(LIVE_KEY_VALIDATION_ENV, v),
            None => std::env::remove_var(LIVE_KEY_VALIDATION_ENV),
        }
    }
}

fn restore_gate(prev: Option<std::ffi::OsString>) {
    unsafe {
        match prev {
            Some(v) => std::env::set_var(LIVE_KEY_VALIDATION_ENV, v),
            None => std::env::remove_var(LIVE_KEY_VALIDATION_ENV),
        }
    }
}

#[test]
fn provider_maps_only_for_shipped_http_manifests() {
    assert_eq!(manifest_id_for_provider("anthropic"), Some("anthropic"));
    assert_eq!(manifest_id_for_provider("openai"), Some("openai-compatible"));
    // No shipped single-key HTTP manifest → skip live check.
    assert_eq!(manifest_id_for_provider("google"), None);
    assert_eq!(manifest_id_for_provider("custom"), None);
    assert_eq!(manifest_id_for_provider("azure"), None);
    assert_eq!(manifest_id_for_provider("bedrock"), None);
}

#[test]
fn auth_failures_classify_as_invalid() {
    for msg in [
        "anthropic HTTP 401: {\"error\":\"invalid x-api-key\"}",
        "openai HTTP 401: incorrect api key",
        "openai HTTP 403: forbidden",
    ] {
        match classify_provider_error(msg) {
            LiveKeyOutcome::Invalid(reason) => {
                assert!(!reason.is_empty(), "invalid verdict must carry a message");
            }
            other => panic!("{msg:?} should be Invalid, got {other:?}"),
        }
    }
}

#[test]
fn non_auth_errors_classify_as_undetermined() {
    // Transient / not-a-key-problem failures must never reject the save.
    for msg in [
        "reqwest send: connection refused",
        "anthropic HTTP 500: server error",
        "openai HTTP 429: rate limited",
        "anthropic HTTP 400: bad request",
        "anthropic response parse: unexpected eof",
    ] {
        assert_eq!(
            classify_provider_error(msg),
            LiveKeyOutcome::Undetermined,
            "{msg:?} should be Undetermined"
        );
    }
}
