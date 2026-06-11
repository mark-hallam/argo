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
//! These tests mutate PROCESS-global env vars, so they serialize on a
//! single mutex and use [`RunEnv`] — an RAII guard that snapshots every
//! env var it touches and restores each to its pre-test value on drop.
//! That makes a panic in one test unable to leak state into the next.
//!
//! Crucially, they do NOT chdir. The artifact bundle root is steered to
//! the tempdir via the `ARGO_OUTPUTS_ROOT` seam ([`RunEnv::point_outputs_to_tmp`])
//! rather than the process cwd. An earlier revision chdir'd into the
//! tempdir and restored cwd to `CARGO_MANIFEST_DIR` on drop; that global
//! cwd mutation leaked `.skycot/outputs/<job>/` bundles into the crate
//! source tree whenever a write raced the restore window, tripping the
//! worker-loop dirty-tree claim guard (`fix-worker-outputs-tests-write-tempdir`).

#![allow(clippy::expect_used)]

use super::{filter_to_worker_tool_set, run_worker_now, RunBlocked};
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
    "ARGO_OUTPUTS_ROOT",
];

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
}

/// RAII test environment: holds the serialization guard + tempdir,
/// snapshots [`TOUCHED_ENV`], and restores env on drop. Does NOT touch
/// the process-global cwd — see the module-level isolation note.
struct RunEnv {
    _guard: OwnedMutexGuard<()>,
    tmp: TempDir,
    saved_env: Vec<(&'static str, Option<String>)>,
}

impl RunEnv {
    /// Acquire the lock, snapshot env, and point `ARGO_HOME` +
    /// `ARGO_VERTICALS_DIR` at the tempdir / workspace. Callers that run
    /// a real turn additionally call [`Self::point_outputs_to_tmp`] so
    /// the artifact bundle lands under the tempdir.
    async fn new() -> Self {
        let guard = crate::handlers::gateway::shared_env_lock().await;
        let tmp = TempDir::new().expect("tempdir");
        let saved_env = TOUCHED_ENV
            .iter()
            .map(|&k| (k, std::env::var(k).ok()))
            .collect();
        std::env::set_var("ARGO_HOME", tmp.path().join("home"));
        std::fs::create_dir_all(tmp.path().join("home")).expect("home dir");
        std::env::set_var("ARGO_VERTICALS_DIR", workspace_root().join("verticals"));
        Self {
            _guard: guard,
            tmp,
            saved_env,
        }
    }

    fn path(&self) -> &std::path::Path {
        self.tmp.path()
    }

    fn root(&self) -> String {
        self.tmp.path().to_string_lossy().to_string()
    }

