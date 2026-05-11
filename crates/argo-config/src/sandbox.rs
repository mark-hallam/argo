//! `[sandbox]` section — sandbox-runtime config.
//!
//! Currently scopes to the `[sandbox.image]` named-image map used by
//! the `argo-sandbox::image_for_executor` resolver. The map binds
//! executor names → OCI image references; missing table parses to an
//! empty map so partial configs (and pre-`[sandbox]` configs) load
//! against the shipped defaults.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// `[sandbox]`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SandboxConfig {
    /// `[sandbox.image]` — executor name → OCI image reference.
    pub image: BTreeMap<String, String>,
}

impl SandboxConfig {
    /// Lookup the configured image for `executor`. Returns `None` when
    /// the table is missing or has no entry for that key.
    ///
    /// ```
    /// use argo_config::SandboxConfig;
    /// let cfg = SandboxConfig::default();
    /// assert_eq!(cfg.image_for_executor("browser"), None);
    /// ```
    #[must_use]
    pub fn image_for_executor(&self, executor: &str) -> Option<&str> {
        self.image.get(executor).map(String::as_str)
    }
}

#[cfg(test)]
#[path = "sandbox_tests.rs"]
mod sandbox_tests;
