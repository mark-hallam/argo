//! Integration pin for evidence-bundle tenant binding. The SPA may
//! still send cosmetic `tenantId` values in JSON/query params, but
//! the gateway must derive the export tenant from the signed session
//! JWT and ignore forged client claims.

#[path = "fixtures/multi_tenant_seed.rs"]
mod seed;

use argo_store::{wal_tuning::WalTuningProfile, SessionStore};
use base64::{engine::general_purpose, Engine as _};
use jsonwebtoken::{EncodingKey, Header};
use seed::{boot_gateway_with_home, provision_tenant, seed_tenant_rows, SeedCounts};
use serde::Serialize;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

const SMOKE_DB_KEY_B64: &str = "QkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkI=";

const COUNTS_A: SeedCounts = SeedCounts {
    audit: 3,
    memory: 0,
    workers: 0,
    skills: 0,
};

const COUNTS_B: SeedCounts = SeedCounts {
    audit: 7,
    memory: 0,
    workers: 0,
    skills: 0,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn audit_export_resolves_tenant_from_session_jwt_not_client_claims() {
    let secret = format!("audit-export-tenant-{}", uuid::Uuid::now_v7().simple());
    std::env::set_var("SKYCOT_AUTH_JWT_SECRET", &secret);
    std::env::set_var("SKYCOT_DB_KEY_BASE64", SMOKE_DB_KEY_B64);
    std::env::set_var("ARGO_TEST_LISTENER_DEADLINE_MS", "60000");
    let export_dir = tempfile::tempdir().expect("export dir");
    std::env::set_var("ARGO_AUDIT_EXPORT_DIR", export_dir.path());

    let gateway = boot_gateway_with_home();
    let store = connect_encrypted_store(&gateway.home).await;
    let tenant_a = provision_tenant(&store, "audit-export-tenant-a")
        .await
        .expect("tenant a");
    let tenant_b = provision_tenant(&store, "audit-export-tenant-b")
        .await
        .expect("tenant b");
    seed_tenant_rows(&store, &tenant_a, "tenant-a-marker", COUNTS_A)
        .await
        .expect("seed tenant a");
    seed_tenant_rows(&store, &tenant_b, "tenant-b-marker", COUNTS_B)
        .await
        .expect("seed tenant b");
    let export_window_to = chrono::Utc::now().to_rfc3339();

    let token = mint_session_jwt(&secret, &tenant_a.tenant_audit_str);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .expect("client");
    let base = format!("http://127.0.0.1:{}", gateway.port);

    let own_path = format!(
        "/api/v1/audit/export?tenantId={}",
        tenant_a.tenant_audit_str
    );
    let own_body = post_json(
        &client,
        &base,
        &own_path,
        &token,
        Some(&tenant_a.tenant_audit_str),
        json!({"format": "json", "tenantId": tenant_a.tenant_audit_str, "to": export_window_to.clone()}),
    )
    .await;
    assert_eq!(own_body["tenant_id"], tenant_a.tenant_audit_str);
    assert_eq!(own_body["row_count"], json!(COUNTS_A.audit));
    assert!(
        own_body["download_url"]
            .as_str()
            .expect("own download_url")
            .contains(&format!("tenant={}", tenant_a.tenant_audit_str)),
        "own export URL must be signed for tenant A: {own_body}",
    );

    let no_param_body = post_json(
        &client,
        &base,
        "/api/v1/audit/export",
        &token,
        None,
        json!({"format": "json", "to": export_window_to.clone()}),
    )
    .await;
    assert_eq!(no_param_body["tenant_id"], tenant_a.tenant_audit_str);
    assert_eq!(no_param_body["row_count"], json!(COUNTS_A.audit));
    assert!(
        no_param_body["download_url"]
            .as_str()
            .expect("no-param download_url")
            .contains(&format!("tenant={}", tenant_a.tenant_audit_str)),
        "no-param export URL must be signed for tenant A: {no_param_body}",
    );

    let forged_path = format!(
        "/api/v1/audit/export?tenantId={}",
        tenant_b.tenant_audit_str
    );
    let export_body = post_json(
        &client,
        &base,
        &forged_path,
        &token,
        Some(&tenant_b.tenant_audit_str),
        json!({"format": "json", "tenantId": tenant_b.tenant_audit_str, "to": export_window_to.clone()}),
    )
    .await;
    assert_eq!(export_body["tenant_id"], tenant_a.tenant_audit_str);
    assert_eq!(export_body["row_count"], json!(COUNTS_A.audit));
    let download_url = export_body["download_url"].as_str().expect("download_url");
    assert!(
        download_url.contains(&format!("tenant={}", tenant_a.tenant_audit_str)),
        "download URL must be signed for tenant A: {download_url}",
    );

    let download_body = get_json(
        &client,
        &base,
        download_url,
        &token,
        Some(&tenant_b.tenant_audit_str),
    )
    .await;
    assert_eq!(download_body["tenant_id"], tenant_a.tenant_audit_str);
    let bytes = general_purpose::STANDARD
        .decode(download_body["b64"].as_str().expect("b64"))
        .expect("decode bundle");
    let ndjson = String::from_utf8(bytes).expect("bundle utf8");
    assert!(
        ndjson.contains("tenant-a-marker"),
        "tenant A rows missing: {ndjson}"
    );
    assert!(
        !ndjson.contains("tenant-b-marker"),
        "tenant B rows leaked into tenant A export: {ndjson}",
    );

    let forged_preview_path = format!(
        "/api/v1/audit/export/preview?tenantId={}",
        tenant_b.tenant_audit_str
    );
    let preview_body = post_json(
        &client,
        &base,
        &forged_preview_path,
        &token,
        Some(&tenant_b.tenant_audit_str),
        json!({"format": "json", "tenantId": tenant_b.tenant_audit_str, "to": export_window_to}),
    )
    .await;
    assert_eq!(preview_body["tenant_id"], tenant_a.tenant_audit_str);
    assert_eq!(preview_body["row_count"], json!(COUNTS_A.audit));
}

async fn connect_encrypted_store(home: &std::path::Path) -> Arc<SessionStore> {
    let hex = argo_store::encryption::base64_to_hex(SMOKE_DB_KEY_B64).expect("hex key");
    let store = SessionStore::open_with_hex_key(
        home.join("sessions.db"),
        WalTuningProfile::default(),
        &hex,
    )
    .await
    .expect("open encrypted store");
    store.migrate().await.expect("migrate store");
    Arc::new(store)
}

async fn post_json(
    client: &reqwest::Client,
    base: &str,
    path: &str,
    token: &str,
    forged_tenant_header: Option<&str>,
    body: Value,
) -> Value {
    let mut req = client
        .post(format!("{base}{path}"))
        .header("Authorization", format!("Bearer {token}"))
        .json(&body);
    if let Some(tenant) = forged_tenant_header {
        req = req.header("X-Argo-Tenant-Id", tenant);
    }
    let resp = req.send().await.expect("post export");
    let status = resp.status();
    let body = resp.text().await.expect("post body");
    assert!(status.is_success(), "status={status} body={body}");
    serde_json::from_str(&body).unwrap_or_else(|e| panic!("json parse: {e}: {body}"))
}

async fn get_json(
    client: &reqwest::Client,
    base: &str,
    path: &str,
    token: &str,
    forged_tenant_header: Option<&str>,
) -> Value {
    let mut req = client
        .get(format!("{base}{path}"))
        .header("Authorization", format!("Bearer {token}"));
    if let Some(tenant) = forged_tenant_header {
        req = req.header("X-Argo-Tenant-Id", tenant);
    }
    let resp = req.send().await.expect("get export");
    let status = resp.status();
    let body = resp.text().await.expect("get body");
    assert!(status.is_success(), "status={status} body={body}");
    serde_json::from_str(&body).unwrap_or_else(|e| panic!("json parse: {e}: {body}"))
}

fn mint_session_jwt(secret: &str, tenant: &str) -> String {
    #[derive(Serialize)]
    struct Claims<'a> {
        sub: &'a str,
        email: &'a str,
        tnt: &'a str,
        role: &'a str,
        iat: i64,
        exp: i64,
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_secs() as i64;
    jsonwebtoken::encode(
        &Header::new(jsonwebtoken::Algorithm::HS256),
        &Claims {
            sub: "session-user",
            email: "session-user@example.com",
            tnt: tenant,
            role: "owner",
            iat: now,
            exp: now + 3600,
        },
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .expect("mint session jwt")
}
