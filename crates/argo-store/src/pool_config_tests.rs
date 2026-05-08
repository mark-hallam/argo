//! Unit tests for [`super::PoolConfig`].
//!
//! Env-mutating tests share `ENV_LOCK` so cargo's parallel test runner
//! cannot interleave reads/writes on the process-wide env table.

use super::PoolConfig;
use std::sync::Mutex;
use std::time::Duration;

static ENV_LOCK: Mutex<()> = Mutex::new(());

const VARS: [&str; 4] = [
    "ARGO_STORE_POOL_MAX",
    "ARGO_STORE_POOL_MIN",
    "ARGO_STORE_POOL_ACQUIRE_TIMEOUT_MS",
    "ARGO_STORE_POOL_IDLE_TIMEOUT_S",
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
    let cfg = PoolConfig::from_env();
    assert_eq!(cfg, PoolConfig::default());
    assert_eq!(cfg.max_connections, 8);
    assert_eq!(cfg.min_connections, 0);
    assert_eq!(cfg.acquire_timeout, Duration::from_secs(30));
    assert_eq!(cfg.idle_timeout, None);
}

#[test]
fn env_overrides_apply() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    clear_env();
    std::env::set_var("ARGO_STORE_POOL_MAX", "32");
    std::env::set_var("ARGO_STORE_POOL_MIN", "4");
    std::env::set_var("ARGO_STORE_POOL_ACQUIRE_TIMEOUT_MS", "1500");
    std::env::set_var("ARGO_STORE_POOL_IDLE_TIMEOUT_S", "45");
    let cfg = PoolConfig::from_env();
    clear_env();
    assert_eq!(cfg.max_connections, 32);
    assert_eq!(cfg.min_connections, 4);
    assert_eq!(cfg.acquire_timeout, Duration::from_millis(1500));
    assert_eq!(cfg.idle_timeout, Some(Duration::from_secs(45)));
}

#[test]
fn invalid_env_falls_back_to_defaults_without_panic() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    clear_env();
    std::env::set_var("ARGO_STORE_POOL_MAX", "not-a-number");
    std::env::set_var("ARGO_STORE_POOL_ACQUIRE_TIMEOUT_MS", "-5");
    let cfg = PoolConfig::from_env();
    clear_env();
    let defaults = PoolConfig::default();
    assert_eq!(cfg.max_connections, defaults.max_connections);
    assert_eq!(cfg.acquire_timeout, defaults.acquire_timeout);
}
