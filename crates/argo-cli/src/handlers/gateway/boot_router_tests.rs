//! Control-flow regression tests for `try_dispatch_gates` (DEC-057 Phase 1).
//!
//! The suspended-tenant write lockout lives in `try_dispatch_gates`, ahead of
//! the streaming / SSE / Genesis peel-offs. The direct-call tests in
//! `middleware_tenant_suspended_tests.rs` exercise the predicate only and
//! CANNOT catch a route that skips the check — which is exactly how the
//! streaming bypass shipped (the check was previously in `dispatch_routes`,
//! below the peel-offs). These tests drive a suspended tenant's request through
//! the REAL gate chain and pin the choke-point PLACEMENT, not just the predicate.

use super::*;
use crate::handlers::gateway::auth_magic_link::mint_jwt;
use crate::handlers::gateway::boot_serve::setup_routing_gates;
use crate::handlers::gateway::request_audit::RequestAuditDraft;
use crate::handlers::gateway::security_headers;
use crate::handlers::gateway::shared_env_lock;
use crate::handlers::gateway::shutdown::ShutdownController;
use argo_config::{ArgoConfig, GatewayConfig};
use argo_identity::tenant::{Tenant, TenantStatus};
use argo_identity::tenant_store::{insert_tenant, set_tenant_status};
use argo_store::SessionStore;
use chrono::Utc;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::net::{TcpListener, TcpStream};

/// Drive a single raw HTTP request through `try_dispatch_gates` with a default
/// gate set + the given store, returning the gate verdict and what was written
/// back to the client. The 403 lockout body is tiny, so the inline write fits
/// the socket buffer without a concurrent reader (no deadlock).
async fn drive_gates(store: &Arc<SessionStore>, req: &str) -> (Option<u16>, String) {
    let cfg = ArgoConfig::default();
    let gateway_cfg = GatewayConfig::default();
    let gates = setup_routing_gates(&cfg, &gateway_cfg).expect("gates");
    let shutdown = ShutdownController::new();
    let ctx = ConnectionCtx {
        behind_proxy: &gateway_cfg.behind_proxy,
        request_audit_cfg: &gateway_cfg.request_audit,
        security_headers_cfg: &gateway_cfg.security_headers,
        ip_limiter: &gates.ip_limiter,
        ip_allowlist: &gates.ip_allowlist,
        store: Some(store),
        shutdown_ctl: &shutdown,
    };
    let sec = security_headers::header_lines("application/json", false, ctx.security_headers_cfg);
    let peer: SocketAddr = "127.0.0.1:40000".parse().unwrap();
    let src_ip = peer.ip();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let mut client = TcpStream::connect(addr).await.unwrap();
    let (mut server_side, _) = listener.accept().await.unwrap();

    let mut audit: Option<RequestAuditDraft> = None;
    let verdict =
        try_dispatch_gates(&mut server_side, peer, req, src_ip, &mut audit, &sec, &ctx).await;

    // On a matched gate (e.g. the 403 lockout) the stream was written + shut.
    // On `None` (no gate matched) nothing was written and the socket is still
    // open — drop the server side so the client read gets a clean EOF instead
    // of blocking.
    drop(server_side);
    let mut buf = Vec::new();
    let _ = client.read_to_end(&mut buf).await;
    (verdict, String::from_utf8_lossy(&buf).into_owned())
}

async fn suspended_store_and_jwt() -> (Arc<SessionStore>, String) {
    std::env::set_var("SKYCOT_AUTH_JWT_SECRET", "test-secret-suspend-gates");
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();
    let t = Tenant::new("Acme".into(), "b@acme.com".into());
    insert_tenant(&store, &t).await.unwrap();
    set_tenant_status(&store, t.id, TenantStatus::Suspended, Utc::now())
        .await
        .unwrap();
    let now = Utc::now().timestamp();
    let (jwt, _jti) = mint_jwt("user-x", "u@acme.com", Some(&t.id.to_string()), Some("owner"), now)
        .expect("mint");
    (store, jwt)
}

/// THE regression for the adversarial-review blocker: every mutating STREAMING /
/// SSE / Genesis route must be refused `403 account_suspended` for a suspended
/// tenant — i.e. the lockout runs BEFORE these routes peel off in
/// `try_dispatch_gates`. Before the fix these returned `Some(200)` (the handler
/// ran); after, they short-circuit to `403` at the gate.
#[tokio::test]
async fn suspended_tenant_blocked_on_streaming_and_genesis_routes() {
    let _env = shared_env_lock().await;
    let (store, jwt) = suspended_store_and_jwt().await;

    for path in [
        "POST /sessions/abc/runs/stream",
        "POST /sessions/abc/replay/stream",
        "POST /forge/optimize/stream",
        "POST /api/v1/genesis",
    ] {
        let req = format!("{path} HTTP/1.1\r\nHost: x\r\nAuthorization: Bearer {jwt}\r\n\r\n");
        let (verdict, body) = drive_gates(&store, &req).await;
        assert_eq!(
            verdict,
            Some(403),
            "{path}: suspended tenant must be 403 at the gate (lockout must run before the streaming peel-off), got {verdict:?}"
        );
        assert!(
            body.contains("account_suspended"),
            "{path}: response must be account_suspended, got: {body}"
        );
    }
}

/// Reads stay OK for a suspended tenant: a GET is not a write-lockout candidate,
/// so the gate does NOT short-circuit to 403 (it falls through — `None` — to the
/// normal route dispatch). Pins the reads-OK half of the lockout at the gate.
#[tokio::test]
async fn suspended_tenant_read_not_blocked_by_lockout() {
    let _env = shared_env_lock().await;
    let (store, jwt) = suspended_store_and_jwt().await;
    let req =
        format!("GET /api/v1/sessions HTTP/1.1\r\nHost: x\r\nAuthorization: Bearer {jwt}\r\n\r\n");
    let (verdict, body) = drive_gates(&store, &req).await;
    assert!(
        verdict != Some(403) && !body.contains("account_suspended"),
        "a GET (read) must not be blocked by the suspend lockout, got verdict={verdict:?} body={body}"
    );
}
