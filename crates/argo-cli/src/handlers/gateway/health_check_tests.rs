//! Tests for [`super::run_startup_checks`]
//! (`claude-startup-health-check`). Five scenarios:
//!
//!   1. happy path — fresh `ARGO_HOME`, both provider keys set
//!      (only one is required, but exercising both confirms the
//!      OR semantics) → `Ok(())`.
//!   2. store unreachable — `ARGO_HOME` points at an existing
//!      regular file, so `ensure_dir` fails before SQLite is
//!      touched → `Err`.
//!   3. schema mismatch (ahead) — migrate the store to the latest
//!      version and then manually bump `schema_version` past
//!      [`argo_store::EXPECTED_SCHEMA_VERSION`]; the check rejects
//!      with the directional "binary is too old" message.
//!   4. no provider keys — both `ANTHROPIC_API_KEY` and
//!      `OPENAI_API_KEY` are unset → `Err`.
//!   5. `--skip-startup-check` bypasses every check, so even with
//!      no DB, no keys, and a bogus `cwd` the call returns `Ok`.
//!
//! Loaded via `#[cfg(test)] #[path = "health_check_tests.rs"] mod
//! tests;` in `startup_check.rs`.
//!
//! Tests serialize on a process-wide [`MutexGuard`] because they
//! mutate `ANTHROPIC_API_KEY` / `OPENAI_API_KEY` / `ARGO_HOME` —
//! `cargo test` runs tests across threads by default and each
//! thread shares the same env block.

use super::{provider_key_present, run_startup_checks};
use argo_store::{SessionStore, EXPECTED_SCHEMA_VERSION};
use tokio::sync::{Mutex, MutexGuard};

// Process-wide serialization for env-mutation tests. `cargo test`
// runs tests across threads by default and each thread shares the
// same env block, so concurrent `set_var` / `remove_var` would
// race. `tokio::sync::Mutex` is async-aware so holding the guard
// across `.await` is sound (clippy::await_holding_lock).
async fn env_lock() -> MutexGuard<'static, ()> {
    static M: Mutex<()> = Mutex::const_new(());
    M.lock().await
}

struct EnvVar {
    key: &'static str,
    prev: Option<String>,
}

impl EnvVar {
    fn set(key: &'static str, value: &str) -> Self {
        let prev = std::env::var(key).ok();
        std::env::set_var(key, value);
        Self { key, prev }
    }
    fn unset(key: &'static str) -> Self {
        let prev = std::env::var(key).ok();
        std::env::remove_var(key);
        Self { key, prev }
    }
}

impl Drop for EnvVar {
    fn drop(&mut self) {
        match &self.prev {
            Some(v) => std::env::set_var(self.key, v),
            None => std::env::remove_var(self.key),
        }
    }
}

fn fresh_home() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().to_path_buf();
    (dir, path)
}

#[tokio::test]
async fn all_pass_returns_ok() {
    let _g = env_lock().await;
    let (home_dir, home_path) = fresh_home();
    let _h = EnvVar::set("ARGO_HOME", home_path.to_str().unwrap());
    let _a = EnvVar::set("ANTHROPIC_API_KEY", "test-anthropic");
    let _o = EnvVar::set("OPENAI_API_KEY", "test-openai");
    let cwd = home_dir.path().to_path_buf();
    run_startup_checks(false, &cwd)
        .await
        .expect("happy path must succeed");
}

#[tokio::test]
async fn store_unreachable_returns_error() {
    let _g = env_lock().await;
    let dir = tempfile::tempdir().expect("tempdir");
    // Point ARGO_HOME at an existing regular file so `ensure_dir`
    // fails before we even ask SQLite to open the DB. Producing a
    // genuine SQLite-open failure portably is brittle; the
    // ensure_dir branch is the same critical-path "store
    // unreachable" treatment.
    let bogus = dir.path().join("not-a-dir");
    std::fs::write(&bogus, b"placeholder").expect("write file");
    let _h = EnvVar::set("ARGO_HOME", bogus.to_str().unwrap());
    let _a = EnvVar::set("ANTHROPIC_API_KEY", "test-key");
    let _o = EnvVar::unset("OPENAI_API_KEY");
    let err = run_startup_checks(false, dir.path())
        .await
        .expect_err("must fail when ARGO_HOME is a regular file");
    let msg = format!("{err}");
    assert!(
        msg.contains("create_dir_all") || msg.contains("startup check"),
        "unexpected error text: {msg}"
    );
}

#[tokio::test]
async fn schema_ahead_returns_directional_error() {
    let _g = env_lock().await;
    let (home_dir, home_path) = fresh_home();
    let _h = EnvVar::set("ARGO_HOME", home_path.to_str().unwrap());
    let _a = EnvVar::set("ANTHROPIC_API_KEY", "test-key");
    let _o = EnvVar::unset("OPENAI_API_KEY");

    let db = home_path.join("sessions.db");
    let store = SessionStore::open(&db).await.expect("open");
    // Don't run full `migrate()` — that runs the ladder of 43
    // migrations, which under heavy parallel test load occasionally
    // races a SQLite file lock and flakes. We only need the
    // `schema_version` row to exist with a forged "ahead" value;
    // create the table directly and seed it.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS schema_version (\
            id INTEGER PRIMARY KEY CHECK (id = 1), \
            version INTEGER NOT NULL, \
            applied_at INTEGER NOT NULL)",
    )
    .execute(store.pool())
    .await
    .expect("create schema_version");
    let bumped = i64::from(EXPECTED_SCHEMA_VERSION) + 5;
    sqlx::query(
        "INSERT OR REPLACE INTO schema_version (id, version, applied_at) \
         VALUES (1, ?, 0)",
    )
    .bind(bumped)
    .execute(store.pool())
    .await
    .expect("forge schema_version");
    store.close().await;

    let cwd = home_dir.path().to_path_buf();
    let err = run_startup_checks(false, &cwd)
        .await
        .expect_err("schema ahead must refuse");
    let msg = format!("{err}");
    assert!(
        msg.contains("ahead") && msg.contains("roll back"),
        "expected directional 'ahead/roll back' message, got: {msg}"
    );
}

#[tokio::test]
async fn missing_provider_keys_returns_error() {
    let _g = env_lock().await;
    let (home_dir, home_path) = fresh_home();
    let _h = EnvVar::set("ARGO_HOME", home_path.to_str().unwrap());
    let _a = EnvVar::unset("ANTHROPIC_API_KEY");
    let _o = EnvVar::unset("OPENAI_API_KEY");
    assert!(
        !provider_key_present(),
        "guard precondition: both keys must be unset"
    );
    let cwd = home_dir.path().to_path_buf();
    let err = run_startup_checks(false, &cwd)
        .await
        .expect_err("missing keys must fail");
    let msg = format!("{err}");
    assert!(
        msg.contains("ANTHROPIC_API_KEY") && msg.contains("OPENAI_API_KEY"),
        "expected key names in error, got: {msg}"
    );
}

#[tokio::test]
async fn skip_flag_bypasses_every_check() {
    let _g = env_lock().await;
    let _a = EnvVar::unset("ANTHROPIC_API_KEY");
    let _o = EnvVar::unset("OPENAI_API_KEY");
    // Path doesn't even need to exist — skip=true short-circuits
    // before any FS access.
    let cwd = std::path::PathBuf::from("/nonexistent-skycot-startup-check");
    run_startup_checks(true, &cwd)
        .await
        .expect("--skip-startup-check must always succeed");
}
