//! v22-era populated DB → forward-migrate to tip integration test
//! (mission-migration-upgrade-test).
//!
//! Mirrors what an operator upgrading an in-the-wild deployment sees:
//! a SQLite file whose `schema_version` is some intermediate version,
//! populated with real data, opened by a newer binary that walks the
//! forward migrations. Assertions cover row preservation, default
//! application for columns added after v22, audit-chain integrity, and
//! idempotency of a second `migrate()` call.
//!
//! The v22 baseline is the post-reconciliation stable point referenced
//! in MISSION_BACKLOG Section 2 #28; rebase to a later baseline once a
//! v1.0 schema is declared.

use argo_store::{SessionStore, EXPECTED_SCHEMA_VERSION};
use sha2::{Digest, Sha256};
use sqlx::Row;
use std::path::Path;
use uuid::Uuid;

const V22_BASELINE: u32 = 22;
const SEED_AUDIT_ROWS: usize = 50;

#[tokio::test]
async fn v22_populated_db_upgrades_to_tip_preserving_data() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("upgrade.db");

    // Phase 1 — fabricate a v22-era populated DB on disk.
    let chain_head = build_v22_populated_db(&db_path).await;

    // Phase 2 — open with the "newer" binary and let migrate() walk
    // every forward migration from v22 to tip.
    let store = SessionStore::open(&db_path).await.expect("reopen");
    store.migrate().await.expect("forward migrate");
    assert_eq!(
        store.schema_version().await.expect("schema_version"),
        EXPECTED_SCHEMA_VERSION,
        "post-upgrade schema_version != EXPECTED_SCHEMA_VERSION",
    );

    // Phase 3 — seeded row counts preserved across the upgrade.
    assert_count(&store, "workspace", 3).await;
    assert_count(&store, "worker", 5).await;
    assert_count(&store, "session", 20).await;
    assert_count(&store, "skill", 10).await;
    assert_count(&store, "memory_entry", 30).await;
    assert_count(&store, "audit_record", SEED_AUDIT_ROWS as i64).await;

    // Phase 4 — columns added in v45 / v46 acquired their declared
    // defaults on rows that pre-date them.
    assert_worker_default_columns(&store).await;

    // Phase 5 — tables added after v22 exist and are queryable.
    for table in [
        "roster",            // v23
        "approval_request",  // v44
        "worker_skill",      // v47
        "persona_revision",  // v48
    ] {
        assert!(
            table_exists(&store, table).await,
            "table {table} missing after upgrade",
        );
    }

    // Phase 6 — audit chain integrity unaffected by additive column
    // changes (v42 added `principal_id`, which is NULLable and does
    // not feed into the chain hash).
    assert_audit_chain_intact(&store, chain_head).await;

    // Phase 7 — second migrate() is a no-op; row counts and head
    // schema_version unchanged.
    store.migrate().await.expect("idempotent migrate");
    assert_eq!(
        store.schema_version().await.expect("schema_version idem"),
        EXPECTED_SCHEMA_VERSION,
    );
    assert_count(&store, "audit_record", SEED_AUDIT_ROWS as i64).await;
    assert_count(&store, "worker", 5).await;

    store.close().await;
}

/// Build a v22-populated DB at `path` and return the final
/// `self_hash` of the seeded audit chain so the post-upgrade verifier
/// can confirm the head row survived.
async fn build_v22_populated_db(path: &Path) -> [u8; 32] {
    let store = SessionStore::open(path).await.expect("open for build");
    // Walk forward then roll back: the runner has no public
    // "apply up to version" helper, and the deny(unwrap_used) gate on
    // production code keeps the test-only `run_up_to` private. The
    // down migrations from v23..v48 are all clean drops/ALTERs (no
    // data loss for rows that don't yet exist), so this leaves the
    // schema at exactly the v22 shape before any seed rows are
    // written.
    store.migrate().await.expect("initial migrate to tip");
    store
        .migrate_down_to(V22_BASELINE)
        .await
        .expect("roll back to v22");
    assert_eq!(
        store.schema_version().await.expect("v22 version"),
        V22_BASELINE,
    );

    apply_seed_sql(&store).await;
    let head = seed_audit_chain(&store, SEED_AUDIT_ROWS).await;

    store.close().await;
    head
}

