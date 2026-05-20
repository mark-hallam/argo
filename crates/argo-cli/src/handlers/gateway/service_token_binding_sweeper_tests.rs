use super::*;
use argo_store::service_token_binding::{insert_binding, list_bindings};
use chrono::TimeZone;
use std::ffi::OsString;
use std::sync::{Mutex, MutexGuard, OnceLock};

static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

async fn fresh_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("open store");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

fn lock_env() -> MutexGuard<'static, ()> {
    ENV_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("env lock")
}

struct EnvRestore {
    key: &'static str,
    previous: Option<OsString>,
}

impl EnvRestore {
    fn set(key: &'static str, value: &str) -> Self {
        let previous = std::env::var_os(key);
        std::env::set_var(key, value);
        Self { key, previous }
    }

    fn remove(key: &'static str) -> Self {
        let previous = std::env::var_os(key);
        std::env::remove_var(key);
        Self { key, previous }
    }
}

impl Drop for EnvRestore {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => std::env::set_var(self.key, value),
            None => std::env::remove_var(self.key),
        }
    }
}

#[tokio::test]
async fn sweep_zero_when_table_empty() {
    let store = fresh_store().await;
    let now = Utc.with_ymd_and_hms(2026, 5, 21, 0, 0, 0).unwrap();

    assert_eq!(sweep_once(&store, now).await.unwrap(), 0);
    let audit_rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM audit_record WHERE kind = ?")
        .bind("service_token_binding.expired")
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(audit_rows, 0);
}

#[tokio::test]
async fn sweep_deletes_stale_rows_and_emits_audit() {
    let store = fresh_store().await;
    let now = Utc.with_ymd_and_hms(2026, 5, 21, 0, 0, 0).unwrap();
    for (hash, created_at) in [
        ("stale-a", "2026-01-01T00:00:00+00:00"),
        ("stale-b", "2026-01-02T00:00:00+00:00"),
        ("stale-c", "2026-01-03T00:00:00+00:00"),
        ("fresh-a", "2026-04-01T00:00:00+00:00"),
        ("fresh-b", "2026-05-20T00:00:00+00:00"),
    ] {
        insert_binding(
            store.pool(),
            hash,
            "human:mark@example.com",
            "00000000-0000-0000-0000-000000000001",
            None,
            created_at,
        )
        .await
        .unwrap();
    }

    assert_eq!(sweep_once(&store, now).await.unwrap(), 3);
    let remaining: Vec<String> = list_bindings(store.pool())
        .await
        .unwrap()
        .into_iter()
        .map(|row| row.token_hash)
        .collect();
    assert_eq!(
        remaining,
        vec!["fresh-b".to_string(), "fresh-a".to_string()]
    );

    let payload: String = sqlx::query_scalar(
        "SELECT payload_json FROM audit_record WHERE kind = 'service_token_binding.expired' LIMIT 1",
    )
    .fetch_one(store.pool())
    .await
    .unwrap();
    let json: serde_json::Value = serde_json::from_str(&payload).unwrap();
    assert_eq!(json["kind"], "service_token_binding.expired");
    assert_eq!(json["count"], 3);
    assert_eq!(json["swept_at_iso"], now.to_rfc3339());
    assert_eq!(
        json["cutoff_iso"],
        (now - ChronoDuration::seconds(DEFAULT_TTL_SECS as i64)).to_rfc3339()
    );
}

#[test]
fn tick_secs_from_env_valid_value() {
    let _guard = lock_env();
    let _tick = EnvRestore::set(TICK_ENV, "120");
    assert_eq!(tick_secs_from_env(), 120);
}

#[test]
fn tick_secs_from_env_too_small_clamped() {
    let _guard = lock_env();
    let _tick = EnvRestore::set(TICK_ENV, "5");
    assert_eq!(tick_secs_from_env(), MIN_TICK_SECS);
}

#[test]
fn tick_secs_from_env_garbage_falls_back() {
    let _guard = lock_env();
    let _tick = EnvRestore::set(TICK_ENV, "nope");
    assert_eq!(tick_secs_from_env(), DEFAULT_TICK_SECS);
}

#[test]
fn ttl_secs_from_env_valid_value() {
    let _guard = lock_env();
    let _ttl = EnvRestore::set(TTL_ENV, "172800");
    assert_eq!(ttl_secs_from_env(), 172_800);
}

#[test]
fn ttl_secs_from_env_below_minimum_clamped() {
    let _guard = lock_env();
    let _ttl = EnvRestore::set(TTL_ENV, "60");
    assert_eq!(ttl_secs_from_env(), MIN_TTL_SECS);
}

#[test]
fn ttl_secs_from_env_garbage_falls_back() {
    let _guard = lock_env();
    let _ttl = EnvRestore::set(TTL_ENV, "forever");
    assert_eq!(ttl_secs_from_env(), DEFAULT_TTL_SECS);
}

#[test]
fn env_defaults_when_unset() {
    let _guard = lock_env();
    let _tick = EnvRestore::remove(TICK_ENV);
    let _ttl = EnvRestore::remove(TTL_ENV);
    assert_eq!(tick_secs_from_env(), DEFAULT_TICK_SECS);
    assert_eq!(ttl_secs_from_env(), DEFAULT_TTL_SECS);
}
