//! `POST /api/v1/billing/{checkout,portal,webhook}` (Stream
//! `claude-stripe-subscription-integration`).
//!
//! The webhook is mounted ahead of the bearer-token gate in
//! `routes.rs` because Stripe authenticates via the `Stripe-Signature`
//! HMAC header, not the operator bearer. Checkout + portal sit inside
//! the bearer-gated POST funnel and resolve the active tenant from the
//! workspace cwd, matching the convention used by the J11 worker
//! settings handlers + `usage_endpoint`.

use argo_billing::{
    checkout::{create_checkout_session, CheckoutInput},
    portal::{create_portal_session, PortalInput},
    tier_store::{apply_subscription_event, has_processed_event, lookup_stripe_customer},
    webhook::{
        classify_event, decode_event, record_apply_failure, verify_signature, FailureOutcome,
        WebhookOutcome, DEFAULT_MAX_RETRIES,
    },
    StripeClient,
};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use chrono::Utc;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

/// `POST /api/v1/billing/checkout` — create a hosted Checkout session.
pub(super) async fn checkout_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let store = match store {
        Some(s) => s,
        None => return super::store_unavailable(),
    };
    let body = super::body_from_request(req);
    let payload: CheckoutPayload = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return bad_request(&format!("checkout body decode: {e}")),
    };
    let client = match StripeClient::from_env() {
        Ok(c) => c,
        Err(e) => return server_error(&format!("stripe client: {e}")),
    };
    let tenant = match resolve_workspace_tenant(store).await {
        Ok(t) => t,
        Err(e) => return server_error(&format!("resolve tenant: {e}")),
    };
    let customer_id = lookup_stripe_customer(store, tenant)
        .await
        .unwrap_or_default();
    let input = CheckoutInput {
        tenant_id: tenant,
        price_id: payload.price_id,
        success_url: payload.success_url,
        cancel_url: payload.cancel_url,
        customer_id,
        customer_email: payload.customer_email,
    };
    match create_checkout_session(&client, &input).await {
        Ok(session) => (
            "200 OK",
            json!({ "id": session.id, "url": session.url }).to_string(),
        ),
        Err(e) => server_error(&format!("checkout create: {e}")),
    }
}

/// `POST /api/v1/billing/portal` — create a Customer Portal session.
pub(super) async fn portal_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let store = match store {
        Some(s) => s,
        None => return super::store_unavailable(),
    };
    let body = super::body_from_request(req);
    let payload: PortalPayload = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => return bad_request(&format!("portal body decode: {e}")),
    };
    let client = match StripeClient::from_env() {
        Ok(c) => c,
        Err(e) => return server_error(&format!("stripe client: {e}")),
    };
    let tenant = match resolve_workspace_tenant(store).await {
        Ok(t) => t,
        Err(e) => return server_error(&format!("resolve tenant: {e}")),
    };
    let Some(customer_id) = lookup_stripe_customer(store, tenant)
        .await
        .unwrap_or_default()
    else {
        return bad_request("no Stripe customer for tenant — run /billing/checkout first");
    };
    let input = PortalInput {
        customer_id,
        return_url: payload.return_url,
    };
    match create_portal_session(&client, &input).await {
        Ok(session) => (
            "200 OK",
            json!({ "id": session.id, "url": session.url }).to_string(),
        ),
        Err(e) => server_error(&format!("portal create: {e}")),
    }
}

/// `POST /api/v1/billing/webhook` — Stripe-signed inbound delivery.
pub(super) async fn webhook_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let store = match store {
        Some(s) => s,
        None => return super::store_unavailable(),
    };
    let header = match header_value(req, "Stripe-Signature") {
        Some(h) => h,
        None => return bad_request("missing Stripe-Signature header"),
    };
    let body = super::body_from_request(req);
    let client = match StripeClient::from_env() {
        Ok(c) => c,
        Err(e) => return server_error(&format!("stripe client: {e}")),
    };
    let now = Utc::now();
    if let Err(e) = verify_signature(header, body.as_bytes(), client.webhook_secret(), now) {
        tracing::warn!(target: "billing::webhook", error = %e, "Stripe signature rejected");
        return (
            "400 Bad Request",
            json!({ "error": { "code": "bad_signature", "message": e.to_string() } })
                .to_string(),
        );
    }
    let event = match decode_event(body.as_bytes()) {
        Ok(e) => e,
        Err(e) => return bad_request(&format!("event decode: {e}")),
    };
    if has_processed_event(store, &event.id).await.unwrap_or(false) {
        return ("200 OK", json!({ "outcome": "duplicate" }).to_string());
    }
    match classify_event(&event) {
        WebhookOutcome::Duplicate => ("200 OK", json!({ "outcome": "duplicate" }).to_string()),
        WebhookOutcome::Ignored => {
            if let Err(e) =
                argo_billing::tier_store::record_event(store, &event.id, now).await
            {
                return server_error(&format!("event ack: {e}"));
            }
            ("200 OK", json!({ "outcome": "ignored" }).to_string())
        }
        WebhookOutcome::Subscription(sub) => {
            // `apply_subscription_event` runs `upsert_stripe_customer`
            // BEFORE `set_workspace_tier`, so the `tenant_id` → `cus_…`
            // mapping is persisted even when the workspace row hasn't
            // been provisioned yet (rows_touched == 0).
            match apply_subscription_event(store, &sub, now).await {
                Ok(out) => {
                    let body = json!({
                        "outcome": "applied",
                        "tier": sub.tier.as_str(),
                        "rows_touched": out.rows_touched,
                        "past_due_warning": out.past_due_warning,
                    });
                    if out.past_due_warning {
                        tracing::warn!(
                            target: "billing::webhook",
                            tenant = %sub.tenant_id,
                            "subscription past_due — surfacing operator warning"
                        );
                    }
                    ("200 OK", body.to_string())
                }
                Err(e) => apply_failure_response(store, &event.id, &event.event_type,
                    Some(sub.tenant_id), body, &e.to_string(), now).await,
            }
        }
    }
}

