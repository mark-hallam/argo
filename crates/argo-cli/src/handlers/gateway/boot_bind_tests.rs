//! Focused bind-default hardening tests for
//! `mission-gateway-bind-default-harden`.

use super::{emit_insecure_bind_audit, insecure_bind_audit_payload, is_loopback_bind};
use argo_store::SessionStore;
use clap::{CommandFactory, Parser};
use serde_json::Value;
use std::sync::Arc;

#[test]
fn boot_bind_loopback_detection_keeps_default_safe() {
    assert!(is_loopback_bind("127.0.0.1"));
    assert!(is_loopback_bind("127.0.0.5"));
    assert!(is_loopback_bind("::1"));
    assert!(!is_loopback_bind("0.0.0.0"));
    assert!(!is_loopback_bind("192.168.1.10"));
}

#[test]
fn boot_bind_gateway_start_help_lists_new_and_deprecated_bind_flags() {
    let err = crate::cli::Cli::try_parse_from(["argo", "gateway", "start", "--help"])
        .expect_err("help exits through clap error");
    let help = err.to_string();
    assert!(help.contains("--bind-non-loopback"), "{help}");
    assert!(help.contains("--accept-insecure-bind-warning"), "{help}");
    assert!(help.contains("--insecure-bind"), "{help}");

    let root = crate::cli::Cli::command();
    assert_eq!(root.get_name(), "argo");
}

#[test]
fn boot_bind_audit_payload_contains_forensic_fields() {
    let payload = insecure_bind_audit_payload("0.0.0.0", 7400, 12345);
    assert_eq!(payload["event_type"], "gateway_insecure_bind");
    assert_eq!(payload["severity"], "warn");
    assert_eq!(payload["actor"], "system");
    assert_eq!(payload["bind"], "0.0.0.0");
    assert_eq!(payload["port"], 7400);
    assert_eq!(payload["pid"], 12345);
    assert_eq!(payload["tls_enabled"].as_bool(), Some(false));
    assert_eq!(payload["bind_source"], "cli");
}

#[tokio::test]
async fn boot_bind_non_loopback_override_emits_audit_row() {
    let store = Arc::new(SessionStore::open(":memory:").await.expect("open store"));
    store.migrate().await.expect("migrate store");

    emit_insecure_bind_audit(&store, "0.0.0.0", 7400).await;

    let row: (String, String) =
        sqlx::query_as("SELECT kind, payload_json FROM audit_record ORDER BY seq DESC LIMIT 1")
            .fetch_one(store.pool())
            .await
            .expect("audit row");
    assert_eq!(row.0, "gateway_insecure_bind");

    let payload: Value = serde_json::from_str(&row.1).expect("payload json");
    assert_eq!(payload["event_type"], "gateway_insecure_bind");
    assert_eq!(payload["actor"], "system");
    assert_eq!(payload["bind"], "0.0.0.0");
    assert_eq!(payload["port"], 7400);
    assert_eq!(payload["tls_enabled"].as_bool(), Some(false));
    assert_eq!(payload["bind_source"], "cli");
}
