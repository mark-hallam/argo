//! Config-driven plugin host registry.
use crate::{PluginError, PluginHost, PluginResult};
pub use argo_config::{PluginManifestEntry, PluginRegistryConfig};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

/// Central registry for configured plugin subprocesses.
#[derive(Debug)]
pub struct PluginRegistry {
    manifests: HashMap<String, PluginManifestEntry>,
    known_ids: Vec<String>,
    hosts: Mutex<HashMap<String, Arc<PluginHost>>>,
}

impl PluginRegistry {
    /// Build a registry from operator-provided plugin config.
    #[must_use]
    pub fn new(config: PluginRegistryConfig) -> Self {
        let mut manifests = HashMap::new();
        let mut known_ids = Vec::new();
        for manifest in config.plugins {
            known_ids.push(manifest.id.clone());
            manifests.insert(manifest.id.clone(), manifest);
        }
        Self {
            manifests,
            known_ids,
            hosts: Mutex::new(HashMap::new()),
        }
    }

    /// Return the configured plugin ids, regardless of spawn state.
    #[must_use]
    pub fn list_known(&self) -> Vec<&str> {
        self.known_ids.iter().map(String::as_str).collect()
    }

    /// Return the cached host for `plugin_id`, spawning it on first use.
    pub async fn get_or_spawn(&self, plugin_id: &str) -> PluginResult<Arc<PluginHost>> {
        if let Some(host) = self.hosts.lock().await.get(plugin_id).cloned() {
            return Ok(host);
        }
        let manifest = self
            .manifests
            .get(plugin_id)
            .cloned()
            .ok_or_else(|| PluginError::UnknownPlugin(plugin_id.to_string()))?;
        let mut hosts = self.hosts.lock().await;
        if let Some(host) = hosts.get(plugin_id).cloned() {
            return Ok(host);
        }
        let timeout = Duration::from_secs(manifest.timeout_secs.max(1));
        let host =
            Arc::new(PluginHost::start_with_timeout(&manifest.binary_path, &[], timeout).await?);
        hosts.insert(plugin_id.to_string(), host.clone());
        Ok(host)
    }

    /// Gracefully stop every spawned plugin host and clear the cache.
    pub async fn shutdown_all(&self) -> PluginResult<()> {
        let hosts = {
            let mut guard = self.hosts.lock().await;
            let hosts = guard.values().cloned().collect::<Vec<_>>();
            guard.clear();
            hosts
        };
        let mut first_error = None;
        for host in hosts {
            if let Err(err) = host.shutdown().await {
                first_error.get_or_insert(err);
            }
        }
        match first_error {
            Some(err) => Err(err),
            None => Ok(()),
        }
    }
}
