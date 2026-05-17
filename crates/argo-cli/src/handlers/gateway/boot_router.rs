//! Per-connection request dispatch + bind-time audit helpers,
//! extracted from `boot::start` so `boot.rs` stays under the 500-LOC
//! ceiling and leaves headroom for new mount lines (see
//! `claude-gateway-boot-split-routes-v2`).
//!
//! The brief originally described an `axum::Router` assemble step;
//! the actual gateway uses a raw `TcpListener` with a manual
//! `if/else if` route-classification chain inside `boot::start`'s
//! `tokio::select!` accept arm. The semantics are the same — every
//! new route mount adds another arm — so this module hosts the
//! extracted dispatch body. No behavior change: arms keep their
//! original order, the metrics observations fire at the same point.

use super::boot_helpers::{http_status_label, parse_status_line};
use super::ip_allowlist::{self, AllowResult, IpAllowlist};
use super::rate_limit_ip::{
    self, rate_limited_response_bytes, IpRateLimiter, RateLimitIpCheckResult,
};
use super::request_audit::RequestAuditDraft;
use super::shutdown::ShutdownController;
use super::{
    metrics, proxy_guard, request_audit, routes, routes_channels, routes_openmetrics,
    routes_orchestrator, routes_skills, runs_streaming, security_headers, streaming,
};
use argo_audit::{AuditKind, AuditSink, SYSTEM_AUDIT_TENANT_ID};
use argo_config::{
    GatewayBehindProxyConfig, GatewayRequestAuditConfig, GatewaySecurityHeadersConfig,
};
use argo_store::SessionStore;
use serde_json::{json, Value};
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Instant;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// Borrowed bag of references the per-connection dispatch needs.
/// Built once in `boot::start` and reused for every accepted
/// connection; nothing escapes the call.
pub(super) struct ConnectionCtx<'a> {
    pub behind_proxy: &'a GatewayBehindProxyConfig,
    pub request_audit_cfg: &'a GatewayRequestAuditConfig,
    pub security_headers_cfg: &'a GatewaySecurityHeadersConfig,
    pub ip_limiter: &'a Arc<IpRateLimiter>,
    pub ip_allowlist: &'a Arc<IpAllowlist>,
    pub store: Option<&'a Arc<SessionStore>>,
    pub shutdown_ctl: &'a ShutdownController,
}

/// Serve one accepted TCP connection end-to-end: parse the request,
/// run the IP-allowlist + rate-limit gates, dispatch to the matching
/// route family, then observe the per-request metric counters.
///
/// Mirrors the original inline body of `boot::start`'s `accept` arm
/// verbatim. Order of arms is load-bearing — axum would express the
/// same precedence via route-registration order; here it lives in
/// the `try_dispatch_gates` → `dispatch_routes` fall-through.
pub(super) async fn handle_connection(
    stream: &mut TcpStream,
    peer: SocketAddr,
    ctx: &ConnectionCtx<'_>,
) {
    let mut buf = [0_u8; 4096];
    let n = stream.read(&mut buf).await.unwrap_or(0);
    let req = std::str::from_utf8(&buf[..n]).unwrap_or("");
    let req_t0 = Instant::now();
    let req_method = req.split_whitespace().next().unwrap_or("").to_string();
    let req_route = super::rate_limit::classify_request(req).into_owned();
    let src_ip = if ctx.behind_proxy.expected_proxy {
        proxy_guard::extract_real_ip(req, peer, ctx.behind_proxy)
    } else {
        rate_limit_ip::extract_ip(req, peer, ctx.ip_limiter.trust_xff())
    };
    let mut audit = request_audit::capture(req, peer.ip(), src_ip, ctx.request_audit_cfg);
    let https = security_headers::request_is_https(req);
    let sec = security_headers::header_lines("application/json", https, ctx.security_headers_cfg);
    let req_status =
        match try_dispatch_gates(stream, peer, req, src_ip, &mut audit, &sec, ctx).await {
            Some(s) => s,
            None => dispatch_routes(stream, req, &mut audit, &sec, ctx).await,
        };
    let req_status_str = http_status_label(req_status);
    routes_openmetrics::brief_request_total()
        .with_label_values(&[&req_method, &req_route, req_status_str])
        .inc();
    routes_openmetrics::brief_request_duration()
        .with_label_values(&[&req_method, &req_route])
        .observe(req_t0.elapsed().as_secs_f64());
}

