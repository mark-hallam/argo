//! Wiremock-backed unit tests for `ClearbitConnector`.
//!
//! Six scenarios per the brief:
//!  1. `enrich_person_by_email` → 200 returns `Some(_)`.
//!  2. `enrich_person_by_email` → 202 polls then 404 returns `Ok(None)`.
//!  3. `enrich_company_by_domain` → 200 with the domain query param.
//!  4. `combined_lookup` → 200 returns populated person + company.
//!  5. `discovery_search` → 200 returns the results array.
//!  6. `enrich_person_by_email` → 202 polls then 200 returns `Some(_)`.

use super::*;
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const TEST_KEY: &str = "test-key";
// base64("test-key:") == "dGVzdC1rZXk6"
const EXPECTED_AUTH: &str = "Basic dGVzdC1rZXk6";

fn connector(server: &MockServer) -> ClearbitConnector {
    ClearbitConnector::new(TEST_KEY.into())
        .unwrap()
        .with_base_url(server.uri())
        // Tests must not actually sleep — schedule zeros so polling is instant.
        .with_poll_backoff_ms(vec![0, 0, 0, 0, 0])
}

fn person_body() -> serde_json::Value {
    serde_json::json!({
        "id": "p_001",
        "name": {"fullName": "Ada Lovelace"},
        "email": "ada@example.test",
        "linkedin": {"handle": "adalovelace"},
        "twitter": {"handle": "ada"},
        "employment": {
            "name": "Analytical Engines Inc",
            "domain": "analytical.example",
            "title": "Chief Mathematician",
            "seniority": "executive",
            "role": "engineering"
        },
        "geo": {"city": "London", "state": null, "country": "UK"}
    })
}

fn company_body() -> serde_json::Value {
    serde_json::json!({
        "id": "c_001",
        "name": "Analytical Engines Inc",
        "domain": "analytical.example",
        "category": {"industry": "Software", "industryGroup": "Developer Tools"},
        "metrics": {"employees": 42, "annualRevenue": 9_500_000_u64},
        "fundingTotal": 12_000_000_u64,
        "tech": ["nginx", "stripe", "datadog"],
        "linkedin": {"handle": "analytical-engines"},
        "twitter": {"handle": "analytical"}
    })
}

#[tokio::test]
async fn person_found_returns_populated_dto() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v2/people/find"))
        .and(query_param("email", "ada@example.test"))
        .and(header("authorization", EXPECTED_AUTH))
        .respond_with(ResponseTemplate::new(200).set_body_json(person_body()))
        .expect(1)
        .mount(&server)
        .await;

    let cb = connector(&server);
    let got = cb
        .enrich_person_by_email("ada@example.test")
        .await
        .unwrap()
        .expect("person should be populated");
    assert_eq!(got.id, "p_001");
    assert_eq!(got.full_name, "Ada Lovelace");
    assert_eq!(got.company_domain.as_deref(), Some("analytical.example"));
    assert_eq!(got.title.as_deref(), Some("Chief Mathematician"));
    assert_eq!(got.linkedin_handle.as_deref(), Some("adalovelace"));
    assert_eq!(got.location.as_deref(), Some("London, UK"));
}

#[tokio::test]
async fn person_not_found_polls_202_then_eventual_404_is_none() {
    let server = MockServer::start().await;
    // First call returns 202 (queued).
    Mock::given(method("GET"))
        .and(path("/v2/people/find"))
        .respond_with(ResponseTemplate::new(202).set_body_string(""))
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;
    // Second call returns 404 (Clearbit could not find).
    Mock::given(method("GET"))
        .and(path("/v2/people/find"))
        .respond_with(ResponseTemplate::new(404).set_body_string(""))
        .expect(1)
        .mount(&server)
        .await;

    let cb = connector(&server);
    let got = cb
        .enrich_person_by_email("ghost@example.test")
        .await
        .unwrap();
    assert!(got.is_none(), "404 after 202 must collapse to Ok(None)");
}

