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
//! `target_glob` (mission-pm-approval-target-scope) narrows a
//! grant from "any call to <tool>" to "calls whose `target`
//! payload matches this glob". `NULL` keeps the v7/v8 behaviour
//! (session-global grant). The consume-side check lives in
//! [`SessionStore::find_session_grant_for_dispatch`].
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
    /// Optional payload-target scope. `None` keeps the v7/v8
    /// behaviour (the grant applies to any dispatch of
    /// `tool_name`). `Some(glob)` restricts the grant to
    /// dispatches whose `target` argument satisfies the glob via
    /// the same matcher `Capability::ToolDispatch::tool_name_glob`
    /// uses.
    pub target_glob: Option<String>,
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
    ///
    /// `target_glob=Some(glob)` (mission-pm-approval-target-scope)
    /// narrows the grant from "any call to tool" to "calls whose
    /// `target` payload matches `glob`" via the existing globset
    /// matcher. `target_glob=None` preserves v7/v8 back-compat —
    /// the grant matches any target.
    pub async fn add_session_grant(
        &self,
        sid: SessionId,
        tool_name: &str,
        source: &str,
        notes: Option<&str>,
        one_shot: bool,
        target_glob: Option<&str>,
    ) -> StoreResult<Uuid> {
        let id = Uuid::now_v7();
        let now = chrono::Utc::now().timestamp();
        let one_shot_int: i32 = if one_shot { 1 } else { 0 };
        sqlx::query(
            "INSERT INTO session_grant
                (id, session_id, tool_name, granted_at, source, notes, is_one_shot, target_glob)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id.as_bytes().as_slice())
        .bind(sid.0.as_bytes().as_slice())
        .bind(tool_name)
        .bind(now)
        .bind(source)
        .bind(notes)
        .bind(one_shot_int)
        .bind(target_glob)
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
    pub async fn list_session_grants(&self, sid: SessionId) -> StoreResult<Vec<SessionGrant>> {
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
        sqlx::query("UPDATE session_grant SET consumed_at = ? WHERE id = ?")
            .bind(now)
            .bind(&id_bytes)
            .execute(self.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("consume_one_shot_grant update: {e}")))?;
        let id = Uuid::from_slice(&id_bytes)
            .map_err(|e| ArgoError::Provider(format!("parse consumed grant id: {e}")))?;
        Ok(Some(id))
    }

    /// Find the active grant that authorises a dispatch of
    /// `(session, tool, target)` (mission-pm-approval-target-scope).
    ///
    /// Resolution rules:
    /// - Grants with `target_glob = NULL` match any `target` (the
    ///   v7/v8 back-compat behaviour).
    /// - Grants with `target_glob = Some(g)` match only when
    ///   `target` is `Some(t)` and the globset matcher reports
    ///   `g` covers `t`.
    /// - **Most-specific wins.** When both a targeted grant and a
    ///   session-global grant could apply, the targeted grant is
    ///   returned — an operator who narrowed a grant doesn't want
    ///   a leftover session-wide grant to mask it.
    ///
    /// Errors:
    /// - `ArgoError::Provider("approval grant target mismatch")`
    ///   when targeted grant(s) exist for `(session, tool)` but
    ///   none of them satisfy the inbound `target` (or `target`
    ///   is `None`) AND no session-global grant is in place.
    ///   This makes the audit trail distinguish "no grant" from
    ///   "your grant doesn't cover this target".
    ///
    /// Returns `Ok(None)` when no grant exists for `(session,
    /// tool)` at all — the gate stays in effect.
    pub async fn find_session_grant_for_dispatch(
        &self,
        sid: SessionId,
        tool_name: &str,
        target: Option<&str>,
    ) -> StoreResult<Option<SessionGrant>> {
        let grants = self.list_session_grants(sid).await?;
        let tool_grants: Vec<SessionGrant> = grants
            .into_iter()
            .filter(|g| g.tool_name == tool_name)
            .collect();
        if tool_grants.is_empty() {
            return Ok(None);
        }
        // Partition by whether the grant declares a target_glob.
        // Targeted grants are checked first (most-specific wins);
        // session-global grants act as a fallback.
        let mut targeted: Vec<&SessionGrant> = Vec::new();
        let mut global: Vec<&SessionGrant> = Vec::new();
        for g in &tool_grants {
            if g.target_glob.is_some() {
                targeted.push(g);
            } else {
                global.push(g);
            }
        }
        // Try targeted matches first. Among matching targeted
        // grants, prefer the longer glob string (a rough but
        // honest specificity metric — "Q3-2026-*" beats "Q3-*"
        // for "Q3-2026-L42"). Ties fall to the newest grant,
        // which `list_session_grants` already orders first.
        let mut targeted_match: Option<&SessionGrant> = None;
        let mut targeted_score: usize = 0;
        for g in &targeted {
            let Some(glob) = g.target_glob.as_deref() else {
                continue;
            };
            let Some(t) = target else {
                continue;
            };
            if !glob_covers(glob, t) {
                continue;
            }
            let score = glob.len();
            if targeted_match.is_none() || score > targeted_score {
                targeted_match = Some(g);
                targeted_score = score;
            }
        }
        if let Some(g) = targeted_match {
            return Ok(Some(g.clone()));
        }
        // No targeted grant matched. Fall back to a session-
        // global grant if one exists (preserves back-compat for
        // sessions that still issue plain `argo session approve`).
        if let Some(g) = global.first() {
            return Ok(Some((*g).clone()));
        }
        // Targeted grants exist for the tool but none matched,
        // and no session-global fallback is in place. Make the
        // mismatch explicit so the audit chain shows "approval
        // grant exists but did not cover this target".
        Err(ArgoError::Provider(
            "approval grant target mismatch".to_string(),
        ))
    }

    /// Revoke all grants for a (session, tool_name) pair.
    /// Returns the number of rows deleted. Used by the planned
    /// `argo session revoke-approval` command (follow-up PR).
    pub async fn revoke_session_grants_for_tool(
        &self,
        sid: SessionId,
        tool_name: &str,
    ) -> StoreResult<u64> {
        let res = sqlx::query("DELETE FROM session_grant WHERE session_id = ? AND tool_name = ?")
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
        "SELECT id, session_id, tool_name, granted_at, source, notes, is_one_shot, consumed_at, target_glob
         FROM session_grant
         WHERE session_id = ?
         ORDER BY granted_at DESC, id DESC"
    } else {
        "SELECT id, session_id, tool_name, granted_at, source, notes, is_one_shot, consumed_at, target_glob
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
            Option<String>,
        ),
    >(sql)
    .bind(sid.0.as_bytes().as_slice())
    .fetch_all(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("list_session_grants: {e}")))?;
    let mut out = Vec::with_capacity(rows.len());
    for (
        id_b,
        _sid_b,
        tool_name,
        granted_at,
        source,
        notes,
        is_one_shot,
        consumed_at,
        target_glob,
    ) in rows
    {
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
            target_glob,
        });
    }
    Ok(out)
}

/// Glob-match `requested` against `grant_glob` using the same
/// `globset` library `argo-types::Capability::ToolDispatch` uses
/// for its `tool_name_glob`. Kept private so the only entry point
/// into the matcher is
/// [`SessionStore::find_session_grant_for_dispatch`] — the brief
/// limits us to the grant layer and we don't want callers
/// reinventing the consume-side semantics.
fn glob_covers(grant_glob: &str, requested: &str) -> bool {
    if grant_glob == requested {
        return true;
    }
    let Ok(glob) = globset::Glob::new(grant_glob) else {
        return false;
    };
    glob.compile_matcher().is_match(requested)
}

#[cfg(test)]
#[path = "grants_tests.rs"]
mod tests;