async fn apply_seed_sql(store: &SessionStore) {
    let sql = include_str!("fixtures/v22_seed.sql");
    for stmt in split_sql_statements(sql) {
        sqlx::query(&stmt)
            .execute(store.pool())
            .await
            .unwrap_or_else(|e| panic!("seed stmt failed:\n{stmt}\n=> {e}"));
    }
}

/// Insert `count` audit_record rows attached to the first seeded
/// session, with parent/self-hash linkage that matches the algorithm
/// in `argo_audit::sink::chain_hash`. Returns the final `self_hash`.
async fn seed_audit_chain(store: &SessionStore, count: usize) -> [u8; 32] {
    let session_id = first_session_id(store).await;
    let mut parent = [0u8; 32];
    for i in 0..count {
        let kind = "turn_start";
        let payload = format!("{{\"turn\":{i}}}");
        let self_hash = chain_hash(&parent, kind, &payload);
        let id = Uuid::now_v7();
        sqlx::query(
            "INSERT INTO audit_record \
             (id, session_id, kind, payload_json, parent_hash, self_hash, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id.as_bytes().as_slice())
        .bind(&session_id)
        .bind(kind)
        .bind(&payload)
        .bind(parent.as_slice())
        .bind(self_hash.as_slice())
        .bind(1_700_000_700_i64 + i as i64)
        .execute(store.pool())
        .await
        .expect("audit insert");
        parent = self_hash;
    }
    parent
}

async fn first_session_id(store: &SessionStore) -> Vec<u8> {
    sqlx::query("SELECT id FROM session ORDER BY started_at ASC LIMIT 1")
        .fetch_one(store.pool())
        .await
        .expect("first session row")
        .try_get::<Vec<u8>, _>("id")
        .expect("session.id blob")
}

/// SHA-256 hash matching `argo_audit::sink::chain_hash`:
/// `sha256(parent || kind || 0x00 || payload)`.
fn chain_hash(parent: &[u8; 32], kind: &str, payload: &str) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(parent);
    h.update(kind.as_bytes());
    h.update(b"\0");
    h.update(payload.as_bytes());
    let out = h.finalize();
    let mut buf = [0u8; 32];
    buf.copy_from_slice(&out);
    buf
}

async fn assert_count(store: &SessionStore, table: &str, expected: i64) {
    let actual: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
        .fetch_one(store.pool())
        .await
        .unwrap_or_else(|e| panic!("count({table}) failed: {e}"));
    assert_eq!(actual, expected, "row count mismatch for {table}");
}

async fn table_exists(store: &SessionStore, table: &str) -> bool {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type IN ('table','view') AND name = ?",
    )
    .bind(table)
    .fetch_optional(store.pool())
    .await
    .expect("sqlite_master probe");
    row.is_some()
}

/// v45 added `worker.archived_at` (nullable TEXT). v46 added
/// `worker.trust_grade` (NOT NULL DEFAULT 0) and `worker.last_used_at`
/// (nullable TEXT). The 5 worker rows seeded at v22 must come back with
/// `trust_grade = 0` and both timestamp columns NULL.
async fn assert_worker_default_columns(store: &SessionStore) {
    let rows = sqlx::query("SELECT id, trust_grade, last_used_at, archived_at FROM worker")
        .fetch_all(store.pool())
        .await
        .expect("worker default probe");
    assert_eq!(rows.len(), 5, "worker row count for default probe");
    for row in rows {
        let id: String = row.try_get("id").expect("worker.id");
        let trust_grade: i64 = row.try_get("trust_grade").expect("worker.trust_grade");
        let last_used_at: Option<String> =
            row.try_get("last_used_at").expect("worker.last_used_at");
        let archived_at: Option<String> = row.try_get("archived_at").expect("worker.archived_at");
        assert_eq!(trust_grade, 0, "trust_grade default for {id}");
        assert!(
            last_used_at.is_none(),
            "last_used_at should be NULL for v22 row {id}",
        );
        assert!(
            archived_at.is_none(),
            "archived_at should be NULL for v22 row {id}",
        );
    }
}

