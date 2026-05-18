//! Cross-tenant SQLCipher isolation (PR 4 of mission-sqlcipher-*).
//!
//! Two tenants with distinct keys must produce two DBs that cannot be
//! read with each other's key. Shared-key-across-tenants would be a
//! cross-tenant data-confidentiality regression
//! (`mission-sqlcipher-4-default-flip` hidden constraints §
//! "Per-tenant keys are non-negotiable").
//!
//! Feature-gated on `bundled-sqlcipher`: without a SQLCipher-linked
//! libsqlite3 the PRAGMA-key calls are silent no-ops and both stores
//! would round-trip as plaintext.

#![cfg(feature = "bundled-sqlcipher")]

use argo_store::wal_tuning::WalTuningProfile;
use argo_store::SessionStore;

const TENANT_A_HEX: &str =
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const TENANT_B_HEX: &str =
    "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

#[tokio::test]
async fn tenant_b_key_cannot_open_tenant_a_db() {
    // The wrong-key branch of this test exercises the pool's retry
    // loop — after_connect rejects every attempted connection so the
    // failure surfaces as "pool timed out". The 30-second default
    // acquire-timeout is fine in production but slow in CI, so we
    // shrink it for this test only. SAFETY: the std::env::set_var is
    // intentionally non-mutex-protected — the value is the same for
    // every test in the suite and we want it set as early as possible
    // before `StorePoolConfig::from_env` is reads it inside
    // `open_with_hex_key`.
    if std::env::var_os("ARGO_STORE_POOL_ACQUIRE_TIMEOUT_MS").is_none() {
        std::env::set_var("ARGO_STORE_POOL_ACQUIRE_TIMEOUT_MS", "2000");
    }
    let tmp = tempfile::tempdir().expect("tempdir");
    let db_a = tmp.path().join("tenant_a.db");

    // Tenant A writes a marker with their key.
    let a_store =
        SessionStore::open_with_hex_key(&db_a, WalTuningProfile::Default, TENANT_A_HEX)
            .await
            .expect("tenant A open");
    a_store.migrate().await.expect("tenant A migrate");
    sqlx::query("CREATE TABLE tenant_a_secret (note TEXT)")
        .execute(a_store.pool())
        .await
        .expect("tenant A create");
    sqlx::query("INSERT INTO tenant_a_secret (note) VALUES ('A-only')")
        .execute(a_store.pool())
        .await
        .expect("tenant A insert");
    a_store.close().await;

    // Tenant B tries to open A's DB with B's key — must fail.
    let attempt =
        SessionStore::open_with_hex_key(&db_a, WalTuningProfile::Default, TENANT_B_HEX).await;
    let err_text = match attempt {
        Ok(store) => match sqlx::query_scalar::<_, String>("SELECT note FROM tenant_a_secret")
            .fetch_optional(store.pool())
            .await
        {
            Ok(Some(row)) => panic!(
                "tenant B key decrypted tenant A's DB — cross-tenant isolation broken \
                 (read row: {row})"
            ),
            Ok(None) => panic!(
                "tenant B key opened tenant A's DB and silently saw no rows — \
                 cross-tenant isolation broken (would expose schema enumeration)"
            ),
            Err(e) => format!("{e}"),
        },
        Err(e) => format!("{e}"),
    };
    let lowered = err_text.to_ascii_lowercase();
    // SQLCipher mismatch surfaces in two shapes depending on whether
    // the pool resolves the bad-key failure during `connect_with`
    // (direct "file is not a database") or during the after-connect
    // hook (pool timeout once the retry budget is exhausted, because
    // the hook rejected every attempted connection). Both count as
    // "tenant B cannot read tenant A's DB."
    assert!(
        lowered.contains("not a database")
            || lowered.contains("encrypted")
            || lowered.contains("malformed")
            || lowered.contains("pool timed out"),
        "expected SQLCipher key-mismatch error, got: {err_text}",
    );

    // Tenant A's own key still works.
    let a_again =
        SessionStore::open_with_hex_key(&db_a, WalTuningProfile::Default, TENANT_A_HEX)
            .await
            .expect("tenant A re-open");
    let v: String = sqlx::query_scalar("SELECT note FROM tenant_a_secret")
        .fetch_one(a_again.pool())
        .await
        .expect("tenant A read");
    assert_eq!(v, "A-only");
}
