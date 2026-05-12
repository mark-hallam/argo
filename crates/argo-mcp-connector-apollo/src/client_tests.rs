//! Wiremock-backed unit tests for `ApolloClient`. Eight scenarios:
//! email-happy, linkedin-happy, null-not-found, company-domain-happy,
//! paginated-2-pages, 429-back-off, 401-auth-failed, 422-credits-exhausted.

use super::*;
use crate::types::ApolloPersonEmailStatus;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const TEST_KEY: &str = "test-key";

fn connector(server: &MockServer) -> ApolloClient {
    ApolloClient::new(TEST_KEY.into())
        .expect("build apollo client")
        .with_base_url(server.uri())
        .with_retry_jitter_ms(0)
}

fn person_json() -> serde_json::Value {
    serde_json::json!({
        "id": "p_001",
        "first_name": "Ada", "last_name": "Lovelace", "name": "Ada Lovelace",
        "title": "Chief Mathematician",
        "email": "ada@example.test", "email_status": "verified",
        "linkedin_url": "https://linkedin.com/in/adalovelace",
        "phone_numbers": [{"sanitized_number": "+44 20 0000 0000"}],
        "organization_id": "c_001", "seniority": "c_suite",
        "departments": ["engineering"]
    })
}
fn person_body() -> serde_json::Value {
    serde_json::json!({ "person": person_json() })
}
fn company_body() -> serde_json::Value {
    serde_json::json!({
        "organization": {
            "id": "c_001", "name": "Analytical Engines Inc",
            "primary_domain": "analytical.example",
            "industry": "Software", "estimated_num_employees": 42,
            "organization_revenue_printed": "$1M-$10M",
            "city": "London", "founded_year": 1843
        }
    })
}

async fn mount_post_once(server: &MockServer, p: &str, status: u16, body: serde_json::Value) {
    Mock::given(method("POST"))
        .and(path(p))
        .respond_with(ResponseTemplate::new(status).set_body_json(body))
        .expect(1)
        .mount(server)
        .await;
}

#[tokio::test]
async fn enrich_person_by_email_returns_populated_dto() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/people/match"))
        .and(header("x-api-key", TEST_KEY))
        .respond_with(ResponseTemplate::new(200).set_body_json(person_body()))
        .expect(1)
        .mount(&server)
        .await;
    let got = connector(&server)
        .enrich_person(&PersonLookup::by_email("ada@example.test"))
        .await
        .expect("happy person enrichment");
    assert_eq!(got.id, "p_001");
    assert_eq!(got.name, "Ada Lovelace");
    assert_eq!(got.email_status, Some(ApolloPersonEmailStatus::Verified));
    assert_eq!(got.seniority.as_deref(), Some("c_suite"));
    assert_eq!(got.phone.as_deref(), Some("+44 20 0000 0000"));
}

#[tokio::test]
async fn enrich_person_by_linkedin_url_round_trips() {
    let server = MockServer::start().await;
    mount_post_once(&server, "/people/match", 200, person_body()).await;
    let got = connector(&server)
        .enrich_person(&PersonLookup::by_linkedin_url(
            "https://linkedin.com/in/adalovelace",
        ))
        .await
        .expect("happy linkedin enrichment");
    assert_eq!(
        got.linkedin_url.as_deref(),
        Some("https://linkedin.com/in/adalovelace")
    );
}

#[tokio::test]
async fn enrich_person_null_body_collapses_to_typed_not_found() {
    let server = MockServer::start().await;
    mount_post_once(&server, "/people/match", 200, serde_json::json!({"person": null})).await;
    let err = connector(&server)
        .enrich_person(&PersonLookup::by_email("ghost@example.test"))
        .await
        .expect_err("null person should error");
    assert!(matches!(err, ApolloError::PersonNotFound), "got {err:?}");
}

#[tokio::test]
async fn enrich_company_by_domain_returns_populated_dto() {
    let server = MockServer::start().await;
    mount_post_once(&server, "/organizations/enrich", 200, company_body()).await;
    let got = connector(&server)
        .enrich_company(&CompanyLookup::by_domain("analytical.example"))
        .await
        .expect("happy company enrichment");
    assert_eq!(got.name, "Analytical Engines Inc");
    assert_eq!(got.domain.as_deref(), Some("analytical.example"));
    assert_eq!(got.employee_count, Some(42));
    assert_eq!(got.revenue_band.as_deref(), Some("$1M-$10M"));
    assert_eq!(got.founded_year, Some(1843));
}

#[tokio::test]
async fn search_people_pages_through_two_result_pages() {
    let server = MockServer::start().await;
    let p1 = serde_json::json!({
        "people": [person_json()],
        "pagination": {"page": 1, "total_pages": 2, "total_entries": 2},
    });
    let p2 = serde_json::json!({
        "people": [{
            "id": "p_002", "first_name": "Charles", "last_name": "Babbage",
            "name": "Charles Babbage", "title": "Inventor",
            "email": "charles@example.test", "email_status": "likely_to_engage",
            "linkedin_url": null, "phone_numbers": [],
            "organization_id": "c_001", "seniority": "founder", "departments": []
        }],
        "pagination": {"page": 2, "total_pages": 2, "total_entries": 2},
    });
    Mock::given(method("POST"))
        .and(path("/mixed_people/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(p1))
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/mixed_people/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(p2))
        .expect(1)
        .mount(&server)
        .await;
    let apollo = connector(&server);
    let r1 = apollo
        .search_people(&PeopleSearchFilters {
            page: Some(1),
            per_page: Some(1),
            keywords: Some("analytical".into()),
            ..Default::default()
        })
        .await
        .expect("page 1");
    assert_eq!(r1.entries.len(), 1);
    assert_eq!(r1.total_pages, 2);
    let r2 = apollo
        .search_people(&PeopleSearchFilters {
            page: Some(2),
            per_page: Some(1),
            keywords: Some("analytical".into()),
            ..Default::default()
        })
        .await
        .expect("page 2");
    assert_eq!(r2.entries[0].name, "Charles Babbage");
    assert_eq!(r2.page, 2);
}

#[tokio::test]
async fn rate_limited_429_with_retry_after_backs_off_then_succeeds() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/people/match"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("Retry-After", "0")
                .set_body_string("{\"error\":\"slow down\"}"),
        )
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;
    mount_post_once(&server, "/people/match", 200, person_body()).await;
    let got = connector(&server)
        .enrich_person(&PersonLookup::by_email("ada@example.test"))
        .await
        .expect("retry should succeed");
    assert_eq!(got.id, "p_001");
}

#[tokio::test]
async fn unauthorized_401_bubbles_as_auth_failed() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/people/match"))
        .respond_with(ResponseTemplate::new(401).set_body_string("{\"error\":\"invalid api key\"}"))
        .expect(1)
        .mount(&server)
        .await;
    let err = connector(&server)
        .enrich_person(&PersonLookup::by_email("ada@example.test"))
        .await
        .expect_err("401 must surface AuthFailed");
    assert!(matches!(err, ApolloError::AuthFailed), "got {err:?}");
}

#[tokio::test]
async fn credit_exhaustion_422_with_marker_surfaces_typed_credits_exhausted() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/people/match"))
        .respond_with(
            ResponseTemplate::new(422)
                .set_body_string("{\"error\":\"OutOfFreeBlocks: monthly quota reached\"}"),
        )
        .expect(1)
        .mount(&server)
        .await;
    let err = connector(&server)
        .enrich_person(&PersonLookup::by_email("ada@example.test"))
        .await
        .expect_err("credits exhausted must error");
    assert!(matches!(err, ApolloError::CreditsExhausted), "got {err:?}");
}
