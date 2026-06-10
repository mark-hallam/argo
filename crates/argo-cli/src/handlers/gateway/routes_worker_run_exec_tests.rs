//! Tests for the real-run exec path (`routes_worker_run_exec`).
//!
//! Two truth-bounded guarantees are pinned here:
//!
//! 1. **Validate-before-claim** — a worker whose `vertical_id` does not
//!    load returns the honest `409` block with NO turn claimed (no
//!    session, no `completed_jobs` row); likewise a missing provider
//!    key blocks before any work.
//! 2. **Happy-path shape** — with a loadable vertical + provider key,
//!    the run goes through the REAL `process_job` engine (driven by the
//!    `mock` provider via the `ARGO_WORKER_RUN_PROVIDER` test seam, so
//!    no network LLM call) and reports `status: "completed"` with a real
//!    `session_id` and `chars > 0`.
//!
//! The mock seam lets us exercise the genuine engine wiring end-to-end
//! without a provider round-trip — the assertion is "a turn really ran
//! and a completion was recorded", which is exactly the claim the
//! handler makes.
//!
//! ## Isolation discipline
//!
//! These tests mutate PROCESS-global state (env vars + cwd), so they
//! serialize on a single mutex and use [`RunEnv`] — an RAII guard that
//! snapshots every env var it touches, chdir's into the test tempdir,
//! and on drop restores cwd to a STABLE absolute path
//! (`CARGO_MANIFEST_DIR`, which always exists) and restores each env var
//! to its pre-test value. That makes a panic in one test unable to leak
//! state into the next.

#![allow(clippy::expect_used)]

use super::{run_worker_now, RunBlocked};
use argo_identity::{IdentityStore, Persona, Role, Worker};
use argo_store::SessionStore;
use argo_types::TenantId;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::OwnedMutexGuard;

/// One real, loadable vertical id from the workspace `verticals/` dir
/// (`RunEnv` points `ARGO_VERTICALS_DIR` there). Used for the
/// runnable-worker paths.
const REAL_VERTICAL: &str = "customer-support";

/// Env vars this suite mutates — snapshotted + restored by [`RunEnv`].
const TOUCHED_ENV: &[&str] = &[
    "ARGO_HOME",
    "ARGO_VERTICALS_DIR",
    "ANTHROPIC_API_KEY",
    "OPENAI_API_KEY",
    "ARGO_WORKER_RUN_PROVIDER",
];

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
}

/// RAII test environment: holds the serialization guard + tempdir,
/// snapshots [`TOUCHED_ENV`], and restores cwd + env on drop.
struct RunEnv {
    _guard: OwnedMutexGuard<()>,
    tmp: TempDir,
    saved_env: Vec<(&'static str, Option<String>)>,
    stable_cwd: PathBuf,
}

impl RunEnv {
    /// Acquire the lock, snapshot env, and point `ARGO_HOME` +
    /// `ARGO_VERTICALS_DIR` at the tempdir / workspace. Does NOT chdir
    /// yet — callers that run a real turn call [`Self::chdir_tmp`].
    async fn new() -> Self {
        let guard = crate::handlers::gateway::shared_env_lock().await;
        let tmp = TempDir::new().expect("tempdir");
        let saved_env = TOUCHED_ENV
            .iter()
            .map(|&k| (k, std::env::var(k).ok()))
            .collect();
        let stable_cwd = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        std::env::set_var("ARGO_HOME", tmp.path().join("home"));
        std::fs::create_dir_all(tmp.path().join("home")).expect("home dir");
        std::env::set_var("ARGO_VERTICALS_DIR", workspace_root().join("verticals"));
        Self {
            _guard: guard,
            tmp,
            saved_env,
            stable_cwd,
        }
    }

    fn path(&self) -> &std::path::Path {
        self.tmp.path()
    }

    fn root(&self) -> String {
        self.tmp.path().to_string_lossy().to_string()
    }

    /// chdir into the tempdir so `process_job` artifact writes land in
    /// (and are cleaned up with) the tempdir.
    fn chdir_tmp(&self) {
        std::env::set_current_dir(self.tmp.path()).expect("chdir tmp");
    }

    fn set(&self, k: &str, v: &str) {
        std::env::set_var(k, v);
    }

    fn unset(&self, k: &str) {
        std::env::remove_var(k);
    }
}

impl Drop for RunEnv {
    fn drop(&mut self) {
        // Restore cwd to a stable, always-present dir FIRST (never a
        // deleted tempdir), then restore each touched env var.
        let _ = std::env::set_current_dir(&self.stable_cwd);
        for (k, v) in &self.saved_env {
            match v {
                Some(val) => std::env::set_var(k, val),
                None => std::env::remove_var(k),
            }
        }
    }
}

async fn mem_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("open mem db");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

/// Register a workspace under a FRESH tenant and return both. The exec
/// path resolves the workspace by the worker's own tenant via
/// `first_workspace_for_tenant`, so the worker must be created under the
/// same tenant the workspace belongs to.
async fn fresh_tenant_workspace(store: &Arc<SessionStore>, root: &str) -> TenantId {
    let (tenant, _ws) = store
        .ensure_workspace("exec-test-ws", root)
        .await
        .expect("ensure_workspace");
    tenant
}

async fn add_worker(store: &Arc<SessionStore>, tenant: TenantId, vertical: &str) -> Worker {
    let worker = Worker::draft(
        tenant,
        vertical,
        Persona::named("Mariah", "human:mark"),
        Role::new("Resolve customer questions."),
    );
    IdentityStore::new(store)
        .create_worker(&worker)
        .await
        .expect("create worker");
    worker
}

async fn session_count(store: &Arc<SessionStore>) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM session")
        .fetch_one(store.pool())
        .await
        .expect("count sessions")
}

