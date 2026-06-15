//! fix-mf1-audit-export-secret-fail-closed — tests for the
//! audit-signing boot-guard (`super::super::audit_signing_secret_boot_guard`).
//!
//! The guard runs right after the JWT-secret guard, gated on the same
//! production-mode signal (`jwt_guard_active`). It refuses to boot a
//! production gateway when `ARGO_AUDIT_EXPORT_SECRET` or
//! `ARGO_AUDIT_SCREENSHOT_SECRET` is unset/empty — without them the signed
//! audit-download + screenshot URLs fall back to a hardcoded, publicly-
//! known dev secret and become forgeable for ANY tenant.
//!
//! These tests set a valid JWT secret so the JWT guard passes and the
//! AUDIT guard is the one under test, mirroring
//! `super::prod_mode_missing_secret_refuses`. Loaded as a child module of
//! `health_check_tests.rs` (split out to keep that file under the 500-LOC
//! budget); helpers (`EnvVar`, `force_production`, `VALID_JWT_SECRET`,
//! `GUARD_CWD`, `LOOPBACK_BIND`, `fresh_home`) are reused from the parent
//! test module via `super::`.

use super::{
    fresh_home, force_production, run_startup_checks, EnvVar, GUARD_CWD, LOOPBACK_BIND,
    VALID_JWT_SECRET,
};

const EXPORT_SECRET_ENV: &str = "ARGO_AUDIT_EXPORT_SECRET";
const SCREENSHOT_SECRET_ENV: &str = "ARGO_AUDIT_SCREENSHOT_SECRET";

/// RAII bundle pinning the two audit-signing env vars. Returned as a
/// tuple so a single binding keeps both alive (and restores prior env on
/// drop) without per-field dead-code lints.
fn set_audit_secrets(export: Option<&str>, screenshot: Option<&str>) -> (EnvVar, EnvVar) {
    let e = match export {
        Some(v) => EnvVar::set(EXPORT_SECRET_ENV, v),
        None => EnvVar::unset(EXPORT_SECRET_ENV),
    };
    let s = match screenshot {
        Some(v) => EnvVar::set(SCREENSHOT_SECRET_ENV, v),
        None => EnvVar::unset(SCREENSHOT_SECRET_ENV),
    };
    (e, s)
}

#[tokio::test]
async fn prod_mode_missing_audit_export_secret_refuses() {
    let _g = crate::handlers::gateway::shared_env_lock().await;
    let _prod = force_production();
    // Valid JWT secret so the JWT guard passes and the AUDIT guard fires.
    let _j = EnvVar::set("SKYCOT_AUTH_JWT_SECRET", VALID_JWT_SECRET);
    // Export secret unset, screenshot set: the export guard must still fire.
    let _secrets = set_audit_secrets(None, Some("a-real-screenshot-secret"));
    let cwd = std::path::PathBuf::from(GUARD_CWD);
    let err = run_startup_checks(false, LOOPBACK_BIND, &cwd)
        .await
        .expect_err("missing audit-export secret in production must refuse to boot");
    let msg = format!("{err}");
    assert!(
        msg.contains("ARGO_AUDIT_EXPORT_SECRET") && msg.contains("must be set in production"),
        "expected fatal missing-export-secret message, got: {msg}"
    );
}

#[tokio::test]
async fn prod_mode_missing_audit_screenshot_secret_refuses() {
    let _g = crate::handlers::gateway::shared_env_lock().await;
    let _prod = force_production();
    let _j = EnvVar::set("SKYCOT_AUTH_JWT_SECRET", VALID_JWT_SECRET);
    // Export set, screenshot unset: the screenshot guard must fire.
    let _secrets = set_audit_secrets(Some("a-real-export-secret"), None);
    let cwd = std::path::PathBuf::from(GUARD_CWD);
    let err = run_startup_checks(false, LOOPBACK_BIND, &cwd)
        .await
        .expect_err("missing audit-screenshot secret in production must refuse to boot");
    let msg = format!("{err}");
    assert!(
        msg.contains("ARGO_AUDIT_SCREENSHOT_SECRET") && msg.contains("must be set in production"),
        "expected fatal missing-screenshot-secret message, got: {msg}"
    );
}

#[tokio::test]
async fn prod_mode_empty_audit_secret_refuses() {
    let _g = crate::handlers::gateway::shared_env_lock().await;
    let _prod = force_production();
    let _j = EnvVar::set("SKYCOT_AUTH_JWT_SECRET", VALID_JWT_SECRET);
    // Empty (whitespace-only) export secret is treated the same as unset.
    let _secrets = set_audit_secrets(Some("   "), Some("a-real-screenshot-secret"));
    let cwd = std::path::PathBuf::from(GUARD_CWD);
    let err = run_startup_checks(false, LOOPBACK_BIND, &cwd)
        .await
        .expect_err("empty audit-export secret in production must refuse to boot");
    assert!(
        format!("{err}").contains("must be set in production"),
        "expected empty-secret to share the missing-secret message"
    );
}

#[tokio::test]
async fn prod_mode_both_audit_secrets_set_boots() {
    let _g = crate::handlers::gateway::shared_env_lock().await;
    let _prod = force_production();
    let _j = EnvVar::set("SKYCOT_AUTH_JWT_SECRET", VALID_JWT_SECRET);
    let _secrets = set_audit_secrets(
        Some("a-real-export-secret"),
        Some("a-real-screenshot-secret"),
    );
    // Full happy env so the run reaches Ok once both guards clear.
    let (home_dir, home_path) = fresh_home();
    let _h = EnvVar::set("ARGO_HOME", home_path.to_str().unwrap());
    let _a = EnvVar::set("ANTHROPIC_API_KEY", "test-key");
    let cwd = home_dir.path().to_path_buf();
    run_startup_checks(false, LOOPBACK_BIND, &cwd)
        .await
        .expect("both audit secrets set in production must boot");
}

#[tokio::test]
async fn loopback_dev_missing_audit_secrets_boots() {
    let _g = crate::handlers::gateway::shared_env_lock().await;
    // No production signal at all (dev/test default) → audit guard
    // inactive even with both secrets unset. Pins that the guard leaves
    // the 35+ loopback integration-test boot posture untouched.
    let _profile = EnvVar::unset("ARGO_GATEWAY_PROFILE");
    let _bind = EnvVar::unset("ARGO_GATEWAY_BIND");
    let _bypass = EnvVar::unset("ARGO_GATEWAY_AUTH_BYPASS");
    let _demo = EnvVar::unset("ARGO_DEMO_MODE");
    let _j = EnvVar::set("SKYCOT_AUTH_JWT_SECRET", VALID_JWT_SECRET);
    let _secrets = set_audit_secrets(None, None);
    let (home_dir, home_path) = fresh_home();
    let _h = EnvVar::set("ARGO_HOME", home_path.to_str().unwrap());
    let _a = EnvVar::set("ANTHROPIC_API_KEY", "test-key");
    let cwd = home_dir.path().to_path_buf();
    run_startup_checks(false, LOOPBACK_BIND, &cwd)
        .await
        .expect("loopback dev run without audit secrets must still boot");
}
