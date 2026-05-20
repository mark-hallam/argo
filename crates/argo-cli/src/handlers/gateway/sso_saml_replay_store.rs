//! Transient SAML AuthnRequest replay guard.

use crate::handlers::gateway::routes_channels;
use crate::handlers::gateway::sso_session::{self, SessionPayload};
use argo_store::SessionStore;
use chrono::{DateTime, Utc};
use std::sync::Arc;

pub(super) const SAML_REQUEST_COOKIE: &str = "argo_saml_request";

pub(super) fn pending_cookie(
    tenant: &str,
    request_id: &str,
    now: DateTime<Utc>,
) -> Result<String, &'static str> {
    let secret = routes_channels::hmac_secret().map_err(|_| "hmac secret")?;
    signed_pending_cookie(tenant, request_id, &secret, now).map_err(|_| "sign")
}

fn signed_pending_cookie(
    tenant: &str,
    request_id: &str,
    secret: &str,
    now: DateTime<Utc>,
) -> Result<String, sso_session::SessionError> {
    let payload = SessionPayload {
        sub: request_id.to_string(),
        email: String::new(),
        tenant: tenant.to_string(),
        mechanism: "saml_request".into(),
        iat: now.timestamp(),
        exp: now.timestamp() + 3600,
    };
    let token = sso_session::sign(&payload, secret)?;
    Ok(sso_session::set_cookie_header(
        SAML_REQUEST_COOKIE,
        &token,
        3600,
        &format!("/auth/saml/{tenant}"),
    ))
}

pub(super) fn expected_request_id(
    req: &str,
    tenant: &str,
    secret: &str,
    now: DateTime<Utc>,
) -> Result<String, &'static str> {
    let token =
        sso_session::read_cookie(req, SAML_REQUEST_COOKIE).ok_or("missing_request_cookie")?;
    let payload = sso_session::verify_with_secret(&token, now.timestamp(), secret)
        .map_err(|_| "bad_request_cookie")?;
    if payload.mechanism != "saml_request" || payload.tenant != tenant {
        return Err("request_cookie_scope");
    }
    Ok(payload.sub)
}

pub(super) fn clear_pending_cookie(tenant: &str) -> String {
    sso_session::clear_cookie_header(SAML_REQUEST_COOKIE, &format!("/auth/saml/{tenant}"))
}

pub(super) async fn audit_acs(
    store: Option<&Arc<SessionStore>>,
    tenant: &str,
    outcome: &super::AcsOutcome,
) {
    let Some(store) = store else { return };
    let event_type = if outcome.audit_outcome.starts_with("verified") {
        "sso.saml.response.verified"
    } else {
        "sso.saml.response.rejected"
    };
    let payload = serde_json::json!({
        "event_type": event_type,
        "outcome": outcome.audit_outcome,
        "tenant": tenant,
        "name_id": outcome.audit_name_id,
        "reason": outcome.audit_reason,
    });
    let sink = argo_audit::AuditSink::new_with_tenant(
        Arc::clone(store),
        argo_audit::ANONYMOUS_AUDIT_TENANT_ID.to_string(),
    );
    let _ = sink
        .write(None, argo_audit::AuditKind::HttpRequest, payload, None)
        .await;
}
