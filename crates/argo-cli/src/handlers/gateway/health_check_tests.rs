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

/// A valid 32-byte HS256 secret. Since sprint-ws1-1c the non-JWT
/// startup-check tests carry it defensively: the fail-closed JWT
/// boot-guard only fires in production mode (per
/// `routes_auth_production_mode::is_production_mode`), which these
/// loopback tests are not — but presenting a valid secret keeps them
/// green even if the suite inherits an ambient
/// `ARGO_GATEWAY_PROFILE=production`.
const VALID_JWT_SECRET: &str = "0123456789abcdef0123456789abcdef";

/// Loopback `--bind` value passed by the non-production startup-check
/// tests. Since sprint-ws1-1c-fix the boot-guard is bind-aware
/// (`is_production_mode_for_bind`), so the `bind` argument must be
/// supplied; a loopback value keeps these tests' production signal
/// driven solely by their explicit `ARGO_GATEWAY_PROFILE` (or absence
/// thereof) rather than by the socket address.
const LOOPBACK_BIND: &str = "127.0.0.1";

#[tokio::test]
async fn all_pass_returns_ok() {
    let _g = env_lock().await;
    let (home_dir, home_path) = fresh_home();
    let _h = EnvVar::set("ARGO_HOME", home_path.to_str().unwrap());
    let _a = EnvVar::set("ANTHROPIC_API_KEY", "test-anthropic");
    let _o = EnvVar::set("OPENAI_API_KEY", "test-openai");
    let _j = EnvVar::set("SKYCOT_AUTH_JWT_SECRET", VALID_JWT_SECRET);
    let cwd = home_dir.path().to_path_buf();
    run_startup_checks(false, LOOPBACK_BIND, &cwd)
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
    let _j = EnvVar::set("SKYCOT_AUTH_JWT_SECRET", VALID_JWT_SECRET);
    let err = run_startup_checks(false, LOOPBACK_BIND, dir.path())
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
    let _j = EnvVar::set("SKYCOT_AUTH_JWT_SECRET", VALID_JWT_SECRET);

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
    let err = run_startup_checks(false, LOOPBACK_BIND, &cwd)
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
    let _j = EnvVar::set("SKYCOT_AUTH_JWT_SECRET", VALID_JWT_SECRET);
    assert!(
        !provider_key_present(),
        "guard precondition: both keys must be unset"
    );
    let cwd = home_dir.path().to_path_buf();
    let err = run_startup_checks(false, LOOPBACK_BIND, &cwd)
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
    // A valid secret keeps the (skip-proof) fail-closed JWT boot-guard
    // satisfied so the test isolates the "skip bypasses DB/provider
    // checks" property it actually exercises.
    let _j = EnvVar::set("SKYCOT_AUTH_JWT_SECRET", VALID_JWT_SECRET);
    // Path doesn't even need to exist — skip=true short-circuits
    // before any FS access.
    let cwd = std::path::PathBuf::from("/nonexistent-skycot-startup-check");
    run_startup_checks(true, LOOPBACK_BIND, &cwd)
        .await
        .expect("--skip-startup-check must always succeed");
}

// ── sprint-ws1-1c — fail-closed JWT-secret boot-guard ──────────────
//
// The guard runs as the first CRITICAL check (after the `skip`
// short-circuit), gated on production mode via
// `routes_auth_production_mode::is_production_mode`. These tests force
// production with `ARGO_GATEWAY_PROFILE=production` and clear
// `ARGO_GATEWAY_BIND` so a non-loopback ambient bind can't muddy the
// signal. They call with `skip=false`; for the refusing cases the
// guard fires before any FS/DB access (a nonexistent cwd is fine), and
// the passing cases use a full fresh-home + provider-key happy env so
// they reach `Ok`.
const GUARD_CWD: &str = "/nonexistent-skycot-jwt-guard";

/// RAII bundle that forces production mode for the guard tests and
/// clears the demo/bypass escape hatches. Returned as a tuple so a
/// single `let _prod = ...` binding keeps every guard alive (and
/// restores the prior env on drop) without dead-field lints.
fn force_production() -> (EnvVar, EnvVar, EnvVar, EnvVar) {
    (
        EnvVar::set("ARGO_GATEWAY_PROFILE", "production"),
        EnvVar::unset("ARGO_GATEWAY_BIND"),
        EnvVar::unset("ARGO_GATEWAY_AUTH_BYPASS"),
        EnvVar::unset("ARGO_DEMO_MODE"),
    )
}

