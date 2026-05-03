//! Per-session human-approval grants for the vertical-template
//! escalation gate (#664). Companion to migration v7's
//! `session_grant` table.
//!
//! The vertical's `[escalation].human_approval_required` list is
//! the *block* — the runner refuses to dispatch any tool whose
//! name appears there. This module is the *unblock*: an operator
//! issues `argo session approve <session> --tool <name>` (or the
//! gateway equivalent), which writes a `session_grant` row, and
//! the agent runner reads grants at every TurnStart and lets the
//! grant override the block for that exact tool name.
//!
//! Today grants are persistent (stay until the session ends or
//! the operator revokes — which is `--yes` based, like uninstall).
//! A future PR can add one-shot semantics if we want to require
//! re-approval per dispatch.
use crate::SessionStore;
use crate::StoreResult;
use argo_types::{ArgoError, SessionId};
use uuid::Uuid;

/// One row from `session_grant`. Read shape only — writes go
/// through the dedicated mutator below.
#[derive(Debug, Clone)]
pub struct SessionGrant {
    /// Grant id (UUID v7).
    pub id: Uuid,
    /// Session this grant applies to.
    pub session_id: SessionId,
    /// Tool name the grant lets through. Exact-match against
    /// the `[escalation].human_approval_required` list.
    pub tool_name: String,
    /// Unix-epoch seconds the grant was issued.
    pub granted_at_secs: i64,
    /// Provenance hint — `"cli"` or `"gateway"` today.
    pub source: String,
    /// Optional operator note.
    pub notes: Option<String>,
    /// `true` when the grant is consumed on first matching
    /// dispatch (#667). `false` for the original always-on
    /// grants from #664.
    pub is_one_shot: bool,
    /// Unix-epoch seconds the one-shot grant was consumed, or
    /// `None` when the grant is still active. Always `None`
    /// for non-one-shot grants — they never consume.
    pub consumed_at_secs: Option<i64>,
}

