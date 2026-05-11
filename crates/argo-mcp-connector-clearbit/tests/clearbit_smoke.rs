//! Live Clearbit smoke test.
//!
//! Gated by two env vars; if either is missing the test no-ops so the suite
//! is safe on CI without credentials. To enable locally:
//!
//! ```bash
//! export ARGO_CLEARBIT_TEST_KEY=<Clearbit API key>
//! export ARGO_CLEARBIT_TEST_EMAIL=<stable public-figure email>
//! cargo test -p argo-mcp-connector-clearbit --test clearbit_smoke -- --ignored
//! ```
//!
//! The test looks the email up and asserts the person + company sides
//! populate. Pick a stable public-figure email (founder of a well-known
//! company) so the test does not flake on absent data — Clearbit ages out
//! private-individual records but keeps well-known public records intact.

use argo_mcp_connector_clearbit::ClearbitConnector;

const KEY_ENV: &str = "ARGO_CLEARBIT_TEST_KEY";
const EMAIL_ENV: &str = "ARGO_CLEARBIT_TEST_EMAIL";

fn read_required(name: &str) -> Option<String> {
    match std::env::var(name) {
        Ok(v) if !v.trim().is_empty() => Some(v),
        _ => None,
    }
}

#[tokio::test]
#[ignore = "requires ARGO_CLEARBIT_TEST_KEY/EMAIL"]
async fn live_combined_lookup_populates_person_and_company() {
    let api_key = match read_required(KEY_ENV) {
        Some(v) => v,
        None => return,
    };
    let email = match read_required(EMAIL_ENV) {
        Some(v) => v,
        None => return,
    };

    let cb = ClearbitConnector::new(api_key).expect("build Clearbit connector");

    // 1) Combined lookup — saves a round trip vs separate person/company calls.
    let combined = cb
        .combined_lookup(&email)
        .await
        .expect("combined_lookup live call");
    let person = combined
        .person
        .expect("Clearbit returned no person for the configured email");
    let company = combined
        .company
        .expect("Clearbit returned no company for the configured email");

    assert!(!person.id.is_empty(), "person id must be populated");
    assert!(
        !person.full_name.is_empty(),
        "person full_name must be populated"
    );
    assert!(!company.id.is_empty(), "company id must be populated");
    assert!(
        !company.domain.is_empty(),
        "company domain must be populated"
    );

    // 2) Company-by-domain — uses the domain we just learned, exercises the
    //    `company.clearbit.com` subdomain independently of the combined call.
    let by_domain = cb
        .enrich_company_by_domain(&company.domain)
        .await
        .expect("company-by-domain live call")
        .expect("company should round-trip back from its own domain");
    assert_eq!(by_domain.domain, company.domain);
}
