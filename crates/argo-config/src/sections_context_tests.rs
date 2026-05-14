use super::*;

#[test]
fn defaults_match_runtime_constants() {
    let cfg = ContextCompressConfig::default();
    assert!(cfg.enabled);
    assert_eq!(cfg.trigger_pct, 80);
    assert_eq!(cfg.keep_last_turns, 10);
    assert_eq!(cfg.summary_max_tokens, 500);
}

#[test]
fn parses_from_toml() {
    let body = r#"
[context_compress]
enabled = false
trigger_pct = 70
keep_last_turns = 5
summary_max_tokens = 800
"#;
    let cfg: RuntimeConfig = toml::from_str(body).expect("parse");
    assert!(!cfg.context_compress.enabled);
    assert_eq!(cfg.context_compress.trigger_pct, 70);
    assert_eq!(cfg.context_compress.keep_last_turns, 5);
    assert_eq!(cfg.context_compress.summary_max_tokens, 800);
}

#[test]
fn empty_toml_yields_defaults() {
    let cfg: RuntimeConfig = toml::from_str("").expect("parse");
    assert_eq!(cfg.context_compress, ContextCompressConfig::default());
}

#[test]
fn action_throttle_defaults_off() {
    let cfg = ActionThrottleConfig::default();
    assert!(!cfg.enabled);
    assert_eq!(cfg.audit_threshold_secs, 10);
    assert!(cfg.policies.is_empty());
}

#[test]
fn sandbox_prewarm_defaults_disabled() {
    let cfg = SandboxPrewarmConfig::default();
    assert!(!cfg.enabled, "pre-warm must be off by default");
    assert_eq!(cfg.spawn_rate_threshold, 3);
    assert_eq!(cfg.max_prewarm, 5);
    assert_eq!(cfg.window_seconds, 300);
    assert_eq!(cfg.burst_size, 2);
    assert_eq!(cfg.idle_ttl_seconds, 600);
    assert!(cfg.active_hours.is_empty());
}

#[test]
fn sandbox_prewarm_parses_from_toml() {
    let body = r#"
[sandbox_prewarm]
enabled = true
spawn_rate_threshold = 5
max_prewarm = 7
window_seconds = 120
burst_size = 3
idle_ttl_seconds = 900
active_hours = [9, 10, 11, 14, 15, 16]
"#;
    let cfg: RuntimeConfig = toml::from_str(body).expect("parse");
    assert!(cfg.sandbox_prewarm.enabled);
    assert_eq!(cfg.sandbox_prewarm.spawn_rate_threshold, 5);
    assert_eq!(cfg.sandbox_prewarm.max_prewarm, 7);
    assert_eq!(cfg.sandbox_prewarm.window_seconds, 120);
    assert_eq!(cfg.sandbox_prewarm.burst_size, 3);
    assert_eq!(cfg.sandbox_prewarm.idle_ttl_seconds, 900);
    assert_eq!(
        cfg.sandbox_prewarm.active_hours,
        vec![9, 10, 11, 14, 15, 16]
    );
}

#[test]
fn mcp_pool_defaults() {
    let cfg = McpPoolConfig::default();
    assert_eq!(cfg.max_clients, 32);
    assert_eq!(cfg.idle_ttl_secs, 600);
}

#[test]
fn mcp_pool_parses_from_toml() {
    let body = r#"
[mcp_pool]
max_clients = 8
idle_ttl_secs = 120
"#;
    let cfg: RuntimeConfig = toml::from_str(body).expect("parse");
    assert_eq!(cfg.mcp_pool.max_clients, 8);
    assert_eq!(cfg.mcp_pool.idle_ttl_secs, 120);
}

#[test]
fn mcp_pool_absent_keeps_defaults() {
    let cfg: RuntimeConfig = toml::from_str("").expect("parse");
    assert_eq!(cfg.mcp_pool, McpPoolConfig::default());
}

