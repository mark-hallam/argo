//! Schema migration v93 — `target_app` column on `executor_promotion`
//! (DEC-050/DEC-051 follow-through: governed promotion of a minted
//! teammate's action onto the native-macOS computer-use executor).
//!
//! The 2026-06-11 E2E stranger walkthrough confirmed a Genesis-minted
//! teammate has NO path to the computer-use executor: Genesis emits
//! `executor = "human"` for every action (correct draft-only posture,
//! DEC-037) and the v91 promotion table could only bind MCP. This
//! column records the macOS bundle id (e.g. `com.apple.Notes`) a
//! computer-use promotion drives, mirroring the manifest's `target_app`
//! field from cu3 (#2816). NULL for every existing row and for MCP /
//! document promotions — additive, no backfill required.
//!
//! Safety note: honouring such a promotion at dispatch still rides the
//! DEC-051 risk-tiered floor inside the CU run loop — committing
//! operations (click / key / launch) hold for operator approval
//! regardless of what the promotion or manifest claims.
//!
//! UP adds the nullable column (guarded by a column-existence probe so
//! re-running is idempotent). DOWN is a no-op: SQLite cannot drop a
//! column without a table rebuild, and a stray nullable column is
//! harmless — the documented additive-migration trade-off (same posture
//! as the v90 `trust_grade` addendum).
use argo_types::ArgoError;
use sqlx::{Row, Sqlite};

pub(super) async fn up<'a, 'b>(tx: &'a mut sqlx::Transaction<'b, Sqlite>) -> Result<(), ArgoError> {
    let cols = sqlx::query("PRAGMA table_info(executor_promotion)")
        .fetch_all(&mut **tx)
        .await
        .map_err(|e| ArgoError::Provider(format!("v93 up (table_info): {e}")))?;
    let has_target_app = cols.iter().any(|row| {
        row.try_get::<String, _>("name")
            .map(|name| name == "target_app")
            .unwrap_or(false)
    });
    if !has_target_app {
        sqlx::query("ALTER TABLE executor_promotion ADD COLUMN target_app TEXT")
            .execute(&mut **tx)
            .await
            .map_err(|e| ArgoError::Provider(format!("v93 up: {e}")))?;
    }
    Ok(())
}

pub(super) async fn down<'a, 'b>(
    _tx: &'a mut sqlx::Transaction<'b, Sqlite>,
) -> Result<(), ArgoError> {
    // No-op: dropping a column needs a full table rebuild in SQLite;
    // a leftover nullable column is harmless on rollback.
    Ok(())
}
