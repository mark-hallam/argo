//! Schema migration v46 — additive `worker.trust_grade` (INTEGER,
//! default 0) + `worker.last_used_at` (TEXT, nullable) columns for
//! Stream `mvp-worker-trust-grade-column`. Tier M prereq —
//! unblocks mission-routing-skillmatch-rolematch (M4) and
//! mission-skill-propagation-loop.
//!
//! v45 (`worker_archived_at`, sibling brief `mvp-identity-archive-worker`)
//! was the most recent versioned slot when this brief landed; this is
//! the next free slot. Reconciles raw SQL `50__worker_trust_grade.sql`
//! (filename slot 50 picks the next free SQL filename — 47 / 48 / 49
//! are taken by `skill_canary` / `approvals` / `worker_archived_at`
//! respectively, and the two counters move independently per the
//! v42 + v43 docstrings).
//!
//! UP adds both columns + supporting DESC indexes for routing-engine
//! ranking lookups. DOWN drops them in reverse order.
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for sql in V46_UP {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v46 up: {e}")))?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V46_DOWN {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v46 down: {e}")))?;
    }
    Ok(())
}

const V46_UP: &[&str] = &[
    "ALTER TABLE worker ADD COLUMN trust_grade INTEGER NOT NULL DEFAULT 0",
    "ALTER TABLE worker ADD COLUMN last_used_at TEXT",
    "CREATE INDEX IF NOT EXISTS idx_worker_trust_grade \
     ON worker(trust_grade DESC)",
    "CREATE INDEX IF NOT EXISTS idx_worker_last_used_at \
     ON worker(last_used_at DESC)",
];

const V46_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_worker_last_used_at",
    "DROP INDEX IF EXISTS idx_worker_trust_grade",
    "ALTER TABLE worker DROP COLUMN last_used_at",
    "ALTER TABLE worker DROP COLUMN trust_grade",
];
