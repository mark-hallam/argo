//! FINDINGS-ONLY tests for `eval-document-executor-cross-tenant-scoping`
//! (2026-05-23). Companion to the audit writeup at
//! `docs/skycot/evals/scheduled/2026-05-23/eval-document-executor-cross-tenant-scoping.md`.
//!
//! Codifies the Google Workspace-specific finding:
//!
//! - **F-4** `GsuiteAuthConfig::from_env()` reads a single
//!   process-wide `ARGO_GSUITE_SERVICE_ACCOUNT_EMAIL` +
//!   `ARGO_GSUITE_SERVICE_ACCOUNT_KEY` pair. Every tenant on the
//!   process writes to the same Drive identity. The dispatch shim's
//!   docstring already concedes this (`Operators rotate by re-issuing
//!   the service-account key and bouncing the gateway.`).

use argo_executor_document_gsuite::{GsuiteAuthConfig, GsuiteWriteRequest};

/// **F-4 P2** — two notional tenants both call
/// `GsuiteAuthConfig::from_env()` on the same process; they get
/// equivalent configs pointing at the same service-account identity.
///
/// **Closed when**: `mission-document-executors-per-tenant-credentials`
/// adds `from_credentials_for_tenant(store, tenant_id)` and routes the
/// dispatch shim through it. At that point the assertion below
/// (configs equal) flips to assert_ne, and the helper retires.
#[test]
#[ignore = "FINDINGS-ONLY: F-4 — GsuiteAuthConfig::from_env is process-global (single SA per process)"]
fn gsuite_auth_config_from_env_is_process_global() {
    let _g_email = EnvGuard::set(
        "ARGO_GSUITE_SERVICE_ACCOUNT_EMAIL",
        "shared-sa@example.iam.gserviceaccount.com",
    );
    let _g_key = EnvGuard::set(
        "ARGO_GSUITE_SERVICE_ACCOUNT_KEY",
        argo_executor_document_gsuite::auth::TEST_RSA_PRIVATE_KEY,
    );

    let cfg_a = GsuiteAuthConfig::from_env().expect("tenant A from_env");
    let cfg_b = GsuiteAuthConfig::from_env().expect("tenant B from_env");

    assert_eq!(
        cfg_a.client_email, cfg_b.client_email,
        "F-4: from_env() returned distinct emails — fix-brief landed; flip to assert_ne!"
    );
    assert_eq!(
        cfg_a.private_key_pem, cfg_b.private_key_pem,
        "F-4: from_env() returned distinct private keys — fix-brief landed; flip to assert_ne!"
    );
    assert_eq!(cfg_a.token_uri, cfg_b.token_uri);
    assert_eq!(cfg_a.scopes, cfg_b.scopes);
    assert_eq!(cfg_a.client_email, "shared-sa@example.iam.gserviceaccount.com");
}

/// **F-4 P2 (signature variant)** — `GsuiteAuthConfig::from_env` is
/// 0-arg; there is no `from_env_for_tenant`. Codified as a signature
/// check.
#[test]
#[ignore = "FINDINGS-ONLY: F-4 — no GsuiteAuthConfig::from_env_for_tenant overload"]
fn gsuite_auth_config_has_no_from_env_for_tenant() {
    let _: fn() -> argo_types::ArgoResult<GsuiteAuthConfig> = GsuiteAuthConfig::from_env;
}

/// **F-1 P2 (Gsuite variant)** — `DocumentGsuite::write(&self, req:
/// GsuiteWriteRequest)` is async and takes no tenant context. The
/// request struct names `kind`, `title`, `drive_folder_id`,
/// `doc_body_text`, `sheet_schema`, `sheet_rows`, `slides` — no
/// `tenant_id`.
#[test]
#[ignore = "FINDINGS-ONLY: F-1 — DocumentGsuite::write + GsuiteWriteRequest have no tenant axis"]
fn gsuite_write_request_has_no_tenant_field() {
    let req = GsuiteWriteRequest {
        kind: argo_executor_document_gsuite::GsuiteDocKind::Doc,
        title: "x".into(),
        drive_folder_id: None,
        doc_body_text: None,
        sheet_schema: None,
        sheet_rows: Vec::new(),
        slides: Vec::new(),
    };
    let json = serde_json::to_value(&req).expect("serialize");
    let obj = json.as_object().expect("object");
    assert!(
        !obj.contains_key("tenant_id"),
        "GsuiteWriteRequest carries no tenant_id field — that IS the finding"
    );
    assert!(!obj.contains_key("tenant"));
}

// =========================================================================
// Helpers
// =========================================================================

struct EnvGuard {
    key: &'static str,
    prev: Option<String>,
}

impl EnvGuard {
    fn set(key: &'static str, value: &str) -> Self {
        let prev = std::env::var(key).ok();
        std::env::set_var(key, value);
        Self { key, prev }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.prev {
            Some(v) => std::env::set_var(self.key, v),
            None => std::env::remove_var(self.key),
        }
    }
}
