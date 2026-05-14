use super::*;
use std::path::PathBuf;
use std::sync::Arc;

fn plugin_path() -> PathBuf {
    if let Ok(path) = std::env::var("CARGO_BIN_EXE_argo-test-plugin-echo") {
        return PathBuf::from(path);
    }
    let exe = std::env::current_exe().expect("current test executable");
    let debug_dir = exe
        .parent()
        .and_then(std::path::Path::parent)
        .expect("target debug directory");
    debug_dir.join(format!(
        "argo-test-plugin-echo{}",
        std::env::consts::EXE_SUFFIX
    ))
}

fn manifest(id: &str) -> PluginManifestEntry {
    PluginManifestEntry {
        id: id.to_string(),
        binary_path: plugin_path(),
        timeout_secs: 5,
    }
}

fn registry(ids: &[&str]) -> PluginRegistry {
    PluginRegistry::new(PluginRegistryConfig {
        plugins: ids.iter().map(|id| manifest(id)).collect(),
    })
}

#[test]
fn empty_config_registry_has_no_known_plugins() {
    let registry = PluginRegistry::new(PluginRegistryConfig::default());
    assert!(registry.list_known().is_empty());
}

#[test]
fn list_known_reflects_config_order_before_spawn() {
    let registry = registry(&["forge", "executor"]);
    assert_eq!(registry.list_known(), vec!["forge", "executor"]);
}

#[tokio::test]
async fn get_or_spawn_caches_host_by_plugin_id() {
    let registry = registry(&["forge"]);
    let first = registry.get_or_spawn("forge").await.unwrap();
    let second = registry.get_or_spawn("forge").await.unwrap();
    assert!(Arc::ptr_eq(&first, &second));
    registry.shutdown_all().await.unwrap();
}

#[tokio::test]
async fn multiple_plugins_get_distinct_hosts() {
    let registry = registry(&["forge", "executor"]);
    let forge = registry.get_or_spawn("forge").await.unwrap();
    let executor = registry.get_or_spawn("executor").await.unwrap();
    assert!(!Arc::ptr_eq(&forge, &executor));
    registry.shutdown_all().await.unwrap();
}

#[tokio::test]
async fn shutdown_all_stops_every_spawned_host() {
    let registry = registry(&["forge", "executor"]);
    let forge = registry.get_or_spawn("forge").await.unwrap();
    let executor = registry.get_or_spawn("executor").await.unwrap();
    registry.shutdown_all().await.unwrap();
    let forge_result = forge.call("ping", serde_json::json!({})).await;
    let executor_result = executor.call("ping", serde_json::json!({})).await;
    assert!(matches!(forge_result, Err(PluginError::Shutdown)));
    assert!(matches!(executor_result, Err(PluginError::Shutdown)));
}

#[tokio::test]
async fn missing_plugin_id_returns_typed_error() {
    let registry = registry(&["forge"]);
    let result = registry.get_or_spawn("missing").await;
    assert!(matches!(result, Err(PluginError::UnknownPlugin(id)) if id == "missing"));
}
