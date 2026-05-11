//! Per-worker persona revision log — append-only diff history with
//! sha256 audit-chain hashes and one-step revert
//! (Stream `mission-persona-evolution-audit-revert`, MISSION_BACKLOG
//! Section 3 Claim 1 #46).
//!
//! Each [`record_persona_change`] call writes one immutable row to
//! `persona_revision` and computes `audit_chain_hash` over the
//! previous row's hash for that worker, in the same shape as
//! [`argo_audit::compute_chain_hash`]: `sha256(prev || field || 0x00 ||
//! rev_id || 0x00 || new_value)`. The audit crate's helper isn't
//! reused directly because `argo-audit` already depends on
//! `argo-store`; adding the reverse edge would create a dependency
//! cycle (see PR body for the documented sensible deviation).
//!
//! [`list_persona_revisions`] returns the newest rows first (capped
//! by `limit`). [`revert_persona_to`] applies the cited revision's
//! `old_value` to the live `persona` row and writes a NEW revision
//! capturing the revert itself — the log never goes backwards.
//!
//! Tamper-evidence is verified offline via [`verify_chain`]:
//! recomputing each row's hash from the previous row's stored hash
//! and the row's own `(rev_id, field, new_value)`. Any mutation to a
//! row's hash propagates through the chain and is detected at the
//! next row.
use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{Pool, Row, Sqlite};
use uuid::Uuid;

/// One immutable row in `persona_revision`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonaRevision {
    /// Stable UUID for the revision.
    pub rev_id: Uuid,
    /// Worker whose persona was edited (UUID).
    pub worker_id: Uuid,
    /// Wall-clock time the edit was recorded.
    pub revised_at: DateTime<Utc>,
    /// Logical column label (`name`, `voice`, `avatar`, `bio`, …).
    pub field: String,
    /// JSON-serialised previous value (`""` when no prior value).
    pub old_value: String,
    /// JSON-serialised new value applied by the edit.
    pub new_value: String,
    /// Operator handle / system label / forge id that drove the edit.
    pub actor: String,
    /// sha256 hash chained over the previous row's hash for tamper-
    /// evidence. Hex-encoded.
    pub audit_chain_hash: String,
}

/// Append one immutable revision for `worker_id`. The
/// `audit_chain_hash` field is computed from the previous row's hash
/// (or the empty 32-byte seed when this is the first revision).
///
/// Returns the freshly-written [`PersonaRevision`] so callers can
/// pipe it into the CLI / audit sink without re-querying.
pub async fn record_persona_change(
    pool: &Pool<Sqlite>,
    worker_id: Uuid,
    field: &str,
    old_value: &str,
    new_value: &str,
    actor: &str,
) -> ArgoResult<PersonaRevision> {
    let prev_hash = latest_chain_hash(pool, worker_id).await?;
    let rev_id = Uuid::now_v7();
    let revised_at = Utc::now();
    let audit_chain_hash = hex_encode(&compute_revision_hash(
        &prev_hash, &rev_id, field, new_value,
    ));

    sqlx::query(INSERT_REVISION_SQL)
        .bind(rev_id.to_string())
        .bind(worker_id.to_string())
        .bind(revised_at.timestamp())
        .bind(field)
        .bind(old_value)
        .bind(new_value)
        .bind(actor)
        .bind(&audit_chain_hash)
        .execute(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("record_persona_change insert: {e}")))?;

    Ok(PersonaRevision {
        rev_id,
        worker_id,
        revised_at,
        field: field.to_string(),
        old_value: old_value.to_string(),
        new_value: new_value.to_string(),
        actor: actor.to_string(),
        audit_chain_hash,
    })
}

/// Return up to `limit` revisions for `worker_id`, newest first.
/// `limit == 0` is normalised to 50.
pub async fn list_persona_revisions(
    pool: &Pool<Sqlite>,
    worker_id: Uuid,
    limit: u32,
) -> ArgoResult<Vec<PersonaRevision>> {
    let bounded = if limit == 0 { 50 } else { limit };
    let rows = sqlx::query(SELECT_BY_WORKER_SQL)
        .bind(worker_id.to_string())
        .bind(i64::from(bounded))
        .fetch_all(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("list_persona_revisions: {e}")))?;
    rows.iter().map(row_to_revision).collect()
}

