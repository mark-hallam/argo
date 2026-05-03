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
}

impl SessionStore {
    /// Persist an approval grant. Returns the new row's UUID v7
    /// id so the caller can echo it back to the operator (and
    /// reference it in audit). Multiple grants for the same
    /// (session, tool) are allowed — the runner reads ALL grants
    /// for a session at TurnStart and de-duplicates by tool_name,
    /// so re-approving is idempotent from the runtime's POV.
    pub async fn add_session_grant(
        &self,
        sid: SessionId,
        tool_name: &str,
        source: &str,
        notes: Option<&str>,
    ) -> StoreResult<Uuid> {
        let id = Uuid::now_v7();
        let now = chrono::Utc::now().timestamp();
        sqlx::query(
            "INSERT INTO session_grant
                (id, session_id, tool_name, granted_at, source, notes)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(id.as_bytes().as_slice())
        .bind(sid.0.as_bytes().as_slice())
        .bind(tool_name)
        .bind(now)
        .bind(source)
        .bind(notes)
        .execute(self.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("add_session_grant: {e}")))?;
        Ok(id)
    }

    /// Load every grant for a session, newest first. The runner
    /// calls this at TurnStart and converts the result into a
    /// `HashSet<String>` of tool_names that the approval gate
    /// should let through.
    pub async fn list_session_grants(
        &self,
        sid: SessionId,
    ) -> StoreResult<Vec<SessionGrant>> {
        let rows = sqlx::query_as::<
            _,
            (Vec<u8>, Vec<u8>, String, i64, String, Option<String>),
        >(
            // UUID v7 is monotonic, so id DESC is a stable
            // secondary sort when two grants land in the same
            // second (granted_at is whole-second granular).
            "SELECT id, session_id, tool_name, granted_at, source, notes
             FROM session_grant
             WHERE session_id = ?
             ORDER BY granted_at DESC, id DESC",
        )
        .bind(sid.0.as_bytes().as_slice())
        .fetch_all(self.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("list_session_grants: {e}")))?;
        let mut out = Vec::with_capacity(rows.len());
        for (id_b, _sid_b, tool_name, granted_at, source, notes) in rows {
            let id = Uuid::from_slice(&id_b)
                .map_err(|e| ArgoError::Provider(format!("parse grant id: {e}")))?;
            out.push(SessionGrant {
                id,
                session_id: sid,
                tool_name,
                granted_at_secs: granted_at,
                source,
                notes,
            });
        }
        Ok(out)
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

#[cfg(test)]
#[path = "grants_tests.rs"]
mod tests;
