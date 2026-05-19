//! Tests for the magic-link signup + verify endpoints.

use super::*;
use argo_store::SessionStore;
use std::sync::Arc;

async fn fresh_store() -> Arc<SessionStore> {
    let store = SessionStore::open(":memory:")
        .await
        .expect("in-memory store");
    // Migrate so the substrate user_tenant + audit_record tables exist
    // when verify_outcome's emit-audit hooks fire from a verify path.
    store.migrate().await.expect("migrate");
    Arc::new(store)
}

fn req(method_path: &str, ip: Option<&str>, body: &str) -> String {
    let mut headers = String::new();
    if let Some(ip) = ip {
        headers.push_str(&format!("X-Forwarded-For: {ip}\r\n"));
    }
    format!(
        "{method_path} HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\n{headers}\r\n{body}",
        body.len()
    )
}

#[tokio::test]
async fn signup_with_valid_email_returns_200() {
    let store = fresh_store().await;
    let r = req(
        "POST /api/v1/auth/signup",
        Some("203.0.113.10"),
        r#"{"email":"alice@example.com"}"#,
    );
    let (status, body) = signup_response(&r, Some(&store)).await;
    assert!(status.starts_with("200"), "status={status} body={body}");
    assert!(body.contains("\"status\":\"sent\""));
}

#[tokio::test]
async fn signup_rejects_bad_email() {
    let store = fresh_store().await;
    let r = req(
        "POST /api/v1/auth/signup",
        Some("203.0.113.11"),
        r#"{"email":"not-an-email"}"#,
    );
    let (status, _) = signup_response(&r, Some(&store)).await;
    assert!(status.starts_with("400"));
}

#[tokio::test]
async fn signup_rejects_malformed_json() {
    let store = fresh_store().await;
    let r = req(
        "POST /api/v1/auth/signup",
        Some("203.0.113.12"),
        "not json at all",
    );
    let (status, _) = signup_response(&r, Some(&store)).await;
    assert!(status.starts_with("400"));
}

#[tokio::test]
async fn signup_rate_limit_uses_shared_signup_bucket() {
    use super::rate_limit_ip::{
        signup_limiter, RateLimitIpCheckResult, SIGNUP_BUCKET_CAPACITY,
    };
    use std::net::{IpAddr, Ipv4Addr};
    use std::time::Instant;
    // Drive the shared limiter directly so the test isn't racy on
    // wall-clock and doesn't share buckets with the live `signup`
    // singleton (we use a process-unique IP).
    let ip: IpAddr = Ipv4Addr::new(192, 0, 2, 99).into();
    let now = Instant::now();
    for _ in 0..SIGNUP_BUCKET_CAPACITY {
        assert!(matches!(
            signup_limiter().check_with_now(ip, now),
            RateLimitIpCheckResult::Allow
        ));
    }
    assert!(matches!(
        signup_limiter().check_with_now(ip, now),
        RateLimitIpCheckResult::Deny { .. }
    ));
}

#[tokio::test]
async fn verify_with_invalid_token_returns_401() {
    let store = fresh_store().await;
    let r = req(
        "POST /api/v1/auth/verify",
        None,
        r#"{"token":"never-issued"}"#,
    );
    let outcome = verify_outcome(&r, Some(&store)).await;
    assert!(
        outcome.status.starts_with("401"),
        "status={} body={}",
        outcome.status,
        outcome.body
    );
    assert!(outcome.body.contains("\"invalid\""));
    assert!(outcome.set_cookie.is_none(), "no cookie on failure");
}