/// Apply `rev_id`'s `old_value` to the live `persona` row and write
/// a NEW revision capturing the revert itself.
///
/// Returns the newly-written revert revision. The original row is
/// never deleted — the log is append-only.
///
/// `actor` is recorded on the revert revision (e.g. `cli:bob@…`).
pub async fn revert_persona_to(
    pool: &Pool<Sqlite>,
    rev_id: Uuid,
    actor: &str,
) -> ArgoResult<PersonaRevision> {
    let target = fetch_revision(pool, rev_id).await?;
    let column = persona_column_for(&target.field)?;

    let current_value: String = sqlx::query_scalar(&format!(
        "SELECT COALESCE({column}, '') FROM persona WHERE worker_id = ?"
    ))
    .bind(target.worker_id.to_string())
    .fetch_optional(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("revert read current: {e}")))?
    .ok_or_else(|| {
        ArgoError::Provider(format!(
            "persona row missing for worker_id {} — cannot revert",
            target.worker_id
        ))
    })?;

    sqlx::query(&format!(
        "UPDATE persona SET {column} = ? WHERE worker_id = ?"
    ))
    .bind(&target.old_value)
    .bind(target.worker_id.to_string())
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("revert apply: {e}")))?;

    record_persona_change(
        pool,
        target.worker_id,
        &target.field,
        &current_value,
        &target.old_value,
        actor,
    )
    .await
}

/// Verify the audit chain for `worker_id`. Returns `Ok(())` when
/// every row's stored hash matches the recomputed
/// `sha256(prev || field || 0x00 || rev_id || 0x00 || new_value)`,
/// or an `ArgoError::Provider` naming the first row that breaks.
///
/// Used by the 8th unit test (`audit_chain_prev_link` /
/// `tamper_detected`) and downstream tooling that wants to assert
/// log integrity without reading rows by hand.
pub async fn verify_chain(pool: &Pool<Sqlite>, worker_id: Uuid) -> ArgoResult<()> {
    let rows = sqlx::query(SELECT_BY_WORKER_ASC_SQL)
        .bind(worker_id.to_string())
        .fetch_all(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("verify_chain read: {e}")))?;
    let mut prev = [0u8; 32];
    for row in &rows {
        let revision = row_to_revision(row)?;
        let recomputed = hex_encode(&compute_revision_hash(
            &prev,
            &revision.rev_id,
            &revision.field,
            &revision.new_value,
        ));
        if recomputed != revision.audit_chain_hash {
            return Err(ArgoError::Provider(format!(
                "persona_revision chain tampered at rev_id {}: expected {recomputed}, got {}",
                revision.rev_id, revision.audit_chain_hash
            )));
        }
        prev = decode_hex_32(&revision.audit_chain_hash)?;
    }
    Ok(())
}

async fn latest_chain_hash(pool: &Pool<Sqlite>, worker_id: Uuid) -> ArgoResult<[u8; 32]> {
    let row = sqlx::query(LATEST_HASH_SQL)
        .bind(worker_id.to_string())
        .fetch_optional(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("latest_chain_hash: {e}")))?;
    match row {
        Some(row) => {
            let hex: String = row.get("audit_chain_hash");
            decode_hex_32(&hex)
        }
        None => Ok([0u8; 32]),
    }
}

async fn fetch_revision(pool: &Pool<Sqlite>, rev_id: Uuid) -> ArgoResult<PersonaRevision> {
    let row = sqlx::query(SELECT_BY_REV_SQL)
        .bind(rev_id.to_string())
        .fetch_optional(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("fetch_revision: {e}")))?
        .ok_or_else(|| ArgoError::Provider(format!("revision {rev_id} not found")))?;
    row_to_revision(&row)
}