/// Pre-route gates: allow/deny lists, rate limits, streaming endpoints,
/// metrics scrape targets, websocket upgrade, SSO. Returns `Some(status)`
/// if a gate matched (and produced its response); `None` to fall through
/// to the route-family dispatch.
async fn try_dispatch_gates(
    stream: &mut TcpStream,
    peer: SocketAddr,
    req: &str,
    src_ip: IpAddr,
    audit: &mut Option<RequestAuditDraft>,
    sec: &str,
    ctx: &ConnectionCtx<'_>,
) -> Option<u16> {
    if let AllowResult::Deny = ctx.ip_allowlist.check(req, peer) {
        let logged_ip = ctx.ip_allowlist.source_ip(req, peer);
        ip_allowlist::log_deny(logged_ip, req);
        let bytes = ip_allowlist::deny_response_bytes();
        let _ = stream.write_all(&bytes).await;
        let _ = stream.shutdown().await;
        return Some(200);
    }
    if let RateLimitIpCheckResult::Deny { retry_after_secs } = ctx.ip_limiter.check(src_ip) {
        tracing::warn!(
            target: "gateway::rate_limit_ip",
            ip = %src_ip,
            retry_after_secs,
            "per-IP rate limit triggered"
        );
        routes_openmetrics::brief_rate_limit_throttle_total()
            .with_label_values(&["ip", ""])
            .inc();
        let bytes = rate_limited_response_bytes(retry_after_secs);
        let _ = stream.write_all(&bytes).await;
        let body_len = request_audit::response_body_len_from_bytes(&bytes);
        request_audit::spawn_record(audit.take(), ctx.store, "429 Too Many Requests", body_len);
        let _ = stream.shutdown().await;
        return Some(429);
    }
    if super::rate_limit_tenant::write_if_throttled(req, stream, sec).await {
        routes_openmetrics::brief_rate_limit_throttle_total()
            .with_label_values(&["tenant", &super::rate_limit::extract_tenant(req)])
            .inc();
        request_audit::spawn_record(audit.take(), ctx.store, "429 Too Many Requests", 0);
        return Some(429);
    }
    if runs_streaming::is_streaming_runs_request(req) {
        let _ = runs_streaming::serve_streamed_run(req, ctx.store, stream).await;
        request_audit::spawn_record(audit.take(), ctx.store, "200 OK", 0);
        let _ = stream.shutdown().await;
        return Some(200);
    }
    if streaming::is_streaming_list_request(req) {
        let _ = streaming::serve_streamed_list(req, ctx.store, stream).await;
        request_audit::spawn_record(audit.take(), ctx.store, "200 OK", 0);
        let _ = stream.shutdown().await;
        return Some(200);
    }
    if metrics::is_metrics_request(req) {
        let bytes = metrics::http_response(ctx.store).await;
        let _ = stream.write_all(&bytes).await;
        let body_len = request_audit::response_body_len_from_bytes(&bytes);
        request_audit::spawn_record(audit.take(), ctx.store, "200 OK", body_len);
        let _ = stream.shutdown().await;
        return Some(200);
    }
    if super::boot_telemetry_admin::try_serve(req, stream).await {
        return Some(200);
    }
    if routes_openmetrics::is_openmetrics_request(req) {
        let bytes = routes_openmetrics::http_response();
        let _ = stream.write_all(&bytes).await;
        let _ = stream.shutdown().await;
        return Some(200);
    }
    if super::ws_upgrade::is_ws_request(req) {
        let _ = super::ws_upgrade::serve(req, ctx.store, stream).await;
        request_audit::spawn_record(audit.take(), ctx.store, "101 Switching Protocols", 0);
        let _ = stream.shutdown().await;
        return Some(200);
    }
    if super::sso_dispatch::is_sso_request(req) {
        super::sso_dispatch::serve(req, ctx.store, stream).await;
        return Some(200);
    }
    None
}

