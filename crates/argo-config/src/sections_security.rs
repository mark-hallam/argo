//! `[security]` — at-rest encryption and compliance posture.
//!
//! Lives in its own sibling module so [`crate::sections`] stays a
//! lean re-export hub. Public symbols are re-exported through
//! `sections.rs` and `lib.rs` so external imports stay
//! `argo_config::SecurityConfig`.
//!
//! ```toml
//! [security.encryption]
//! key_source = "env:ARGO_DB_ENCRYPTION_KEY"   # or "file:/path", or "none"
//!
//! [security.hipaa]
//! enabled = false
//! ```
//!
//! Default `key_source` is `"none"` — backward-compatible plaintext.

use serde::{Deserialize, Serialize};

/// `[security]`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SecurityConfig {
    /// `[security.encryption]`.
    pub encryption: SecurityEncryptionConfig,
    /// `[security.hipaa]`.
    pub hipaa: SecurityHipaaConfig,
}

/// `[security.encryption]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SecurityEncryptionConfig {
    /// Where to load the at-rest encryption key from.
    ///
    /// Accepted forms (parsed by
    /// `argo_store::encryption::KeySource::parse`):
    /// - `"none"` — plaintext store (default).
    /// - `"env:VAR_NAME"` — raw 64-hex-char key from env var.
    /// - `"file:/path/to/key"` — raw 64-hex-char key from file.
    ///
    /// The key value is NEVER logged. Losing the key = losing the
    /// data; see `docs/skycot/RUNBOOK.md` § "At-rest encryption".
    pub key_source: String,
}

impl Default for SecurityEncryptionConfig {
    fn default() -> Self {
        Self {
            key_source: "none".into(),
        }
    }
}

/// `[security.hipaa]`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SecurityHipaaConfig {
    /// Whether this deployment claims HIPAA-ready mode.
    ///
    /// `false` is the safe default: startup evidence is still emitted,
    /// but the audit row records `not_claimed`.
    pub enabled: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_none_and_backward_compatible() {
        let cfg = SecurityConfig::default();
        assert_eq!(cfg.encryption.key_source, "none");
        assert!(!cfg.hipaa.enabled);
    }

    #[test]
    fn parses_from_toml() {
        let toml_src = r#"
            [encryption]
            key_source = "env:ARGO_DB_ENCRYPTION_KEY"
        "#;
        let cfg: SecurityConfig = toml::from_str(toml_src).unwrap();
        assert_eq!(cfg.encryption.key_source, "env:ARGO_DB_ENCRYPTION_KEY");
    }

    #[test]
    fn missing_section_falls_back_to_default() {
        let cfg: SecurityConfig = toml::from_str("").unwrap();
        assert_eq!(cfg.encryption.key_source, "none");
        assert!(!cfg.hipaa.enabled);
    }

    #[test]
    fn hipaa_defaults_false_and_parses_true() {
        let toml_src = r#"
            [hipaa]
            enabled = true
        "#;
        let cfg: SecurityConfig = toml::from_str(toml_src).unwrap();
        assert!(cfg.hipaa.enabled);
    }
}
