//! `[runtime.plugin_registry]` config section.
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Default per-plugin call timeout for registry-launched plugin hosts.
pub const DEFAULT_PLUGIN_REGISTRY_TIMEOUT_SECS: u64 = 30;

/// One configured plugin subprocess known to the runtime registry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PluginManifestEntry {
    /// Stable plugin id used by runtime call sites.
    pub id: String,
    /// Filesystem path to the plugin executable.
    pub binary_path: PathBuf,
    /// Per-call timeout applied by the spawned plugin host.
    pub timeout_secs: u64,
}

impl Default for PluginManifestEntry {
    fn default() -> Self {
        Self {
            id: String::new(),
            binary_path: PathBuf::new(),
            timeout_secs: DEFAULT_PLUGIN_REGISTRY_TIMEOUT_SECS,
        }
    }
}

/// `[runtime.plugin_registry]` — configured plugin subprocesses.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PluginRegistryConfig {
    /// Plugin manifests declared by the operator.
    pub plugins: Vec<PluginManifestEntry>,
}
