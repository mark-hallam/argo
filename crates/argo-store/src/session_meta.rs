//! Session metadata mutations — title rename, delete. Lives in
//! its own file so `lib.rs` stays under the 500-LOC ceiling as
//! new one-shot mutators get added.
use crate::SessionStore;
use crate::StoreResult;
use argo_types::{ArgoError, SessionId};
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