#[tokio::test]
async fn prod_mode_missing_secret_refuses() {
    let _g = env_lock().await;
    let _prod = force_production();
    let _j = EnvVar::unset("SKYCOT_AUTH_JWT_SECRET");
    let cwd = std::path::PathBuf::from(GUARD_CWD);
    let err = run_startup_checks(false, LOOPBACK_BIND, &cwd)
        .await
        .expect_err("missing secret in production must refuse to boot");
    let msg = format!("{err}");
    assert!(
        msg.contains("SKYCOT_AUTH_JWT_SECRET") && msg.contains("must be set in production"),
        "expected fatal missing-secret message, got: {msg}"
    );
}

#[tokio::test]
async fn prod_mode_empty_secret_refuses() {
    let _g = env_lock().await;
    let _prod = force_production();
    let _j = EnvVar::set("SKYCOT_AUTH_JWT_SECRET", "");
    let cwd = std::path::PathBuf::from(GUARD_CWD);
    let err = run_startup_checks(false, LOOPBACK_BIND, &cwd)
        .await
        .expect_err("empty secret in production must refuse to boot");
    assert!(
        format!("{err}").contains("must be set in production"),
        "expected empty-secret to share the missing-secret message"
    );
}

#[tokio::test]
async fn prod_mode_short_secret_refuses() {
    let _g = env_lock().await;
    let _prod = force_production();
    // 31 bytes — one under the 32-byte HS256 floor.
    let _j = EnvVar::set("SKYCOT_AUTH_JWT_SECRET", "0123456789abcdef0123456789abcde");
    let cwd = std::path::PathBuf::from(GUARD_CWD);
    let err = run_startup_checks(false, LOOPBACK_BIND, &cwd)
        .await
        .expect_err("too-short secret in production must refuse to boot");
    let msg = format!("{err}");
    assert!(
        msg.contains("too short") && msg.contains("31 bytes"),
        "expected too-short message with byte count, got: {msg}"
    );
}

#[tokio::test]
async fn prod_mode_valid_secret_boots() {
    let _g = env_lock().await;
    let _prod = force_production();
    let _j = EnvVar::set("SKYCOT_AUTH_JWT_SECRET", VALID_JWT_SECRET);
    // Full happy env so the run reaches Ok once the guard clears.
    let (home_dir, home_path) = fresh_home();
    let _h = EnvVar::set("ARGO_HOME", home_path.to_str().unwrap());
    let _a = EnvVar::set("ANTHROPIC_API_KEY", "test-key");
    let cwd = home_dir.path().to_path_buf();
    run_startup_checks(false, LOOPBACK_BIND, &cwd)
        .await
        .expect("valid >=32-byte secret in production must boot");
}

#[tokio::test]
async fn prod_mode_bypass_missing_secret_boots() {
    let _g = env_lock().await;
    // Production by profile, but auth-bypass is the documented dev
    // recovery path → the guard must NOT fire even with no secret.
    let _profile = EnvVar::set("ARGO_GATEWAY_PROFILE", "production");
    let _bind = EnvVar::unset("ARGO_GATEWAY_BIND");
    let _bypass = EnvVar::set("ARGO_GATEWAY_AUTH_BYPASS", "1");
    let _demo = EnvVar::unset("ARGO_DEMO_MODE");
    let _j = EnvVar::unset("SKYCOT_AUTH_JWT_SECRET");
    let (home_dir, home_path) = fresh_home();
    let _h = EnvVar::set("ARGO_HOME", home_path.to_str().unwrap());
    let _a = EnvVar::set("ANTHROPIC_API_KEY", "test-key");
    let cwd = home_dir.path().to_path_buf();
    run_startup_checks(false, LOOPBACK_BIND, &cwd)
        .await
        .expect("auth-bypass run without a secret must still boot");
}

#[tokio::test]
async fn prod_mode_demo_missing_secret_boots() {
    let _g = env_lock().await;
    // ARGO_DEMO_MODE requires ARGO_GATEWAY_AUTH_BYPASS=1 (existing
    // demo_mode_compatibility_check); both set, no secret → guard must
    // not fire because demo-mode is an escape hatch.
    let _profile = EnvVar::set("ARGO_GATEWAY_PROFILE", "production");
    let _bind = EnvVar::unset("ARGO_GATEWAY_BIND");
    let _bypass = EnvVar::set("ARGO_GATEWAY_AUTH_BYPASS", "1");
    let _demo = EnvVar::set("ARGO_DEMO_MODE", "1");
    let _j = EnvVar::unset("SKYCOT_AUTH_JWT_SECRET");
    let (home_dir, home_path) = fresh_home();
    let _h = EnvVar::set("ARGO_HOME", home_path.to_str().unwrap());
    let _a = EnvVar::set("ANTHROPIC_API_KEY", "test-key");
    let cwd = home_dir.path().to_path_buf();
    run_startup_checks(false, LOOPBACK_BIND, &cwd)
        .await
        .expect("demo-mode run without a secret must still boot");
}