#[test]
fn action_throttle_parses_from_toml() {
    let body = r#"
[action_throttle]
enabled = true
audit_threshold_secs = 20

[action_throttle.policies.send_email]
tokens_per_sec = 5.0
burst = 10

[action_throttle.policies.post_listing]
tokens_per_sec = 2.0
burst = 3
"#;
    let cfg: RuntimeConfig = toml::from_str(body).expect("parse");
    assert!(cfg.action_throttle.enabled);
    assert_eq!(cfg.action_throttle.audit_threshold_secs, 20);
    let email = &cfg.action_throttle.policies["send_email"];
    assert!((email.tokens_per_sec - 5.0).abs() < 1e-6);
    assert_eq!(email.burst, 10);
    let listing = &cfg.action_throttle.policies["post_listing"];
    assert!((listing.tokens_per_sec - 2.0).abs() < 1e-6);
    assert_eq!(listing.burst, 3);
}

#[test]
fn prompt_cache_affinity_defaults_enabled_with_270s_ttl() {
    let cfg = PromptCacheAffinityConfig::default();
    assert!(cfg.enabled, "must default enabled");
    assert_eq!(cfg.ttl_secs, 270);
}

#[test]
fn prompt_cache_affinity_parses_from_toml() {
    let body = r#"
[prompt_cache_affinity]
enabled = false
ttl_secs = 120
"#;
    let cfg: RuntimeConfig = toml::from_str(body).expect("parse");
    assert!(!cfg.prompt_cache_affinity.enabled);
    assert_eq!(cfg.prompt_cache_affinity.ttl_secs, 120);
}

#[test]
fn empty_toml_runtime_yields_affinity_defaults() {
    let cfg: RuntimeConfig = toml::from_str("").expect("parse");
    assert_eq!(
        cfg.prompt_cache_affinity,
        PromptCacheAffinityConfig::default()
    );
}

#[test]
fn worker_resource_cage_defaults_enabled_with_documented_caps() {
    let cfg = WorkerResourceConfig::default();
    assert!(cfg.enabled, "cage must default-enabled");
    assert_eq!(cfg.memory_max_mib, 1024);
    assert_eq!(cfg.cpu_weight, 100);
}

#[test]
fn worker_resource_cage_parses_from_toml() {
    let body = r#"
[worker_resource_cage]
enabled = false
memory_max_mib = 2048
cpu_weight = 250
"#;
    let cfg: RuntimeConfig = toml::from_str(body).expect("parse");
    assert!(!cfg.worker_resource_cage.enabled);
    assert_eq!(cfg.worker_resource_cage.memory_max_mib, 2048);
    assert_eq!(cfg.worker_resource_cage.cpu_weight, 250);
}

#[test]
fn empty_toml_runtime_yields_cage_defaults() {
    let cfg: RuntimeConfig = toml::from_str("").expect("parse");
    assert_eq!(cfg.worker_resource_cage, WorkerResourceConfig::default());
}

#[test]
fn plugin_registry_defaults_empty() {
    let cfg = PluginRegistryConfig::default();
    assert!(cfg.plugins.is_empty());
    let runtime: RuntimeConfig = toml::from_str("").expect("parse");
    assert_eq!(runtime.plugin_registry, cfg);
}

#[test]
fn plugin_registry_parses_from_toml() {
    let body = r#"
[[plugin_registry.plugins]]
id = "forge"
binary_path = "/opt/skycot/plugins/forge-worker"
timeout_secs = 45

[[plugin_registry.plugins]]
id = "executor"
binary_path = "/opt/skycot/plugins/executor-worker"
"#;
    let cfg: RuntimeConfig = toml::from_str(body).expect("parse");
    assert_eq!(cfg.plugin_registry.plugins.len(), 2);
    assert_eq!(cfg.plugin_registry.plugins[0].id, "forge");
    assert_eq!(
        cfg.plugin_registry.plugins[0].binary_path,
        std::path::PathBuf::from("/opt/skycot/plugins/forge-worker")
    );
    assert_eq!(cfg.plugin_registry.plugins[0].timeout_secs, 45);
    assert_eq!(cfg.plugin_registry.plugins[1].id, "executor");
    assert_eq!(
        cfg.plugin_registry.plugins[1].timeout_secs,
        DEFAULT_PLUGIN_REGISTRY_TIMEOUT_SECS
    );
}
