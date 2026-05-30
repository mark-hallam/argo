//! Unit tests for `routes_genesis_stream` (`POST /api/v1/genesis`).
//! Drives the handler over a loopback TCP pair so the full HTTP/1.1
//! and SSE wire format is exercised end-to-end. Each test owns its
//! own `ARGO_VERTICALS_DIR` tempdir so concurrent runs do not fight
//! over generated vertical paths. Env vars are process-global, so a
//! single mutex serializes mutations across tests.
use super::*;
use argo_store::SessionStore;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tempfile::TempDir;
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream};

struct EnvGuard {
    keys: Vec<(&'static str, Option<String>)>,
    // Owned guard on the gateway-wide shared env lock — serializes env
    // mutations against the hire-flow test modules too (the Genesis hire
    // gate reads ARGO_VERTICALS_DIR / ARGO_GENESIS_LLM_PROVIDER). Per-module
    // locks let a concurrent hire test clobber ARGO_VERTICALS_DIR mid-stream.
    _guard: tokio::sync::OwnedMutexGuard<()>,
}

impl EnvGuard {
    async fn set(entries: &[(&'static str, &str)]) -> Self {
        let guard = super::super::shared_env_lock().await;
        let mut keys = Vec::with_capacity(entries.len());
        for (k, v) in entries {
            keys.push((*k, std::env::var(k).ok()));
            std::env::set_var(k, v);
        }
        Self { keys, _guard: guard }
    }

    async fn unset(entries: &[&'static str]) -> Self {
        let guard = super::super::shared_env_lock().await;
        let mut keys = Vec::with_capacity(entries.len());
        for k in entries {
            keys.push((*k, std::env::var(k).ok()));
            std::env::remove_var(k);
        }
        Self { keys, _guard: guard }
    }

    // Combine `set` + `unset` under a single mutex window so a test
    // can both force-set one key and force-unset another in one
    // atomic update — needed by alias-coverage tests that prove the
    // alias alone satisfies the gate (PROVIDER_KEY_ENV must be off
    // while ANTHROPIC_API_KEY is on).
    async fn set_and_unset(
        set_entries: &[(&'static str, &str)],
        unset_entries: &[&'static str],
    ) -> Self {
        let guard = super::super::shared_env_lock().await;
        let mut keys = Vec::with_capacity(set_entries.len() + unset_entries.len());
        for (k, v) in set_entries {
            keys.push((*k, std::env::var(k).ok()));
            std::env::set_var(k, v);
        }
        for k in unset_entries {
            keys.push((*k, std::env::var(k).ok()));
            std::env::remove_var(k);
        }
        Self { keys, _guard: guard }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (k, old) in &self.keys {
            match old {
                Some(v) => std::env::set_var(k, v),
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

fn build_request(body: &str, tenant: Option<&str>) -> String {
    build_request_with_origin(body, tenant, None)
}

fn build_request_with_origin(body: &str, tenant: Option<&str>, origin: Option<&str>) -> String {
    let tenant_hdr = tenant
        .map(|t| format!("X-Argo-Tenant-Id: {t}\r\n"))
        .unwrap_or_default();
    let origin_hdr = origin
        .map(|o| format!("Origin: {o}\r\n"))
        .unwrap_or_default();
    format!(
        "POST /api/v1/genesis HTTP/1.1\r\nHost: x\r\n{tenant_hdr}{origin_hdr}Content-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
}

/// Install the global CORS registry once so the SSE handler can splice
/// CORS headers into its hand-rolled response head. The registry is a
/// `OnceLock` — first caller wins; subsequent installs are no-ops, so
/// any test in the binary that needs CORS coverage shares the same
/// configuration. Dev-loopback mode keeps the install permissive enough
/// that any `http://localhost:*` / `http://127.0.0.1:*` origin is
/// accepted, mirroring how the live gateway behaves outside production.
fn ensure_cors_registry_installed() {
    static INIT: OnceLock<()> = OnceLock::new();
    INIT.get_or_init(|| {
        let cfg = argo_config::GatewayCorsConfig {
            allowed_origins: vec!["http://localhost:3000".into()],
        };
        super::super::cors::install(&cfg, false);
    });
}

async fn drive_handler(req: String, store: Arc<SessionStore>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut server_side, _) = listener.accept().await.unwrap();
        serve_genesis_stream(&req, Some(&store), &mut server_side)
            .await
            .unwrap();
    });
    let mut client = TcpStream::connect(addr).await.unwrap();
    // Half-close after the request body is not required — the
    // handler doesn't read; it inspects the pre-built `&str` we
    // passed in. We just need the connection alive to read the
    // response.
    let mut buf = Vec::new();
    let n = tokio::time::timeout(Duration::from_secs(10), client.read_to_end(&mut buf))
        .await
        .unwrap()
        .unwrap();
    buf.truncate(n);
    server.await.unwrap();
    String::from_utf8(buf).unwrap()
}

#[tokio::test]
async fn fails_closed_without_provider_key() {
    // Unset the canonical key AND every alias the gateway accepts as a
    // satisfier — `ANTHROPIC_API_KEY` / `OPENAI_API_KEY` may be set in
    // the operator's parent env (env.local auto-sourced) and would
    // mask the fail-closed contract otherwise.
    let mut to_unset = vec![PROVIDER_KEY_ENV];
    to_unset.extend(PROVIDER_KEY_ALIASES.iter().copied());
    let _env = EnvGuard::unset(&to_unset).await;
    let store = mem_store().await;
    let body = serde_json::json!({
        "role_description": "I need a customer success lead who triages tier-1 tickets",
    })
    .to_string();
    let req = build_request(&body, Some("tenant-x"));
    let raw = drive_handler(req, store).await;
    assert!(
        raw.starts_with("HTTP/1.1 503 Service Unavailable"),
        "expected 503 fail-closed; got: {raw}"
    );
    assert!(raw.contains("genesis_provider_key_missing"));
}

#[tokio::test]
async fn anthropic_alias_satisfies_provider_key_gate() {
    // Closes fix-genesis-orphan-discoverability-and-provider-key-503:
    // the operator's existing `ANTHROPIC_API_KEY` (the same key the
    // LLM driver bills against) is an accepted satisfier for the
    // cost-guardrail gate — removing the dev-box 503 without
    // weakening the no-silent-fallback intent.
    let tmp = TempDir::new().unwrap();
    let _env = EnvGuard::set_and_unset(
        &[
            ("ANTHROPIC_API_KEY", "sk-ant-stub-key-for-alias-test"),
            ("ARGO_VERTICALS_DIR", tmp.path().to_str().unwrap()),
        ],
        &[PROVIDER_KEY_ENV],
    )
    .await;
    let store = mem_store().await;
    let body = serde_json::json!({
        "role_description": "I need a customer success lead who triages tier-1 tickets and handles refunds",
    })
    .to_string();
    let req = build_request(&body, Some("tenant-alias"));
    let raw = drive_handler(req, store).await;
    assert!(
        raw.starts_with("HTTP/1.1 200 OK"),
        "expected 200 with anthropic alias satisfying gate; got: {raw}"
    );
    assert!(!raw.contains("genesis_provider_key_missing"));
}

#[tokio::test]
async fn rejects_short_role_description() {
    let tmp = TempDir::new().unwrap();
    let _env = EnvGuard::set(&[
        (PROVIDER_KEY_ENV, "test-key"),
        ("ARGO_VERTICALS_DIR", tmp.path().to_str().unwrap()),
    ])
    .await;
    let store = mem_store().await;
    let body = serde_json::json!({ "role_description": "too short" }).to_string();
    let req = build_request(&body, Some("tenant-x"));
    let raw = drive_handler(req, store).await;
    assert!(raw.starts_with("HTTP/1.1 400 Bad Request"), "got: {raw}");
    assert!(raw.contains("role_description"));
}

#[tokio::test]
async fn rejects_malformed_json_body() {
    let tmp = TempDir::new().unwrap();
    let _env = EnvGuard::set(&[
        (PROVIDER_KEY_ENV, "test-key"),
        ("ARGO_VERTICALS_DIR", tmp.path().to_str().unwrap()),
    ])
    .await;
    let store = mem_store().await;
    let req = build_request("{not valid json", Some("tenant-x"));
    let raw = drive_handler(req, store).await;
    assert!(raw.starts_with("HTTP/1.1 400 Bad Request"), "got: {raw}");
    assert!(raw.contains("bad_request"));
}

#[tokio::test]
async fn happy_path_emits_sse_stream_with_vertical_ready() {
    let tmp = TempDir::new().unwrap();
    // Force an unresolvable safety-judge provider so the gate degrades
    // deterministically (needs_review) rather than depending on whether the
    // test cwd can resolve `claude-code-cli` + spawn the subprocess.
    let _env = EnvGuard::set(&[
        (PROVIDER_KEY_ENV, "test-key"),
        ("ARGO_VERTICALS_DIR", tmp.path().to_str().unwrap()),
        ("ARGO_GENESIS_LLM_PROVIDER", "definitely-not-a-real-provider-xyz"),
    ])
    .await;
    let store = mem_store().await;
    let body = serde_json::json!({
        "role_description": "I need a customer success lead who triages tier-1 tickets and escalates refunds over $500",
        "persona": "Casey",
    })
    .to_string();
    let req = build_request(&body, Some("11111111-1111-1111-1111-111111111111"));
    let raw = drive_handler(req, store).await;

    assert!(raw.starts_with("HTTP/1.1 200 OK"), "got: {raw}");
    let lower = raw.to_lowercase();
    assert!(
        lower.contains("content-type: text/event-stream"),
        "expected text/event-stream content-type; got: {raw}"
    );
    // Progress phases must all appear, including the REAL safety-gate phase.
    for phase in [
        "\"phase\":\"drafting_persona\"",
        "\"phase\":\"compiling_charter\"",
        "\"phase\":\"generating_skill_bundle\"",
        "\"phase\":\"validating_eval_seeds\"",
        "\"phase\":\"running_safety_gate\"",
        "\"phase\":\"ready\"",
    ] {
        assert!(raw.contains(phase), "expected phase {phase}; got: {raw}");
    }
    let ready_idx = raw.find("event: vertical_ready").expect("vertical_ready event");
    let after_ready = &raw[ready_idx..];
    assert!(after_ready.contains("\"vertical_dir_path\":"));
    assert!(after_ready.contains("\"eval_seed_count\":"));
    // The verdict is REAL now — NOT a hardcoded "pass" (DEC-025 / P0). With
    // no live provider the safety judge degrades, so the composed gate is
    // `needs_review` (fail-safe), `hireable:false`, with honest eval evidence.
    assert!(
        !after_ready.contains("\"judge_verdict\":\"pass\""),
        "hardcoded pass must be gone; got: {after_ready}"
    );
    for needle in [
        "\"judge_verdict\":\"needs_review\"",
        "\"hireable\":false",
        "\"gate\":",
        "\"eval_total\":10",
        "\"safety_verdict\":",
    ] {
        assert!(after_ready.contains(needle), "expected {needle}; got: {after_ready}");
    }
}

#[tokio::test]
async fn cross_tenant_paths_isolate_generated_verticals() {
    let tmp = TempDir::new().unwrap();
    let _env = EnvGuard::set(&[
        (PROVIDER_KEY_ENV, "test-key"),
        ("ARGO_VERTICALS_DIR", tmp.path().to_str().unwrap()),
    ])
    .await;
    let store = mem_store().await;
    let body = serde_json::json!({
        "role_description": "I need a marketing intern who writes weekly newsletters for B2B SaaS",
        "persona": "Maya",
    })
    .to_string();

    let tenant_a = "11111111-1111-1111-1111-111111111111";
    let tenant_b = "22222222-2222-2222-2222-222222222222";
    let raw_a = drive_handler(build_request(&body, Some(tenant_a)), Arc::clone(&store)).await;
    let raw_b = drive_handler(build_request(&body, Some(tenant_b)), Arc::clone(&store)).await;
    assert!(raw_a.starts_with("HTTP/1.1 200 OK"));
    assert!(raw_b.starts_with("HTTP/1.1 200 OK"));

    // Each tenant's vertical_dir_path lives under its own sanitized
    // tenant subdirectory; load_dir scoped to one tenant must not see
    // the other.
    let genesis_root = tmp.path().join(".genesis");
    let tenant_a_dir = genesis_root.join(tenant_a);
    let tenant_b_dir = genesis_root.join(tenant_b);
    assert!(tenant_a_dir.is_dir(), "tenant A subdir created");
    assert!(tenant_b_dir.is_dir(), "tenant B subdir created");
    let (reg_a, errs_a) = argo_vertical::load_dir(&tenant_a_dir).unwrap();
    let (reg_b, errs_b) = argo_vertical::load_dir(&tenant_b_dir).unwrap();
    assert!(errs_a.is_empty(), "tenant A load errors: {errs_a:?}");
    assert!(errs_b.is_empty(), "tenant B load errors: {errs_b:?}");
    assert_eq!(reg_a.len(), 1);
    assert_eq!(reg_b.len(), 1);
    let id_a: Vec<&str> = reg_a.list().map(|(id, _)| id).collect();
    let id_b: Vec<&str> = reg_b.list().map(|(id, _)| id).collect();
    // Same vertical_id (same role + persona) but at different
    // tenant-scoped paths — neither registry can observe the other.
    assert_eq!(id_a, id_b, "same vertical id under both tenants");
    assert_ne!(
        reg_a.get(id_a[0]).unwrap().root,
        reg_b.get(id_b[0]).unwrap().root,
        "tenant filesystem roots differ",
    );
}

/// Closes fix-spa-genesis-fetch-fails-while-curl-succeeds: the SPA's
/// cross-origin fetch from `http://localhost:3000` against the gateway
/// on `:7400` requires the SSE response head to carry both the standard
/// `Access-Control-Allow-Origin` family AND
/// `Access-Control-Allow-Private-Network: true` (chrome's Private
/// Network Access policy treats `localhost:<spa>` → `localhost:<gw>` as
/// public→private and refuses the response without the header). The
/// streaming handler writes the head directly to the socket, so the
/// generic `cors::enhance_status` splice does not apply — coverage has
/// to live here to lock in the hand-rolled CORS header block.
#[tokio::test]
async fn sse_response_carries_cors_headers_for_allowlisted_origin() {
    ensure_cors_registry_installed();
    let tmp = TempDir::new().unwrap();
    let _env = EnvGuard::set(&[
        (PROVIDER_KEY_ENV, "test-key"),
        ("ARGO_VERTICALS_DIR", tmp.path().to_str().unwrap()),
    ])
    .await;
    let store = mem_store().await;
    let body = serde_json::json!({
        "role_description": "I need a customer success lead who triages tier-1 tickets and escalates refunds over $500",
        "persona": "Casey",
    })
    .to_string();
    let req = build_request_with_origin(
        &body,
        Some("11111111-1111-1111-1111-111111111111"),
        Some("http://localhost:3000"),
    );
    let raw = drive_handler(req, store).await;

    assert!(raw.starts_with("HTTP/1.1 200 OK"), "got: {raw}");
    // Take the response head (everything before the SSE body) so the
    // assertions are not confused by stream payload that happens to
    // mention CORS-shaped substrings.
    let head_end = raw
        .find("\r\n\r\n")
        .expect("response head terminator present");
    let head = &raw[..head_end];
    assert!(
        head.contains("Access-Control-Allow-Origin: http://localhost:3000"),
        "SSE head missing Access-Control-Allow-Origin; got head: {head}"
    );
    assert!(
        head.contains("Access-Control-Allow-Private-Network: true"),
        "SSE head missing Access-Control-Allow-Private-Network; got head: {head}"
    );
    assert!(
        head.contains("Access-Control-Allow-Credentials: true"),
        "SSE head missing Access-Control-Allow-Credentials; got head: {head}"
    );
    assert!(
        head.contains("Vary: Origin"),
        "SSE head missing Vary: Origin; got head: {head}"
    );
}

/// Companion to the success-path CORS test: the one-shot 503 error
/// (missing provider key) ALSO has to carry CORS headers, otherwise the
/// SPA's fetch promise rejects with `TypeError: Failed to fetch` before
/// the page can surface the gateway's diagnostic message.
#[tokio::test]
async fn provider_key_503_carries_cors_headers_for_allowlisted_origin() {
    ensure_cors_registry_installed();
    let mut to_unset = vec![PROVIDER_KEY_ENV];
    to_unset.extend(PROVIDER_KEY_ALIASES.iter().copied());
    let _env = EnvGuard::unset(&to_unset).await;
    let store = mem_store().await;
    let body = serde_json::json!({
        "role_description": "I need a customer success lead who triages tier-1 tickets",
    })
    .to_string();
    let req = build_request_with_origin(
        &body,
        Some("tenant-x"),
        Some("http://localhost:3000"),
    );
    let raw = drive_handler(req, store).await;

    assert!(
        raw.starts_with("HTTP/1.1 503 Service Unavailable"),
        "expected 503 fail-closed; got: {raw}"
    );
    let head_end = raw
        .find("\r\n\r\n")
        .expect("response head terminator present");
    let head = &raw[..head_end];
    assert!(
        head.contains("Access-Control-Allow-Origin: http://localhost:3000"),
        "503 head missing Access-Control-Allow-Origin; got head: {head}"
    );
    assert!(
        head.contains("Access-Control-Allow-Private-Network: true"),
        "503 head missing Access-Control-Allow-Private-Network; got head: {head}"
    );
}

#[tokio::test]
async fn second_attempt_without_overwrite_emits_error_event() {
    // First attempt creates `verticals/.genesis/<tenant>/<persona>/`;
    // the second attempt for the same tenant+role must surface the
    // OutputDirNotEmpty refusal as an SSE error event rather than
    // succeeding. Closes the regression in
    // `fix-genesis-idempotency-output-dir-not-empty`.
    let tmp = TempDir::new().unwrap();
    let _env = EnvGuard::set(&[
        (PROVIDER_KEY_ENV, "test-key"),
        ("ARGO_VERTICALS_DIR", tmp.path().to_str().unwrap()),
    ])
    .await;
    let store = mem_store().await;
    let body = serde_json::json!({
        "role_description": "I need a marketing intern who writes weekly newsletters for B2B SaaS",
        "persona": "Maya",
    })
    .to_string();
    let tenant = "33333333-3333-3333-3333-333333333333";
    let raw1 = drive_handler(build_request(&body, Some(tenant)), Arc::clone(&store)).await;
    assert!(raw1.starts_with("HTTP/1.1 200 OK"));
    assert!(raw1.contains("event: vertical_ready"));
    let raw2 = drive_handler(build_request(&body, Some(tenant)), Arc::clone(&store)).await;
    assert!(raw2.starts_with("HTTP/1.1 200 OK"), "still streams; got: {raw2}");
    assert!(raw2.contains("event: error"), "expected error event; got: {raw2}");
    assert!(raw2.contains("is not empty"), "expected refusal text; got: {raw2}");
}

#[tokio::test]
async fn overwrite_true_lets_second_attempt_succeed() {
    let tmp = TempDir::new().unwrap();
    let _env = EnvGuard::set(&[
        (PROVIDER_KEY_ENV, "test-key"),
        ("ARGO_VERTICALS_DIR", tmp.path().to_str().unwrap()),
    ])
    .await;
    let store = mem_store().await;
    let first = serde_json::json!({
        "role_description": "I need a marketing intern who writes weekly newsletters for B2B SaaS",
        "persona": "Maya",
    })
    .to_string();
    let retry = serde_json::json!({
        "role_description": "I need a marketing intern who writes weekly newsletters for B2B SaaS",
        "persona": "Maya",
        "overwrite": true,
    })
    .to_string();
    let tenant = "44444444-4444-4444-4444-444444444444";
    let raw1 = drive_handler(build_request(&first, Some(tenant)), Arc::clone(&store)).await;
    assert!(raw1.contains("event: vertical_ready"));
    let raw2 = drive_handler(build_request(&retry, Some(tenant)), Arc::clone(&store)).await;
    assert!(raw2.starts_with("HTTP/1.1 200 OK"));
    assert!(raw2.contains("event: vertical_ready"), "expected success on retry; got: {raw2}");
    assert!(!raw2.contains("event: error"), "must not emit error event; got: {raw2}");
}
