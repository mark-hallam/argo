//! Schema migration v40 — reserved tombstone (no-op).
//!
//! v40 is intentionally a no-op safety margin between the v34..v39
//! cluster (the final raw-SQL reconciliation slot, mission
//! `migration-reconciliation-v28-37`) and any future raw SQL that
//! lands while the swarm has not yet absorbed it. Reserving v40
//! prevents an accidental future collision: a developer who adds a
//! one-shot raw migration with a `30__` prefix can still be mapped
//! into `v41+` without rewriting v34..v39 — the schema_version
//! ladder stays monotonically extensible.
//!
//! Rationale: the closing brief promised that the v12..v40 ladder
//! is the SOLE source of schema truth post-reconciliation. Leaving
//! v40 empty documents that promise in-tree.
use argo_types::ArgoError;
use sqlx::Sqlite;

pub(super) async fn up<'a, 'b>(
    _tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    _tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    Ok(())
}
