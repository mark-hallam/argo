//! Wiremock-backed unit tests for `ZoomInfoClient`. Eight scenarios per
//! the brief: enrich-person happy + 404, enrich-company happy, paginated
//! person search, OAuth token cached + reused, 429 back-off, 401 →
//! AuthFailed (vs 403 credits → CreditsExhausted), 401 refresh-then-retry.

use super::*;
use wiremock::matchers::{header_exists, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn make_client(server: &MockServer) -> ZoomInfoClient {
    ZoomInfoClient::new(ZoomInfoCredentials::new("u", "p", "cid"))
        .unwrap()
        .with_base_url(server.uri())
        .with_jitter_ms(0)
}

async fn mount_auth(server: &MockServer, jwt: &str, expect: Option<u64>) {
    let mut m = Mock::given(method("POST")).and(path("/authenticate")).respond_with(
        ResponseTemplate::new(200).set_body_json(serde_json::json!({ "jwt": jwt })),
    );
    if let Some(n) = expect {
        m = m.expect(n);
    }
    m.mount(server).await;
}

fn person_body() -> serde_json::Value {
    serde_json::json!({
        "id": "p_001", "firstName": "Ada", "lastName": "Lovelace",
        "email": "ada@example.test", "jobTitle": "Chief Mathematician",
        "managementLevel": "C_Suite", "department": "Engineering",
        "companyId": 42, "linkedInUrl": "https://linkedin.com/in/adalovelace"
    })
}

fn company_body() -> serde_json::Value {
    serde_json::json!({
        "id": "c_001", "name": "Analytical Engines Inc", "website": "analytical.example",
        "industry": "Software", "employeeCount": 42, "revenue": "$1M-$10M",
        "hqCity": "London", "hqCountry": "UK", "foundedYear": 1842, "duns": "123456789"
    })
}

#[tokio::test]
async fn enrich_person_by_email_returns_populated_person() {
    let server = MockServer::start().await;
    mount_auth(&server, "tok", None).await;
    Mock::given(method("POST"))
        .and(path("/lookup/contact"))
        .and(header_exists("authorization"))
        .respond_with(ResponseTemplate::new(200).set_body_json(person_body()))
        .expect(1)
        .mount(&server)
        .await;

    let p = make_client(&server)
        .enrich_person_by_email("ada@example.test")
        .await
        .expect("happy path");
    assert_eq!(p.id, "p_001");
    assert_eq!(p.first_name, "Ada");
    assert_eq!(p.seniority_level.as_deref(), Some("C_Suite"));
    assert_eq!(p.company_id.as_deref(), Some("42"));
}

#[tokio::test]
async fn enrich_person_404_bubbles_as_person_not_found() {
    let server = MockServer::start().await;
    mount_auth(&server, "tok", None).await;
    Mock::given(method("POST"))
        .and(path("/lookup/contact"))
        .respond_with(ResponseTemplate::new(404).set_body_string(""))
        .mount(&server)
        .await;

    let err = make_client(&server)
        .enrich_person_by_email("ghost@example.test")
        .await
        .unwrap_err();
    assert!(matches!(err, ZoomInfoError::PersonNotFound));
}

#[tokio::test]
async fn enrich_company_by_domain_returns_populated_company() {
    let server = MockServer::start().await;
    mount_auth(&server, "tok", None).await;
    Mock::given(method("POST"))
        .and(path("/lookup/company"))
        .respond_with(ResponseTemplate::new(200).set_body_json(company_body()))
        .mount(&server)
        .await;

    let c = make_client(&server)
        .enrich_company_by_domain("analytical.example")
        .await
        .expect("happy path");
    assert_eq!(c.id, "c_001");
    assert_eq!(c.domain, "analytical.example");
    assert_eq!(c.revenue_band.as_deref(), Some("$1M-$10M"));
    assert_eq!(c.duns.as_deref(), Some("123456789"));
}

#[tokio::test]
async fn search_people_paginates_with_size_cap_and_two_pages() {
    let server = MockServer::start().await;
    mount_auth(&server, "tok", None).await;
    let page1 = serde_json::json!({
        "data": [person_body()], "currentPage": 1, "maxResults": 2, "totalResults": 26
    });
    let page2 = serde_json::json!({
        "data": [{ "id": "p_002", "firstName": "Grace" }],
        "currentPage": 2, "maxResults": 2, "totalResults": 26
    });
    Mock::given(method("POST")).and(path("/search/contact"))
        .respond_with(ResponseTemplate::new(200).set_body_json(page1))
        .up_to_n_times(1).expect(1).mount(&server).await;
    Mock::given(method("POST")).and(path("/search/contact"))
        .respond_with(ResponseTemplate::new(200).set_body_json(page2))
        .expect(1).mount(&server).await;

    let zi = make_client(&server);
    let r1 = zi.search_people(serde_json::json!({"jobTitle": "eng"}), 1, 99).await.unwrap();
    assert_eq!((r1.page, r1.total_entries, r1.entries.len()), (1, 26, 1));
    let r2 = zi.search_people(serde_json::json!({"jobTitle": "eng"}), 2, 99).await.unwrap();
    assert_eq!(r2.page, 2);
    assert_eq!(r2.entries[0].first_name, "Grace");
}

#[tokio::test]
async fn oauth_token_cached_and_reused_across_two_lookups() {
    let server = MockServer::start().await;
    mount_auth(&server, "tok", Some(1)).await;
    Mock::given(method("POST")).and(path("/lookup/contact"))
        .respond_with(ResponseTemplate::new(200).set_body_json(person_body()))
        .expect(2).mount(&server).await;

    let zi = make_client(&server);
    zi.enrich_person_by_email("a@x.test").await.unwrap();
    zi.enrich_person_by_email("b@x.test").await.unwrap();
}

#[tokio::test]
async fn rate_limit_429_back_off_honors_retry_after() {
    let server = MockServer::start().await;
    mount_auth(&server, "tok", None).await;
    Mock::given(method("POST")).and(path("/lookup/contact"))
        .respond_with(
            ResponseTemplate::new(429).insert_header("Retry-After", "0").set_body_string(""),
        )
        .up_to_n_times(1).expect(1).mount(&server).await;
    Mock::given(method("POST")).and(path("/lookup/contact"))
        .respond_with(ResponseTemplate::new(200).set_body_json(person_body()))
        .expect(1).mount(&server).await;

    let p = make_client(&server).enrich_person_by_email("a@x.test").await.unwrap();
    assert_eq!(p.id, "p_001");
}

#[tokio::test]
async fn persistent_401_yields_auth_failed_and_403_credits_distinguishes_quota() {
    // 401 path — refresh fires, second 401 → AuthFailed.
    let server = MockServer::start().await;
    mount_auth(&server, "tok", None).await;
    Mock::given(method("POST")).and(path("/lookup/contact"))
        .respond_with(ResponseTemplate::new(401).set_body_string(""))
        .expect(2).mount(&server).await;
    let err = make_client(&server).enrich_person_by_email("a@x.test").await.unwrap_err();
    assert!(matches!(err, ZoomInfoError::AuthFailed));

    // 403-credits path — same shape, different status + body.
    let s2 = MockServer::start().await;
    mount_auth(&s2, "tok", None).await;
    Mock::given(method("POST")).and(path("/lookup/contact"))
        .respond_with(ResponseTemplate::new(403).set_body_string("Monthly credit quota exhausted"))
        .expect(1).mount(&s2).await;
    let err = make_client(&s2).enrich_person_by_email("a@x.test").await.unwrap_err();
    assert!(matches!(err, ZoomInfoError::CreditsExhausted));
}

#[tokio::test]
async fn token_refresh_on_401_then_retry_succeeds() {
    let server = MockServer::start().await;
    // First auth → "stale"; force_refresh fetches "fresh" on second call.
    Mock::given(method("POST")).and(path("/authenticate"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"jwt": "stale"})))
        .up_to_n_times(1).expect(1).mount(&server).await;
    Mock::given(method("POST")).and(path("/authenticate"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"jwt": "fresh"})))
        .expect(1).mount(&server).await;
    // First lookup → 401, triggers refresh; second lookup → 200.
    Mock::given(method("POST")).and(path("/lookup/contact"))
        .respond_with(ResponseTemplate::new(401).set_body_string(""))
        .up_to_n_times(1).expect(1).mount(&server).await;
    Mock::given(method("POST")).and(path("/lookup/contact"))
        .respond_with(ResponseTemplate::new(200).set_body_json(person_body()))
        .expect(1).mount(&server).await;

    let p = make_client(&server).enrich_person_by_email("a@x.test").await.unwrap();
    assert_eq!(p.id, "p_001");
}
