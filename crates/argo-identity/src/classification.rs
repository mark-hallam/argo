//! Stream `claude-data-classification` — typed data-class tags
//! over memory rows, audit payloads, and worker actions.
//!
//! Operators tag records with one or more [`DataClass`] values to
//! drive per-class handling rules: retention windows (how long the
//! record may live before it's a candidate for deletion), redaction
//! during compliance exports, and audit-export visibility.
//!
//! The enum is the **vocabulary**; the SQL persistence lives in
//! `argo_store::classification`. Workspace overrides are read from
//! `<workspace>/.argo/classification.toml` if present; otherwise the
//! shipped [`ClassificationRules::default`] applies.
//!
//! ## Resolution order for the rules
//!
//! 1. `<workspace>/.argo/classification.toml` if it parses cleanly
//! 2. Shipped defaults (this module)
//!
//! A malformed file is *ignored* rather than fatal — the operator
//! gets a warning via `argo classify rules` and the shipped defaults
//! apply. Missing tables fall back per-field to the defaults.
//!
//! ## Hard rules
//!
//! - `retention_days = 0` means "no retention rule" (record kept
//!   indefinitely). Use a positive value to opt in.
//! - `redact_in_export = true` means audit-export's
//!   `--gdpr-redact-emails` flag will redact this row's emails — the
//!   tag is the *marker*; the export flag is the *mechanism*.
//! - Untagged records get the `Untagged` rules (no retention, no
//!   redaction). Operators must explicitly classify to opt in.

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::str::FromStr;

/// One of six canonical data classes. `Untagged` is the implicit
/// class for records that haven't been explicitly classified.
///
/// ```
/// use argo_identity::classification::DataClass;
/// use std::str::FromStr;
/// assert_eq!(DataClass::from_str("pii").unwrap(), DataClass::Pii);
/// assert_eq!(DataClass::Pii.as_str(), "pii");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DataClass {
    /// Personally identifiable information (names, emails, phone).
    Pii,
    /// Protected health information (HIPAA scope).
    Phi,
    /// Financial records, payment data, account numbers.
    Financial,
    /// Secrets, API keys, tokens, credentials.
    Secret,
    /// Public, freely shareable content.
    Public,
    /// Default — no class set yet.
    Untagged,
}

impl DataClass {
    /// String form persisted in sqlite + emitted by JSON output.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pii => "pii",
            Self::Phi => "phi",
            Self::Financial => "financial",
            Self::Secret => "secret",
            Self::Public => "public",
            Self::Untagged => "untagged",
        }
    }

    /// All six variants in canonical order — useful for `classify
    /// rules` rendering and exhaustive iteration.
    #[must_use]
    pub fn all() -> [Self; 6] {
        [
            Self::Pii,
            Self::Phi,
            Self::Financial,
            Self::Secret,
            Self::Public,
            Self::Untagged,
        ]
    }

    /// Default retention window in days for this class. `0` means
    /// indefinite retention. These ship-defaults can be overridden
    /// per workspace via [`ClassificationRules::from_workspace`].
    #[must_use]
    pub fn default_retention_days(self) -> u32 {
        match self {
            Self::Pii => 90,
            Self::Phi => 365,
            Self::Financial => 730,
            Self::Secret => 30,
            Self::Public | Self::Untagged => 0,
        }
    }

    /// Default `redact_in_export` toggle for this class. PII / PHI
    /// / financial / secret ⇒ redact emails when the export flag is
    /// on. Public / untagged ⇒ leave alone.
    #[must_use]
    pub fn default_redact_in_export(self) -> bool {
        matches!(
            self,
            Self::Pii | Self::Phi | Self::Financial | Self::Secret
        )
    }
}

impl std::fmt::Display for DataClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Errors returned by [`DataClass::from_str`].
#[derive(Debug, thiserror::Error)]
pub enum DataClassParseError {
    /// The input did not match any known variant.
    #[error("unknown data class '{0}' (expected pii/phi/financial/secret/public/untagged)")]
    Unknown(String),
}

impl FromStr for DataClass {
    type Err = DataClassParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "pii" => Ok(Self::Pii),
            "phi" => Ok(Self::Phi),
            "financial" => Ok(Self::Financial),
            "secret" => Ok(Self::Secret),
            "public" => Ok(Self::Public),
            "untagged" | "" => Ok(Self::Untagged),
            other => Err(DataClassParseError::Unknown(other.to_string())),
        }
    }
}

