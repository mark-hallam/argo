//! Unit tests for [`super::ProviderPoolConfig`].
//!
//! Env-mutating tests share `ENV_LOCK` so cargo's parallel test runner
//! cannot interleave reads/writes on the process-wide env table.

use super::ProviderPoolConfig;
use std::sync::Mutex;
use std::time::Duration;

static ENV_LOCK: Mutex<()> = Mutex::new(());

const VARS: [&str; 4] = [
    "ARGO_HTTP_POOL_MAX_IDLE_PER_HOST",
    "ARGO_HTTP_POOL_IDLE_TIMEOUT_S",
    "ARGO_HTTP_CONNECT_TIMEOUT_S",
    "ARGO_HTTP_REQUEST_TIMEOUT_S",
];

fn clear_env() {
    for v in VARS {
        std::env::remove_var(v);
    }
}

#[test]
fn from_env_returns_defaults_when_unset() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    clear_env();
    let cfg = ProviderPoolConfig::from_env();
    assert_eq!(cfg, ProviderPoolConfig::default());
    assert_eq!(cfg.pool_max_idle_per_host, usize::MAX);
    assert_eq!(cfg.pool_idle_timeout, Duration::from_secs(90));
    assert_eq!(cfg.connect_timeout, Duration::from_secs(10));
    assert_eq!(cfg.request_timeout, Duration::from_secs(120));
}

#[test]
fn env_overrides_apply() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    clear_env();
    std::env::set_var("ARGO_HTTP_POOL_MAX_IDLE_PER_HOST", "16");
    std::env::set_var("ARGO_HTTP_POOL_IDLE_TIMEOUT_S", "30");
    std::env::set_var("ARGO_HTTP_CONNECT_TIMEOUT_S", "5");
    std::env::set_var("ARGO_HTTP_REQUEST_TIMEOUT_S", "45");
    let cfg = ProviderPoolConfig::from_env();
    clear_env();
    assert_eq!(cfg.pool_max_idle_per_host, 16);
    assert_eq!(cfg.pool_idle_timeout, Duration::from_secs(30));
    assert_eq!(cfg.connect_timeout, Duration::from_secs(5));
    assert_eq!(cfg.request_timeout, Duration::from_secs(45));
}

#[test]
fn invalid_env_falls_back_to_defaults_without_panic() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    clear_env();
    std::env::set_var("ARGO_HTTP_POOL_MAX_IDLE_PER_HOST", "not-a-number");
    std::env::set_var("ARGO_HTTP_POOL_IDLE_TIMEOUT_S", "-3");
    let cfg = ProviderPoolConfig::from_env();
    clear_env();
    let defaults = ProviderPoolConfig::default();
    assert_eq!(cfg.pool_max_idle_per_host, defaults.pool_max_idle_per_host);
    assert_eq!(cfg.pool_idle_timeout, defaults.pool_idle_timeout);
}

#[test]
fn apply_pool_returns_builder() {
    let cfg = ProviderPoolConfig::default();
    // Smoke: building succeeds; reqwest validates internally.
    let _client = cfg
        .apply_pool(reqwest::Client::builder())
        .build()
        .expect("default config builds a reqwest client");
}
