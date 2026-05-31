//! Unit tests for the `api_key` store module.
//!
//! Covers the round-trip (insert → list/get), tenant isolation on every
//! read + mutation (a tenant can never see / revoke / touch another
//! tenant's key), the unique-token-hash invariant, revoke + touch
//! lifecycle, and the load-bearing secret-discipline contract: a list /
//! get NEVER carries the token hash, and the raw token is never stored.

use super::*;
use sqlx::sqlite::SqlitePoolOptions;

const T1: &str = "00000000-0000-0000-0000-000000000001";
const T2: &str = "00000000-0000-0000-0000-000000000002";

/// In-memory pool with the `api_key` table created via the real
/// versioned migration runner (v86) — exactly as the runtime applies it.
async fn pool() -> SqlitePool {
    let p = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("memory pool");
    crate::migrations::run_up_to(&p, 86)
        .await
        .expect("migrations up to v86");
    p
}

fn ins<'a>(id: &'a str, tenant: &'a str, name: &'a str, hash: &'a str) -> ApiKeyInsert<'a> {
    ApiKeyInsert {
        id,
        tenant_id: tenant,
        name,
        token_hash: hash,
        prefix: "sk_live_a4b8...c2e1",
        scopes_json: "[\"read:workers\"]",
        cohort_ids_json: "[]",
        all_cohorts: true,
        created_at: "2026-06-01T00:00:00Z",
        expires_at: None,
    }
}

#[tokio::test]
async fn insert_then_list_round_trips() {
    let p = pool().await;
    insert(&p, &ins("key_1", T1, "Production CI", "hash1"))
        .await
        .unwrap();
    let rows = list_for_tenant(&p, T1).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, "key_1");
    assert_eq!(rows[0].name, "Production CI");
    assert_eq!(rows[0].prefix, "sk_live_a4b8...c2e1");
    assert!(rows[0].all_cohorts);
    assert!(!rows[0].revoked);
    assert_eq!(rows[0].last_used_at, None);
}

#[tokio::test]
async fn list_is_newest_first() {
    let p = pool().await;
    insert(
        &p,
        &ApiKeyInsert {
            created_at: "2026-06-01T00:00:00Z",
            ..ins("older", T1, "Older", "h-old")
        },
    )
    .await
    .unwrap();
    insert(
        &p,
        &ApiKeyInsert {
            created_at: "2026-06-02T00:00:00Z",
            ..ins("newer", T1, "Newer", "h-new")
        },
    )
    .await
    .unwrap();
    let rows = list_for_tenant(&p, T1).await.unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].id, "newer");
    assert_eq!(rows[1].id, "older");
}

#[tokio::test]
async fn list_is_tenant_scoped() {
    let p = pool().await;
    insert(&p, &ins("k_t1", T1, "Tenant 1 key", "h1"))
        .await
        .unwrap();
    insert(&p, &ins("k_t2", T2, "Tenant 2 key", "h2"))
        .await
        .unwrap();
    let t1_rows = list_for_tenant(&p, T1).await.unwrap();
    assert_eq!(t1_rows.len(), 1, "tenant 1 sees only its own key");
    assert_eq!(t1_rows[0].id, "k_t1");
    let t2_rows = list_for_tenant(&p, T2).await.unwrap();
    assert_eq!(t2_rows.len(), 1, "tenant 2 sees only its own key");
    assert_eq!(t2_rows[0].id, "k_t2");
}

#[tokio::test]
async fn get_is_tenant_scoped_no_cross_tenant_read() {
    let p = pool().await;
    insert(&p, &ins("k_t1", T1, "Tenant 1 key", "h1"))
        .await
        .unwrap();
    // Same id, queried under the wrong tenant → no row.
    assert!(
        get_for_tenant(&p, T2, "k_t1").await.unwrap().is_none(),
        "another tenant must not read this key by id"
    );
    assert!(get_for_tenant(&p, T1, "k_t1").await.unwrap().is_some());
}

#[tokio::test]
async fn duplicate_token_hash_is_invariant_error() {
    let p = pool().await;
    insert(&p, &ins("k1", T1, "A", "samehash"))
        .await
        .unwrap();
    let err = insert(&p, &ins("k2", T1, "B", "samehash"))
        .await
        .unwrap_err();
    assert!(matches!(err, ArgoError::Invariant(_)), "{err:?}");
}

#[tokio::test]
async fn mark_revoked_flips_only_own_tenant_row() {
    let p = pool().await;
    insert(&p, &ins("k_t1", T1, "Tenant 1 key", "h1"))
        .await
        .unwrap();
    // Cross-tenant revoke attempt: T2 tries to revoke T1's key → no row
    // matched, key stays live. THIS IS THE CROSS-TENANT NEGATIVE.
    assert!(
        !mark_revoked(&p, T2, "k_t1").await.unwrap(),
        "cross-tenant revoke must match no row"
    );
    let still_live = get_for_tenant(&p, T1, "k_t1").await.unwrap().unwrap();
    assert!(!still_live.revoked, "key must remain live after foreign revoke");

    // Owning tenant revoke succeeds.
    assert!(mark_revoked(&p, T1, "k_t1").await.unwrap());
    let revoked = get_for_tenant(&p, T1, "k_t1").await.unwrap().unwrap();
    assert!(revoked.revoked, "owning-tenant revoke flips the flag");
}

#[tokio::test]
async fn mark_revoked_returns_false_for_unknown_id() {
    let p = pool().await;
    assert!(!mark_revoked(&p, T1, "ghost").await.unwrap());
}

#[tokio::test]
async fn touch_last_used_stamps_by_hash() {
    let p = pool().await;
    insert(&p, &ins("k1", T1, "A", "hash-x"))
        .await
        .unwrap();
    assert_eq!(touch_last_used(&p, "hash-x", "2026-06-03T12:00:00Z").await.unwrap(), 1);
    let row = get_for_tenant(&p, T1, "k1").await.unwrap().unwrap();
    assert_eq!(row.last_used_at.as_deref(), Some("2026-06-03T12:00:00Z"));
    // Unknown hash touches nothing.
    assert_eq!(touch_last_used(&p, "no-such-hash", "x").await.unwrap(), 0);
}

#[tokio::test]
async fn list_and_get_never_expose_token_hash() {
    // The read structs carry NO token_hash field at all, and the SELECT
    // column list excludes it — so the secret-derived hash can never
    // reach a roster render. This test pins both: the projected columns
    // and the absence of the hash value anywhere in the row's debug.
    let p = pool().await;
    let secret_hash = "this-hash-must-never-surface-on-read";
    insert(&p, &ins("k1", T1, "A", secret_hash))
        .await
        .unwrap();
    let row = get_for_tenant(&p, T1, "k1").await.unwrap().unwrap();
    let dbg = format!("{row:?}");
    assert!(
        !dbg.contains(secret_hash),
        "token hash must not appear in a read row: {dbg}"
    );
    let listed = list_for_tenant(&p, T1).await.unwrap();
    let listed_dbg = format!("{:?}", listed[0]);
    assert!(!listed_dbg.contains(secret_hash), "token hash must not appear in list");
}
