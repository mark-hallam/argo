//! Stripe Customer Portal session creation.
//!
//! Returns a short-lived URL the SPA redirects to so the customer can
//! manage payment methods, view invoices, switch plans, or cancel
//! their subscription from Stripe's hosted UI.
//!
//! Portal configuration uses Stripe's default — pricing tiers + cancel
//! + invoices visible — so v1 doesn't need a custom portal config id.

use crate::stripe_client::{StripeClient, StripeError};

/// Inputs for [`create_portal_session`].
#[derive(Debug, Clone)]
pub struct PortalInput {
    /// Stripe customer id (`cus_…`) to scope the session to.
    pub customer_id: String,
    /// URL Stripe redirects to after the customer closes the portal.
    pub return_url: String,
}

/// Stripe portal-session response narrowed to the URL the SPA needs.
#[derive(Debug, Clone)]
pub struct PortalSession {
    /// Stripe-issued portal session id (`bps_…`).
    pub id: String,
    /// Hosted Customer Portal URL — single-use, short-lived redirect target.
    pub url: String,
}

/// Create a Stripe Customer Portal session for an existing customer.
pub async fn create_portal_session(
    client: &StripeClient,
    input: &PortalInput,
) -> Result<PortalSession, StripeError> {
    let form: Vec<(String, String)> = vec![
        ("customer".into(), input.customer_id.clone()),
        ("return_url".into(), input.return_url.clone()),
    ];
    let body = client
        .post_form("/v1/billing_portal/sessions", &form)
        .await?;
    let id = body
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| StripeError::Decode("missing portal.session.id".into()))?
        .to_string();
    let url = body
        .get("url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| StripeError::Decode("missing portal.session.url".into()))?
        .to_string();
    Ok(PortalSession { id, url })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portal_input_round_trip() {
        let input = PortalInput {
            customer_id: "cus_x".into(),
            return_url: "https://app.skycot.com/billing".into(),
        };
        assert_eq!(input.customer_id, "cus_x");
    }
}