fn row_to_revision(row: &sqlx::sqlite::SqliteRow) -> ArgoResult<PersonaRevision> {
    let rev_id_str: String = row.get("rev_id");
    let rev_id = Uuid::parse_str(&rev_id_str)
        .map_err(|e| ArgoError::Provider(format!("parse rev_id {rev_id_str}: {e}")))?;
    let worker_id_str: String = row.get("worker_id");
    let worker_id = Uuid::parse_str(&worker_id_str)
        .map_err(|e| ArgoError::Provider(format!("parse worker_id {worker_id_str}: {e}")))?;
    let revised_at_secs: i64 = row.get("revised_at");
    let revised_at = Utc
        .timestamp_opt(revised_at_secs, 0)
        .single()
        .ok_or_else(|| ArgoError::Provider(format!("invalid revised_at {revised_at_secs}")))?;
    Ok(PersonaRevision {
        rev_id,
        worker_id,
        revised_at,
        field: row.get("field"),
        old_value: row.get("old_value"),
        new_value: row.get("new_value"),
        actor: row.get("actor"),
        audit_chain_hash: row.get("audit_chain_hash"),
    })
}

fn persona_column_for(field: &str) -> ArgoResult<&'static str> {
    match field {
        "name" => Ok("name"),
        "voice" | "voice_profile" | "voice_preamble" => Ok("voice_preamble"),
        "avatar" => Ok("avatar"),
        "bio" => Ok("bio"),
        other => Err(ArgoError::Schema(format!(
            "persona field '{other}' is not revertible (allowed: name, voice, avatar, bio)"
        ))),
    }
}

fn compute_revision_hash(
    parent: &[u8; 32],
    rev_id: &Uuid,
    field: &str,
    new_value: &str,
) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(parent);
    hasher.update(field.as_bytes());
    hasher.update(b"\0");
    hasher.update(rev_id.as_bytes());
    hasher.update(b"\0");
    hasher.update(new_value.as_bytes());
    let mut buf = [0u8; 32];
    buf.copy_from_slice(&hasher.finalize());
    buf
}

fn hex_encode(bytes: &[u8; 32]) -> String {
    let mut out = String::with_capacity(64);
    for byte in bytes {
        use std::fmt::Write;
        let _ = write!(out, "{byte:02x}");
    }
    out
}

fn decode_hex_32(hex: &str) -> ArgoResult<[u8; 32]> {
    if hex.len() != 64 {
        return Err(ArgoError::Schema(format!(
            "audit_chain_hash must be 64 hex chars, got {}",
            hex.len()
        )));
    }
    let mut out = [0u8; 32];
    for (i, chunk) in hex.as_bytes().chunks(2).enumerate() {
        let pair = std::str::from_utf8(chunk)
            .map_err(|e| ArgoError::Schema(format!("audit_chain_hash utf8: {e}")))?;
        out[i] = u8::from_str_radix(pair, 16)
            .map_err(|e| ArgoError::Schema(format!("audit_chain_hash hex: {e}")))?;
    }
    Ok(out)
}

const INSERT_REVISION_SQL: &str = "INSERT INTO persona_revision \
    (rev_id, worker_id, revised_at, field, old_value, new_value, actor, audit_chain_hash) \
    VALUES (?, ?, ?, ?, ?, ?, ?, ?)";

const SELECT_BY_WORKER_SQL: &str = "SELECT rev_id, worker_id, revised_at, field, old_value, \
    new_value, actor, audit_chain_hash FROM persona_revision \
    WHERE worker_id = ? ORDER BY revised_at DESC, rev_id DESC LIMIT ?";

const SELECT_BY_WORKER_ASC_SQL: &str = "SELECT rev_id, worker_id, revised_at, field, old_value, \
    new_value, actor, audit_chain_hash FROM persona_revision \
    WHERE worker_id = ? ORDER BY revised_at ASC, rev_id ASC";

const SELECT_BY_REV_SQL: &str = "SELECT rev_id, worker_id, revised_at, field, old_value, \
    new_value, actor, audit_chain_hash FROM persona_revision WHERE rev_id = ?";

const LATEST_HASH_SQL: &str = "SELECT audit_chain_hash FROM persona_revision \
    WHERE worker_id = ? ORDER BY revised_at DESC, rev_id DESC LIMIT 1";

#[cfg(test)]
#[path = "persona_revision_tests.rs"]
mod tests;