#[tokio::test]
async fn loopback_dev_missing_secret_boots() {
    let _g = env_lock().await;
    // No production signal at all (dev/test default) → guard inactive
    // even with no secret. This is the 35+ loopback integration-test
    // boot posture; pin that the guard leaves it untouched.
    let _profile = EnvVar::unset("ARGO_GATEWAY_PROFILE");
    let _bind = EnvVar::unset("ARGO_GATEWAY_BIND");
    let _bypass = EnvVar::unset("ARGO_GATEWAY_AUTH_BYPASS");
    let _demo = EnvVar::unset("ARGO_DEMO_MODE");
    let _j = EnvVar::unset("SKYCOT_AUTH_JWT_SECRET");
    let (home_dir, home_path) = fresh_home();
    let _h = EnvVar::set("ARGO_HOME", home_path.to_str().unwrap());
    let _a = EnvVar::set("ANTHROPIC_API_KEY", "test-key");
    let cwd = home_dir.path().to_path_buf();
    run_startup_checks(false, LOOPBACK_BIND, &cwd)
        .await
        .expect("loopback dev run without a secret must still boot");
}

// ── sprint-ws1-1c-fix — bind-aware production detection ────────────
//
// The original sprint-ws1-1c guard gated on the bind-BLIND
// `is_production_mode()`, which checks the compiled default host
// (loopback) and `ARGO_GATEWAY_BIND`, but NOT the explicit `--bind`
// CLI value the socket actually binds to. So a real production deploy
// — `argo gateway start --bind 0.0.0.0` with no `ARGO_GATEWAY_PROFILE`
// export — produced `is_production_mode() == false`, the guard was
// skipped, and the fail-open hole survived. These two tests pin the
// fix: production is now detected from the non-loopback `--bind`
// value alone (no profile env), exactly as the sibling bearer-token
// and enterprise-encryption boot gates already do.

/// A non-loopback `--bind` value WITHOUT any production-profile env
/// must, on its own, activate the guard and refuse a missing secret.
/// This is the deploy posture the bind-blind detector missed and is
/// the case the profile-env-only tests above cannot exercise.
#[tokio::test]
async fn non_loopback_bind_missing_secret_refuses() {
    let _g = env_lock().await;
    // No profile env, no ARGO_GATEWAY_BIND, no bypass/demo — the ONLY
    // production signal is the explicit non-loopback `--bind`.
    let _profile = EnvVar::unset("ARGO_GATEWAY_PROFILE");
    let _bind = EnvVar::unset("ARGO_GATEWAY_BIND");
    let _bypass = EnvVar::unset("ARGO_GATEWAY_AUTH_BYPASS");
    let _demo = EnvVar::unset("ARGO_DEMO_MODE");
    let _j = EnvVar::unset("SKYCOT_AUTH_JWT_SECRET");
    let cwd = std::path::PathBuf::from(GUARD_CWD);
    let err = run_startup_checks(false, "0.0.0.0", &cwd)
        .await
        .expect_err("non-loopback --bind in production must refuse a missing secret");
    let msg = format!("{err}");
    assert!(
        msg.contains("SKYCOT_AUTH_JWT_SECRET") && msg.contains("must be set in production"),
        "expected fatal missing-secret message, got: {msg}"
    );
}

/// Inverse pin: a loopback `--bind` with no profile env is NOT
/// production, so the guard stays inactive and a missing secret still
/// boots. Proves the fix is bind-PRECISE (it didn't just start always
/// enforcing) — the 35+ loopback integration boots remain green.
#[tokio::test]
async fn loopback_bind_missing_secret_boots() {
    let _g = env_lock().await;
    let _profile = EnvVar::unset("ARGO_GATEWAY_PROFILE");
    let _bind = EnvVar::unset("ARGO_GATEWAY_BIND");
    let _bypass = EnvVar::unset("ARGO_GATEWAY_AUTH_BYPASS");
    let _demo = EnvVar::unset("ARGO_DEMO_MODE");
    let _j = EnvVar::unset("SKYCOT_AUTH_JWT_SECRET");
    let (home_dir, home_path) = fresh_home();
    let _h = EnvVar::set("ARGO_HOME", home_path.to_str().unwrap());
    let _a = EnvVar::set("ANTHROPIC_API_KEY", "test-key");
    let cwd = home_dir.path().to_path_buf();
    run_startup_checks(false, "127.0.0.1:7400", &cwd)
        .await
        .expect("loopback --bind without a secret must still boot");
}
