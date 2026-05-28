//! `[subprocess]` block of the provider manifest.
//!
//! Present only when `provider.transport_kind = "subprocess"`. Drives the
//! subprocess-based driver (e.g. the Claude Code CLI). Every HTTP-based
//! manifest leaves this absent.
//!
//! Brief: `provider-claude-code-cli-subprocess`.
use serde::{Deserialize, Serialize};

/// `[subprocess]` table — config for a subprocess-based provider driver.
///
/// ```
/// use argo_provider::SubprocessSpec;
/// let s: SubprocessSpec = toml::from_str(r#"
/// binary_path_env = "ARGO_CLAUDE_CODE_BINARY"
/// binary_path_fallback = "claude"
/// default_args = ["--print", "--output-format", "json"]
/// timeout_ms = 30000
/// streaming = false
/// "#).unwrap();
/// assert_eq!(s.binary_path_env, "ARGO_CLAUDE_CODE_BINARY");
/// assert_eq!(s.binary_path_fallback, "claude");
/// assert_eq!(s.default_args, vec!["--print", "--output-format", "json"]);
/// assert_eq!(s.timeout_ms, 30000);
/// assert!(!s.streaming);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubprocessSpec {
    /// Env var the driver consults first for the binary path. When unset,
    /// the driver falls back to `binary_path_fallback` (resolved through
    /// `$PATH` if not absolute).
    pub binary_path_env: String,
    /// Default binary name / path. Used when `binary_path_env` is unset.
    pub binary_path_fallback: String,
    /// Args prepended to the binary on every invocation (e.g.
    /// `["--print", "--output-format", "json"]`).
    #[serde(default)]
    pub default_args: Vec<String>,
    /// Hard timeout for one subprocess invocation (ms). The driver kills
    /// the child + returns a `ProviderError` when this elapses.
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
    /// Whether the subprocess emits streaming output. v1 of the
    /// Claude Code CLI driver is one-shot only — this is always false
    /// for now. Reserved for v1.1 when `--output-format stream-json`
    /// support lands.
    #[serde(default)]
    pub streaming: bool,
}

fn default_timeout_ms() -> u64 {
    30_000
}
