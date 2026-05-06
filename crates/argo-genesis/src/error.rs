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
}

impl From<GenesisError> for argo_types::ArgoError {
    fn from(e: GenesisError) -> Self {
        Self::Provider(e.to_string())
    }
}