#[tokio::test]
async fn company_by_domain_round_trips() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v2/companies/find"))
        .and(query_param("domain", "analytical.example"))
        .and(header("authorization", EXPECTED_AUTH))
        .respond_with(ResponseTemplate::new(200).set_body_json(company_body()))
        .expect(1)
        .mount(&server)
        .await;

    let cb = connector(&server);
    let got = cb
        .enrich_company_by_domain("analytical.example")
        .await
        .unwrap()
        .expect("company should be populated");
    assert_eq!(got.id, "c_001");
    assert_eq!(got.name, "Analytical Engines Inc");
    assert_eq!(got.industry.as_deref(), Some("Software"));
    assert_eq!(got.category_industry.as_deref(), Some("Developer Tools"));
    assert_eq!(got.employees, Some(42));
    assert_eq!(got.annual_revenue_usd, Some(9_500_000));
    assert_eq!(got.funding_total_usd, Some(12_000_000));
    assert_eq!(got.tech_stack, vec!["nginx", "stripe", "datadog"]);
}

#[tokio::test]
async fn combined_lookup_returns_person_plus_company() {
    let server = MockServer::start().await;
    let combined = serde_json::json!({
        "person": person_body(),
        "company": company_body(),
    });
    Mock::given(method("GET"))
        .and(path("/v2/combined/find"))
        .and(query_param("email", "ada@example.test"))
        .respond_with(ResponseTemplate::new(200).set_body_json(combined))
        .expect(1)
        .mount(&server)
        .await;

    let cb = connector(&server);
    let got = cb.combined_lookup("ada@example.test").await.unwrap();
    let person = got.person.expect("person side populated");
    let company = got.company.expect("company side populated");
    assert_eq!(person.full_name, "Ada Lovelace");
    assert_eq!(company.domain, "analytical.example");
}

#[tokio::test]
async fn discovery_search_returns_results_array() {
    let server = MockServer::start().await;
    let body = serde_json::json!({
        "results": [company_body(), {
            "id": "c_002",
            "name": "Second Co",
            "domain": "second.example",
            "category": null,
            "metrics": null,
            "tech": []
        }],
        "total": 2,
        "page": 1
    });
    Mock::given(method("GET"))
        .and(path("/v1/companies/search"))
        .and(query_param("query", "analytical"))
        .and(query_param("limit", "5"))
        .and(header("authorization", EXPECTED_AUTH))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .expect(1)
        .mount(&server)
        .await;

    let cb = connector(&server);
    let got = cb.discovery_search("analytical", 5).await.unwrap();
    assert_eq!(got.len(), 2);
    assert_eq!(got[0].id, "c_001");
    assert_eq!(got[1].id, "c_002");
    assert_eq!(got[1].domain, "second.example");
    assert!(got[1].tech_stack.is_empty());
}

#[tokio::test]
async fn async_polling_202_then_200_returns_some() {
    let server = MockServer::start().await;
    // First call: 202 (queued).
    Mock::given(method("GET"))
        .and(path("/v2/people/find"))
        .respond_with(ResponseTemplate::new(202).set_body_string(""))
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;
    // Second call: still 202 — exercise multi-poll.
    Mock::given(method("GET"))
        .and(path("/v2/people/find"))
        .respond_with(ResponseTemplate::new(202).set_body_string(""))
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;
    // Third call: 200 with the real body.
    Mock::given(method("GET"))
        .and(path("/v2/people/find"))
        .respond_with(ResponseTemplate::new(200).set_body_json(person_body()))
        .expect(1)
        .mount(&server)
        .await;

    let cb = connector(&server);
    let got = cb
        .enrich_person_by_email("ada@example.test")
        .await
        .unwrap()
        .expect("polling should resolve to populated person");
    assert_eq!(got.id, "p_001");
}
