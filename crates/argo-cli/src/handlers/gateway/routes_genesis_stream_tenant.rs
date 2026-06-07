//! Tenant resolution for the Genesis SSE write path (`routes_genesis_stream`).
//!
//! Extracted from `routes_genesis_stream.rs` to keep that file under the
//! per-file LOC ceiling. Owns the single load-bearing decision behind the
//! tenant-segment-mismatch fix: under DEC-025 #9 (single-tenant-per-cwd for v1)
//! the generated vertical is written under the SAME **cwd-derived workspace
//! tenant the hire-commit endpoint reads under** — not the request
//! `X-Argo-Tenant-Id` header — so the SSE-write segment and the hire-read
//! segment can never diverge.

use argo_store::SessionStore;
use std::sync::Arc;

/// Resolve the tenant the generated vertical is **written under**.
///
/// This MUST agree with the tenant `routes_workers_hire::resolve_tenant_principal`
/// reads under, or the hire gate searches a different `.genesis/<slug>/` segment
/// than the SSE write wrote to (`422 genesis_gate_failed`). Both now resolve
/// IDENTITY-FIRST (WS1): the signed-in user's `__Host-session` JWT tenant wins,
/// then an explicit tenant override, then the cwd/anonymous fallback — via the
/// shared `resolve_tenant::resolve_tenant_for_request`. Previously both used the
/// cwd-derived workspace tenant (DEC-025 #9, single-tenant-per-cwd for v1); the
/// real-mode walk moved signed-in users onto their OWN tenant, and this write
/// segment follows so it still matches the hire read (pt2).
///
/// `store` `None` (test / CLI callers with no DB), or a resolver error, → fall
/// back to the request `X-Argo-Tenant-Id` header via [`request_tenant_id`],
/// preserving prior behaviour for those paths.
pub(super) async fn resolve_write_tenant(store: Option<&Arc<SessionStore>>, req: &str) -> String {
    if let Some(store) = store {
        if let Ok(tenant) = super::resolve_tenant::resolve_tenant_for_request(req, store).await {
            return tenant.to_string();
        }
    }
    request_tenant_id(req)
}

/// Read the operator-facing `X-Argo-Tenant-Id` header (the rate-limit helper
/// reads a separate `X-Tenant-Id` convention). Falls back to `?tenant=` query
/// param, then `anonymous`. Retained as the [`resolve_write_tenant`] fallback
/// for store-less callers; the write-segment under DEC-025 #9 now prefers the
/// cwd-derived workspace tenant when a store is present.
pub(super) fn request_tenant_id(req: &str) -> String {
    if let Some(value) = header_value(req, "x-argo-tenant-id") {
        if !value.is_empty() {
            return value;
        }
    }
    if let Some(target) = req.lines().next().and_then(|l| l.split_whitespace().nth(1)) {
        if let Some((_, qs)) = target.split_once('?') {
            for kv in qs.split('&') {
                if let Some(("tenant", value)) = kv.split_once('=') {
                    if !value.is_empty() {
                        return value.to_string();
                    }
                }
            }
        }
    }
    "anonymous".to_string()
}

fn header_value(req: &str, name_lower: &str) -> Option<String> {
    let headers = req
        .split_once("\r\n\r\n")
        .map_or(req, |(headers, _body)| headers);
    headers.lines().skip(1).find_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.trim()
            .eq_ignore_ascii_case(name_lower)
            .then(|| value.trim().to_string())
            .filter(|s| !s.is_empty())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req_with(headers: &str) -> String {
        format!("POST /api/v1/genesis HTTP/1.1\r\n{headers}\r\n\r\n")
    }

    #[test]
    fn request_tenant_id_reads_header_first() {
        let r = req_with("Host: x\r\nX-Argo-Tenant-Id: op-alice\r\n");
        assert_eq!(request_tenant_id(&r), "op-alice");
    }

    #[test]
    fn request_tenant_id_falls_back_to_query_then_anonymous() {
        let with_query = "POST /api/v1/genesis?tenant=op-bob HTTP/1.1\r\nHost: x\r\n\r\n";
        assert_eq!(request_tenant_id(with_query), "op-bob");
        let none = req_with("Host: x\r\n");
        assert_eq!(request_tenant_id(&none), "anonymous");
    }
}
