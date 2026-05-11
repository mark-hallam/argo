//! End-to-end smoke test against a real QuickBooks Online sandbox.
//!
//! Gated by three env vars; without them the test is a no-op so the suite is
//! safe to run on CI without credentials. To enable locally:
//!
//! ```bash
//! export ARGO_QB_TEST_TOKEN=<oauth2 access token for a sandbox company>
//! export ARGO_QB_TEST_REALM=<sandbox realmId>
//! export ARGO_QB_TEST_CUSTOMER_ID=<customer id in that sandbox>
//! cargo test -p argo-mcp-connector-quickbooks --test quickbooks_smoke -- --ignored
//! ```
//!
//! The test creates a small invoice for the configured customer, reads it
//! back via `list_invoices`, and asserts the round-trip totals match. It uses
//! the production sandbox host (`sandbox-quickbooks.api.intuit.com`).

use argo_mcp_connector_quickbooks::{
    QbInvoiceDraft, QbLine, QuickBooksAuth, QuickBooksConnector, QuickBooksHost,
};
use chrono::{Duration, Utc};

const TOKEN_ENV: &str = "ARGO_QB_TEST_TOKEN";
const REALM_ENV: &str = "ARGO_QB_TEST_REALM";
const CUSTOMER_ENV: &str = "ARGO_QB_TEST_CUSTOMER_ID";

#[tokio::test]
#[ignore = "requires ARGO_QB_TEST_TOKEN/REALM/CUSTOMER_ID"]
async fn sandbox_round_trip_invoice() {
    let token = match std::env::var(TOKEN_ENV) {
        Ok(v) if !v.trim().is_empty() => v,
        _ => return,
    };
    let realm = match std::env::var(REALM_ENV) {
        Ok(v) if !v.trim().is_empty() => v,
        _ => return,
    };
    let customer_id = match std::env::var(CUSTOMER_ENV) {
        Ok(v) if !v.trim().is_empty() => v,
        _ => return,
    };

    // Use a "pre-seeded" auth that already holds an access token; we don't
    // exercise the refresh path in smoke since refresh tokens are sensitive
    // to test/CI rotation. The connector's auth holder is happy with a
    // seeded token via `set_access_token`.
    let auth = QuickBooksAuth::new("smoke".into(), "smoke".into(), "smoke".into());
    auth.set_access_token(token);
    let qb = QuickBooksConnector::new(auth, realm, QuickBooksHost::Sandbox)
        .expect("build QuickBooks connector");

    let due = (Utc::now() + Duration::days(14)).date_naive();
    let draft = QbInvoiceDraft {
        customer_id: customer_id.clone(),
        line_items: vec![QbLine {
            description: "argo-smoke".into(),
            amount_cents: 100,
        }],
        due_date: due,
    };
    let created = qb
        .create_invoice(draft)
        .await
        .expect("create_invoice succeeds");
    assert_eq!(created.total_cents, 100, "invoice total round-trips");
    assert!(
        !created.id.is_empty(),
        "QBO must return a non-empty invoice id"
    );

    let recent_cutoff = Utc::now() - Duration::minutes(5);
    let invoices = qb
        .list_invoices(Some(recent_cutoff))
        .await
        .expect("list_invoices succeeds");
    assert!(
        invoices.iter().any(|inv| inv.id == created.id),
        "newly created invoice {} not seen in list_invoices",
        created.id,
    );
}
