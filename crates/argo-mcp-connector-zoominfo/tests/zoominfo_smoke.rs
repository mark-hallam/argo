//! Live ZoomInfo smoke test.
//!
//! Gated by four env vars; if any is missing the test no-ops so the suite
//! is safe on CI without credentials. To enable locally:
//!
//! ```bash
//! export ARGO_TEST_ZOOMINFO_USERNAME=<account username>
//! export ARGO_TEST_ZOOMINFO_PASSWORD=<account password>
//! export ARGO_TEST_ZOOMINFO_CLIENT_ID=<oauth client id>
//! export ARGO_TEST_ZOOMINFO_EMAIL_FOR_ENRICH=<stable public-figure email>
//! cargo test -p argo-mcp-connector-zoominfo --test zoominfo_smoke -- --ignored
//! ```
//!
//! Pick a stable public-figure email (founder of a well-known company) so
//! the test does not flake on absent records — ZoomInfo ages out
//! private-individual records but keeps well-known public records intact.

use argo_mcp_connector_zoominfo::{ZoomInfoClient, ZoomInfoCredentials};

const USERNAME_ENV: &str = "ARGO_TEST_ZOOMINFO_USERNAME";
const PASSWORD_ENV: &str = "ARGO_TEST_ZOOMINFO_PASSWORD";
const CLIENT_ID_ENV: &str = "ARGO_TEST_ZOOMINFO_CLIENT_ID";
const EMAIL_ENV: &str = "ARGO_TEST_ZOOMINFO_EMAIL_FOR_ENRICH";

fn read_required(name: &str) -> Option<String> {
    match std::env::var(name) {
        Ok(v) if !v.trim().is_empty() => Some(v),
        _ => None,
    }
}

#[tokio::test]
#[ignore = "requires ARGO_TEST_ZOOMINFO_USERNAME/PASSWORD/CLIENT_ID/EMAIL_FOR_ENRICH"]
async fn live_enrich_person_and_company() {
    let username = match read_required(USERNAME_ENV) {
        Some(v) => v,
        None => return,
    };
    let password = match read_required(PASSWORD_ENV) {
        Some(v) => v,
        None => return,
    };
    let client_id = match read_required(CLIENT_ID_ENV) {
        Some(v) => v,
        None => return,
    };
    let email = match read_required(EMAIL_ENV) {
        Some(v) => v,
        None => return,
    };

    let zi = ZoomInfoClient::new(ZoomInfoCredentials::new(username, password, client_id))
        .expect("build ZoomInfo client");

    // 1) Person enrichment by email — exercises /lookup/contact + OAuth.
    let person = zi
        .enrich_person_by_email(&email)
        .await
        .expect("person enrichment live call");
    assert!(!person.id.is_empty(), "person id must be populated");
    assert!(
        !person.first_name.is_empty() || !person.last_name.is_empty(),
        "at least one of first_name / last_name must be populated"
    );

    // 2) Company enrichment by domain (derived from the email) — exercises
    //    /lookup/company and validates token reuse across two requests.
    let domain = email
        .rsplit('@')
        .next()
        .expect("email has @ separator")
        .to_string();
    let company = zi
        .enrich_company_by_domain(&domain)
        .await
        .expect("company enrichment live call");
    assert!(!company.id.is_empty(), "company id must be populated");
    assert!(!company.domain.is_empty(), "company domain must be populated");
}
