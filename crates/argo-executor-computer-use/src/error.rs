//! Crate-wide error type. The `Executor` trait surface in Stream
//! A speaks `argo_types::ArgoError`; `CuError` converts `into` it
//! on its way out, so callers in `argo-cli` see the standard
//! error surface and don't have to depend on this crate's
//! internals.

use thiserror::Error;

/// All failure modes inside the computer-use executor. Caller
/// code in `argo-cli` converts these into
/// `argo_types::ArgoError::Provider` via the `From` impl below.
#[derive(Debug, Error)]
pub enum CuError {
    /// A required environment variable was missing or empty.
    #[error("missing required environment variable {0}")]
    MissingEnv(&'static str),

    /// The profile the manifest declared isn't registered. The
    /// loaded profile registry lists what's available; the caller
    /// gets the registered set in the message.
    #[error("unknown computer-use profile '{requested}'; \
             registered profiles: [{registered}]")]
    UnknownProfile {
        /// The profile the manifest asked for.
        requested: String,
        /// Comma-joined list of profiles loaded at startup.
        registered: String,
    },

    /// Network / HTTP failure talking to E2B (sandbox open,
    /// screenshot, action, close) or Anthropic (CU loop).
    #[error("{stage} HTTP failure: {source}")]
    Http {
        /// Which call was in flight when the failure happened
        /// (e.g. `"e2b/sandbox.open"`, `"anthropic/messages"`).
        stage: &'static str,
        /// Underlying reqwest error.
        #[source]
        source: reqwest::Error,
    },

    /// The remote returned a non-2xx with a body. Body is
    /// truncated to keep error logs readable.
    #[error("{stage} returned HTTP {status}: {body}")]
    HttpStatus {
        stage: &'static str,
        status: u16,
        body: String,
    },

    /// Local I/O failure (audit log, screenshot persist).
    #[error("io failure at {what}: {source}")]
    Io {
        what: &'static str,
        #[source]
        source: std::io::Error,
    },

    /// JSON serialise / deserialise failure.
    #[error("json failure at {what}: {source}")]
    Json {
        what: &'static str,
        #[source]
        source: serde_json::Error,
    },

    /// TOML parse failure (loading a profile manifest).
    #[error("toml parse failure at {what}: {source}")]
    Toml {
        what: &'static str,
        #[source]
        source: toml::de::Error,
    },

    /// The CU loop hit its budget cap. The caller turns this into
    /// a `HumanHandoff` outcome rather than a hard failure — the
    /// operator finishes the action manually.
    #[error("computer-use budget exhausted at step {step}: {reason}")]
    BudgetExhausted { step: u32, reason: String },

    /// Logical / data-shape failure that doesn't fit a network
    /// or budget bucket — e.g. the model emitted an unparseable
    /// tool call, or the sandbox returned no screenshot.
    #[error("computer-use protocol failure: {0}")]
    Protocol(String),
}

impl From<CuError> for argo_types::ArgoError {
    fn from(e: CuError) -> Self {
        Self::Provider(e.to_string())
    }
}

impl CuError {
    /// True when the failure should turn into a `HumanHandoff`
    /// rather than propagate as a hard error to the caller. The
    /// dispatch shim uses this to decide outcome polarity.
    #[must_use]
    pub fn is_recoverable_to_handoff(&self) -> bool {
        matches!(
            self,
            Self::BudgetExhausted { .. } | Self::Http { .. } | Self::HttpStatus { .. }
        )
    }
}