/// Replays the chain-hash verification that `argo_audit::verify_all`
/// performs in-process: every row's `self_hash` must match the
/// recomputed hash, and consecutive rows must link via
/// `parent_hash == prev.self_hash`. The final row's `self_hash` must
/// equal `expected_head` — proves the chain survived the upgrade
/// end-to-end, not just that it re-verifies among itself.
async fn assert_audit_chain_intact(store: &SessionStore, expected_head: [u8; 32]) {
    let rows = sqlx::query(
        "SELECT seq, kind, payload_json, parent_hash, self_hash \
         FROM audit_record ORDER BY seq ASC",
    )
    .fetch_all(store.pool())
    .await
    .expect("audit_record fetch");
    assert_eq!(rows.len(), SEED_AUDIT_ROWS, "audit row count");

    let mut prev: Option<[u8; 32]> = None;
    let mut last_self: [u8; 32] = [0u8; 32];
    for row in rows {
        let kind: String = row.try_get("kind").expect("audit.kind");
        let payload: String = row.try_get("payload_json").expect("audit.payload_json");
        let parent_blob: Vec<u8> = row.try_get("parent_hash").expect("audit.parent_hash");
        let self_blob: Vec<u8> = row.try_get("self_hash").expect("audit.self_hash");
        assert_eq!(parent_blob.len(), 32, "parent_hash length");
        assert_eq!(self_blob.len(), 32, "self_hash length");
        let parent: [u8; 32] = parent_blob.try_into().expect("parent_hash to [u8;32]");
        let self_h: [u8; 32] = self_blob.try_into().expect("self_hash to [u8;32]");
        if let Some(prev_h) = prev {
            assert_eq!(
                prev_h, parent,
                "chain link broken: parent_hash != previous self_hash"
            );
        }
        let recomputed = chain_hash(&parent, &kind, &payload);
        assert_eq!(
            recomputed, self_h,
            "self_hash != recomputed chain hash (corruption)",
        );
        prev = Some(self_h);
        last_self = self_h;
    }
    assert_eq!(
        last_self, expected_head,
        "chain head changed across upgrade",
    );
}

/// Split a hand-written seed SQL blob into individual statements.
/// `--` line comments are stripped first so a stray `;` inside a
/// comment cannot fragment a statement; the fixture is hand-authored
/// so we know `;` never appears inside a string literal.
fn split_sql_statements(sql: &str) -> Vec<String> {
    let stripped: String = sql
        .lines()
        .map(|line| match line.find("--") {
            Some(idx) => &line[..idx],
            None => line,
        })
        .collect::<Vec<_>>()
        .join("\n");
    stripped
        .split(';')
        .map(|frag| frag.trim().to_owned())
        .filter(|frag| !frag.is_empty())
        .collect()
}

/// Compile-time sanity check that the seed fixture is present and
/// non-trivial. Catches the "fixture rename / cargo include miss"
/// failure mode early.
#[test]
fn seed_fixture_loads() {
    let sql = include_str!("fixtures/v22_seed.sql");
    assert!(sql.len() > 500, "seed fixture suspiciously small");
    assert!(sql.contains("INSERT INTO workspace"));
    assert!(sql.contains("INSERT INTO worker"));
    assert!(sql.contains("INSERT INTO session"));
    assert!(sql.contains("INSERT INTO skill"));
    assert!(sql.contains("INSERT INTO memory_entry"));
}
