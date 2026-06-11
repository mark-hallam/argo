//! Tests for `routes_worker_capability_grant` (tg3 durable per-worker
//! capability grant + revoke).
//!
//! The load-bearing acceptance pin is **deny → grant → allowed on the NEXT
//! run**: we grant a previously-fenced tool, reload the worker's manifest the
//! exact way the runtime does (`load_for_worker` →
//! `WorkerCapabilityProfile::derive`), and assert `permits_tool` flips true —
//! then revoke and assert it flips back. The security pins are: an
//! always-denied tool is refused with an explicit error and writes no grant;
//! a non-owner is refused; a cross-tenant / unknown worker reads as 404; a
//! flat SHARED role is refused (no cross-tenant leak).
//!
//! Mirrors the `routes_worker_executor_promote` harness: drives the handlers
//! directly against a `:memory:` store, with `X-Argo-Role` trusted in the
//! test build. These tests mutate PROCESS-global env (`ARGO_VERTICALS_DIR`)
//! so they serialize on the shared env lock.

#![allow(clippy::expect_used)]

use super::{delete_capability_grant, post_capability_grant, resolve_tool, Access};
use argo_capability::WorkerCapabilityProfile;
use argo_identity::{IdentityStore, Persona, Role, Worker, WorkerId};
use argo_store::SessionStore;
use argo_types::TenantId;
use std::path::Path;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::OwnedMutexGuard;

const VID: &str = "genesis-personal-tg3test";
const FENCED_TOOL: &str = "web_search";

/// A minimal fenced Genesis manifest: the always-denied set is in `denied`,
/// the allow-set has only baseline read tools — `web_search` is NOT granted.
const FENCED_MANIFEST: &str = r#"schema_version = "1"

[vertical]
id = "genesis-personal-tg3test"
name = "TG3 Test Teammate"
version = "0.1.0"

[tools]
allowed = ["file_read", "clarify", "memory"]
denied = ["code_execute", "delegate_task", "file_patch", "process", "send_message", "terminal"]
"#;

struct Env {
    _guard: OwnedMutexGuard<()>,
    _tmp: TempDir,
    store: Arc<SessionStore>,
    tenant: TenantId,
    verticals: std::path::PathBuf,
}

async fn mem_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:").await.expect("open mem db");
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

/// The tenant `resolve_tenant_for_request` resolves to with no JWT — the cwd
/// workspace tenant. The worker + its Genesis pack must live under it.
async fn workspace_tenant(store: &Arc<SessionStore>) -> TenantId {
    let cwd = std::env::current_dir().expect("cwd");
    let name = cwd.file_name().and_then(|s| s.to_str()).unwrap_or("ws");
    let (tenant, _) = store
        .ensure_workspace(name, &cwd.to_string_lossy())
        .await
        .expect("ws");
    tenant
}

/// Stand up: a tempdir `verticals/` (pointed at via `ARGO_VERTICALS_DIR`), a
/// Genesis pack for `tenant` at `.genesis/<slug>/<VID>/`, and a worker bound
/// to `VID`. Returns the env handle + worker.
async fn setup() -> (Env, Worker) {
    let guard = crate::handlers::gateway::shared_env_lock().await;
    let tmp = TempDir::new().expect("tempdir");
    std::env::set_var("ARGO_HOME", tmp.path().join("home"));
    std::fs::create_dir_all(tmp.path().join("home")).expect("home");
    let verticals = tmp.path().join("verticals");
    std::env::set_var("ARGO_VERTICALS_DIR", &verticals);
    let store = mem_store().await;
    let tenant = workspace_tenant(&store).await;
    write_genesis_pack(&verticals, tenant, FENCED_MANIFEST);
    let worker = add_worker(&store, tenant, "Nora").await;
    (
        Env {
            _guard: guard,
            _tmp: tmp,
            store,
            tenant,
            verticals,
        },
        worker,
    )
}

fn write_genesis_pack(verticals: &Path, tenant: TenantId, manifest: &str) {
    let slug = argo_vertical::sanitize_tenant(&tenant.to_string());
    let dir = verticals.join(".genesis").join(slug).join(VID);
    std::fs::create_dir_all(&dir).expect("genesis dir");
    std::fs::write(dir.join("vertical.toml"), manifest).expect("write manifest");
}