/// Route-family dispatch: email webhook → orchestrator → skills → default.
/// Every arm writes a formatted `HTTP/1.1` response and records the audit
/// row. Returns the parsed status code.
async fn dispatch_routes(
    stream: &mut TcpStream,
    req: &str,
    audit: &mut Option<RequestAuditDraft>,
    sec: &str,
    ctx: &ConnectionCtx<'_>,
) -> u16 {
    if routes_channels::is_email_webhook(req) {
        let (status_line, body) = routes_channels::email_webhook_response(req).await;
        let resp = format_response(status_line, &body, sec);
        let _ = stream.write_all(resp.as_bytes()).await;
        let _ = stream.shutdown().await;
        request_audit::spawn_record(audit.take(), ctx.store, status_line, body.len());
        return parse_status_line(status_line);
    }
    if let Some((status_line, body)) = routes_orchestrator::dispatch_route(req, ctx.store).await {
        let resp = format_response(status_line, &body, sec);
        let _ = stream.write_all(resp.as_bytes()).await;
        request_audit::spawn_record(audit.take(), ctx.store, status_line, body.len());
        let _ = stream.shutdown().await;
        return parse_status_line(status_line);
    }
    if let Some((status_line, body)) = routes_skills::dispatch_route(req, ctx.store).await {
        let resp = format_response(status_line, &body, sec);
        let _ = stream.write_all(resp.as_bytes()).await;
        request_audit::spawn_record(audit.take(), ctx.store, status_line, body.len());
        let _ = stream.shutdown().await;
        return parse_status_line(status_line);
    }
    let (status_line, body) = routes::dispatch_route(req, ctx.store, ctx.shutdown_ctl).await;
    let resp = format_response(status_line, &body, sec);
    let _ = stream.write_all(resp.as_bytes()).await;
    request_audit::spawn_record(audit.take(), ctx.store, status_line, body.len());
    let _ = stream.shutdown().await;
    parse_status_line(status_line)
}

fn format_response(status_line: &str, body: &str, sec: &str) -> String {
    format!(
        "HTTP/1.1 {status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{sec}\r\n{}",
        body.len(),
        body
    )
}

pub(super) fn is_loopback_bind(bind: &str) -> bool {
    bind.parse::<IpAddr>()
        .map(|ip| ip.is_loopback())
        .unwrap_or(false)
}

pub(super) fn insecure_bind_audit_payload(bind: &str, port: u16, pid: u32) -> Value {
    json!({
        "event_type": "gateway_insecure_bind",
        "severity": "warn",
        "actor": "system",
        "bind": bind,
        "port": port,
        "pid": pid,
        "tls_enabled": false,
        "bind_source": "cli"
    })
}

pub(super) async fn emit_insecure_bind_audit(store: &Arc<SessionStore>, bind: &str, port: u16) {
    let sink = AuditSink::new_with_tenant(Arc::clone(store), SYSTEM_AUDIT_TENANT_ID.to_string());
    let payload = insecure_bind_audit_payload(bind, port, std::process::id());
    if let Err(e) = sink
        .write(None, AuditKind::GatewayInsecureBind, payload, None)
        .await
    {
        tracing::warn!(target: "gateway::bind", error = %e, "gateway insecure bind audit write failed");
        return;
    }
    if let Err(e) = sink.flush_pending().await {
        tracing::warn!(target: "gateway::bind", error = %e, "gateway insecure bind audit flush failed");
    }
}
