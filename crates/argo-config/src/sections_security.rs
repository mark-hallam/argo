//! `[security]` — at-rest encryption, compliance posture, and
//! gateway defence-in-depth knobs.
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
//!
//! [security.ip_allowlist]
//! enabled = false
//! cidrs = []
//! trust_x_forwarded_for_hops = 0
//! ```
//!
//! Default `key_source` is `"none"` and IP allowlisting is disabled,
//! preserving backward-compatible local development behavior.

use serde::{Deserialize, Serialize};

/// `[security]`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SecurityConfig {
    /// `[security.encryption]`.
    pub encryption: SecurityEncryptionConfig,
    /// `[security.hipaa]`.
    pub hipaa: SecurityHipaaConfig,
    /// `[security.ip_allowlist]` — CIDR gate at the gateway edge.
    pub ip_allowlist: IpAllowlistConfig,
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
    /// data; see `docs/skycot/runbook/at-rest-encryption.md`.
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

/// `[security.ip_allowlist]` — operator-defined source-CIDR gate
/// applied before bearer-auth so token leaks alone are insufficient
/// for access (`mission-ip-allowlist-deployment-mode`).
///
/// `enabled = true` with `cidrs = []` is **deny-all**, not allow-all.
/// Empty-list-with-gate-on is a misconfiguration symptom the gateway
/// refuses to paper over.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct IpAllowlistConfig {
    /// Master enable. Off by default so dev/test gateways are not
    /// accidentally locked out.
    pub enabled: bool,
    /// Allowed source CIDRs (IPv4 or IPv6). Each entry is parsed at
    /// gate-construction time; malformed entries are rejected with a
    /// hard error rather than silently widening the gate.
    pub cidrs: Vec<String>,
    /// Trusted-proxy hop count for `X-Forwarded-For` IP derivation.
    /// `0` ignores `X-Forwarded-For` and uses the peer IP.
    pub trust_x_forwarded_for_hops: u8,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_backward_compatible() {
        let cfg = SecurityConfig::default();
        assert_eq!(cfg.encryption.key_source, "none");
        assert!(!cfg.hipaa.enabled);
        assert!(!cfg.ip_allowlist.enabled);
        assert!(cfg.ip_allowlist.cidrs.is_empty());
        assert_eq!(cfg.ip_allowlist.trust_x_forwarded_for_hops, 0);
    }

    #[test]
    fn parses_encryption_from_toml() {
        let toml_src = r#"
            [encryption]
            key_source = "env:ARGO_DB_ENCRYPTION_KEY"
        "#;
        let cfg: SecurityConfig = toml::from_str(toml_src).unwrap();
        assert_eq!(cfg.encryption.key_source, "env:ARGO_DB_ENCRYPTION_KEY");
    }

    #[test]
    fn parses_hipaa_from_toml() {
        let toml_src = r#"
            [hipaa]
            enabled = true
        "#;
        let cfg: SecurityConfig = toml::from_str(toml_src).unwrap();
        assert!(cfg.hipaa.enabled);
    }

    #[test]
    fn parses_ip_allowlist_from_toml() {
        let toml_src = r#"
            [ip_allowlist]
            enabled = true
            cidrs = ["10.0.0.0/8", "192.168.1.0/24", "fd00::/8"]
            trust_x_forwarded_for_hops = 2
        "#;
        let cfg: SecurityConfig = toml::from_str(toml_src).unwrap();
        assert!(cfg.ip_allowlist.enabled);
        assert_eq!(cfg.ip_allowlist.cidrs.len(), 3);
        assert_eq!(cfg.ip_allowlist.trust_x_forwarded_for_hops, 2);
    }
}