async fn add_worker(store: &Arc<SessionStore>, tenant: TenantId, name: &str) -> Worker {
    let worker = Worker::draft(
        tenant,
        VID,
        Persona::named(name, "human:mark"),
        Role::new("Personal assistant."),
    );
    IdentityStore::new(store)
        .create_worker(&worker)
        .await
        .expect("create");
    worker
}

fn grant_req(id: &WorkerId, role: Option<&str>, body: &str) -> String {
    let mut req = format!("POST /api/v1/workers/{id}/capability-grants HTTP/1.1\r\nHost: x\r\n");
    if let Some(r) = role {
        req.push_str(&format!("X-Argo-Role: {r}\r\n"));
    }
    req.push_str("X-Argo-Principal: human:alice@acme\r\n");
    req.push_str(&format!("Content-Length: {}\r\n\r\n{body}", body.len()));
    req
}

fn revoke_req(id: &WorkerId, role: Option<&str>, tool: &str) -> String {
    let mut req = format!(
        "DELETE /api/v1/workers/{id}/capability-grants/{tool} HTTP/1.1\r\nHost: x\r\n"
    );
    if let Some(r) = role {
        req.push_str(&format!("X-Argo-Role: {r}\r\n"));
    }
    req.push_str("X-Argo-Principal: human:alice@acme\r\n\r\n");
    req
}

fn parse(body: &str) -> serde_json::Value {
    serde_json::from_str(body).unwrap_or_else(|_| panic!("invalid JSON: {body}"))
}

/// Reload the worker's manifest the way the runtime does and ask whether
/// `tool` would dispatch on the NEXT run. This is the real fence, not a peek
/// at the route's own state.
fn permits_on_next_run(env: &Env, tool: &str) -> bool {
    let loaded = argo_vertical::load_for_worker(&env.verticals, &env.tenant.to_string(), VID)
        .expect("manifest must resolve");
    let declared: Vec<String> = loaded
        .manifest
        .actions
        .as_ref()
        .map(|a| a.actions.keys().cloned().collect())
        .unwrap_or_default();
    let (allowed, denied): (Vec<String>, Vec<String>) = loaded
        .manifest
        .tools
        .as_ref()
        .map(|t| (t.allowed.clone(), t.denied.clone()))
        .unwrap_or_default();
    WorkerCapabilityProfile::derive(declared, allowed, denied, Vec::<String>::new())
        .permits_tool(tool)
}

async fn rbac_deny_count(env: &Env) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM audit_record WHERE kind = 'rbac.deny'")
        .fetch_one(env.store.pool())
        .await
        .expect("count")
}

// ── Acceptance: deny → grant → allowed on next run, then revoke restores deny.

