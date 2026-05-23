//! # Mission Claim 4 MVP row 6 — `POST /workers/<id>/skills/<sid>/promote`
//! audit-emit pin.
//!
//! Source: evaluator round 30 SHARED.md handoff (2026-05-23T22:30:00Z).
//!
//! The pre-existing assertion in `routes_worker_skills_tests.rs`
//! (`promote_attaches_skill_and_writes_audit_row`) only checks that
//! `COUNT(*) FROM audit_record WHERE kind = 'skill.candidate.promoted'`
//! is at least 1. A refactor that dropped `payload[skill_id]`, inverted
//! `promotion_kind`, dropped `AuditSink::new_with_tenant` stamping, or
//! leaked rows across tenants would NOT flip the existing test red —
//! the assertion is theater-grade. This file ratchets each of those
//! invariants up to a byte-exact pin.
//!
//! ## Sensible deviations from the round-30 brief
//!
//! 1. **Path.** Brief names `crates/argo-gateway/tests/...`. The
//!    `argo-gateway` crate does not exist on `origin/main`; the gateway
//!    lives in `argo-cli/src/handlers/gateway/`. `promote_response()`
//!    is `pub(super)`, so an integration test under `crates/argo-cli/tests/`
//!    cannot reach it. Matches the in-module pattern already in
//!    `routes_worker_skills_tests.rs`.
//! 2. **Test 4 substitution.** Brief asks for a `FaultyAuditSink`
//!    injectable proving `ratchet must roll back` when audit-write
//!    fails. `helpers::emit_audit` constructs `AuditSink::new_with_tenant`
//!    inline (no injection seam), and the current call order writes
//!    the ratchet before the audit row in two separate awaits — the
//!    contract today is "audit is best-effort, ratchet is the source
//!    of truth", not atomic. Adding a `FaultyAuditSink` seam + an
//!    atomic transaction boundary is a production refactor that lives
//!    outside this brief's read-only may-touch on `routes_worker_skills.rs`.
//!    Substituted: pin that the `audit_record.principal_id` column is
//!    stamped from `X-Argo-Principal` (the brief's `promoted_by` field,
//!    expressed via the existing principal-stamp seam). Anti-theater
//!    in the same dimension — guards against a regression that drops
//!    the principal-id stamping from the promote audit row.
//!
//! ## Payload shape (current, byte-exact)
//!
//! After `helpers::emit_audit` builds and `AuditSink::write_with_tenant`
//! stamps, the `audit_record.payload_json` for `kind =
//! 'skill.candidate.promoted'` carries exactly:
//!
//! ```json
//! {
//!   "endpoint": "worker.skills.promoted",
//!   "tenant_id": "<workspace tenant uuid string>",
//!   "worker_id": "<worker uuid string>",
//!   "skill_id":  "<skill uuid string>",
//!   "promotion_kind": "learned_to_active"
//! }
//! ```

#![cfg(test)]
#![allow(clippy::too_many_lines)]

use super::promote_response;
use argo_identity::{IdentityStore, Persona, Role, Worker};
use argo_skill::SkillStore;
use argo_store::SessionStore;
use argo_types::{SkillId, TenantId};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, MutexGuard};

// ---------------------------------------------------------------------------
// Test infrastructure (mirrors routes_worker_skills_tests.rs)
// ---------------------------------------------------------------------------

async fn env_lock() -> MutexGuard<'static, ()> {
    static M: Mutex<()> = Mutex::const_new(());
    M.lock().await
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
}

fn set_test_env(tmp: &std::path::Path) {
    std::env::set_var("ARGO_HOME", tmp.join("home"));
    std::fs::create_dir_all(tmp.join("home")).expect("home dir");
    std::env::set_var("ARGO_VERTICALS_DIR", workspace_root().join("verticals"));
}

async fn mem_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("open mem db");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

async fn workspace_tenant(store: &Arc<SessionStore>) -> TenantId {
    let cwd = std::env::current_dir().expect("cwd");
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, _) = store
        .ensure_workspace(name, &root)
        .await
        .expect("ensure workspace");
    tenant
}

async fn add_worker(store: &Arc<SessionStore>, tenant: TenantId, name: &str) -> Worker {
    let mut worker = Worker::draft(
        tenant,
        "support",
        Persona::named(name, "human:mark"),
        Role::new("Resolve customer questions."),
    );
    worker.trust_grade = 3;
    IdentityStore::new(store)
        .create_worker(&worker)
        .await
        .expect("create worker");
    worker
}

fn skill_doc(name: &str, version: &str) -> String {
    format!(
        "---\nname: {name}\nversion: \"{version}\"\ndescription: d\n\
         provenance:\n  source: builtin\n  hash: sha256-x\n---\nbody"
    )
}

