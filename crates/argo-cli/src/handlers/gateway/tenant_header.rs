//! Shared `X-Argo-Tenant-Id` header extractor used by gateway
//! handlers that need to bind their DB reads to the caller's
//! tenant. Mirrors the per-file `request_tenant_id` helpers in
//! `routes_audit_replay.rs:156`, `routes_audit_detail.rs:85`,
//! and `routes_audit_export_signing.rs:153` so the cluster-2
//! handlers in `mission-gateway-session-routes-tenant-bind`
//! share one implementation.
//!
//! Falls back to [`argo_audit::DEFAULT_AUDIT_TENANT_ID`] when
//! the header is absent — same fallback the audit family uses.
//! Placed in its own module so the cluster-2 fix stays inside
//! gateway/mod.rs's LOC budget.

use argo_audit::DEFAULT_AUDIT_TENANT_ID;
use argo_store::SessionStore;
use argo_types::{JudgeRecordId, SessionId};

pub(crate) fn request_tenant_id(req: &str) -> String {
    let headers = req
        .split_once("\r\n\r\n")
        .map_or(req, |(headers, _body)| headers);
    headers
        .lines()
        .skip(1)
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.trim()
                .eq_ignore_ascii_case("x-argo-tenant-id")
                .then(|| value.trim().to_string())
                .filter(|s| !s.is_empty())
        })
        .unwrap_or_else(|| DEFAULT_AUDIT_TENANT_ID.to_string())
}

pub(crate) async fn assert_session_belongs_to_tenant(
    store: &SessionStore,
    sid: SessionId,
    raw_sid: &str,
    tenant_id: &str,
) -> Result<(), (&'static str, String)> {
    match store.session_belongs_to_tenant(sid, tenant_id).await {
        Ok(true) => Ok(()),
        Ok(false) => Err((
            "404 Not Found",
            format!("{{\"error\":\"no session {raw_sid}\"}}"),
        )),
        Err(e) => Err((
            "500 Internal Server Error",
            format!("{{\"error\":\"session_belongs_to_tenant: {e}\"}}"),
        )),
    }
}

pub(crate) async fn assert_judge_record_belongs_to_tenant(
    store: &SessionStore,
    record_id: JudgeRecordId,
    raw_record: &str,
    tenant_id: &str,
) -> Result<(), (&'static str, String)> {
    let Ok(tenant_uuid) = uuid::Uuid::parse_str(tenant_id) else {
        return Err((
            "404 Not Found",
            format!("{{\"error\":\"no judge record with id {raw_record}\"}}"),
        ));
    };
    let exists: Option<i64> = sqlx::query_scalar(
        "SELECT 1 FROM judge_record jr \
         JOIN session s ON s.id = jr.session_id \
         WHERE jr.id = ? AND s.tenant_id = ? LIMIT 1",
    )
    .bind(record_id.0.as_bytes().as_slice())
    .bind(tenant_uuid.as_bytes().as_slice())
    .fetch_optional(store.pool())
    .await
    .map_err(|e| {
        (
            "500 Internal Server Error",
            format!("{{\"error\":\"judge_record_belongs_to_tenant: {e}\"}}"),
        )
    })?;

    if exists.is_some() {
        Ok(())
    } else {
        Err((
            "404 Not Found",
            format!("{{\"error\":\"no judge record with id {raw_record}\"}}"),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::request_tenant_id;
    use argo_audit::DEFAULT_AUDIT_TENANT_ID;

    fn req(headers: &[(&str, &str)]) -> String {
        let mut out = String::from("GET / HTTP/1.1\r\n");
        for (k, v) in headers {
            out.push_str(&format!("{k}: {v}\r\n"));
        }
        out.push_str("\r\n");
        out
    }

    #[test]
    fn reads_exact_header_case_insensitively() {
        let r = req(&[("X-Argo-Tenant-Id", "tenant-a")]);
        assert_eq!(request_tenant_id(&r), "tenant-a");
        let r = req(&[("x-argo-tenant-id", "tenant-b")]);
        assert_eq!(request_tenant_id(&r), "tenant-b");
    }

    #[test]
    fn missing_header_falls_back_to_default() {
        let r = req(&[("Host", "x")]);
        assert_eq!(request_tenant_id(&r), DEFAULT_AUDIT_TENANT_ID);
    }

    #[test]
    fn empty_header_value_falls_back_to_default() {
        let r = req(&[("X-Argo-Tenant-Id", "")]);
        assert_eq!(request_tenant_id(&r), DEFAULT_AUDIT_TENANT_ID);
    }
}