impl SessionStore {
    /// Persist an approval grant. Returns the new row's UUID v7
    /// id so the caller can echo it back to the operator (and
    /// reference it in audit). Multiple grants for the same
    /// (session, tool) are allowed — the runner reads ALL active
    /// grants for a session at TurnStart and de-duplicates by
    /// tool_name, so re-approving is idempotent from the
    /// runtime's POV.
    ///
    /// `one_shot=true` (#667) marks the grant for consumption on
    /// the first matching dispatch — useful for "let it through
    /// just this once" approvals where the operator doesn't want
    /// the grant to persist across the whole session.
    pub async fn add_session_grant(
        &self,
        sid: SessionId,
        tool_name: &str,
        source: &str,
        notes: Option<&str>,
        one_shot: bool,
    ) -> StoreResult<Uuid> {
        let id = Uuid::now_v7();
        let now = chrono::Utc::now().timestamp();
        let one_shot_int: i32 = if one_shot { 1 } else { 0 };
        sqlx::query(
            "INSERT INTO session_grant
                (id, session_id, tool_name, granted_at, source, notes, is_one_shot)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id.as_bytes().as_slice())
        .bind(sid.0.as_bytes().as_slice())
        .bind(tool_name)
        .bind(now)
        .bind(source)
        .bind(notes)
        .bind(one_shot_int)
        .execute(self.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("add_session_grant: {e}")))?;
        Ok(id)
    }

    /// Load **active** grants for a session, newest first.
    /// "Active" = persistent grants + one-shot grants that
    /// haven't been consumed yet. The runner calls this at
    /// TurnStart and converts the result into a
    /// `HashSet<String>` of tool_names the approval gate should
    /// let through.
    ///
    /// Consumed one-shot grants stay in the table for audit
    /// trail purposes — to see them, use
    /// [`Self::list_session_grants_with_consumed`].
    pub async fn list_session_grants(
        &self,
        sid: SessionId,
    ) -> StoreResult<Vec<SessionGrant>> {
        list_session_grants_inner(self.pool(), sid, false).await
    }

    /// Same as [`Self::list_session_grants`] but includes
    /// already-consumed one-shot grants. Used by `argo session
    /// list-grants --include-consumed` (and by the audit timeline
    /// view).
    pub async fn list_session_grants_with_consumed(
        &self,
        sid: SessionId,
    ) -> StoreResult<Vec<SessionGrant>> {
        list_session_grants_inner(self.pool(), sid, true).await
    }

    /// Mark the oldest active one-shot grant for a (session,
    /// tool) pair as consumed (#667). Returns the UUID of the
    /// row that was consumed, or `None` if no matching
    /// unconsumed one-shot grant exists.
    ///
    /// The agent runner calls this AFTER a successful dispatch
    /// of an approval-gated tool, so the next turn re-blocks
    /// the tool unless another grant is in place.
    pub async fn consume_one_shot_grant(
        &self,
        sid: SessionId,
        tool_name: &str,
    ) -> StoreResult<Option<Uuid>> {
        // Pick the oldest unconsumed one-shot grant — first-in-
        // first-out so an operator who issues two `--once`
        // grants in a row sees them consumed in issue order.
        let row: Option<Vec<u8>> = sqlx::query_scalar(
            "SELECT id FROM session_grant
             WHERE session_id = ?
               AND tool_name = ?
               AND is_one_shot = 1
               AND consumed_at IS NULL
             ORDER BY granted_at ASC, id ASC
             LIMIT 1",
        )
        .bind(sid.0.as_bytes().as_slice())
        .bind(tool_name)
        .fetch_optional(self.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("consume_one_shot_grant lookup: {e}")))?;
        let Some(id_bytes) = row else {
            return Ok(None);
        };
        let now = chrono::Utc::now().timestamp();
        sqlx::query(
            "UPDATE session_grant SET consumed_at = ? WHERE id = ?",
        )
        .bind(now)
        .bind(&id_bytes)
        .execute(self.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("consume_one_shot_grant update: {e}")))?;
        let id = Uuid::from_slice(&id_bytes)
            .map_err(|e| ArgoError::Provider(format!("parse consumed grant id: {e}")))?;
        Ok(Some(id))
    }

    /// Revoke all grants for a (session, tool_name) pair.
    /// Returns the number of rows deleted. Used by the planned
    /// `argo session revoke-approval` command (follow-up PR).
    pub async fn revoke_session_grants_for_tool(
        &self,
        sid: SessionId,
        tool_name: &str,
    ) -> StoreResult<u64> {
        let res = sqlx::query(
            "DELETE FROM session_grant WHERE session_id = ? AND tool_name = ?",
        )
        .bind(sid.0.as_bytes().as_slice())
        .bind(tool_name)
        .execute(self.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("revoke_session_grants_for_tool: {e}")))?;
        Ok(res.rows_affected())
    }
}

async fn list_session_grants_inner(
    pool: &sqlx::Pool<sqlx::Sqlite>,
    sid: SessionId,
    include_consumed: bool,
) -> StoreResult<Vec<SessionGrant>> {
    // The active query filters out consumed one-shot grants;
    // include_consumed=true returns everything (active +
    // consumed) for the audit timeline view.
    let sql = if include_consumed {
        "SELECT id, session_id, tool_name, granted_at, source, notes, is_one_shot, consumed_at
         FROM session_grant
         WHERE session_id = ?
         ORDER BY granted_at DESC, id DESC"
    } else {
        "SELECT id, session_id, tool_name, granted_at, source, notes, is_one_shot, consumed_at
         FROM session_grant
         WHERE session_id = ? AND consumed_at IS NULL
         ORDER BY granted_at DESC, id DESC"
    };
    let rows = sqlx::query_as::<
        _,
        (
            Vec<u8>,
            Vec<u8>,
            String,
            i64,
            String,
            Option<String>,
            i64,
            Option<i64>,
        ),
    >(sql)
    .bind(sid.0.as_bytes().as_slice())
    .fetch_all(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("list_session_grants: {e}")))?;
    let mut out = Vec::with_capacity(rows.len());
    for (id_b, _sid_b, tool_name, granted_at, source, notes, is_one_shot, consumed_at) in rows {
        let id = Uuid::from_slice(&id_b)
            .map_err(|e| ArgoError::Provider(format!("parse grant id: {e}")))?;
        out.push(SessionGrant {
            id,
            session_id: sid,
            tool_name,
            granted_at_secs: granted_at,
            source,
            notes,
            is_one_shot: is_one_shot != 0,
            consumed_at_secs: consumed_at,
        });
    }
    Ok(out)
}

#[cfg(test)]
#[path = "grants_tests.rs"]
mod tests;