async fn install_skill(store: &Arc<SessionStore>, name: &str) -> SkillId {
    let skills = SkillStore::new(Arc::clone(store));
    let skill = skills
        .install(&skill_doc(name, "0.1.0"), None)
        .await
        .expect("install skill");
    skill.id
}

fn post_req_no_principal(path: &str, body: &str) -> String {
    format!(
        "POST {path} HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
}

fn post_req_with_principal(path: &str, body: &str, principal: &str) -> String {
    format!(
        "POST {path} HTTP/1.1\r\nHost: x\r\nX-Argo-Principal: {principal}\r\n\
         Content-Length: {}\r\n\r\n{body}",
        body.len()
    )
}

/// Fetch the single `skill.candidate.promoted` audit row that the
/// caller just wrote. Returns `(kind, payload_json, tenant_id,
/// principal_id)` as stored on disk — the test asserts directly
/// against the column values.
async fn fetch_promoted_audit_row(
    store: &Arc<SessionStore>,
) -> (String, String, String, Option<String>) {
    let row: (String, String, String, Option<String>) = sqlx::query_as(
        "SELECT kind, payload_json, tenant_id, principal_id \
         FROM audit_record \
         WHERE kind = 'skill.candidate.promoted' \
         ORDER BY seq DESC LIMIT 1",
    )
    .fetch_one(store.pool())
    .await
    .expect("fetch promoted audit row");
    row
}

// ---------------------------------------------------------------------------
// Test 1 — payload shape pin
// ---------------------------------------------------------------------------

/// Byte-exact assertion on every payload key the locked Skills tab +
/// `/audit` timeline depend on. A refactor that drops `skill_id`,
/// inverts `promotion_kind`, or renames `endpoint` flips this red.
#[tokio::test]
async fn promoted_audit_row_payload_carries_skill_worker_promotion_kind_and_tenant() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    let skill_id = install_skill(&store, "payload-pin").await;

    let path = format!(
        "/api/v1/workers/{}/skills/{}/promote",
        worker.id, skill_id.0
    );
    let (status, body) = promote_response(
        &worker.id.to_string(),
        &skill_id.0.to_string(),
        &post_req_no_principal(&path, "{}"),
        Some(&store),
    )
    .await;
    assert_eq!(status, "200 OK", "body: {body}");

    let (kind, payload_json, _tenant_col, _principal) = fetch_promoted_audit_row(&store).await;
    assert_eq!(kind, "skill.candidate.promoted");
    let payload: Value = serde_json::from_str(&payload_json).expect("payload parse");
    assert_eq!(
        payload["endpoint"],
        Value::String("worker.skills.promoted".to_string()),
        "endpoint dimension drives audit timeline grouping: {payload}"
    );
    assert_eq!(
        payload["worker_id"],
        Value::String(worker.id.to_string()),
        "worker_id binds the row to the caller's worker: {payload}"
    );
    assert_eq!(
        payload["skill_id"],
        Value::String(skill_id.0.to_string()),
        "skill_id binds the row to the promoted skill: {payload}"
    );
    assert_eq!(
        payload["promotion_kind"],
        Value::String("learned_to_active".to_string()),
        "promotion_kind direction must not invert: {payload}"
    );
    assert_eq!(
        payload["tenant_id"],
        Value::String(tenant.to_string()),
        "payload-side tenant_id mirrors the denorm column: {payload}"
    );
}

// ---------------------------------------------------------------------------
// Test 2 — denorm tenant_id stamping
// ---------------------------------------------------------------------------

/// `AuditSink::new_with_tenant` is load-bearing: it stamps the denorm
/// `audit_record.tenant_id` column with the workspace tenant. A
/// refactor that swaps to bare `AuditSink::new()` would leave the
/// column as `__legacy__` / `ws-default`. This test guards the
/// stamping path.
#[tokio::test]
async fn promoted_audit_row_denorm_tenant_id_column_matches_workspace_tenant() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    let skill_id = install_skill(&store, "tenant-stamp-pin").await;

    let path = format!(
        "/api/v1/workers/{}/skills/{}/promote",
        worker.id, skill_id.0
    );
    let (status, _body) = promote_response(
        &worker.id.to_string(),
        &skill_id.0.to_string(),
        &post_req_no_principal(&path, "{}"),
        Some(&store),
    )
    .await;
    assert_eq!(status, "200 OK");

    let (_kind, _payload, tenant_col, _principal) = fetch_promoted_audit_row(&store).await;
    assert_eq!(
        tenant_col,
        tenant.to_string(),
        "denorm tenant_id column must match workspace tenant — \
         dropping new_with_tenant would leave this `__legacy__` or `ws-default`"
    );
    assert_ne!(
        tenant_col, "__legacy__",
        "row must not fall back to legacy sentinel"
    );
    assert_ne!(
        tenant_col, "ws-default",
        "row must not fall back to default-sink sentinel"
    );
}

