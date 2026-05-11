//! Schema migration v48 — `persona_revision` table for Stream
//! `mission-persona-evolution-audit-revert` (MISSION_BACKLOG
//! Section 3 Claim 1 #46).
//!
//! Append-only revision log: one row per persona field touched, with
//! a sha256 hash chain over (`prev_hash`, `field`, `rev_id`,
//! `new_value`) for tamper-evidence. The `argo persona history` /
//! `argo persona revert` CLI surfaces are powered by
//! `crates/argo-store/src/persona_revision.rs`.
//!
//! Slot rationale: v46 (`mvp-worker-trust-grade-column`) and v47
//! (`mvp-worker-skills-table` PR #1272) occupied the prior versioned
//! slots when this brief rebased onto current `origin/main`, so v48
//! is the next free versioned slot. Reconciles raw SQL
//! `52__persona_revision.sql`: filename slots 49
//! (`worker_archived_at`), 50 (`worker_trust_grade`), and 51
//! (`worker_skill` + `audit_worker_deleted_at`) are taken, so the
//! raw counter rolls forward to 52 per the brief's "verify slot ≥
//! v52" directive.
//!
//! UP creates the table + a `(worker_id, revised_at DESC)` index
//! that backs the newest-first `list_persona_revisions` query.
//! DOWN drops both in reverse order. Idempotent via
//! `IF NOT EXISTS` / `IF EXISTS` guards.
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    for sql in V48_UP {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v48 up: {e}")))?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    for sql in V48_DOWN {
        sqlx::query(sql)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v48 down: {e}")))?;
    }
    Ok(())
}

const V48_UP: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS persona_revision (
        rev_id            TEXT PRIMARY KEY,
        worker_id         TEXT NOT NULL,
        revised_at        INTEGER NOT NULL,
        field             TEXT NOT NULL,
        old_value         TEXT NOT NULL,
        new_value         TEXT NOT NULL,
        actor             TEXT NOT NULL,
        audit_chain_hash  TEXT NOT NULL
    )",
    "CREATE INDEX IF NOT EXISTS idx_persona_revision_worker_revised_at \
     ON persona_revision(worker_id, revised_at DESC)",
];

const V48_DOWN: &[&str] = &[
    "DROP INDEX IF EXISTS idx_persona_revision_worker_revised_at",
    "DROP TABLE IF EXISTS persona_revision",
];
