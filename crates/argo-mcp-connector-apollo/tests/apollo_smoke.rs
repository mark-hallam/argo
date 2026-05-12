//! Live Apollo.io smoke test.
//!
//! Gated by two env vars; if either is missing the test no-ops so the suite
//! is safe on CI without credentials. To enable locally:
//!
//! ```bash
//! export ARGO_TEST_APOLLO_API_KEY=<Apollo API key>
//! export ARGO_TEST_APOLLO_EMAIL_FOR_ENRICH=<stable public-figure email>
//! cargo test -p argo-mcp-connector-apollo --test apollo_smoke -- --ignored
//! ```
//!
//! Use a stable public-figure email (founder of a well-known company)
//! for the smoke recipient. Apollo ages out private-individual records
//! but keeps well-known public records intact. Do NOT bake personal-
//! identifying data into the test fixture; the operator selects the
//! email at run time.

use argo_mcp_connector_apollo::{ApolloClient, CompanyLookup, PersonLookup};

const KEY_ENV: &str = "ARGO_TEST_APOLLO_API_KEY";
const EMAIL_ENV: &str = "ARGO_TEST_APOLLO_EMAIL_FOR_ENRICH";

fn read_required(name: &str) -> Option<String> {
    match std::env::var(name) {
        Ok(v) if !v.trim().is_empty() => Some(v),
        _ => None,
    }
}

#[tokio::test]
#[ignore = "requires ARGO_TEST_APOLLO_API_KEY/ARGO_TEST_APOLLO_EMAIL_FOR_ENRICH"]
async fn live_enrich_person_and_company_returns_typed_structs() {
    let api_key = match read_required(KEY_ENV) {
        Some(v) => v,
        None => return,
    };
    let email = match read_required(EMAIL_ENV) {
        Some(v) => v,
        None => return,
    };

    let apollo = ApolloClient::new(api_key).expect("build Apollo client");

    let person = apollo
        .enrich_person(&PersonLookup::by_email(&email))
        .await
        .expect("apollo person enrichment");

    assert!(!person.id.is_empty(), "person id must be populated");
    assert!(!person.name.is_empty(), "person name must be populated");
    // email_status is the operationally-critical signal for downstream
    // outreach; require Apollo to have returned some grade.
    assert!(
        person.email_status.is_some(),
        "person email_status must be populated (Apollo grades verified/unverified/risky/…)"
    );

    // Derive a company domain to enrich. Prefer Apollo's organization id
    // → company by name fallback; for a smoke we use a known stable
    // domain pulled from the public-figure email host (skip if the
    // person's company is not surfaced).
    let domain = email
        .split('@')
        .nth(1)
        .filter(|s| !s.is_empty())
        .expect("email must contain a domain to derive company lookup");

    let company = apollo
        .enrich_company(&CompanyLookup::by_domain(domain))
        .await
        .expect("apollo company enrichment");
    assert!(!company.id.is_empty(), "company id must be populated");
    assert!(!company.name.is_empty(), "company name must be populated");
}
