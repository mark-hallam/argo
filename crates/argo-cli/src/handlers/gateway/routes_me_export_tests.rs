//! Integration tests for [`super::routes_me_export`].
//! Covers the auth + rate-limit gates, the cross-user scope, and a
//! round-trip of POST → status → download.

use super::*;
use argo_identity::tenant::Tenant;
use argo_identity::tenant_store::insert_tenant;
use argo_identity::user::UserAccount;
use argo_identity::user_store::insert_user;
use argo_identity::user_tenant::UserTenantRole;
use argo_identity::user_tenant_store::link_user_to_tenant;
use argo_store::SessionStore;
use std::sync::Arc;

async fn fresh_store() -> Arc<SessionStore> {
    let s = SessionStore::open(":memory:").await.expect("memory store");
    s.migrate().await.expect("migrate");
    Arc::new(s)
}

fn req(method: &str, path: &str, principal: Option<&str>, body: &str) -> String {
    let mut r = format!("{method} {path} HTTP/1.1\r\nHost: x\r\n");
    if let Some(p) = principal {
        r.push_str(&format!("X-Argo-Principal: {p}\r\n"));
    }
    r.push_str(&format!("Content-Length: {}\r\n", body.len()));
    r.push_str("\r\n");
    r.push_str(body);
    r
}

#[tokio::test]
async fn post_without_principal_is_unauthorized() {
    let store = fresh_store().await;
    let resp = dispatch(
        &req("POST", "/api/v1/me/export", None, "{}"),
        Some(&store),
    )
    .await
    .expect("matched");
    assert_eq!(resp.0, "401 Unauthorized");
}

#[tokio::test]
async fn post_with_unknown_email_is_unauthorized() {
    let store = fresh_store().await;
    let resp = dispatch(
        &req(
            "POST",
            "/api/v1/me/export",
            Some("human:ghost@example.com"),
            "{}",
        ),
        Some(&store),
    )
    .await
    .expect("matched");
    assert_eq!(resp.0, "401 Unauthorized");
}

#[tokio::test]
async fn full_round_trip_post_status_download() {
    let store = fresh_store().await;
    let user = UserAccount::new("flo@acme.com".into());
    insert_user(&store, &user).await.unwrap();
    let tenant = Tenant::new("Acme".into(), "billing@acme.com".into());
    insert_tenant(&store, &tenant).await.unwrap();
    link_user_to_tenant(&store, user.id, tenant.id, UserTenantRole::Owner)
        .await
        .unwrap();

    let post = dispatch(
        &req(
            "POST",
            "/api/v1/me/export",
            Some("human:flo@acme.com"),
            "{}",
        ),
        Some(&store),
    )
    .await
    .expect("post matched");
    assert_eq!(post.0, "202 Accepted");
    let body: serde_json::Value = serde_json::from_str(&post.1).unwrap();
    let job_id = body["job_id"].as_str().unwrap().to_string();
    let download_url = body["download_url"].as_str().unwrap().to_string();
    assert!(download_url.starts_with("/api/v1/me/export/download/"));

    let status = dispatch(
        &req(
            "GET",
            &format!("/api/v1/me/export/{job_id}"),
            Some("human:flo@acme.com"),
            "",
        ),
        Some(&store),
    )
    .await
    .expect("status matched");
    assert_eq!(status.0, "200 OK");
    let status_body: serde_json::Value = serde_json::from_str(&status.1).unwrap();
    assert_eq!(status_body["status"], "ready");

    let path_part = download_url.trim_start_matches('/');
    let download = dispatch(
        &req("GET", &format!("/{path_part}"), Some("human:flo@acme.com"), ""),
        Some(&store),
    )
    .await
    .expect("download matched");
    assert_eq!(download.0, "200 OK");
    let dl_body: serde_json::Value = serde_json::from_str(&download.1).unwrap();
    assert_eq!(dl_body["content_type"], "application/gzip");
    assert!(dl_body["size_bytes"].as_u64().unwrap() > 0);
    let b64 = dl_body["body_b64"].as_str().unwrap();
    let raw = base64::engine::general_purpose::STANDARD
        .decode(b64)
        .unwrap();
    // gzip magic bytes — confirms we actually shipped a tar.gz.
    assert_eq!(&raw[..2], &[0x1f, 0x8b]);
}

