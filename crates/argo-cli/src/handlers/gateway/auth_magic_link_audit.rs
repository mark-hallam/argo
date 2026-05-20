use super::super::rate_limit_ip;
use argo_audit::{AuditKind, AuditSink, ANONYMOUS_AUDIT_TENANT_ID};
use argo_store::SessionStore;
use dashmap::DashMap;
use serde_json::json;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

/// Sliding-window verify rate limit: 60 attempts/IP/minute.
pub(crate) const VERIFY_RATE_LIMIT_PER_MIN: usize = 60;
pub(crate) const VERIFY_RATE_LIMIT_WINDOW: Duration = Duration::from_secs(60);

pub(crate) fn signup_source_ip(req: &str) -> IpAddr {
    rate_limit_ip::parse_first_xff_hop(req).unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST))
}

pub(crate) fn error_resp(status: &'static str, msg: &str) -> (&'static str, String) {
    (status, serde_json::json!({ "error": msg }).to_string())
}

/// Sliding-window /verify rate-limit map. Process-global so every
/// connection-handling task sees the same counters.
fn verify_buckets() -> &'static DashMap<IpAddr, Vec<Instant>> {
    static INSTANCE: OnceLock<DashMap<IpAddr, Vec<Instant>>> = OnceLock::new();
    INSTANCE.get_or_init(DashMap::new)
}

/// Returns `true` when the request fits inside the 60/min budget.
pub(crate) fn verify_rate_limit_check(ip: IpAddr, now: Instant) -> bool {
    let mut entry = verify_buckets().entry(ip).or_default();
    entry.retain(|t| now.saturating_duration_since(*t) < VERIFY_RATE_LIMIT_WINDOW);
    if entry.len() >= VERIFY_RATE_LIMIT_PER_MIN {
        return false;
    }
    entry.push(now);
    true
}

pub(crate) async fn emit_auth_failed(store: &Arc<SessionStore>, reason: &str, source_ip: IpAddr) {
    let payload = json!({
        "method": "magic_link",
        "reason": reason,
        "source_ip": source_ip.to_string(),
    });
    write_audit(
        store,
        ANONYMOUS_AUDIT_TENANT_ID,
        AuditKind::AuthFailed,
        payload,
    )
    .await;
}

pub(crate) async fn emit_auth_succeeded(
    store: &Arc<SessionStore>,
    user_id: &str,
    tenant_id: Option<&str>,
    source_ip: IpAddr,
) {
    let payload = json!({
        "method": "magic_link",
        "user_id": user_id,
        "tenant_id": tenant_id,
        "source_ip": source_ip.to_string(),
    });
    write_audit(
        store,
        tenant_id.unwrap_or(ANONYMOUS_AUDIT_TENANT_ID),
        AuditKind::AuthSucceeded,
        payload,
    )
    .await;
}

async fn write_audit(
    store: &Arc<SessionStore>,
    tenant_id: &str,
    kind: AuditKind,
    payload: serde_json::Value,
) {
    let sink = AuditSink::new_with_tenant(Arc::clone(store), tenant_id.to_string());
    if let Err(e) = sink.write(None, kind, payload, None).await {
        tracing::warn!(
            target: "gateway::auth_magic_link",
            error = %e,
            kind = ?kind,
            "auth audit write failed",
        );
    }
}
