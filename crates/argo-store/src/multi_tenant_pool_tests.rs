use super::*;

async fn registry() -> (tempfile::TempDir, MultiTenantStore) {
    let dir = tempfile::tempdir().unwrap();
    let default = SessionStore::open(dir.path().join("default.db"))
        .await
        .unwrap();
    let cfgs = vec![
        TenantPoolConfig {
            tenant_id: "eu-a".into(),
            db_url: dir.path().join("eu-a.db").display().to_string(),
            region: "eu-west-1".into(),
        },
        TenantPoolConfig {
            tenant_id: "us-a".into(),
            db_url: dir.path().join("us-a.db").display().to_string(),
            region: "us-east-1".into(),
        },
    ];
    let store = MultiTenantStore::open(default, cfgs).await.unwrap();
    (dir, store)
}

#[tokio::test]
async fn resolves_configured_tenant_without_default() {
    let (_dir, store) = registry().await;
    let resolved = store.resolve("eu-a");
    assert!(!resolved.used_default);
    assert_eq!(resolved.region.as_deref(), Some("eu-west-1"));
}

#[tokio::test]
async fn unknown_tenant_falls_back_to_default() {
    let (_dir, store) = registry().await;
    let resolved = store.resolve("missing");
    assert!(resolved.used_default);
    assert_eq!(resolved.region, None);
}

#[tokio::test]
async fn migrate_tenant_is_idempotent() {
    let (_dir, store) = registry().await;
    let first = store.migrate_tenant("eu-a").await.unwrap();
    let second = store.migrate_tenant("eu-a").await.unwrap();
    assert_eq!(first, second);
    assert!(first > 0);
}

#[tokio::test]
async fn tenant_status_reports_schema_version() {
    let (_dir, store) = registry().await;
    store.migrate_tenant("eu-a").await.unwrap();
    let rows = store.tenant_statuses().await;
    let eu = rows.iter().find(|r| r.tenant_id == "eu-a").unwrap();
    assert!(eu.healthy);
    assert!(eu.schema_version.unwrap() > 0);
}

#[tokio::test]
async fn configured_tenant_dbs_are_distinct() {
    let (_dir, store) = registry().await;
    let eu = store.resolve("eu-a");
    let us = store.resolve("us-a");
    eu.store.migrate().await.unwrap();
    us.store.migrate().await.unwrap();
    sqlx::query("CREATE TABLE marker_eu (id INTEGER)")
        .execute(eu.store.pool())
        .await
        .unwrap();
    let exists: Option<String> =
        sqlx::query_scalar("SELECT name FROM sqlite_master WHERE name = 'marker_eu'")
            .fetch_optional(us.store.pool())
            .await
            .unwrap();
    assert_eq!(exists, None);
}

#[tokio::test]
async fn rejects_empty_db_url() {
    let dir = tempfile::tempdir().unwrap();
    let default = SessionStore::open(dir.path().join("default.db"))
        .await
        .unwrap();
    let result = MultiTenantStore::open(
        default,
        vec![TenantPoolConfig {
            tenant_id: "eu-a".into(),
            db_url: String::new(),
            region: "eu-west-1".into(),
        }],
    )
    .await;
    let err = match result {
        Ok(_) => panic!("empty db_url should fail"),
        Err(e) => e,
    };
    assert!(err.to_string().contains("db_url is empty"));
}

#[tokio::test]
async fn has_tenant_tracks_configured_entries() {
    let (_dir, store) = registry().await;
    assert!(store.has_tenant("eu-a"));
    assert!(!store.has_tenant("missing"));
}