async fn resolve_workspace_tenant(store: &SessionStore) -> ArgoResult<TenantId> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Io(format!("current_dir: {e}")))?;
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (tenant, _ws) = store.ensure_workspace(name, &root).await?;
    Ok(tenant)
}

fn header_value<'a>(req: &'a str, name: &str) -> Option<&'a str> {
    let headers = req
        .split_once("\r\n\r\n")
        .map_or(req, |(headers, _body)| headers);
    headers.lines().skip(1).find_map(|line| {
        let (k, v) = line.split_once(':')?;
        if k.trim().eq_ignore_ascii_case(name) {
            Some(v.trim())
        } else {
            None
        }
    })
}

fn bad_request(message: &str) -> (&'static str, String) {
    (
        "400 Bad Request",
        json!({ "error": { "code": "bad_request", "message": message } }).to_string(),
    )
}

fn server_error(message: &str) -> (&'static str, String) {
    (
        "500 Internal Server Error",
        json!({ "error": { "code": "internal_error", "message": message } }).to_string(),
    )
}

/// Resolve the operator-configured retry ceiling for webhook apply
/// failures. Falls back to [`DEFAULT_MAX_RETRIES`] when config can't
/// be loaded so the webhook continues to make progress.
fn webhook_max_retries() -> u32 {
    argo_config::load()
        .map(|c| c.billing.webhook.max_retries)
        .unwrap_or(DEFAULT_MAX_RETRIES)
}

/// Translate an apply failure into the right HTTP response: 500
/// while we still have retry budget (Stripe re-delivers), 200 once
/// the DLQ has absorbed the event (Stripe stops). The DLQ row is
/// written either way so operators see ongoing failures via `argo
/// billing dlq list`.
async fn apply_failure_response(
    store: &SessionStore,
    event_id: &str,
    event_type: &str,
    tenant_id: Option<TenantId>,
    raw_payload: &str,
    reason: &str,
    now: chrono::DateTime<Utc>,
) -> (&'static str, String) {
    let max = webhook_max_retries();
    match record_apply_failure(store, event_id, event_type, tenant_id, raw_payload, reason, max, now).await {
        Ok(FailureOutcome::Retry { retry_count, .. }) => {
            tracing::warn!(
                target: "billing::webhook",
                event_id, retry_count, reason,
                "apply failed — returning 500 so Stripe retries",
            );
            (
                "500 Internal Server Error",
                json!({
                    "outcome": "retry",
                    "retry_count": retry_count,
                    "error": { "code": "apply_retry", "message": reason },
                })
                .to_string(),
            )
        }
        Ok(FailureOutcome::DeadLettered { retry_count, .. }) => {
            tracing::error!(
                target: "billing::webhook",
                event_id, retry_count, reason,
                "apply failed — dead-lettered after retry budget exhausted",
            );
            (
                "200 OK",
                json!({
                    "outcome": "dead_lettered",
                    "retry_count": retry_count,
                    "error": { "code": "apply_dead_letter", "message": reason },
                })
                .to_string(),
            )
        }
        Err(e) => server_error(&format!("dlq upsert: {e}")),
    }
}

#[derive(Debug, Deserialize)]
struct CheckoutPayload {
    price_id: String,
    success_url: String,
    cancel_url: String,
    #[serde(default)]
    customer_email: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PortalPayload {
    return_url: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_value_is_case_insensitive() {
        let req = "POST /api/v1/billing/webhook HTTP/1.1\r\n\
                   Host: x\r\n\
                   stripe-signature: t=1,v1=abc\r\n\r\n{}";
        assert_eq!(header_value(req, "Stripe-Signature"), Some("t=1,v1=abc"));
    }

    #[test]
    fn missing_header_is_none() {
        let req = "POST /api/v1/billing/webhook HTTP/1.1\r\nHost: x\r\n\r\n{}";
        assert_eq!(header_value(req, "Stripe-Signature"), None);
    }
}
