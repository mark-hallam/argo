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

/// DEC-025 #9 (single-tenant-per-cwd) write-segment contract: when a store is
/// wired, the SSE writes the manifest under the **cwd-derived workspace
/// tenant** (`ensure_workspace(name, cwd)`), NOT the request `X-Argo-Tenant-Id`
/// header — so the write segment matches the segment the hire-commit endpoint
/// (which also resolves the cwd tenant) reads under. Two requests bearing
/// DIFFERENT headers but sharing the same store + cwd collapse onto the SAME
/// cwd segment. (The genuine cross-tenant *security* guarantee — tenant B
/// cannot resolve tenant A's vertical — is enforced + tested at the hire gate
/// in `genesis_tenant_tests.rs`; that primitive is unchanged. Per-header write
/// isolation is the DEFERRED multi-tenant-routing follow-up.)
#[tokio::test]
async fn write_segment_uses_cwd_tenant_not_request_header() {
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

    // A request with a fresh/unique header — the exact shape the SPA sends and
    // the one that PROVOKED the bug (header != cwd tenant).
    let header_tenant = "11111111-1111-1111-1111-111111111111";
    let raw = drive_handler(build_request(&body, Some(header_tenant)), Arc::clone(&store)).await;
    assert!(raw.starts_with("HTTP/1.1 200 OK"), "got: {raw}");

    // The header-named subdirectory must NOT be created — the write segment is
    // the cwd-derived workspace tenant, not the header.
    let genesis_root = tmp.path().join(".genesis");
    assert!(
        !genesis_root.join(header_tenant).is_dir(),
        "header tenant must NOT drive the write segment under single-tenant-per-cwd"
    );

    // The ONE segment that exists is the cwd-derived workspace tenant — the
    // identical `ensure_workspace(name, cwd)` the hire-commit endpoint resolves.
    let cwd = std::env::current_dir().unwrap();
    let name = cwd.file_name().and_then(|s| s.to_str()).unwrap_or("workspace");
    let (cwd_tenant, _ws) = store
        .ensure_workspace(name, &cwd.to_string_lossy())
        .await
        .unwrap();
    let cwd_dir = genesis_root.join(sanitize_tenant(&cwd_tenant.to_string()));
    assert!(cwd_dir.is_dir(), "cwd-tenant segment must be created");
    let (reg, errs) = argo_vertical::load_dir(&cwd_dir).unwrap();
    assert!(errs.is_empty(), "cwd-tenant load errors: {errs:?}");
    assert_eq!(reg.len(), 1, "exactly one generated vertical under cwd tenant");

    // A second request with a DIFFERENT header lands in the SAME segment: it is
    // a re-generate of the same persona, so the no-overwrite refusal proves the
    // path collided — both headers wrote the one cwd segment.
    let other_header = "22222222-2222-2222-2222-222222222222";
    let raw2 = drive_handler(build_request(&body, Some(other_header)), Arc::clone(&store)).await;
    assert!(raw2.starts_with("HTTP/1.1 200 OK"), "got: {raw2}");
    assert!(
        raw2.contains("event: error") && raw2.contains("is not empty"),
        "second header must collide with the first in the SAME cwd segment; got: {raw2}"
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

/// Extract the JSON object on the `data:` line of the `vertical_ready` SSE
/// event from a raw chunked response. Returns the parsed payload the SPA reads
/// (carrying `vertical_id` + `contract_digest`).
fn parse_vertical_ready(raw: &str) -> serde_json::Value {
    let ready_idx = raw.find("event: vertical_ready").expect("vertical_ready event present");
    let after = &raw[ready_idx..];
    let data_line = after
        .lines()
        .find(|l| l.starts_with("data: "))
        .expect("data: line after vertical_ready");
    let json = data_line.trim_start_matches("data: ");
    serde_json::from_str(json).expect("vertical_ready data parses as JSON")
}

/// THE REGRESSION TEST for the tenant-segment mismatch (the proven
/// demo-blocking Genesis bug).
///
/// PROVES write-segment == read-segment: a Genesis vertical the SSE writes
/// under a fresh/unique `X-Argo-Tenant-Id` HEADER (the exact shape the SPA
/// sends, and the one that USED to write under the header slug while the hire
/// searched the cwd slug → `422 genesis_gate_failed` /
/// `contract_confirmation: mismatch`) is now:
///
///   1. written under the **cwd-derived workspace tenant** (the SAME
///      `ensure_workspace(name, cwd)` the hire-commit endpoint resolves), so
///   2. the hire-gate resolver (`genesis_vertical_dir_for_tenant`) FINDS it in
///      that cwd segment, and
///   3. when the hire confirms with the EXACT `contract_digest` the SSE
///      surfaced, the re-derived contract MATCHES (`contract_confirmation ==
///      "confirmed"`, NOT `"mismatch"`) — i.e. the operator's confirmation
///      would clear the gate's contract axis, which is precisely the axis that
///      flipped 201 → 422 before the fix.
///
/// (The composed gate may still hold at `needs_review` in CI because the
/// safety judge has no live provider and degrades fail-safe — that is the
/// SEPARATE, correct safety behaviour, not the tenant bug. This test asserts
/// the contract axis confirms + the segment matches, which is exactly what the
/// fix changes; it does not assert a full `pass`, which would require a live
/// provider.)
#[tokio::test]
async fn sse_written_vertical_is_resolved_by_hire_gate_in_same_cwd_segment() {
    let tmp = TempDir::new().unwrap();
    // Force the safety judge to degrade deterministically (no live provider)
    // so the test is provider-independent; the contract axis we assert on is
    // computed from disk, not the provider.
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

    // A FRESH session header — divergent from the cwd tenant. This is the
    // SPA-shaped request that provoked the 422 before the fix.
    let fresh_header = "deadbeef-0000-4000-8000-feedfacecafe";
    let raw = drive_handler(build_request(&body, Some(fresh_header)), Arc::clone(&store)).await;
    assert!(raw.starts_with("HTTP/1.1 200 OK"), "got: {raw}");
    let payload = parse_vertical_ready(&raw);
    let vertical_id = payload["vertical_id"].as_str().expect("vertical_id present");
    let sse_digest = payload["contract_digest"]
        .as_str()
        .expect("contract_digest present")
        .to_string();
    assert!(!sse_digest.is_empty(), "SSE must surface a non-empty contract digest");

    // Resolve the cwd-derived workspace tenant EXACTLY as the hire-commit
    // endpoint does (`routes_workers_hire::resolve_tenant_principal`).
    let cwd = std::env::current_dir().unwrap();
    let name = cwd.file_name().and_then(|s| s.to_str()).unwrap_or("workspace");
    let (cwd_tenant, _ws) = store
        .ensure_workspace(name, &cwd.to_string_lossy())
        .await
        .unwrap();
    let cwd_tenant_str = cwd_tenant.to_string();

    // (1) + (2): the hire-gate resolver finds the SSE-written vertical under
    // the cwd tenant segment — the SAME `genesis_vertical_dir_for_tenant` call
    // the hire fence uses. Pre-fix this returned None (the manifest was under
    // the fresh-header slug, not the cwd slug) → 422.
    let genesis_dir = super::super::genesis_tenant::genesis_vertical_dir_for_tenant(
        tmp.path(),
        &cwd_tenant_str,
        vertical_id,
    )
    .expect(
        "hire gate MUST resolve the SSE-written vertical under the cwd tenant \
         segment (write-segment == read-segment)",
    );
    assert!(genesis_dir.join("vertical.toml").is_file());

    // It must NOT be discoverable under the fresh HEADER segment any more (that
    // is the bug: the SSE used to write there, splitting write from read).
    assert!(
        super::super::genesis_tenant::genesis_vertical_dir_for_tenant(
            tmp.path(),
            fresh_header,
            vertical_id,
        )
        .is_none(),
        "vertical must NOT be written under the request-header segment"
    );

    // (3): the hire fence re-runs the gate against that SAME on-disk dir and
    // confirms with the digest the SSE surfaced. The re-derived contract MUST
    // match — `contract_confirmation == "confirmed"`, NOT `"mismatch"` — which
    // is the axis that flipped 201 → 422 before the fix.
    let gate = super::super::routes_genesis_gate::run_hire_gate_from_dir_with_confirmation(
        &genesis_dir,
        Some(&sse_digest),
    )
    .await;
    assert_eq!(
        gate.contract_confirmation, "confirmed",
        "the operator's confirmation of the SSE-surfaced digest must MATCH the \
         re-derived contract (write-segment == read-segment); got reasons: {:?}",
        gate.reasons
    );
    assert_eq!(
        gate.contract_digest, sse_digest,
        "the hire-gate re-derives the IDENTICAL contract digest the SSE surfaced"
    );
    // And it is NOT a hard contract mismatch block — the failure mode the bug
    // produced. (A degraded safety judge may still hold the composed state at
    // needs_review in CI; that is the separate, correct safety behaviour.)
    assert!(
        !gate.reasons.iter().any(|r| r.contains("role-contract confirmation mismatch")),
        "must not be a contract-mismatch block; got reasons: {:?}",
        gate.reasons
    );
}
