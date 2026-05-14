//! Separate per-HTTP-request audit table capture.
use super::routes_auth_principal::principal_id_for_audit;
use argo_config::GatewayRequestAuditConfig;
use argo_store::gateway_request_audit::{self, NewGatewayRequestAudit};
use argo_store::SessionStore;
use chrono::Utc;
use sha2::{Digest, Sha256};
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

const BODY_HASH_LIMIT: usize = 1024 * 1024;

pub(super) struct RequestAuditDraft {
    request_id: Uuid,
    received_at: i64,
    started: Instant,
    principal: Option<String>,
    source_ip: String,
    method: String,
    path: String,
    request_body_sha256: Option<[u8; 32]>,
    user_agent: Option<String>,
}

pub(super) fn capture(
    req: &str,
    peer_ip: IpAddr,
    observed_ip: IpAddr,
    cfg: &GatewayRequestAuditConfig,
) -> Option<RequestAuditDraft> {
    if !cfg.enabled {
        return None;
    }
    let (method, path) = method_and_path(req)?;
    let source_ip = if cfg.behind_proxy {
        x_forwarded_for(req).unwrap_or(observed_ip).to_string()
    } else {
        peer_ip.to_string()
    };
    Some(RequestAuditDraft {
        request_id: Uuid::now_v7(),
        received_at: Utc::now().timestamp_micros(),
        started: Instant::now(),
        principal: principal_id_for_audit(req),
        source_ip,
        method: method.to_string(),
        path: path.to_string(),
        request_body_sha256: body_hash(req, method, &cfg.body_hash_for_methods),
        user_agent: user_agent(req, cfg.user_agent_max_len),
    })
}

pub(super) fn spawn_record(
    draft: Option<RequestAuditDraft>,
    store: Option<&Arc<SessionStore>>,
    status_line: &str,
    response_body_len: usize,
) {
    let (Some(draft), Some(store)) = (draft, store.cloned()) else {
        return;
    };
    let status_code = parse_status(status_line);
    let latency_ms = u32::try_from(draft.started.elapsed().as_millis()).unwrap_or(u32::MAX);
    let response_body_len = u32::try_from(response_body_len).unwrap_or(u32::MAX);
    tokio::spawn(async move {
        let entry = NewGatewayRequestAudit {
            request_id: draft.request_id,
            received_at: draft.received_at,
            principal: draft.principal,
            source_ip: draft.source_ip,
            method: draft.method,
            path: draft.path,
            status_code,
            latency_ms,
            request_body_sha256: draft.request_body_sha256,
            response_body_len,
            user_agent: draft.user_agent,
        };
        if let Err(e) = gateway_request_audit::record(store.pool(), &entry).await {
            tracing::warn!(target: "gateway::request_audit", error = %e, "request audit write failed");
        }
    });
}

pub(super) fn response_body_len_from_bytes(resp: &[u8]) -> usize {
    let needle = b"\r\n\r\n";
    resp.windows(needle.len())
        .position(|w| w == needle)
        .map(|idx| resp.len().saturating_sub(idx + needle.len()))
        .unwrap_or(0)
}

fn method_and_path(req: &str) -> Option<(&str, &str)> {
    let line = req.split("\r\n").next()?;
    let (method, rest) = line.split_once(' ')?;
    let target = rest.split_once(' ').map(|(t, _)| t).unwrap_or(rest);
    let path = target.split_once('?').map(|(p, _)| p).unwrap_or(target);
    if method.is_empty() || path.is_empty() {
        return None;
    }
    Some((method, path))
}

fn parse_status(status_line: &str) -> u16 {
    status_line
        .split_whitespace()
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

fn body_hash(req: &str, method: &str, methods: &[String]) -> Option<[u8; 32]> {
    if !methods.iter().any(|m| m.eq_ignore_ascii_case(method)) {
        return None;
    }
    let body = req.split_once("\r\n\r\n").map(|(_, b)| b.as_bytes())?;
    if body.is_empty() {
        return None;
    }
    let mut hasher = Sha256::new();
    hasher.update(&body[..body.len().min(BODY_HASH_LIMIT)]);
    Some(hasher.finalize().into())
}

fn x_forwarded_for(req: &str) -> Option<IpAddr> {
    header_value(req, "x-forwarded-for")
        .and_then(|v| v.split(',').next())
        .and_then(|v| v.trim().parse().ok())
}

fn user_agent(req: &str, max_len: u32) -> Option<String> {
    let limit = usize::try_from(max_len).unwrap_or(200);
    Some(
        header_value(req, "user-agent")?
            .trim()
            .chars()
            .take(limit)
            .collect(),
    )
}

fn header_value<'a>(req: &'a str, name: &str) -> Option<&'a str> {
    let needle = format!("{name}:");
    req.split("\r\n").find_map(|line| {
        if line.to_ascii_lowercase().starts_with(&needle) {
            line.split_once(':').map(|(_, v)| v.trim())
        } else {
            None
        }
    })
}

#[cfg(test)]
#[path = "request_audit_tests.rs"]
mod tests;
