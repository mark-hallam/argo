//! Crate-wide error type. The CLI handler maps to
//! `argo_types::ArgoError::Provider` via the `From` impl below
//! so callers don't have to depend on this crate's internals.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum GenesisError {
    /// Charter input was empty / whitespace-only.
    #[error("charter is empty; provide a description of the role")]
    EmptyCharter,

    /// Persona name failed validation (must be a non-empty
    /// kebab-case-friendly identifier).
    #[error("invalid persona '{0}'; must be non-empty and contain only [a-zA-Z0-9_-]")]
    InvalidPersona(String),

    /// Output dir already exists with content. Genesis refuses
    /// to clobber an existing vertical — operator must remove
    /// or pick a fresh path.
    #[error("output dir '{0}' is not empty; refusing to overwrite an existing vertical")]
    OutputDirNotEmpty(String),

    /// Charter didn't classify into any known archetype.
    #[error("charter did not classify into any archetype; available: {available}")]
    UnknownArchetype { available: String },

    /// Safety judge rejected the generated manifest.
    #[error("safety judge rejected the generated manifest: \
             {} action(s) match a sensitive verb but are not requires_approval=true: [{}]",
             offenders.len(), offenders.join(", "))]
    SafetyViolation { offenders: Vec<String> },

    /// `argo_vertical::load_dir` rejected the generated tree.
    #[error("generated vertical failed round-trip load: {0}")]
    LoadFailed(String),

    /// Local I/O failure writing the generated tree.
    #[error("io failure at {what}: {source}")]
    Io {
        what: &'static str,
        #[source]
        source: std::io::Error,
    },

    /// JSON serialise failure.
    #[error("json failure at {what}: {source}")]
    Json {
        what: &'static str,
        #[source]
        source: serde_json::Error,
    },

    /// TOML serialise failure.
    #[error("toml serialisation failure at {what}: {source}")]
    Toml {
        what: &'static str,
        #[source]
        source: toml::ser::Error,
    },

    /// Refinement-pass validation failure (LLM response failed
    /// the strict JSON schema, or a config knob was missing).
    /// Stream `claude-genesis-llm-refinement`.
    #[error("refinement validation failed at {stage}: {detail}")]
    Validation { stage: &'static str, detail: String },

    /// The provider returned prose instead of the requested JSON envelope —
    /// commonly the model declining to synthesize a destructive / unsafe role
    /// (e.g. "deletes customer records and wipes accounts without asking").
    ///
    /// This is a GRACEFUL, **recoverable** outcome, NOT a hard provider error:
    /// the role can't be set up *as written*, but the request itself is
    /// retryable after a rephrase. The SSE layer maps it to `recoverable:true`
    /// (see [`Self::is_role_refusal`]) so the operator can reword and retry
    /// rather than dead-ending on a raw "no JSON object" parser error. Reusing
    /// the safety-rejection framing keeps a single user-facing message surface.
    /// See `fix-genesis-broad-role-graceful-on-non-json-refusal`.
    #[error("We couldn't set up this role. It may involve actions we can't safely \
             automate, so try rephrasing it without destructive or irreversible steps.")]
    RoleRefused,
}

impl GenesisError {
    /// `true` for the graceful, **recoverable** role-refusal case — the
    /// provider returned prose / declined to synthesize a destructive role
    /// ([`Self::RoleRefused`]). The Genesis SSE layer maps this to
    /// `recoverable:true` so the operator can rephrase and retry, instead of
    /// surfacing a non-recoverable technical parser error.
    ///
    /// ```
    /// use argo_genesis::GenesisError;
    /// assert!(GenesisError::RoleRefused.is_role_refusal());
    /// assert!(!GenesisError::EmptyCharter.is_role_refusal());
    /// ```
    #[must_use]
    pub fn is_role_refusal(&self) -> bool {
        matches!(self, Self::RoleRefused)
    }
}

impl From<GenesisError> for argo_types::ArgoError {
    fn from(e: GenesisError) -> Self {
        Self::Provider(e.to_string())
    }
}