/// (a) A worker whose vertical does NOT load → honest 409, no turn.
#[tokio::test]
async fn run_with_unloadable_vertical_blocks_no_vertical() {
    let env = RunEnv::new().await;
    // Provider key PRESENT so we isolate the no-vertical branch.
    env.set("ANTHROPIC_API_KEY", "test-key-not-used");

    let store = mem_store().await;
    let tenant = fresh_tenant_workspace(&store, &env.root()).await;
    let worker = add_worker(&store, tenant, "definitely-not-a-real-vertical").await;

    let result = run_worker_now(&store, &worker, "do a thing")
        .await
        .expect("no internal error");
    match result {
        Err(RunBlocked::NoVertical) => {}
        other => panic!("expected NoVertical block, got {other:?}"),
    }
    // Truth-bound: NO session row was created (no turn ran).
    assert_eq!(session_count(&store).await, 0, "no turn should have started");
}

/// (a') No provider key → honest 409, no turn — even with a good vertical.
#[tokio::test]
async fn run_without_provider_key_blocks_no_provider() {
    let env = RunEnv::new().await;
    env.unset("ANTHROPIC_API_KEY");
    env.unset("OPENAI_API_KEY");

    let store = mem_store().await;
    let tenant = fresh_tenant_workspace(&store, &env.root()).await;
    let worker = add_worker(&store, tenant, REAL_VERTICAL).await;

    let result = run_worker_now(&store, &worker, "do a thing")
        .await
        .expect("no internal error");
    assert!(
        matches!(result, Err(RunBlocked::NoProvider)),
        "expected NoProvider block when no key is set, got {result:?}"
    );
    assert_eq!(
        session_count(&store).await,
        0,
        "no turn should have started without a provider"
    );
}

/// (b) Happy path: loadable vertical + provider key + mock-provider seam
/// → the REAL engine runs one turn and a completion is recorded.
#[tokio::test]
async fn run_happy_path_completes_with_real_fields() {
    let env = RunEnv::new().await;
    env.set("ANTHROPIC_API_KEY", "test-key-present");
    // Drive the genuine `process_job` engine with the deterministic mock
    // provider — exercises every phase with NO network call.
    env.set("ARGO_WORKER_RUN_PROVIDER", "mock");
    env.chdir_tmp();

    let store = mem_store().await;
    let tenant = fresh_tenant_workspace(&store, &env.root()).await;
    let worker = add_worker(&store, tenant, REAL_VERTICAL).await;

    let outcome = run_worker_now(&store, &worker, "Summarise today's open tickets.")
        .await
        .expect("no internal error")
        .expect("a real turn should have run (not blocked)");

    assert_eq!(outcome.status, "completed", "mock turn completes cleanly");
    assert!(!outcome.session_id.is_empty(), "a real session id is reported");
    assert!(outcome.chars > 0, "the mock reply has non-zero length");

    // Truth-bound: exactly one session row + one completed_jobs row.
    assert_eq!(session_count(&store).await, 1, "exactly one turn ran");
    let completed: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM memory_entry WHERE body LIKE '%completed_jobs%'")
            .fetch_one(store.pool())
            .await
            .expect("count completed");
    assert_eq!(completed, 1, "engine recorded exactly one completion");
    // path() kept alive via `env`; touch it so the field isn't dead.
    assert!(env.path().exists());
}

/// (c) Worker attribution (e2e R2-F1): a worker-run turn stamps the
/// run's worker onto the session row. The audit feed resolves a turn
/// row's `worker` from `session.worker_id` (the agent runner copies it
/// onto every `turn_start`/`tool_call`/`tool_result`/`turn_end` it
/// emits), so a NULL here is exactly what made a hired worker's live
/// work show as `worker: {"id":"","name":"unknown"}`. Pinning the
/// stamped column proves the fix at the source.
#[tokio::test]
async fn run_stamps_worker_id_onto_session() {
    let env = RunEnv::new().await;
    env.set("ANTHROPIC_API_KEY", "test-key-present");
    // Mock seam → the REAL `process_job` engine, no network LLM call.
    env.set("ARGO_WORKER_RUN_PROVIDER", "mock");
    env.chdir_tmp();

    let store = mem_store().await;
    let tenant = fresh_tenant_workspace(&store, &env.root()).await;
    let worker = add_worker(&store, tenant, REAL_VERTICAL).await;

    run_worker_now(&store, &worker, "Summarise today's open tickets.")
        .await
        .expect("no internal error")
        .expect("a real turn should have run (not blocked)");

    // Exactly one session ran; its `worker_id` BLOB must equal the run's
    // worker. Without the fix this column is NULL → "unknown" in the feed.
    let worker_blob: Vec<u8> = sqlx::query_scalar("SELECT worker_id FROM session LIMIT 1")
        .fetch_one(store.pool())
        .await
        .expect("session worker_id should be stamped (not NULL)");
    assert_eq!(
        worker_blob,
        worker.id.0.as_bytes().to_vec(),
        "the worker-run session must attribute to the run's worker, not 'unknown'"
    );
    assert!(env.path().exists());
}
