//! Unit tests for [`super`] — covers the brief's acceptance: revoke/
//! is_revoked round-trip, double-revoke idempotency, cross-tenant
//! isolation in `list_for_tenant`, schema-bootstrap idempotency.

use super::{ensure_schema, is_revoked, list_for_tenant, revoke};
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::SqlitePool;

async fn pool() -> SqlitePool {
    SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("memory pool")
}

#[tokio::test]
async fn ensure_schema_is_idempotent() {
    let p = pool().await;
    ensure_schema(&p).await.expect("first call");
    ensure_schema(&p).await.expect("second call");
}

#[tokio::test]
async fn revoke_then_is_revoked_round_trip() {
    let p = pool().await;
    assert!(!is_revoked(&p, "jti-1").await.unwrap(), "jti not yet inserted");
    revoke(&p, "jti-1", "tenant-a", Some("user-1"), Some("logout"), 1_700)
        .await
        .unwrap();
    assert!(is_revoked(&p, "jti-1").await.unwrap(), "jti revoked");
    assert!(
        !is_revoked(&p, "jti-other").await.unwrap(),
        "other jti unaffected"
    );
}

#[tokio::test]
async fn double_revoke_is_idempotent() {
    let p = pool().await;
    revoke(&p, "jti-dup", "tenant-a", Some("user-1"), Some("logout"), 1_000)
        .await
        .unwrap();
    // Second revoke with different metadata — should NOT overwrite the
    // original row; `revoked_at` stays at 1_000 per ON CONFLICT DO NOTHING.
    revoke(&p, "jti-dup", "tenant-a", Some("user-2"), Some("admin"), 2_000)
        .await
        .unwrap();
    let rows = list_for_tenant(&p, "tenant-a").await.unwrap();
    assert_eq!(rows.len(), 1, "no duplicate row inserted");
    assert_eq!(rows[0].revoked_at, 1_000, "original revoked_at preserved");
    assert_eq!(rows[0].principal.as_deref(), Some("user-1"));
    assert_eq!(rows[0].reason.as_deref(), Some("logout"));
}

#[tokio::test]
async fn list_for_tenant_is_cross_tenant_isolated() {
    let p = pool().await;
    revoke(&p, "jti-a-1", "tenant-a", None, Some("logout"), 100)
        .await
        .unwrap();
    revoke(&p, "jti-a-2", "tenant-a", None, Some("logout"), 200)
        .await
        .unwrap();
    revoke(&p, "jti-b-1", "tenant-b", None, Some("logout"), 300)
        .await
        .unwrap();
    let a = list_for_tenant(&p, "tenant-a").await.unwrap();
    let b = list_for_tenant(&p, "tenant-b").await.unwrap();
    assert_eq!(a.len(), 2, "tenant-a sees its 2 rows");
    assert_eq!(b.len(), 1, "tenant-b sees its 1 row");
    assert!(a.iter().all(|r| r.tenant_id == "tenant-a"));
    assert!(b.iter().all(|r| r.tenant_id == "tenant-b"));
    // ORDER BY revoked_at DESC — newer first.
    assert_eq!(a[0].jti, "jti-a-2");
    assert_eq!(a[1].jti, "jti-a-1");
}

#[tokio::test]
async fn is_revoked_does_not_leak_across_tenants() {
    let p = pool().await;
    // `is_revoked` is keyed by `jti` only — the load-bearing assertion
    // is that an unrelated tenant's `jti` set does NOT influence the
    // lookup for the current tenant's `jti`. (The middleware caller
    // separately enforces tenant binding by reading `claims.tnt`.)
    revoke(&p, "jti-tenant-b", "tenant-b", None, None, 500)
        .await
        .unwrap();
    assert!(
        !is_revoked(&p, "jti-tenant-a-unrevoked").await.unwrap(),
        "tenant-a's distinct jti is not revoked"
    );
    assert!(
        is_revoked(&p, "jti-tenant-b").await.unwrap(),
        "tenant-b's jti is still queryable by jti alone"
    );
}