/// Per-class handling rules. The default values ship with the
/// platform; a workspace can override fields via `classification.toml`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassRule {
    /// Retention window in days (`0` ⇒ indefinite).
    pub retention_days: u32,
    /// Whether `argo audit export --gdpr-redact-emails` should
    /// redact rows tagged with this class.
    pub redact_in_export: bool,
}

impl ClassRule {
    /// Build the shipped-default rule for a class.
    #[must_use]
    pub fn default_for(class: DataClass) -> Self {
        Self {
            retention_days: class.default_retention_days(),
            redact_in_export: class.default_redact_in_export(),
        }
    }
}

/// Internal: per-class override parsed from TOML. Each field is
/// optional so operators can override one knob at a time without
/// having to re-state the other defaults.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
struct ClassRuleOverride {
    retention_days: Option<u32>,
    redact_in_export: Option<bool>,
}

impl ClassRuleOverride {
    fn merge_with_default(self, class: DataClass) -> ClassRule {
        ClassRule {
            retention_days: self
                .retention_days
                .unwrap_or_else(|| class.default_retention_days()),
            redact_in_export: self
                .redact_in_export
                .unwrap_or_else(|| class.default_redact_in_export()),
        }
    }
}

/// Workspace-resolved classification rules — one [`ClassRule`] per
/// [`DataClass`]. Missing TOML keys fall through to shipped defaults.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ClassificationRules {
    pii: ClassRuleOverride,
    phi: ClassRuleOverride,
    financial: ClassRuleOverride,
    secret: ClassRuleOverride,
    public: ClassRuleOverride,
    untagged: ClassRuleOverride,
}

impl ClassificationRules {
    /// Look up the effective rule for one class. Returns the
    /// override merged over the shipped default.
    #[must_use]
    pub fn rule_for(&self, class: DataClass) -> ClassRule {
        let override_rule = match class {
            DataClass::Pii => self.pii,
            DataClass::Phi => self.phi,
            DataClass::Financial => self.financial,
            DataClass::Secret => self.secret,
            DataClass::Public => self.public,
            DataClass::Untagged => self.untagged,
        };
        override_rule.merge_with_default(class)
    }

