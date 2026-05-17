//! Unit tests for [`crate::data_export`]. Verify per-user scope is
//! strict — cross-user data must not appear in a single user's bundle.
use super::*;
use crate::tenant::Tenant;
use crate::tenant_store::insert_tenant;
use crate::user_store::insert_user;
use crate::user_tenant_store::link_user_to_tenant;
use argo_store::SessionStore;
use chrono::TimeZone;

async fn fresh_store() -> SessionStore {
    let s = SessionStore::open(":memory:").await.expect("memory store");
    s.migrate().await.expect("migrate");
    s
}

#[tokio::test]
async fn empty_export_has_manifest_and_empty_sections() {
    let store = fresh_store().await;
    let user = UserAccount::new("solo@acme.com".into());
    insert_user(&store, &user).await.unwrap();
    let bundle = gather_for_user(&store, user.id, Utc::now()).await.unwrap();
    let paths: Vec<&str> = bundle.files.iter().map(|f| f.path.as_str()).collect();
    assert!(paths.contains(&"manifest.json"));
    assert!(paths.contains(&"user_account.json"));
    assert!(paths.contains(&"tenants.json"));
    assert!(paths.contains(&"workers.jsonl"));
    assert!(paths.contains(&"audit/chain.jsonl"));
    assert!(bundle.total_bytes > 0, "manifest is always present");
}

#[tokio::test]
async fn user_file_round_trips_through_parse_helper() {
    let store = fresh_store().await;
    let mut user = UserAccount::new("round@acme.com".into());
    user.display_name = Some("Round Tripper".into());
    user.last_login_at = Some(1_234);
    insert_user(&store, &user).await.unwrap();
    let bundle = gather_for_user(&store, user.id, Utc::now()).await.unwrap();
    let user_file = bundle
        .files
        .iter()
        .find(|f| f.path == "user_account.json")
        .expect("user_account.json present");
    let parsed = parse_user_file_for_test(&user_file.contents).unwrap();
    assert_eq!(parsed.email, "round@acme.com");
    assert_eq!(parsed.display_name.as_deref(), Some("Round Tripper"));
    assert_eq!(parsed.last_login_at, Some(1_234));
}

#[tokio::test]
async fn cross_user_data_is_not_included() {
    let store = fresh_store().await;
    let alice = UserAccount::new("alice@acme.com".into());
    let bob = UserAccount::new("bob@acme.com".into());
    insert_user(&store, &alice).await.unwrap();
    insert_user(&store, &bob).await.unwrap();
    // Both belong to the same tenant; only Alice is Owner so workers
    // (none here) would only appear for Alice; the tenants file
    // should ONLY list rows for the requesting user.
    let tenant = Tenant::new("Acme".into(), "billing@acme.com".into());
    insert_tenant(&store, &tenant).await.unwrap();
    link_user_to_tenant(&store, alice.id, tenant.id, UserTenantRole::Owner)
        .await
        .unwrap();
    link_user_to_tenant(&store, bob.id, tenant.id, UserTenantRole::User)
        .await
        .unwrap();

    let alice_bundle = gather_for_user(&store, alice.id, Utc::now()).await.unwrap();
    let alice_user = alice_bundle
        .files
        .iter()
        .find(|f| f.path == "user_account.json")
        .unwrap();
    assert!(
        alice_user.contents.contains("alice@acme.com"),
        "alice file must carry her own email"
    );
    assert!(
        !alice_user.contents.contains("bob@acme.com"),
        "alice file must NOT carry bob's email"
    );
    let alice_tenants = alice_bundle
        .files
        .iter()
        .find(|f| f.path == "tenants.json")
        .unwrap();
    // The tenant-name string is shared, but the role projection MUST be `owner` only.
    assert!(alice_tenants.contents.contains("\"role\": \"owner\""));
    assert!(!alice_tenants.contents.contains("\"role\": \"user\""));

    let bob_bundle = gather_for_user(&store, bob.id, Utc::now()).await.unwrap();
    let bob_tenants = bob_bundle
        .files
        .iter()
        .find(|f| f.path == "tenants.json")
        .unwrap();
    assert!(bob_tenants.contents.contains("\"role\": \"user\""));
    assert!(!bob_tenants.contents.contains("\"role\": \"owner\""));
}

#[tokio::test]
async fn audit_rows_scoped_to_principal_email() {
    let store = fresh_store().await;
    let user = UserAccount::new("auditor@acme.com".into());
    insert_user(&store, &user).await.unwrap();
    // Insert two audit rows: one stamped with this user's principal,
    // one with another principal. Only the first must appear.
    let bytes_self = uuid::Uuid::now_v7();
    let bytes_other = uuid::Uuid::now_v7();
    sqlx::query(
        "INSERT INTO audit_record (id, kind, payload_json, parent_hash, self_hash, created_at, principal_id) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(bytes_self.as_bytes().as_slice())
    .bind("tenant.provisioned")
    .bind("{}")
    .bind(vec![0u8; 32])
    .bind(vec![0u8; 32])
    .bind(100_i64)
    .bind("human:auditor@acme.com")
    .execute(store.pool())
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO audit_record (id, kind, payload_json, parent_hash, self_hash, created_at, principal_id) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(bytes_other.as_bytes().as_slice())
    .bind("tenant.provisioned")
    .bind("{}")
    .bind(vec![0u8; 32])
    .bind(vec![0u8; 32])
    .bind(101_i64)
    .bind("human:stranger@example.com")
    .execute(store.pool())
    .await
    .unwrap();

    let bundle = gather_for_user(&store, user.id, Utc.timestamp_opt(200, 0).unwrap())
        .await
        .unwrap();
    let audit = bundle
        .files
        .iter()
        .find(|f| f.path == "audit/chain.jsonl")
        .unwrap();
    let lines: Vec<&str> = audit.contents.lines().collect();
    assert_eq!(lines.len(), 1, "exactly one row scoped to this user");
    assert!(lines[0].contains(&bytes_self.to_string()));
    assert!(!audit.contents.contains(&bytes_other.to_string()));
}

#[tokio::test]
async fn missing_user_surfaces_schema_error() {
    let store = fresh_store().await;
    let ghost = UserId::new();
    let err = gather_for_user(&store, ghost, Utc::now()).await.unwrap_err();
    assert!(format!("{err}").contains("not found"));
}

#[tokio::test]
async fn manifest_records_known_limitations() {
    let store = fresh_store().await;
    let user = UserAccount::new("manifest@acme.com".into());
    insert_user(&store, &user).await.unwrap();
    let bundle = gather_for_user(&store, user.id, Utc::now()).await.unwrap();
    let manifest = bundle
        .files
        .iter()
        .find(|f| f.path == "manifest.json")
        .unwrap();
    // Every known limitation entry's path must appear; the manifest
    // is the contract for the customer about what's deliberately
    // empty in v1.
    for (path, _reason) in KNOWN_LIMITATIONS {
        assert!(
            manifest.contents.contains(path),
            "manifest missing limitation entry for {path}"
        );
    }
    assert!(manifest.contents.contains("GDPR Article 15"));
}
