use super::super::{routes::dispatch_route, shutdown::ShutdownController};
use argo_store::SessionStore;
use base64::Engine;
use ed25519_dalek::SigningKey;
use std::sync::Arc;

const TENANT_A: &str = "019aff00-0000-7000-8000-000000000101";
const TENANT_B: &str = "019aff00-0000-7000-8000-000000000202";

fn reset_auth_env() {
    for key in [
        "ARGO_GATEWAY_BEARER_TOKEN",
        "ARGO_GATEWAY_AUTH_BYPASS",
        "ARGO_STORE_BATCHED_WRITES",
        "ARGO_DEFAULT_TENANT",
        "ARGO_INBOX_WS_JWT_PUBKEY",
        "ARGO_INBOX_WS_JWT_PUBKEY_PEM",
    ] {
        unsafe { std::env::remove_var(key) };
    }
}

#[rustfmt::skip]
fn authed(path: &str, body: &str) -> String {
    format!("POST {path} HTTP/1.1\r\nAuthorization: Bearer secret\r\nX-Argo-Principal: human:mark@example.com\r\nX-Tenant-Id: attacker\r\nContent-Length: {}\r\n\r\n{body}", body.len())
}

#[rustfmt::skip]
#[tokio::test]
async fn marketplace_install_authz_and_audit() {
    reset_auth_env();
    unsafe {
        std::env::set_var("ARGO_GATEWAY_BEARER_TOKEN", "secret");
        std::env::set_var("ARGO_STORE_BATCHED_WRITES", "0");
        std::env::set_var("ARGO_DEFAULT_TENANT", TENANT_A);
    }
    let shutdown = ShutdownController::new();
    let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
    store.migrate().await.unwrap();

    let (status, _) = dispatch_route("GET /marketplace HTTP/1.1\r\n\r\n", None, &shutdown).await;
    assert!(status.contains("text/html"));
    let (status, _) = dispatch_route("POST /marketplace/marketplace-listing-draft/install HTTP/1.1\r\n\r\n", Some(&store), &shutdown).await;
    assert_eq!(status, "401 Unauthorized");

    let (status, _) = dispatch_route(&authed(&format!("/marketplace/marketplace-listing-draft/install?tenant={TENANT_A}"), ""), Some(&store), &shutdown).await;
    assert!(status.contains("text/html"));

    let signing_key = SigningKey::from_bytes(&[7; 32]);
    let pubkey = base64::engine::general_purpose::STANDARD.encode(signing_key.verifying_key().to_bytes());
    unsafe { std::env::set_var("ARGO_INBOX_WS_JWT_PUBKEY", pubkey) };
    let now = chrono::Utc::now().timestamp();
    let jwt = super::super::jwt::sign_jwt_for_test(&signing_key, &super::super::jwt::Claims {
        sub: "human:jwt-marketplace".to_string(),
        tnt: TENANT_A.to_string(),
        iat: now - 1,
        exp: now + 60,
    });
    let jwt_good = format!("POST /marketplace/marketplace-listing-draft/install?tenant={TENANT_A} HTTP/1.1\r\nAuthorization: Bearer {jwt}\r\nX-Tenant-Id: attacker\r\n\r\n");
    let (status, _) = dispatch_route(&jwt_good, Some(&store), &shutdown).await;
    assert!(status.contains("text/html"));

    let (status, _) = dispatch_route(&authed(&format!("/marketplace/marketplace-listing-draft/install?tenant={TENANT_B}"), ""), Some(&store), &shutdown).await;
    assert_eq!(status, "403 Forbidden");
    let (status, body) = dispatch_route(&authed("/marketplace/marketplace-listing-draft/install", &format!("{{\"tenant_id\":\"{TENANT_B}\"}}")), Some(&store), &shutdown).await;
    assert_eq!(status, "403 Forbidden");
    assert!(body.contains("tenant mismatch"));

    for principal in ["human:mark@example.com", "human:jwt-marketplace"] {
        let rows: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM audit_record WHERE payload_json LIKE '%marketplace.skill_installed%' AND principal_id = ?")
            .bind(principal)
            .fetch_one(store.pool())
            .await
            .unwrap();
        assert_eq!(rows, 1, "{principal}");
    }
    reset_auth_env();
}