#[tokio::test]
async fn rate_limit_rejects_second_request_in_window() {
    let store = fresh_store().await;
    let user = UserAccount::new("twice@acme.com".into());
    insert_user(&store, &user).await.unwrap();

    let first = dispatch(
        &req(
            "POST",
            "/api/v1/me/export",
            Some("human:twice@acme.com"),
            "{}",
        ),
        Some(&store),
    )
    .await
    .expect("first matched");
    assert_eq!(first.0, "202 Accepted");

    let second = dispatch(
        &req(
            "POST",
            "/api/v1/me/export",
            Some("human:twice@acme.com"),
            "{}",
        ),
        Some(&store),
    )
    .await
    .expect("second matched");
    assert_eq!(second.0, "429 Too Many Requests");
    let body: serde_json::Value = serde_json::from_str(&second.1).unwrap();
    assert_eq!(body["error"]["code"], "rate_limited");
}

#[tokio::test]
async fn status_for_other_users_job_is_404() {
    let store = fresh_store().await;
    let alice = UserAccount::new("alice2@acme.com".into());
    let bob = UserAccount::new("bob2@acme.com".into());
    insert_user(&store, &alice).await.unwrap();
    insert_user(&store, &bob).await.unwrap();
    let post = dispatch(
        &req(
            "POST",
            "/api/v1/me/export",
            Some("human:alice2@acme.com"),
            "{}",
        ),
        Some(&store),
    )
    .await
    .expect("matched");
    assert_eq!(post.0, "202 Accepted");
    let body: serde_json::Value = serde_json::from_str(&post.1).unwrap();
    let job_id = body["job_id"].as_str().unwrap();
    let status = dispatch(
        &req(
            "GET",
            &format!("/api/v1/me/export/{job_id}"),
            Some("human:bob2@acme.com"),
            "",
        ),
        Some(&store),
    )
    .await
    .expect("matched");
    assert_eq!(status.0, "404 Not Found");
}

#[tokio::test]
async fn download_with_unknown_token_is_404() {
    let store = fresh_store().await;
    let resp = dispatch(
        &req(
            "GET",
            "/api/v1/me/export/download/totally-unknown-token",
            None,
            "",
        ),
        Some(&store),
    )
    .await
    .expect("matched");
    assert_eq!(resp.0, "404 Not Found");
}

#[tokio::test]
async fn dispatch_returns_none_on_unrelated_path() {
    let store = fresh_store().await;
    let resp = dispatch(
        &req("GET", "/api/v1/foo/bar", Some("human:x@example.com"), ""),
        Some(&store),
    )
    .await;
    assert!(resp.is_none(), "non-matching path must fall through");
}

#[tokio::test]
async fn audit_row_emitted_on_successful_post() {
    let store = fresh_store().await;
    let user = UserAccount::new("audit@acme.com".into());
    insert_user(&store, &user).await.unwrap();
    let resp = dispatch(
        &req(
            "POST",
            "/api/v1/me/export",
            Some("human:audit@acme.com"),
            "{}",
        ),
        Some(&store),
    )
    .await
    .expect("matched");
    assert_eq!(resp.0, "202 Accepted");
    // Give the spawned audit-sink flush a moment to land — the
    // background task posts the row asynchronously.
    for _ in 0..20 {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM audit_record WHERE kind = 'data_export.requested'")
                .fetch_one(store.pool())
                .await
                .unwrap();
        if count >= 1 {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    panic!("audit row never landed");
}