// ---------------------------------------------------------------------------
// Test 3 — cross-tenant isolation
// ---------------------------------------------------------------------------

/// A promote in tenant_a (the cwd-resolved workspace tenant) must
/// leave zero rows in tenant_b's audit feed when queried by
/// `tenant_id` column. Pins the guarantee that
/// `mission-enterprise-cross-tenant-audit` filters can rely on the
/// denorm column for hard isolation — no `__legacy__` smuggling, no
/// `ws-default` smuggling, no missing column.
#[tokio::test]
async fn promote_in_tenant_a_does_not_leak_into_tenant_b_audit_rows() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant_a = workspace_tenant(&store).await;
    // tenant_b exists in the same store but the gateway never resolves
    // its workspace from cwd; this models the cross-tenant query an
    // operator on tenant_b would run from their own session.
    let tenant_b = TenantId::new();
    assert_ne!(tenant_a, tenant_b, "tenant ids must be distinct");

    let worker = add_worker(&store, tenant_a, "Maya").await;
    let skill_id = install_skill(&store, "isolation-pin").await;
    let path = format!(
        "/api/v1/workers/{}/skills/{}/promote",
        worker.id, skill_id.0
    );
    let (status, _body) = promote_response(
        &worker.id.to_string(),
        &skill_id.0.to_string(),
        &post_req_no_principal(&path, "{}"),
        Some(&store),
    )
    .await;
    assert_eq!(status, "200 OK");

    // tenant_b's perspective: zero promote rows.
    let leaked: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_record \
         WHERE kind = 'skill.candidate.promoted' AND tenant_id = ?",
    )
    .bind(tenant_b.to_string())
    .fetch_one(store.pool())
    .await
    .expect("count");
    assert_eq!(
        leaked, 0,
        "tenant_b must see zero `skill.candidate.promoted` rows after a tenant_a promote"
    );

    // tenant_a's perspective: exactly one row (sanity check the same
    // query returns >=1 for the caller's tenant — otherwise an
    // accidental `tenant_id IS NULL` could pass test_3 vacuously).
    let owned: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_record \
         WHERE kind = 'skill.candidate.promoted' AND tenant_id = ?",
    )
    .bind(tenant_a.to_string())
    .fetch_one(store.pool())
    .await
    .expect("count");
    assert!(
        owned >= 1,
        "tenant_a must see its own promote row (sanity guard against vacuous pass)"
    );
}

// ---------------------------------------------------------------------------
// Test 4 — principal_id column stamping (sensible deviation, see header)
// ---------------------------------------------------------------------------

/// `audit_record.principal_id` must capture the caller resolved from
/// `X-Argo-Principal` (the brief's `promoted_by` semantic, expressed
/// via the existing principal-stamp seam from `mvp-audit-principal-stamp`).
/// Without this pin, a refactor dropping `principal_id_for_audit(req)`
/// from the promote call site would silently leave the column NULL on
/// authenticated promotes — exactly the kind of audit gap the row-6
/// pin is meant to catch.
#[tokio::test]
async fn promote_audit_row_principal_id_column_is_stamped_when_caller_sends_x_argo_principal() {
    let _g = env_lock().await;
    let tmp = tempfile::tempdir().expect("tempdir");
    set_test_env(tmp.path());
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    let worker = add_worker(&store, tenant, "Maya").await;
    let skill_id = install_skill(&store, "principal-pin").await;

    let path = format!(
        "/api/v1/workers/{}/skills/{}/promote",
        worker.id, skill_id.0
    );
    // `human:mark` is the canonical operator principal used in
    // `routes_auth_principal_tests.rs` fixtures.
    let req = post_req_with_principal(&path, "{}", "human:mark");
    let (status, _body) = promote_response(
        &worker.id.to_string(),
        &skill_id.0.to_string(),
        &req,
        Some(&store),
    )
    .await;
    assert_eq!(status, "200 OK");

    let (_kind, _payload, _tenant_col, principal) = fetch_promoted_audit_row(&store).await;
    assert_eq!(
        principal.as_deref(),
        Some("human:mark"),
        "principal_id column must be stamped from X-Argo-Principal — \
         dropping principal_id_for_audit(req) would regress to NULL"
    );
}
