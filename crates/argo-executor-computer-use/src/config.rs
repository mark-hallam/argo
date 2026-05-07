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

use crate::CuError;
use std::path::PathBuf;

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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn for_tests_uses_supplied_urls() {
        let c = CuConfig::for_tests("http://e2b.local".into(), "http://anth.local".into());
        assert_eq!(c.e2b_base_url, "http://e2b.local");
        assert_eq!(c.anthropic_base_url, "http://anth.local");
        assert!(!c.e2b_api_key.is_empty());
        assert!(!c.anthropic_api_key.is_empty());
    }

    #[test]
    fn from_env_errors_when_keys_missing() {
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
