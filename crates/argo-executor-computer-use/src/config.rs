//! Environment-driven configuration for the computer-use
//! executor. Centralises every env-var the crate reads so the
//! integration test + the production wiring both go through one
//! constructor.
//!
//! - `E2B_API_KEY` — required to open E2B sandboxes.
//! - `ANTHROPIC_API_KEY` — required to drive the CU loop.
//! - `SKYCOT_CU_E2B_BASE_URL` — overrides the E2B HTTP base
//!   (`https://api.e2b.dev`). Mostly for tests.
//! - `SKYCOT_CU_ANTHROPIC_BASE_URL` — overrides the Anthropic
//!   base (`https://api.anthropic.com`). Mostly for tests.
//! - `SKYCOT_CU_PROFILES_PATH` — overrides where the executor
//!   loads `computer_use.toml` from. Defaults to
//!   `manifests/executors/computer_use.toml` (cwd-relative) or
//!   the built-in `marketplace_browser` if the file is absent.
//! - `SKYCOT_CU_TOOL_VARIANT` — overrides the Anthropic
//!   computer-use tool variant string sent in the `tools[0].type`
//!   field. Defaults to [`DEFAULT_TOOL_VARIANT`]. The matching
//!   `anthropic-beta` header is derived from the variant inside
//!   `anthropic.rs`.

use crate::CuError;
use std::path::PathBuf;

/// Current Anthropic-published computer-use tool variant. Paired
/// with the `computer-use-2025-11-24` beta header. Supports the
/// Claude Opus 4.x / Sonnet 4.6 model family. When Anthropic
/// ships a newer variant, bump this constant and the matching
/// arm in `anthropic::beta_header_for`.
pub const DEFAULT_TOOL_VARIANT: &str = "computer_20251124";

/// Resolved runtime config. Built once at executor startup.
#[derive(Debug, Clone)]
pub struct CuConfig {
    /// E2B API key. Required.
    pub e2b_api_key: String,
    /// Anthropic API key. Required.
    pub anthropic_api_key: String,
    /// E2B HTTP base URL. Defaults to `https://api.e2b.dev`.
    pub e2b_base_url: String,
    /// Anthropic HTTP base URL. Defaults to
    /// `https://api.anthropic.com`.
    pub anthropic_base_url: String,
    /// Path to the profiles manifest (TOML).
    pub profiles_path: PathBuf,
    /// Per-CU-step request timeout (seconds). Each individual
    /// HTTP call honours this; the wallclock budget honoured by
    /// [`crate::budget::BudgetTracker`] is the cumulative cap.
    pub per_request_timeout_seconds: u32,
    /// Anthropic computer-use tool variant. Sent verbatim as
    /// `tools[0].type` in `/v1/messages`. Defaults to
    /// [`DEFAULT_TOOL_VARIANT`]; override via
    /// `SKYCOT_CU_TOOL_VARIANT` when Anthropic ships a newer
    /// variant before this crate bumps its default.
    pub tool_variant: String,
}

