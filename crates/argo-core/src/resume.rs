//! Resume-on-restart (Amendment 13, §6.5).
//!
//! On daemon boot, we query `tool_call WHERE ended_at IS NULL` to
//! find sessions that were interrupted mid-tool-call (process kill,
//! crash, host reboot). The user's `[loop.on_restart]` policy picks
//! one of three actions per session:
//!
//! - `prompt_user` (default) — surface the list to the CLI / TUI /
//!   gateway DM; the user picks resume / discard / cancel-session.
//! - `auto_resume` — re-emit the last tool call to the model with
//!   a "this was interrupted; verify and continue" prefix.
//! - `discard` — close the session with `end_reason =
//!   "restart_discard"` and archive.
//!
//! The functions in this module are intentionally **pure** plus a
//! thin SQL pair — the daemon owns the actual user prompting, the
//! re-emit, and the archive. Tests in this module exercise the
//! state-machine + the SQL scan against an in-memory `SessionStore`.
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, SessionId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Restart policy.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RestartPolicy {
    /// Surface the list and wait for the user to pick.
    #[default]
    PromptUser,
    /// Re-emit the last tool call with an "interrupted" prefix.
    AutoResume,
    /// Mark the session ended; archive.
    Discard,
}

/// One interrupted session — surfaces to the user (or feeds the
/// auto-resume re-emit) on daemon startup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterruptedSession {
    /// Owning session.
    pub session_id: SessionId,
    /// Tool name of the last call (e.g. `"file_write"`).
    pub last_tool_name: String,
    /// JSON-encoded arguments of the last call.
    pub last_tool_arguments_json: String,
    /// When the call started.
    pub started_at: DateTime<Utc>,
    /// Wallclock age (seconds).
    pub age_seconds: i64,
}

/// Per-session resolution decided by [`apply_restart_policy`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RestartAction {
    /// Show the user this session and wait. The daemon owns the
    /// prompt — this variant just carries the descriptor.
    Prompt {
        /// The session being prompted on.
        session: InterruptedSession,
    },
    /// Re-emit the last tool call with an "interrupted" prefix.
    Resume {
        /// The session being resumed.
        session: InterruptedSession,
        /// Prefix the daemon prepends when re-emitting.
        prefix: String,
    },
    /// Discard the session — close + archive.
    Discard {
        /// The session being discarded.
        session: InterruptedSession,
    },
}

/// Resolve `policy` against `sessions`, producing one
/// [`RestartAction`] per input. Pure function — no I/O.
#[must_use]
pub fn apply_restart_policy(
    policy: RestartPolicy,
    sessions: Vec<InterruptedSession>,
) -> Vec<RestartAction> {
    sessions
        .into_iter()
        .map(|s| match policy {
            RestartPolicy::PromptUser => RestartAction::Prompt { session: s },
            RestartPolicy::AutoResume => RestartAction::Resume {
                prefix: "this was interrupted; verify and continue".to_string(),
                session: s,
            },
            RestartPolicy::Discard => RestartAction::Discard { session: s },
        })
        .collect()
}

/// Scan `store` for orphaned tool-call records — the §6.5 invariant
/// is "tool_call rows with `ended_at IS NULL` belong to interrupted
/// turns." Returns one row per orphaned call, ordered by `started_at`
/// so the surfacing UI is deterministic.
///
/// # Errors
/// Returns [`ArgoError`] on SQL failure.
pub async fn scan_interrupted(
    store: &SessionStore,
    now: DateTime<Utc>,
) -> ArgoResult<Vec<InterruptedSession>> {
    let rows: Vec<(Vec<u8>, String, String, i64)> = sqlx::query_as(
        "SELECT session_id, name, arguments_json, started_at \
         FROM tool_call \
         WHERE ended_at IS NULL \
         ORDER BY started_at ASC",
    )
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("scan_interrupted: {e}")))?;
    let mut out = Vec::with_capacity(rows.len());
    for (sid_blob, name, args, started) in rows {
        let sid = parse_session_id(&sid_blob)?;
        let started_at = DateTime::<Utc>::from_timestamp_micros(started)
            .unwrap_or_else(|| now - chrono::Duration::seconds(1));
        out.push(InterruptedSession {
            session_id: sid,
            last_tool_name: name,
            last_tool_arguments_json: args,
            started_at,
            age_seconds: now.signed_duration_since(started_at).num_seconds(),
        });
    }
    Ok(out)
}

/// Mark a session as `end_reason = "restart_discard"`. Idempotent —
/// callers can invoke it for already-ended sessions and the row's
/// existing `ended_at` won't be reset.
///
/// # Errors
/// Returns [`ArgoError`] on SQL failure.
pub async fn mark_session_discarded(
    store: &SessionStore,
    session: SessionId,
    now: DateTime<Utc>,
) -> ArgoResult<()> {
    let ts = now.timestamp_micros();
    sqlx::query(
        "UPDATE session \
         SET ended_at = COALESCE(ended_at, ?), end_reason = 'restart_discard' \
         WHERE id = ?",
    )
    .bind(ts)
    .bind(session.0.as_bytes().as_slice())
    .execute(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("mark_session_discarded: {e}")))?;
    Ok(())
}

fn parse_session_id(blob: &[u8]) -> ArgoResult<SessionId> {
    let mut a = [0u8; 16];
    let n = blob.len().min(16);
    a[..n].copy_from_slice(&blob[..n]);
    Ok(SessionId(uuid::Uuid::from_bytes(a)))
}
