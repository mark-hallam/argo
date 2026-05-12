//! End-to-end smoke test against the real Xero Accounting API.
//!
//! Gated by two env vars; without them the test is a no-op so the suite
//! is safe to run on CI without credentials. To enable locally:
//!
//! ```bash
//! export ARGO_XERO_TEST_TOKEN=<oauth2 access token>
//! export ARGO_XERO_TEST_TENANT_ID=<xero tenant id (organization)>
//! cargo test -p argo-mcp-connector-xero --test xero_smoke -- --ignored
//! ```
//!
//! The test lists invoices and accounts to confirm the OAuth bearer
//! token plus `xero-tenant-id` header are accepted end-to-end. It is
//! read-only so it is safe to run against production tenants.

use argo_mcp_connector_xero::{
    XeroConnector, XERO_TEST_TENANT_ENV, XERO_TEST_TOKEN_ENV,
};

fn env_or_skip(name: &str) -> Option<String> {
    match std::env::var(name) {
        Ok(v) if !v.trim().is_empty() => Some(v),
        _ => None,
    }
}

#[tokio::test]
#[ignore = "requires ARGO_XERO_TEST_TOKEN + ARGO_XERO_TEST_TENANT_ID"]
async fn live_list_invoices_and_accounts() {
    let token = match env_or_skip(XERO_TEST_TOKEN_ENV) {
        Some(v) => v,
        None => return,
    };
    let tenant = match env_or_skip(XERO_TEST_TENANT_ENV) {
        Some(v) => v,
        None => return,
    };

    let xero = XeroConnector::new(token, tenant);

    let invoices = xero
        .list_invoices(None)
        .await
        .expect("list_invoices against real Xero");
    // Even on a fresh tenant, the call must succeed and return an array
    // (possibly empty). The assertion is about reachability + auth, not
    // tenant content.
    eprintln!("xero smoke: {} invoices visible", invoices.len());

    let accounts = xero
        .list_accounts()
        .await
        .expect("list_accounts against real Xero");
    assert!(
        !accounts.is_empty(),
        "every Xero org has at least the default chart of accounts"
    );
    eprintln!("xero smoke: {} accounts in chart", accounts.len());
}
