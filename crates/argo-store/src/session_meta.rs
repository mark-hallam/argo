//! Session metadata mutations — title rename, delete, judge
//! override. Lives in its own file so `lib.rs` stays under the
//! 500-LOC ceiling as new one-shot mutators get added.
use crate::SessionStore;
use crate::StoreResult;
use argo_types::{ArgoError, JudgeRecordId, SessionId};
use sqlx::{Pool, Sqlite};

impl SessionStore {
    /// Update `session.title`. Whitespace is trimmed; an empty
    /// result clears the title (writes SQL NULL). Returns the
    /// number of rows affected.
    pub async fn set_session_title(
        &self,
        sid: SessionId,
        title: &str,
    ) -> StoreResult<u64> {
        set_session_title(self.pool(), sid, title).await
            .map_err(|e| ArgoError::Provider(format!("set_session_title: {e}")))
    }

    /// Update a judge_record's verdict (and optionally its
    /// reason) in place. Returns
    /// `Some((session_id, old_verdict))` for the row that was
    /// updated so the caller can write a JudgeVerdict audit row;
    /// `None` when no row matches the id.
    pub async fn override_judge_verdict(
        &self,
        record_id: JudgeRecordId,
        new_verdict: &str,
        reason: Option<&str>,
    ) -> StoreResult<Option<(SessionId, String)>> {
        override_judge_verdict(self.pool(), record_id, new_verdict, reason)
            .await
            .map_err(|e| ArgoError::Provider(format!("override_judge_verdict: {e}")))
    }
}

/// Update `session.title` for the given session id. Returns the
/// number of rows affected (0 when no session matches the id).
///
/// Trims whitespace from `title`; an empty result clears the
/// title (writes SQL `NULL`) so the TUI / gateway can "untitle" a
/// row without a separate API.
pub(super) async fn set_session_title(
    pool: &Pool<Sqlite>,
    sid: SessionId,
    title: &str,
) -> Result<u64, sqlx::Error> {
    let trimmed = title.trim();
    let value: Option<&str> = if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    };
    let res = sqlx::query("UPDATE session SET title = ? WHERE id = ?")
        .bind(value)
        .bind(sid.0.as_bytes().as_slice())
        .execute(pool)
        .await?;
    Ok(res.rows_affected())
}

/// Delete a session row. Cascading FKs drop messages + spend +
/// audit children automatically. Returns the number of rows
/// deleted (0 when the session was not found).
pub(super) async fn delete_session(
    pool: &Pool<Sqlite>,
    sid: SessionId,
) -> Result<u64, sqlx::Error> {
    let res = sqlx::query("DELETE FROM session WHERE id = ?")
        .bind(sid.0.as_bytes().as_slice())
        .execute(pool)
        .await?;
    Ok(res.rows_affected())
}

/// Look up a judge_record's `(session_id, verdict)` pair, then
/// UPDATE the row's verdict + reason. Two-statement transaction
/// inside one connection so the audit-row writer downstream can
/// trust the returned `old_verdict`.
async fn override_judge_verdict(
    pool: &Pool<Sqlite>,
    record_id: JudgeRecordId,
    new_verdict: &str,
    reason: Option<&str>,
) -> Result<Option<(SessionId, String)>, sqlx::Error> {
    let id_bytes = record_id.0.as_bytes().as_slice();
    let prior: Option<(Vec<u8>, String)> = sqlx::query_as(
        "SELECT session_id, verdict FROM judge_record WHERE id = ?",
    )
    .bind(id_bytes)
    .fetch_optional(pool)
    .await?;
    let Some((sid_blob, old_verdict)) = prior else {
        return Ok(None);
    };
    sqlx::query("UPDATE judge_record SET verdict = ?, reason = ? WHERE id = ?")
        .bind(new_verdict)
        .bind(reason)
        .bind(id_bytes)
        .execute(pool)
        .await?;
    let sid_bytes: [u8; 16] = sid_blob
        .as_slice()
        .try_into()
        .map_err(|_| sqlx::Error::Protocol("session_id is not 16 bytes".into()))?;
    Ok(Some((SessionId(uuid::Uuid::from_bytes(sid_bytes)), old_verdict)))
}