    /// Resolve from `<workspace>/.argo/classification.toml`. Missing
    /// file or parse error ⇒ shipped defaults. The file shape:
    ///
    /// ```toml
    /// [pii]
    /// retention_days = 60
    /// redact_in_export = true
    ///
    /// [secret]
    /// retention_days = 7
    /// ```
    pub fn from_workspace(workspace: &Path) -> Self {
        let path = workspace.join(".argo").join("classification.toml");
        let raw = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => return Self::default(),
        };
        toml::from_str(&raw).unwrap_or_default()
    }

    /// Compute whether a tag with `tagged_at_micros` is past the
    /// retention window for `class` as of `now_micros`. Returns
    /// `false` for `retention_days = 0` (indefinite).
    #[must_use]
    pub fn is_expired(&self, class: DataClass, tagged_at_micros: i64, now_micros: i64) -> bool {
        let rule = self.rule_for(class);
        if rule.retention_days == 0 {
            return false;
        }
        let day_micros: i64 = 86_400 * 1_000_000;
        let cutoff = now_micros.saturating_sub(i64::from(rule.retention_days) * day_micros);
        tagged_at_micros < cutoff
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn parses_canonical_strings() {
        assert_eq!(DataClass::from_str("pii").unwrap(), DataClass::Pii);
        assert_eq!(DataClass::from_str("PII").unwrap(), DataClass::Pii);
        assert_eq!(DataClass::from_str(" phi ").unwrap(), DataClass::Phi);
        assert_eq!(
            DataClass::from_str("financial").unwrap(),
            DataClass::Financial
        );
        assert_eq!(DataClass::from_str("secret").unwrap(), DataClass::Secret);
        assert_eq!(DataClass::from_str("public").unwrap(), DataClass::Public);
        assert_eq!(
            DataClass::from_str("untagged").unwrap(),
            DataClass::Untagged
        );
        assert_eq!(DataClass::from_str("").unwrap(), DataClass::Untagged);
    }

    #[test]
    fn rejects_unknown_string() {
        let err = DataClass::from_str("medical").unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("medical"), "msg = {msg}");
    }

    #[test]
    fn default_retention_per_class_matches_spec() {
        assert_eq!(DataClass::Pii.default_retention_days(), 90);
        assert_eq!(DataClass::Phi.default_retention_days(), 365);
        assert_eq!(DataClass::Financial.default_retention_days(), 730);
        assert_eq!(DataClass::Secret.default_retention_days(), 30);
        assert_eq!(DataClass::Public.default_retention_days(), 0);
        assert_eq!(DataClass::Untagged.default_retention_days(), 0);
    }

    #[test]
    fn default_redact_per_class_matches_spec() {
        assert!(DataClass::Pii.default_redact_in_export());
        assert!(DataClass::Phi.default_redact_in_export());
        assert!(DataClass::Financial.default_redact_in_export());
        assert!(DataClass::Secret.default_redact_in_export());
        assert!(!DataClass::Public.default_redact_in_export());
        assert!(!DataClass::Untagged.default_redact_in_export());
    }

    #[test]
    fn default_rules_match_shipped_defaults() {
        let rules = ClassificationRules::default();
        let pii = rules.rule_for(DataClass::Pii);
        assert_eq!(pii.retention_days, 90);
        assert!(pii.redact_in_export);
        let untagged = rules.rule_for(DataClass::Untagged);
        assert_eq!(untagged.retention_days, 0);
        assert!(!untagged.redact_in_export);
    }

    #[test]
    fn workspace_override_takes_precedence() {
        let dir = tempdir().unwrap();
        let argo_dir = dir.path().join(".argo");
        fs::create_dir_all(&argo_dir).unwrap();
        fs::write(
            argo_dir.join("classification.toml"),
            "[pii]\nretention_days = 14\nredact_in_export = false\n\n[secret]\nretention_days = 7\n",
        )
        .unwrap();
        let rules = ClassificationRules::from_workspace(dir.path());
        let pii = rules.rule_for(DataClass::Pii);
        assert_eq!(pii.retention_days, 14);
        assert!(!pii.redact_in_export);
        // Secret's redact_in_export is missing — falls back to per-class default.
        let secret = rules.rule_for(DataClass::Secret);
        assert_eq!(secret.retention_days, 7);
        assert!(secret.redact_in_export);
    }

    #[test]
    fn malformed_workspace_file_falls_back() {
        let dir = tempdir().unwrap();
        let argo_dir = dir.path().join(".argo");
        fs::create_dir_all(&argo_dir).unwrap();
        fs::write(argo_dir.join("classification.toml"), "this is not toml @@@@").unwrap();
        let rules = ClassificationRules::from_workspace(dir.path());
        assert_eq!(rules.rule_for(DataClass::Pii).retention_days, 90);
    }

    #[test]
    fn missing_workspace_file_falls_back() {
        let dir = tempdir().unwrap();
        let rules = ClassificationRules::from_workspace(dir.path());
        assert_eq!(rules.rule_for(DataClass::Pii).retention_days, 90);
    }

    #[test]
    fn is_expired_respects_retention_window() {
        let rules = ClassificationRules::default();
        let now = 100_000_000_000_000_i64;
        let day_micros: i64 = 86_400 * 1_000_000;
        // Pii: 90 days
        let just_over_90 = now - 91 * day_micros;
        let just_under_90 = now - 89 * day_micros;
        assert!(rules.is_expired(DataClass::Pii, just_over_90, now));
        assert!(!rules.is_expired(DataClass::Pii, just_under_90, now));
        // Public: never expires (retention_days = 0)
        assert!(!rules.is_expired(DataClass::Public, 0, now));
    }

    #[test]
    fn all_returns_six_unique_variants() {
        let all = DataClass::all();
        assert_eq!(all.len(), 6);
        let mut seen = std::collections::HashSet::new();
        for c in all {
            assert!(seen.insert(c));
        }
    }

    #[test]
    fn display_round_trips_with_from_str() {
        for c in DataClass::all() {
            let s = format!("{c}");
            assert_eq!(DataClass::from_str(&s).unwrap(), c);
        }
    }
}
