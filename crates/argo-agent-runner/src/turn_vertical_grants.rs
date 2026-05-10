//! Per-session approval-grant state for vertical-aware turns.
//! Sibling of [`crate::turn_vertical`]; extracted so that file
//! stays under the 500-LOC ceiling.
//!
//! Manifest-side approval _declarations_ (which tools require
//! human approval) live in [`crate::turn_vertical`] alongside
//! the rest of the manifest reads. This module owns the runtime
//! state: which grants the operator has actually issued for a
//! given session, and how those grants are consumed at dispatch.
//!
//! - [`SessionGrantState`] — in-memory snapshot of grants for
//!   one session, refreshed each `TurnStart`.
//! - [`build_session_grant_state`] — store-backed loader (#664 +
//!   #667).
//! - [`consume_one_shot_if_present`] — decrements one-shot
//!   grants after a gate-passing dispatch and emits the
//!   `ApprovalConsumed` audit row (#667).

/// Per-turn snapshot of operator-issued approval grants for a
/// session (#664 + #667). The agent runner consults this on
/// every approval-gated tool dispatch:
///
/// - Persistent grants (#664) live in `persistent` — they let
///   the tool through every dispatch, never consumed.
/// - One-shot grants (#667) live in `one_shot_remaining` as a
///   per-tool count — each successful gate-passing dispatch
///   decrements one. When the count hits zero AND the tool has
///   no persistent grant, subsequent calls in the same turn
///   re-block.
#[derive(Debug, Default, Clone)]
pub(crate) struct SessionGrantState {
    pub(crate) persistent: std::collections::HashSet<String>,
    pub(crate) one_shot_remaining: std::collections::HashMap<String, usize>,
}

impl SessionGrantState {
    /// `true` if `tool` has at least one active grant — either
    /// persistent or unconsumed one-shot.
    pub(crate) fn allows(&self, tool: &str) -> bool {
        self.persistent.contains(tool)
            || self
                .one_shot_remaining
                .get(tool)
                .copied()
                .unwrap_or(0)
                > 0
    }

    /// Called after a successful gate-passing dispatch. If the
    /// tool has only one-shot grants, decrements the in-memory
    /// count so subsequent calls in the same turn re-block once
    /// the count hits zero. Persistent grants are unaffected.
    pub(crate) fn record_one_shot_consumption(&mut self, tool: &str) {
        if self.persistent.contains(tool) {
            return;
        }
        if let Some(count) = self.one_shot_remaining.get_mut(tool) {
            if *count > 0 {
                *count -= 1;
            }
        }
    }
}

/// Build the [`SessionGrantState`] for a session at TurnStart.
/// Returns the empty default when:
/// - the session has no grants (the common case),
/// - the store query errors (degrade closed — better the gate
///   stays in effect than to spuriously "approve"),
/// - the session id is unknown.
pub(crate) async fn build_session_grant_state(
    store: &argo_store::SessionStore,
    session_id: argo_types::SessionId,
) -> SessionGrantState {
    let mut state = SessionGrantState::default();
    let Ok(grants) = store.list_session_grants(session_id).await else {
        return state;
    };
    for g in grants {
        if g.is_one_shot {
            *state.one_shot_remaining.entry(g.tool_name).or_insert(0) += 1;
        } else {
            state.persistent.insert(g.tool_name);
        }
    }
    state
}

/// Consume one one-shot grant for `(session, tool)` after a
/// gate-passing dispatch (#667). No-op when the tool only has
/// persistent grants — the helper detects that by checking the
/// in-memory state. Emits an `ApprovalConsumed` audit row when a
/// grant actually consumes so the operator can correlate
/// "approval given → approval used" in the chain.
pub(crate) async fn consume_one_shot_if_present(
    store: &argo_store::SessionStore,
    session_id: argo_types::SessionId,
    tool: &str,
    state: &mut SessionGrantState,
    audit: &argo_audit::AuditSink,
    events: Option<&crate::helpers::RunEventSink>,
) -> argo_types::ArgoResult<()> {
    // Persistent grants never consume; skip the store query.
    if state.persistent.contains(tool) {
        return Ok(());
    }
    let consumed = match store.consume_one_shot_grant(session_id, tool).await {
        Ok(opt) => opt,
        Err(_) => {
            // Degrade closed: leave the in-memory state alone.
            // Worst case the next dispatch in this turn also
            // goes through, but the next turn's TurnStart
            // re-reads the store and corrects.
            return Ok(());
        }
    };
    if let Some(grant_id) = consumed {
        state.record_one_shot_consumption(tool);
        let payload = serde_json::json!({
            "tool": tool,
            "grant_id": grant_id.to_string(),
            "reason": "one_shot_dispatched",
        });
        audit
            .write(
                Some(session_id),
                argo_audit::AuditKind::ApprovalConsumed,
                payload.clone(),
                None,
            )
            .await?;
        crate::helpers::emit_event(events, "ApprovalConsumed", payload);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_returns_true_for_persistent_grant() {
        let mut s = SessionGrantState::default();
        s.persistent.insert("file_write".to_string());
        assert!(s.allows("file_write"));
        assert!(!s.allows("terminal"));
    }

    #[test]
    fn allows_returns_true_for_unconsumed_one_shot() {
        let mut s = SessionGrantState::default();
        s.one_shot_remaining.insert("terminal".to_string(), 1);
        assert!(s.allows("terminal"));
    }

    #[test]
    fn allows_returns_false_when_one_shot_exhausted() {
        let mut s = SessionGrantState::default();
        s.one_shot_remaining.insert("terminal".to_string(), 0);
        assert!(!s.allows("terminal"));
    }

    #[test]
    fn record_one_shot_decrements_count() {
        let mut s = SessionGrantState::default();
        s.one_shot_remaining.insert("terminal".to_string(), 2);
        s.record_one_shot_consumption("terminal");
        assert_eq!(s.one_shot_remaining.get("terminal").copied(), Some(1));
        s.record_one_shot_consumption("terminal");
        assert_eq!(s.one_shot_remaining.get("terminal").copied(), Some(0));
    }

    #[test]
    fn record_one_shot_skips_when_persistent_present() {
        let mut s = SessionGrantState::default();
        s.persistent.insert("file_write".to_string());
        s.one_shot_remaining.insert("file_write".to_string(), 1);
        s.record_one_shot_consumption("file_write");
        // Persistent grants never consume their one-shot siblings.
        assert_eq!(s.one_shot_remaining.get("file_write").copied(), Some(1));
    }
}