#[tokio::test]
async fn signup_then_verify_round_trip_sets_host_cookie() {
    let store = fresh_store().await;
    // Signup mints a token; we can't fish the raw token out of the
    // response (it goes only into the email/log), so insert one
    // directly via the store API to mimic the post-email click.
    let raw_token = "0123456789abcdef0123456789abcdef";
    let token_hash = sha256_hex(raw_token);
    let now = chrono::Utc::now().timestamp();
    argo_store::user_account::insert_magic_link(
        store.pool(),
        &token_hash,
        "bob@example.com",
        now + 900,
    )
    .await
    .expect("insert");
    let verify_req = req(
        "POST /api/v1/auth/verify",
        None,
        &format!(r#"{{"token":"{raw_token}"}}"#),
    );
    let outcome = verify_outcome(&verify_req, Some(&store)).await;
    assert!(
        outcome.status.starts_with("200"),
        "status={} body={}",
        outcome.status,
        outcome.body
    );
    assert!(outcome.body.contains("\"needs_onboarding\":true"));
    // JWT is no longer in the body; cookie is the canonical carrier.
    assert!(!outcome.body.contains("\"jwt\""));
    let cookie = outcome.set_cookie.expect("verify must emit Set-Cookie");
    assert!(cookie.starts_with("__Host-session="), "cookie={cookie}");
    assert!(cookie.contains("HttpOnly"), "cookie={cookie}");
    assert!(cookie.contains("Secure"), "cookie={cookie}");
    assert!(cookie.contains("SameSite=Lax"), "cookie={cookie}");
    assert!(cookie.contains("Path=/"), "cookie={cookie}");
    assert!(cookie.contains("Max-Age=604800"), "cookie={cookie}");
}

#[tokio::test]
async fn verify_is_single_use() {
    let store = fresh_store().await;
    let raw_token = "single-use-1234567890";
    let token_hash = sha256_hex(raw_token);
    let now = chrono::Utc::now().timestamp();
    argo_store::user_account::insert_magic_link(
        store.pool(),
        &token_hash,
        "carol@example.com",
        now + 900,
    )
    .await
    .unwrap();
    let r = req(
        "POST /api/v1/auth/verify",
        None,
        &format!(r#"{{"token":"{raw_token}"}}"#),
    );
    let first = verify_outcome(&r, Some(&store)).await;
    assert!(first.status.starts_with("200"));
    let second = verify_outcome(&r, Some(&store)).await;
    assert!(
        second.status.starts_with("401"),
        "{} {}",
        second.status,
        second.body
    );
    assert!(second.body.contains("already_used"));
    assert!(second.set_cookie.is_none());
}

#[tokio::test]
async fn verify_expired_token_returns_401() {
    let store = fresh_store().await;
    let raw_token = "expired-token-xxxxxxxxxxxxxxxxxxxxxxxxx";
    let token_hash = sha256_hex(raw_token);
    let now = chrono::Utc::now().timestamp();
    argo_store::user_account::insert_magic_link(
        store.pool(),
        &token_hash,
        "dan@example.com",
        now - 100,
    )
    .await
    .unwrap();
    let r = req(
        "POST /api/v1/auth/verify",
        None,
        &format!(r#"{{"token":"{raw_token}"}}"#),
    );
    let outcome = verify_outcome(&r, Some(&store)).await;
    assert!(
        outcome.status.starts_with("401"),
        "{} {}",
        outcome.status,
        outcome.body
    );
    assert!(outcome.body.contains("expired"));
    assert!(outcome.set_cookie.is_none());
}

#[test]
fn is_verify_request_matches_only_verify() {
    assert!(is_verify_request(
        "POST /api/v1/auth/verify HTTP/1.1\r\n\r\n"
    ));
    assert!(is_verify_request(
        "POST /api/v1/auth/verify?x=1 HTTP/1.1\r\n\r\n"
    ));
    assert!(!is_verify_request(
        "POST /api/v1/auth/signup HTTP/1.1\r\n\r\n"
    ));
    assert!(!is_verify_request("GET /api/v1/auth/verify HTTP/1.1\r\n\r\n"));
}

#[test]
fn looks_like_email_accepts_normal_addresses() {
    assert!(looks_like_email("a@b.co"));
    assert!(looks_like_email("first.last+tag@example.co.uk"));
}

#[test]
fn looks_like_email_rejects_garbage() {
    assert!(!looks_like_email(""));
    assert!(!looks_like_email("nope"));
    assert!(!looks_like_email("@example.com"));
    assert!(!looks_like_email("user@.tld"));
    assert!(!looks_like_email("user@no-dot"));
}

#[tokio::test]
async fn verify_binds_newest_tenant_for_multi_tenant_user() {
    use argo_identity::tenant::Tenant;
    use argo_identity::tenant_store::insert_tenant;
    use argo_identity::user::UserAccount;
    use argo_identity::user_store::insert_user;
    use argo_identity::user_tenant::UserTenantRole;
    use argo_identity::user_tenant_store::link_user_to_tenant;
    let store = fresh_store().await;
    let user = UserAccount::new("multi@example.com".into());
    insert_user(&store, &user).await.unwrap();
    let older = Tenant::new("Older".into(), "older@example.com".into());
    let newer = Tenant::new("Newer".into(), "newer@example.com".into());
    insert_tenant(&store, &older).await.unwrap();
    insert_tenant(&store, &newer).await.unwrap();
    link_user_to_tenant(&store, user.id, older.id, UserTenantRole::Owner)
        .await
        .unwrap();
    link_user_to_tenant(&store, user.id, newer.id, UserTenantRole::Owner)
        .await
        .unwrap();
    // `created_at` on the link rows is second-granular; force a
    // deterministic DESC ordering so the test pins the newest-tenant
    // contract regardless of wall-clock collisions.
    sqlx::query("UPDATE user_tenant SET created_at = ? WHERE tenant_id = ?")
        .bind(1_000i64)
        .bind(older.id.0.as_bytes().as_slice())
        .execute(store.pool())
        .await
        .unwrap();
    sqlx::query("UPDATE user_tenant SET created_at = ? WHERE tenant_id = ?")
        .bind(2_000i64)
        .bind(newer.id.0.as_bytes().as_slice())
        .execute(store.pool())
        .await
        .unwrap();
    let raw_token = "multi-tenant-token-xxxxxxxxxxxxxxxx";
    let token_hash = sha256_hex(raw_token);
    let now = chrono::Utc::now().timestamp();
    argo_store::user_account::insert_magic_link(store.pool(), &token_hash, &user.email, now + 900)
        .await
        .unwrap();
    let r = req(
        "POST /api/v1/auth/verify",
        None,
        &format!(r#"{{"token":"{raw_token}"}}"#),
    );
    let outcome = verify_outcome(&r, Some(&store)).await;
    assert!(outcome.status.starts_with("200"), "{}", outcome.status);
    let expected = newer.id.to_string();
    assert!(
        outcome.body.contains(&expected),
        "expected newer tenant {expected} in body: {}",
        outcome.body
    );
    assert!(
        outcome.body.contains("\"needs_onboarding\":false"),
        "user has linked tenants; needs_onboarding must be false: {}",
        outcome.body
    );
}

#[test]
fn verify_rate_limit_denies_61st_attempt_in_window() {
    use std::net::Ipv4Addr;
    use std::time::Instant;
    // Process-unique IP so the static bucket map doesn't see other
    // tests' counters. Same trick the signup rate-limit test uses.
    let ip: IpAddr = Ipv4Addr::new(192, 0, 2, 200).into();
    let t0 = Instant::now();
    for i in 0..VERIFY_RATE_LIMIT_PER_MIN {
        assert!(
            verify_rate_limit_check(ip, t0),
            "attempt {i} should be allowed"
        );
    }
    assert!(!verify_rate_limit_check(ip, t0), "61st must deny");
    // After the sliding window elapses the bucket refills.
    let t1 = t0 + VERIFY_RATE_LIMIT_WINDOW + std::time::Duration::from_secs(1);
    assert!(verify_rate_limit_check(ip, t1), "post-window must allow");
}

#[tokio::test]
async fn verify_emits_auth_succeeded_and_auth_failed_audit_rows() {
    let store = fresh_store().await;
    // Mint a real token so the success path runs end-to-end.
    let raw_token = "audit-emit-roundtrip-token-xxxxxx";
    let token_hash = sha256_hex(raw_token);
    let now = chrono::Utc::now().timestamp();
    argo_store::user_account::insert_magic_link(
        store.pool(),
        &token_hash,
        "audit-emit@example.com",
        now + 900,
    )
    .await
    .unwrap();
    // Success → AuthSucceeded.
    let ok = verify_outcome(
        &req(
            "POST /api/v1/auth/verify",
            Some("203.0.113.42"),
            &format!(r#"{{"token":"{raw_token}"}}"#),
        ),
        Some(&store),
    )
    .await;
    assert!(ok.status.starts_with("200"), "{}", ok.status);
    // Failure → AuthFailed (NotFound).
    let bad = verify_outcome(
        &req(
            "POST /api/v1/auth/verify",
            Some("203.0.113.43"),
            r#"{"token":"never-minted"}"#,
        ),
        Some(&store),
    )
    .await;
    assert!(bad.status.starts_with("401"), "{}", bad.status);
    // Both rows live in the audit table under the `auth.*` kinds.
    let (success_count, success_payload): (i64, String) = sqlx::query_as(
        "SELECT COUNT(*), COALESCE(MAX(payload_json), '') FROM audit_record WHERE kind = ?",
    )
    .bind("auth.succeeded")
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert_eq!(success_count, 1, "expected one auth.succeeded row");
    assert!(
        success_payload.contains("\"method\":\"magic_link\""),
        "{success_payload}"
    );
    assert!(
        success_payload.contains("\"source_ip\":\"203.0.113.42\""),
        "{success_payload}"
    );
    let (failed_count, failed_payload): (i64, String) = sqlx::query_as(
        "SELECT COUNT(*), COALESCE(MAX(payload_json), '') FROM audit_record WHERE kind = ?",
    )
    .bind("auth.failed")
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert!(failed_count >= 1, "expected at least one auth.failed row");
    assert!(
        failed_payload.contains("\"reason\":\"invalid\""),
        "{failed_payload}"
    );
}