#[tokio::test]
async fn grant_then_revoke_flips_the_fence() {
    let (env, worker) = setup().await;
    assert!(
        !permits_on_next_run(&env, FENCED_TOOL),
        "tool must start fenced"
    );

    let body = format!(r#"{{"tool_or_connector":"{FENCED_TOOL}","access":"read"}}"#);
    let req = grant_req(&worker.id, Some("owner"), &body);
    let (status, resp) =
        post_capability_grant(&worker.id.to_string(), &req, Some(&env.store)).await;
    assert_eq!(status, "200 OK", "body: {resp}");
    assert_eq!(parse(&resp)["outcome"], "granted");
    assert!(
        permits_on_next_run(&env, FENCED_TOOL),
        "granted tool must pass on the next run"
    );

    // Revoke → fence restored.
    let req = revoke_req(&worker.id, Some("owner"), FENCED_TOOL);
    let (status, resp) =
        delete_capability_grant(&worker.id.to_string(), FENCED_TOOL, &req, Some(&env.store)).await;
    assert_eq!(status, "200 OK", "body: {resp}");
    assert_eq!(parse(&resp)["outcome"], "revoked");
    assert!(
        !permits_on_next_run(&env, FENCED_TOOL),
        "revoke must restore the deny"
    );
}

// ── Security pin: an always-denied tool is refused, even when requested.

#[tokio::test]
async fn always_denied_tool_is_refused_with_no_grant() {
    let (env, worker) = setup().await;
    let body = r#"{"tool_or_connector":"terminal","access":"write"}"#;
    let req = grant_req(&worker.id, Some("owner"), body);
    let (status, resp) =
        post_capability_grant(&worker.id.to_string(), &req, Some(&env.store)).await;
    assert_eq!(status, "403 Forbidden", "body: {resp}");
    assert_eq!(parse(&resp)["error"]["code"], "always_denied_tool");
    assert!(
        !permits_on_next_run(&env, "terminal"),
        "an always-denied tool can never be granted"
    );
}

// ── Owner-only.

#[tokio::test]
async fn non_owner_is_refused_and_audited() {
    let (env, worker) = setup().await;
    let body = format!(r#"{{"tool_or_connector":"{FENCED_TOOL}","access":"read"}}"#);
    let req = grant_req(&worker.id, Some("admin"), &body);
    let (status, resp) =
        post_capability_grant(&worker.id.to_string(), &req, Some(&env.store)).await;
    assert_eq!(status, "403 Forbidden", "body: {resp}");
    assert_eq!(rbac_deny_count(&env).await, 1, "one RbacDeny row");
    assert!(
        !permits_on_next_run(&env, FENCED_TOOL),
        "a denied call writes no grant"
    );
}

// ── Tenant isolation: an unknown / cross-tenant worker reads as 404.

#[tokio::test]
async fn cross_tenant_worker_returns_404() {
    let (env, _own) = setup().await;
    let alien = Worker::draft(
        TenantId::new(),
        VID,
        Persona::named("Alien", "human:x"),
        Role::new("x"),
    );
    IdentityStore::new(&env.store)
        .create_worker(&alien)
        .await
        .expect("create alien");
    let body = format!(r#"{{"tool_or_connector":"{FENCED_TOOL}","access":"read"}}"#);
    let req = grant_req(&alien.id, Some("owner"), &body);
    let (status, resp) =
        post_capability_grant(&alien.id.to_string(), &req, Some(&env.store)).await;
    assert_eq!(status, "404 Not Found", "body: {resp}");
}

// ── A flat SHARED role cannot receive a per-worker durable grant (would leak
//    the grant to every tenant using that role).

#[tokio::test]
async fn shared_catalog_role_is_refused() {
    let (env, _w) = setup().await;
    // A flat vertical (no `.genesis` segment) + a worker bound to it.
    let flat_dir = env.verticals.join("shared-role");
    std::fs::create_dir_all(&flat_dir).expect("flat dir");
    std::fs::write(
        flat_dir.join("vertical.toml"),
        FENCED_MANIFEST.replace(VID, "shared-role"),
    )
    .expect("write flat");
    let worker = Worker::draft(
        env.tenant,
        "shared-role",
        Persona::named("Shared", "human:mark"),
        Role::new("x"),
    );
    IdentityStore::new(&env.store)
        .create_worker(&worker)
        .await
        .expect("create");
    let body = format!(r#"{{"tool_or_connector":"{FENCED_TOOL}","access":"read"}}"#);
    let req = grant_req(&worker.id, Some("owner"), &body);
    let (status, resp) =
        post_capability_grant(&worker.id.to_string(), &req, Some(&env.store)).await;
    assert_eq!(status, "409 Conflict", "body: {resp}");
    assert_eq!(parse(&resp)["error"]["code"], "shared_role_not_grantable");
}

// ── tg3b hardening 1: write-path containment. A worker whose `vertical_id`
//    `..`-traverses into ANOTHER tenant's `.genesis` subtree (still carrying a
//    `.genesis` component, so the substring `is_genesis_pack` check passes) is
//    refused BEFORE any write — the crafted root escapes the authenticated
//    tenant's subtree.

#[tokio::test]
async fn manifest_root_escaping_tenant_subtree_is_refused_before_write() {
    let (env, _own) = setup().await;
    // A victim pack under a DIFFERENT tenant's slug.
    let victim_tenant = TenantId::new();
    let victim_slug = argo_vertical::sanitize_tenant(&victim_tenant.to_string());
    let victim_dir = env
        .verticals
        .join(".genesis")
        .join(&victim_slug)
        .join("victim-vid");
    std::fs::create_dir_all(&victim_dir).expect("victim dir");
    let victim_manifest = victim_dir.join("vertical.toml");
    std::fs::write(&victim_manifest, FENCED_MANIFEST.replace(VID, "victim-vid"))
        .expect("write victim");
    let before = std::fs::read_to_string(&victim_manifest).expect("read before");

    // A worker under OUR tenant whose vertical_id traverses out of our slug into
    // the victim's. `load_for_worker` resolves it (the genesis-tree branch loads
    // the manifest at the computed path); `is_genesis_pack` passes (the path
    // still contains `.genesis`); containment must catch the escape.
    let traversal_vid = format!("../{victim_slug}/victim-vid");
    let worker = Worker::draft(
        env.tenant,
        &traversal_vid,
        Persona::named("Trav", "human:mark"),
        Role::new("x"),
    );
    IdentityStore::new(&env.store)
        .create_worker(&worker)
        .await
        .expect("create");
    let body = format!(r#"{{"tool_or_connector":"{FENCED_TOOL}","access":"write"}}"#);
    let req = grant_req(&worker.id, Some("owner"), &body);
    let (status, resp) =
        post_capability_grant(&worker.id.to_string(), &req, Some(&env.store)).await;
    assert_eq!(status, "409 Conflict", "body: {resp}");
    assert_eq!(parse(&resp)["error"]["code"], "manifest_root_out_of_tenant");
    let after = std::fs::read_to_string(&victim_manifest).expect("read after");
    assert_eq!(before, after, "the cross-tenant manifest must be untouched");
}

// ── tg3b hardening 2: audit is a hard prerequisite. When the sink errors the
//    grant request FAILS (500) and the manifest on disk is unchanged — a grant
//    can never persist without its audit row.

#[tokio::test]
async fn audit_sink_error_fails_grant_and_leaves_manifest_unchanged() {
    let (env, worker) = setup().await;
    assert!(!permits_on_next_run(&env, FENCED_TOOL), "tool starts fenced");
    // Force the audit sink to error via the cfg(test) seam (process env is held
    // under the shared env lock acquired in setup()).
    std::env::set_var("SKYCOT_TEST_FORCE_AUDIT_SINK_ERROR", "1");
    let body = format!(r#"{{"tool_or_connector":"{FENCED_TOOL}","access":"read"}}"#);
    let req = grant_req(&worker.id, Some("owner"), &body);
    let (status, resp) =
        post_capability_grant(&worker.id.to_string(), &req, Some(&env.store)).await;
    std::env::remove_var("SKYCOT_TEST_FORCE_AUDIT_SINK_ERROR");
    assert_eq!(status, "500 Internal Server Error", "body: {resp}");
    assert_eq!(parse(&resp)["error"]["code"], "internal_error");
    assert!(
        !permits_on_next_run(&env, FENCED_TOOL),
        "no grant may persist when the audit row could not be written"
    );
}

// ── Bad bodies.

#[tokio::test]
async fn missing_access_returns_400() {
    let (env, worker) = setup().await;
    let body = r#"{"tool_or_connector":"web_search"}"#;
    let req = grant_req(&worker.id, Some("owner"), body);
    let (status, resp) =
        post_capability_grant(&worker.id.to_string(), &req, Some(&env.store)).await;
    assert_eq!(status, "400 Bad Request", "body: {resp}");
    assert_eq!(parse(&resp)["error"]["code"], "bad_request");
}

// ── Read/write access maps onto the runtime's existing file_read/file_write
//    split (no new taxonomy).

#[test]
fn resolve_tool_maps_file_access_to_existing_split() {
    assert_eq!(resolve_tool("file", Access::Read), "file_read");
    assert_eq!(resolve_tool("file", Access::Write), "file_write");
    assert_eq!(resolve_tool("filesystem", Access::Write), "file_write");
    // A literal tool / connector name passes through unchanged.
    assert_eq!(resolve_tool("web_search", Access::Read), "web_search");
    assert_eq!(resolve_tool("salesforce", Access::Write), "salesforce");
}

// ── Security pin: a case-variant always-denied name ("Terminal") must take
//    the same explicit 403, not slip past as an inert allowed entry.

#[tokio::test]
async fn always_denied_case_variant_is_refused() {
    let (env, worker) = setup().await;
    let body = r#"{"tool_or_connector":"Terminal","access":"write"}"#;
    let req = grant_req(&worker.id, Some("owner"), body);
    let (status, resp) =
        post_capability_grant(&worker.id.to_string(), &req, Some(&env.store)).await;
    assert_eq!(status, "403 Forbidden", "body: {resp}");
    assert_eq!(parse(&resp)["error"]["code"], "always_denied_tool");
    assert!(
        !permits_on_next_run(&env, "terminal"),
        "case variants of always-denied tools can never be granted"
    );
}

// ── Audit honesty pin: the recorded grantor is the verified session
//    identity; a forged `X-Argo-Principal` header cannot stamp someone else
//    as the grantor of a fence-loosening action (same class as the
//    fix-manual-run-audit-row JWT-first fix).

#[tokio::test]
async fn granted_by_is_session_identity_not_forged_header() {
    let (env, worker) = setup().await;
    std::env::set_var("SKYCOT_AUTH_JWT_SECRET", GRANT_TEST_JWT_SECRET);
    let jwt = mint_session("owner@acme", &env.tenant);
    let body = format!(r#"{{"tool_or_connector":"{FENCED_TOOL}","access":"read"}}"#);
    let mut req = format!(
        "POST /api/v1/workers/{}/capability-grants HTTP/1.1\r\nHost: x\r\n",
        worker.id
    );
    req.push_str("X-Argo-Role: owner\r\n");
    req.push_str("X-Argo-Principal: human:attacker@evil\r\n");
    req.push_str(&format!("Cookie: __Host-session={jwt}\r\n"));
    req.push_str(&format!("Content-Length: {}\r\n\r\n{body}", body.len()));
    let (status, resp) =
        post_capability_grant(&worker.id.to_string(), &req, Some(&env.store)).await;
    std::env::remove_var("SKYCOT_AUTH_JWT_SECRET");
    assert_eq!(status, "200 OK", "body: {resp}");
    let (payload, principal): (String, Option<String>) = sqlx::query_as(
        "SELECT payload_json, principal_id FROM audit_record \
         WHERE payload_json LIKE '%worker.capability_granted%' \
         ORDER BY created_at DESC LIMIT 1",
    )
    .fetch_one(env.store.pool())
    .await
    .expect("fetch grant audit row");
    assert_eq!(
        parse(&payload)["granted_by"],
        "human:owner@acme",
        "granted_by must be the session identity, not the forged header"
    );
    assert_eq!(
        principal.as_deref(),
        Some("human:owner@acme"),
        "audit principal column must be the session identity"
    );
}

/// Shared HS256 secret for the session-grantor pin — set on the process env
/// under the held `shared_env_lock`, mirroring `routes_worker_run_tests`.
const GRANT_TEST_JWT_SECRET: &str = "capability-grant-audit-actor-test-secret";

/// Minimal `__Host-session` claim shape (`sub`, `email`, `tnt`, `exp`) —
/// the fields the session resolver reads.
#[derive(serde::Serialize)]
struct GrantTestClaims<'a> {
    sub: &'a str,
    email: &'a str,
    tnt: String,
    iat: i64,
    exp: i64,
}

/// Mint a `__Host-session` cookie JWT for `email` on `tenant`, signed with
/// [`GRANT_TEST_JWT_SECRET`].
fn mint_session(email: &str, tenant: &TenantId) -> String {
    let now = chrono::Utc::now().timestamp();
    jsonwebtoken::encode(
        &jsonwebtoken::Header::new(jsonwebtoken::Algorithm::HS256),
        &GrantTestClaims {
            sub: "33333333-3333-3333-3333-333333333333",
            email,
            tnt: tenant.to_string(),
            iat: now - 10,
            exp: now + 3600,
        },
        &jsonwebtoken::EncodingKey::from_secret(GRANT_TEST_JWT_SECRET.as_bytes()),
    )
    .expect("mint session jwt")
}