    /// Steer `process_job`'s artifact bundle root to the tempdir via the
    /// `ARGO_OUTPUTS_ROOT` seam (the bundle lands at
    /// `<tmp>/.skycot/outputs/<job>/`). Replaces an earlier `chdir`-based
    /// approach that mutated the process-global cwd and leaked bundles
    /// into the crate source tree on restore.
    fn point_outputs_to_tmp(&self) {
        std::env::set_var("ARGO_OUTPUTS_ROOT", self.tmp.path());
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
        // Restore each touched env var to its pre-test value. cwd is never
        // mutated by this suite, so there is nothing to restore there.
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

    let result = run_worker_now(&store, &worker, "do a thing", &[], None)
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

    let result = run_worker_now(&store, &worker, "do a thing", &[], None)
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
    env.point_outputs_to_tmp();

    let store = mem_store().await;
    let tenant = fresh_tenant_workspace(&store, &env.root()).await;
    let worker = add_worker(&store, tenant, REAL_VERTICAL).await;

    let outcome = run_worker_now(&store, &worker, "Summarise today's open tickets.", &[], None)
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

/// (b-canary) Regression guard for `fix-worker-outputs-tests-write-tempdir`:
/// a real run with the `ARGO_OUTPUTS_ROOT` seam pointed at the tempdir
/// writes its artifact bundle UNDER the tempdir and leaves NO `.skycot/`
/// directory in the crate source tree. This pins the root fix for the
/// 2026-06-12 worker stall, where the suite's old chdir-to-`CARGO_MANIFEST_DIR`
/// discipline leaked `.skycot/outputs/<job>/` into `crates/argo-cli/`,
/// tripping the worker-loop dirty-tree claim guard.
#[tokio::test]
async fn run_writes_bundle_into_tempdir_not_crate_tree() {
    let env = RunEnv::new().await;
    env.set("ANTHROPIC_API_KEY", "test-key-present");
    env.set("ARGO_WORKER_RUN_PROVIDER", "mock");
    env.point_outputs_to_tmp();

    let store = mem_store().await;
    let tenant = fresh_tenant_workspace(&store, &env.root()).await;
    let worker = add_worker(&store, tenant, REAL_VERTICAL).await;

    run_worker_now(&store, &worker, "Summarise today's open tickets.", &[], None)
        .await
        .expect("no internal error")
        .expect("a real turn should have run (not blocked)");

    // The bundle landed under the tempdir's outputs root...
    let tmp_outputs = env.path().join(".skycot").join("outputs");
    assert!(
        tmp_outputs.exists(),
        "bundle should be written under the tempdir outputs root: {}",
        tmp_outputs.display()
    );
    // ...and NOTHING leaked into the crate source tree.
    let crate_skycot = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".skycot");
    assert!(
        !crate_skycot.exists(),
        "no .skycot/ may appear in the crate source tree: {}",
        crate_skycot.display()
    );
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
    env.point_outputs_to_tmp();

    let store = mem_store().await;
    let tenant = fresh_tenant_workspace(&store, &env.root()).await;
    let worker = add_worker(&store, tenant, REAL_VERTICAL).await;

    run_worker_now(&store, &worker, "Summarise today's open tickets.", &[], None)
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

// ---- Genesis tenant-scoped run path (this brief) ---------------------
//
// A Genesis-hired teammate is materialized under
// `verticals/.genesis/<tenant>/<vid>/`, NOT the flat catalog. Before this
// brief the run gate used the flat-only loader, so EVERY genesis teammate
// returned the honest-but-wrong 409 `no_runnable_role`. These pin that the
// gate now resolves a genesis vertical for its OWN tenant (it runs) and
// stays fail-closed for a DIFFERENT tenant (it blocks).

/// Materialize a minimal genesis vertical under
/// `<vroot>/.genesis/<sanitize_tenant(tenant_id)>/<vid>/`.
fn write_genesis_vertical(vroot: &std::path::Path, tenant_id: &str, vid: &str) {
    let slug = argo_vertical::sanitize_tenant(tenant_id);
    let dir = vroot.join(".genesis").join(slug).join(vid);
    std::fs::create_dir_all(&dir).expect("mkdir genesis");
    std::fs::write(
        dir.join("vertical.toml"),
        format!("schema_version = \"1\"\n[vertical]\nid = \"{vid}\"\nname = \"G\"\nversion = \"0.1.0\"\n"),
    )
    .expect("write genesis manifest");
}

/// A Genesis worker whose pack lives under its OWN tenant's segment runs
/// (no 409) — through the REAL `process_job` engine via the mock seam.
#[tokio::test]
async fn genesis_worker_runs_for_own_tenant() {
    let env = RunEnv::new().await;
    env.set("ANTHROPIC_API_KEY", "test-key-present");
    env.set("ARGO_WORKER_RUN_PROVIDER", "mock");
    // Point the loader at a genesis tree we control (NOT the repo's
    // verticals/ dir, which must stay clean).
    let vroot = env.path().join("vroot");
    env.set("ARGO_VERTICALS_DIR", vroot.to_string_lossy().as_ref());
    env.point_outputs_to_tmp();

    let store = mem_store().await;
    let tenant = fresh_tenant_workspace(&store, &env.root()).await;
    let vid = "genesis-personal-019eaf2871121498";
    write_genesis_vertical(&vroot, &tenant.to_string(), vid);
    let worker = add_worker(&store, tenant, vid).await;

    let outcome = run_worker_now(&store, &worker, "Do your next task.", &[], None)
        .await
        .expect("no internal error")
        .expect("genesis worker should run for its own tenant, not be blocked");
    assert_eq!(outcome.status, "completed", "genesis turn completes via mock");
    assert_eq!(session_count(&store).await, 1, "exactly one genesis turn ran");
    assert!(env.path().exists());
}

/// A Genesis worker hired under tenant B cannot run tenant A's pack —
/// fail-closed cross-tenant: honest 409, NO turn.
#[tokio::test]
async fn genesis_worker_blocked_cross_tenant() {
    let env = RunEnv::new().await;
    env.set("ANTHROPIC_API_KEY", "test-key-present");
    let vroot = env.path().join("vroot");
    env.set("ARGO_VERTICALS_DIR", vroot.to_string_lossy().as_ref());

    let store = mem_store().await;
    // Pack materialized for tenant ALICE...
    let alice = fresh_tenant_workspace(&store, &env.root()).await;
    let vid = "genesis-personal-019eaf2871121498";
    write_genesis_vertical(&vroot, &alice.to_string(), vid);
    // ...but the worker is hired under a DIFFERENT tenant BOB (distinct
    // cwd root → distinct tenant).
    let bob = store
        .ensure_workspace("bob-ws", &format!("{}/bob", env.root()))
        .await
        .expect("ensure bob workspace")
        .0;
    let worker = add_worker(&store, bob, vid).await;

    let result = run_worker_now(&store, &worker, "Do a thing", &[], None)
        .await
        .expect("no internal error");
    match result {
        Err(RunBlocked::NoVertical) => {}
        other => panic!("expected cross-tenant NoVertical block, got {other:?}"),
    }
    assert_eq!(
        session_count(&store).await,
        0,
        "no cross-tenant genesis run should have started"
    );
}

// ---- `approve_tools` membership filter (keystone fix) -----------------
//
// Pure-function pins for the no-privilege-escalation property: a
// requested tool is granted ONLY when the worker would legitimately
// hold it under the runtime's own least-privilege derivation
// (manifest `[actions]` + `[tools].allowed`, deny-wins, plus the
// always-safe MINIMAL_TEAMMATE_TOOLS).

/// Parse a minimal manifest with the given tool + action declarations.
fn manifest_with(allowed: &str, denied: &str, action: Option<&str>) -> argo_vertical::VerticalManifest {
    let actions = action
        .map(|a| {
            format!(
                "[actions]\n[actions.{a}]\nrisk = \"low\"\nrequires_approval = false\ndescription = \"d\"\n"
            )
        })
        .unwrap_or_default();
    let toml = format!(
        "schema_version = \"1\"\n[vertical]\nid = \"t\"\nname = \"T\"\nversion = \"0.1.0\"\n\
         [tools]\nallowed = [{allowed}]\ndenied = [{denied}]\n{actions}"
    );
    argo_vertical::parse(&toml).expect("parse test manifest")
}

#[test]
fn filter_keeps_allowed_and_action_tools_drops_unknown_and_denied() {
    let m = manifest_with("\"file_read\", \"web_search\"", "\"terminal\"", Some("draft_reply"));
    let requested: Vec<String> = [
        "file_read",   // [tools].allowed → kept
        "draft_reply", // [actions] vocabulary → kept
        "terminal",    // [tools].denied → dropped (deny wins)
        "wire_funds",  // not declared anywhere → dropped
    ]
    .iter()
    .map(|s| (*s).to_string())
    .collect();
    assert_eq!(
        filter_to_worker_tool_set(&m, &requested),
        vec!["file_read".to_string(), "draft_reply".to_string()]
    );
}

#[test]
fn filter_admits_minimal_teammate_tools_unless_denied() {
    // `memory` is in MINIMAL_TEAMMATE_TOOLS — the runtime grants it to
    // every fenced teammate, so an operator may pre-approve it even
    // when the manifest doesn't list it...
    let m = manifest_with("\"file_read\"", "", None);
    let req = vec!["memory".to_string()];
    assert_eq!(filter_to_worker_tool_set(&m, &req), vec!["memory".to_string()]);
    // ...but an explicit manifest deny still wins.
    let m_denied = manifest_with("\"file_read\"", "\"memory\"", None);
    assert!(filter_to_worker_tool_set(&m_denied, &req).is_empty());
}

#[test]
fn filter_dedupes_and_trims_and_handles_no_tools_block() {
    let m = manifest_with("\"file_read\"", "", None);
    let req = vec![
        " file_read ".to_string(),
        "file_read".to_string(),
        String::new(),
    ];
    assert_eq!(filter_to_worker_tool_set(&m, &req), vec!["file_read".to_string()]);

    // A manifest with no [tools]/[actions] declares nothing → deny-all
    // for non-minimal tools (the safest default for a novel role).
    let bare = argo_vertical::parse(
        "schema_version = \"1\"\n[vertical]\nid = \"t\"\nname = \"T\"\nversion = \"0.1.0\"\n",
    )
    .expect("parse bare manifest");
    assert!(filter_to_worker_tool_set(&bare, &["file_read".to_string()]).is_empty());
}