impl CuConfig {
    /// Build from environment. Returns
    /// [`CuError::MissingEnv`] if a required key is unset.
    pub fn from_env() -> Result<Self, CuError> {
        let e2b_api_key = argo_config::cred::get("E2B_API_KEY")
            .map_err(|_| CuError::MissingEnv("E2B_API_KEY"))?;
        let anthropic_api_key = argo_config::cred::get("ANTHROPIC_API_KEY")
            .map_err(|_| CuError::MissingEnv("ANTHROPIC_API_KEY"))?;
        Ok(Self {
            e2b_api_key,
            anthropic_api_key,
            e2b_base_url: std::env::var("SKYCOT_CU_E2B_BASE_URL")
                .unwrap_or_else(|_| "https://api.e2b.dev".into()),
            anthropic_base_url: std::env::var("SKYCOT_CU_ANTHROPIC_BASE_URL")
                .unwrap_or_else(|_| "https://api.anthropic.com".into()),
            profiles_path: std::env::var("SKYCOT_CU_PROFILES_PATH")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("manifests/executors/computer_use.toml")),
            per_request_timeout_seconds: 30,
            tool_variant: std::env::var("SKYCOT_CU_TOOL_VARIANT")
                .unwrap_or_else(|_| DEFAULT_TOOL_VARIANT.into()),
        })
    }

    /// Test-only constructor. Uses placeholder credentials and
    /// the supplied URLs — handy for integration tests that
    /// drive a mock HTTP server.
    #[must_use]
    pub fn for_tests(e2b_base_url: String, anthropic_base_url: String) -> Self {
        Self {
            e2b_api_key: "test-e2b-key".into(),
            anthropic_api_key: "test-anthropic-key".into(),
            e2b_base_url,
            anthropic_base_url,
            profiles_path: PathBuf::from("manifests/executors/computer_use.toml"),
            per_request_timeout_seconds: 5,
            tool_variant: DEFAULT_TOOL_VARIANT.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Serialise env-mutating tests in this module so cargo's
    // parallel test runner doesn't race on the shared process env.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn for_tests_uses_supplied_urls() {
        let c = CuConfig::for_tests("http://e2b.local".into(), "http://anth.local".into());
        assert_eq!(c.e2b_base_url, "http://e2b.local");
        assert_eq!(c.anthropic_base_url, "http://anth.local");
        assert!(!c.e2b_api_key.is_empty());
        assert!(!c.anthropic_api_key.is_empty());
    }

    #[test]
    fn for_tests_defaults_tool_variant_to_current() {
        let c = CuConfig::for_tests("http://e2b.local".into(), "http://anth.local".into());
        assert_eq!(c.tool_variant, DEFAULT_TOOL_VARIANT);
        assert_eq!(c.tool_variant, "computer_20251124");
    }

    #[test]
    fn from_env_reads_tool_variant_override() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let prev_e2b = std::env::var("E2B_API_KEY").ok();
        let prev_anth = std::env::var("ANTHROPIC_API_KEY").ok();
        let prev_variant = std::env::var("SKYCOT_CU_TOOL_VARIANT").ok();
        std::env::set_var("E2B_API_KEY", "stub-e2b");
        std::env::set_var("ANTHROPIC_API_KEY", "stub-anth");
        std::env::set_var("SKYCOT_CU_TOOL_VARIANT", "computer_20250124");
        let cfg = CuConfig::from_env().expect("from_env succeeds with stub creds");
        assert_eq!(cfg.tool_variant, "computer_20250124");
        std::env::remove_var("SKYCOT_CU_TOOL_VARIANT");
        if let Some(v) = prev_e2b {
            std::env::set_var("E2B_API_KEY", v);
        } else {
            std::env::remove_var("E2B_API_KEY");
        }
        if let Some(v) = prev_anth {
            std::env::set_var("ANTHROPIC_API_KEY", v);
        } else {
            std::env::remove_var("ANTHROPIC_API_KEY");
        }
        if let Some(v) = prev_variant {
            std::env::set_var("SKYCOT_CU_TOOL_VARIANT", v);
        }
    }

    #[test]
    fn from_env_errors_when_keys_missing() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        // Snapshot + clear the env so this test is hermetic
        // against ambient credentials in dev environments.
        let prev_e2b = std::env::var("E2B_API_KEY").ok();
        let prev_anth = std::env::var("ANTHROPIC_API_KEY").ok();
        std::env::remove_var("E2B_API_KEY");
        std::env::remove_var("ANTHROPIC_API_KEY");
        let err = CuConfig::from_env().expect_err("missing E2B_API_KEY should error");
        assert!(matches!(err, CuError::MissingEnv("E2B_API_KEY")));
        if let Some(v) = prev_e2b {
            std::env::set_var("E2B_API_KEY", v);
        }
        if let Some(v) = prev_anth {
            std::env::set_var("ANTHROPIC_API_KEY", v);
        }
    }
}
