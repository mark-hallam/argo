//! Audit config section: `[audit]`. Extracted from [`crate::sections`]
//! to keep that module under the 500-LOC ceiling. Re-exported through
//! `sections` so the public API is unchanged.
use crate::defaults;
use serde::{Deserialize, Serialize};

/// `[audit]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AuditConfig {
    /// Master enable.
    pub enabled: bool,
    /// Hash algo for the chain (`sha256`).
    pub chain_algo: String,
    /// Export format (`ndjson`).
    pub export_format: String,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            chain_algo: defaults::DEFAULT_AUDIT_CHAIN_ALGO.into(),
            export_format: defaults::DEFAULT_AUDIT_EXPORT_FORMAT.into(),
        }
    }
}
