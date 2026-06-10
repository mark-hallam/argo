//! Concurrency regression for the audit hash chain
//! (`fix-audit-chain-append-serialization`).
//!
//! N parallel appends through BOTH the canonical sink writer and a
//! formerly-side-door raw `BEGIN IMMEDIATE` append must collapse into
//! ONE linear chain: `verify_all` stays clean and no two rows share a
//! `parent_hash` (a fork is exactly "two rows with the same parent").
//!
//! The test runs against a FILE-backed store so the 8-connection WAL
//! pool exercises real multi-connection contention — a `:memory:` pool
//! serializes on a single shared connection and would hide the race
//! this test exists to catch.
use crate::{compute_chain_hash, verify_all, AuditKind, AuditSink, DEFAULT_AUDIT_TENANT_ID};
use argo_store::SessionStore;
use std::sync::Arc;

const KIND: &str = AuditKind::HttpRequest.label();

/// A formerly-side-door append: read the global chain tail and insert,
/// both inside one `BEGIN IMMEDIATE` transaction (the shape every
/// converted call site now uses).
async fn side_door_append(store: &SessionStore, i: usize) {
    let payload = serde_json::json!({ "path": "/side", "i": i });
    let canonical = serde_json::to_string(&payload).unwrap();
    let mut tx = store.pool().begin_with("BEGIN IMMEDIATE").await.unwrap();
    let parent: [u8; 32] = {
        let bytes: Option<Vec<u8>> = sqlx::query_scalar(
            "SELECT self_hash FROM audit_record WHERE session_id IS NULL ORDER BY seq DESC LIMIT 1",
        )
        .fetch_optional(&mut *tx)
        .await
        .unwrap();
        let mut out = [0u8; 32];
        if let Some(b) = bytes {
            if b.len() == 32 {
                out.copy_from_slice(&b);
            }
        }
        out
    };
    let self_hash = compute_chain_hash(&parent, KIND, &canonical);
    sqlx::query(
        "INSERT INTO audit_record (id, session_id, kind, payload_json, parent_hash, self_hash, created_at) \
         VALUES (?, NULL, ?, ?, ?, ?, ?)",
    )
    .bind(uuid::Uuid::now_v7().as_bytes().as_slice())
    .bind(KIND)
    .bind(canonical)
    .bind(parent.as_slice())
    .bind(self_hash.as_slice())
    .bind(chrono::Utc::now().timestamp_micros())
    .execute(&mut *tx)
    .await
    .unwrap();
    tx.commit().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_mixed_appends_keep_chain_linear() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("audit-concurrency.db");
    let store = Arc::new(
        SessionStore::open_unencrypted_for_tests(&path)
            .await
            .unwrap(),
    );
    store.migrate().await.unwrap();
    let sink = Arc::new(AuditSink::new_with_tenant(
        Arc::clone(&store),
        DEFAULT_AUDIT_TENANT_ID.to_string(),
    ));

    const N: usize = 24;
    let mut handles = Vec::with_capacity(N);
    for i in 0..N {
        if i % 2 == 0 {
            // Canonical path (BatchedWriter -> flush_audit).
            let sink = Arc::clone(&sink);
            handles.push(tokio::spawn(async move {
                sink.write(
                    None,
                    AuditKind::HttpRequest,
                    serde_json::json!({ "path": "/canonical", "i": i }),
                    None,
                )
                .await
                .unwrap();
            }));
        } else {
            // Formerly side-door path (now BEGIN IMMEDIATE).
            let store = Arc::clone(&store);
            handles.push(tokio::spawn(async move {
                side_door_append(&store, i).await;
            }));
        }
    }
    for h in handles {
        h.await.unwrap();
    }
    // Drain any batched canonical writes still in flight.
    sink.flush_pending().await.unwrap();

    // 1) Every append landed (BEGIN IMMEDIATE + busy_timeout makes the
    //    contended writers wait, not fail).
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM audit_record")
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(count, N as i64, "all {N} appends should land");

    // 2) Zero forks: in a linear chain every row has a distinct parent
    //    (genesis [0;32] once, then each prior self_hash once). A fork
    //    means two rows share a parent_hash, dropping the distinct count.
    let distinct_parents: i64 =
        sqlx::query_scalar("SELECT COUNT(DISTINCT parent_hash) FROM audit_record")
            .fetch_one(store.pool())
            .await
            .unwrap();
    assert_eq!(
        distinct_parents, N as i64,
        "every row must have a unique parent_hash — a duplicate is a forked chain"
    );

    // 3) The shipped verifier agrees: parents thread + hashes recompute.
    let report = verify_all(&store).await.unwrap();
    assert!(report.ok, "chain must verify clean: {:?}", report.breaks);
    assert_eq!(report.rows, N, "verifier should see every appended row");
}

/// Revert-tripwire for the canonical UNBATCHED fallback (security re-review of
/// #2805). With `ARGO_STORE_BATCHED_WRITES=0` the canonical `AuditSink::write`
/// takes the single-row fallback; that path MUST wrap its read-then-insert in
/// `BEGIN IMMEDIATE`, else concurrent appends fork the chain. If someone
/// reverts the fallback to a bare-pool read-then-insert, `COUNT(DISTINCT
/// parent_hash)` drops below N here and this fails. The shared-env toggle is
/// safe: both append paths are linear now, so it cannot make the batched
/// sibling test above fail — it would only exercise an also-correct path.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_unbatched_canonical_appends_keep_chain_linear() {
    let prev = std::env::var("ARGO_STORE_BATCHED_WRITES").ok();
    std::env::set_var("ARGO_STORE_BATCHED_WRITES", "0");

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("audit-unbatched-concurrency.db");
    let store = Arc::new(
        SessionStore::open_unencrypted_for_tests(&path)
            .await
            .unwrap(),
    );
    store.migrate().await.unwrap();
    let sink = Arc::new(AuditSink::new_with_tenant(
        Arc::clone(&store),
        DEFAULT_AUDIT_TENANT_ID.to_string(),
    ));

    const N: usize = 24;
    let mut handles = Vec::with_capacity(N);
    for i in 0..N {
        let sink = Arc::clone(&sink);
        handles.push(tokio::spawn(async move {
            sink.write(
                None,
                AuditKind::HttpRequest,
                serde_json::json!({ "path": "/unbatched", "i": i }),
                None,
            )
            .await
            .unwrap();
        }));
    }
    for h in handles {
        h.await.unwrap();
    }

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM audit_record")
        .fetch_one(store.pool())
        .await
        .unwrap();
    assert_eq!(count, N as i64, "all {N} unbatched appends should land");
    let distinct_parents: i64 =
        sqlx::query_scalar("SELECT COUNT(DISTINCT parent_hash) FROM audit_record")
            .fetch_one(store.pool())
            .await
            .unwrap();
    assert_eq!(
        distinct_parents, N as i64,
        "unbatched fallback must not fork — every row needs a unique parent_hash"
    );
    let report = verify_all(&store).await.unwrap();
    assert!(report.ok, "chain must verify clean: {:?}", report.breaks);

    match prev {
        Some(v) => std::env::set_var("ARGO_STORE_BATCHED_WRITES", v),
        None => std::env::remove_var("ARGO_STORE_BATCHED_WRITES"),
    }
}
