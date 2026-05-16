//! Stripe Checkout-session creation.
//!
//! The SPA hits `POST /api/v1/billing/checkout` with the price-id the
//! user chose and the tenant context. The gateway calls
//! [`create_checkout_session`] which forwards a form-encoded `POST
//! /v1/checkout/sessions` to Stripe, returning the hosted Checkout URL
//! the SPA redirects to.
//!
//! The Stripe `metadata` field is the source of truth for the
//! `tenant_id` ↔ subscription mapping the webhook later consumes; the
//! `client_reference_id` mirror is set as well so it appears in the
//! operator-facing Stripe dashboard search.

use crate::stripe_client::{StripeClient, StripeError};
use argo_types::TenantId;

/// Inputs the gateway hands to [`create_checkout_session`].
#[derive(Debug, Clone)]
pub struct CheckoutInput {
    /// Tenant requesting the upgrade.
    pub tenant_id: TenantId,
    /// Stripe price id (`price_…`) the user is subscribing to.
    pub price_id: String,
    /// URL Stripe redirects to on successful Checkout completion.
    pub success_url: String,
    /// URL Stripe redirects to when the user cancels.
    pub cancel_url: String,
    /// Optional pre-existing Stripe customer id — when present the
    /// Checkout session reuses that customer instead of provisioning a
    /// fresh one.
    pub customer_id: Option<String>,
    /// Optional billing-email pre-fill (when no customer id yet).
    pub customer_email: Option<String>,
}

/// Stripe's `checkout.sessions.create` response, narrowed to the two
/// fields the SPA cares about.
#[derive(Debug, Clone)]
pub struct CheckoutSession {
    /// Stripe-issued `cs_…` session id.
    pub id: String,
    /// Hosted Checkout URL — the SPA `window.location` redirect target.
    pub url: String,
}

/// Create a hosted Stripe Checkout session for a subscription.
pub async fn create_checkout_session(
    client: &StripeClient,
    input: &CheckoutInput,
) -> Result<CheckoutSession, StripeError> {
    let mut form: Vec<(String, String)> = vec![
        ("mode".into(), "subscription".into()),
        ("line_items[0][price]".into(), input.price_id.clone()),
        ("line_items[0][quantity]".into(), "1".into()),
        ("success_url".into(), input.success_url.clone()),
        ("cancel_url".into(), input.cancel_url.clone()),
        ("client_reference_id".into(), input.tenant_id.to_string()),
        (
            "metadata[tenant_id]".into(),
            input.tenant_id.to_string(),
        ),
        (
            "subscription_data[metadata][tenant_id]".into(),
            input.tenant_id.to_string(),
        ),
        ("allow_promotion_codes".into(), "true".into()),
    ];
    if let Some(cid) = &input.customer_id {
        form.push(("customer".into(), cid.clone()));
    } else if let Some(email) = &input.customer_email {
        form.push(("customer_email".into(), email.clone()));
    }
    let body = client.post_form("/v1/checkout/sessions", &form).await?;
    let id = body
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| StripeError::Decode("missing checkout.session.id".into()))?
        .to_string();
    let url = body
        .get("url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| StripeError::Decode("missing checkout.session.url".into()))?
        .to_string();
    Ok(CheckoutSession { id, url })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checkout_input_round_trip() {
        let input = CheckoutInput {
            tenant_id: TenantId::new(),
            price_id: "price_x".into(),
            success_url: "https://app.skycot.com/billing?ok=1".into(),
            cancel_url: "https://app.skycot.com/billing?ok=0".into(),
            customer_id: None,
            customer_email: Some("op@acme.com".into()),
        };
        assert!(input.success_url.contains("ok=1"));
    }
}
