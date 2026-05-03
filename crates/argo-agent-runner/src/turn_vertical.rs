//! Vertical-template integration helpers for the agent runner
//! (#654). Pure-ish helpers — no async, minimal I/O — extracted
//! from `turn.rs` so that file stays under the 500-LOC ceiling.
//!
//! Two responsibilities:
//!
//! 1. [`build_turn_start_payload`] — pure constructor for the
//!    JSON payload of the TurnStart audit row, optionally
//!    enriched with `vertical_id` so per-vertical rollups
//!    (`/audit/by-vertical`) become possible.
//! 2. [`resolve_vertical_for_session`] — read-only lookup that
//!    walks the discovery dir + returns the vertical's
//!    `[role].system_prompt` markdown. Returns `None` for every
//!    "not bound / not loaded / no prompt" path so the agent
//!    loop never aborts because of a missing manifest detail.

/// Build the TurnStart audit payload. Pure — no I/O, no
/// fallible bits.
pub(crate) fn build_turn_start_payload(
    provider: &str,
    model: &str,
    vertical_id: Option<&str>,
) -> serde_json::Value {
    let mut payload = serde_json::json!({
        "provider": provider,
        "model": model,
    });
    if let Some(vid) = vertical_id {
        if let Some(obj) = payload.as_object_mut() {
            obj.insert(
                "vertical_id".to_string(),
                serde_json::Value::String(vid.to_string()),
            );
        }
    }
    payload
}

/// Build the deny-set of tool names the vertical refuses to
/// dispatch (#655). Returns an empty set for unbound sessions
/// or when the manifest declares no `[tools]` block — the
/// caller can `is_empty()` to short-circuit the per-tool check.
///
/// Deny-wins semantics: if a tool name appears in the manifest's
/// `denied` list, dispatch must be blocked even if the engine's
/// role grants would otherwise permit it.
pub(crate) fn build_vertical_deny_set(
    vertical_id: Option<&str>,
) -> std::collections::HashSet<String> {
    let mut out = std::collections::HashSet::new();
    let Some(vid) = vertical_id else {
        return out;
    };
    let dir = argo_vertical::default_dir();
    let Ok((registry, _errors)) = argo_vertical::load_dir(&dir) else {
        return out;
    };
    let Some(loaded) = registry.get(vid) else {
        return out;
    };
    if let Some(tools) = &loaded.manifest.tools {
        for name in &tools.denied {
            out.insert(name.clone());
        }
    }
    out
}

/// Build the set of tool names that the vertical declares
/// require human approval before dispatch (#660). Mirror of
/// [`build_vertical_deny_set`] for the `[escalation]
/// .human_approval_required` list.
///
/// Returns an empty set when:
/// - the session isn't vertical-bound,
/// - the discovery dir can't be loaded,
/// - the vertical isn't registered,
/// - the manifest declares no `[escalation]` block,
/// - or the list is empty.
///
/// Approval semantics: tools in the returned set are NOT
/// dispatched. The caller emits an `EscalationRequired` audit
/// row + returns a synthetic-error tool result so the agent
/// learns the boundary instead of seeing a silent no-op.
pub(crate) fn build_vertical_approval_set(
    vertical_id: Option<&str>,
) -> std::collections::HashSet<String> {
    let mut out = std::collections::HashSet::new();
    let Some(vid) = vertical_id else {
        return out;
    };
    let dir = argo_vertical::default_dir();
    let Ok((registry, _errors)) = argo_vertical::load_dir(&dir) else {
        return out;
    };
    let Some(loaded) = registry.get(vid) else {
        return out;
    };
    if let Some(esc) = &loaded.manifest.escalation {
        for name in &esc.human_approval_required {
            out.insert(name.clone());
        }
    }
    out
}

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
            )
            .await?;
        crate::helpers::emit_event(events, "ApprovalConsumed", payload);
    }
    Ok(())
}

/// Resolve the vertical's `[role].system_prompt` markdown for a
/// vertical-bound session. Returns `None` (silently) when:
/// - the session isn't vertical-bound,
/// - no vertical with that id is loaded in the discovery dir,
/// - the manifest doesn't declare `[role].system_prompt`,
/// - the markdown file can't be read.
///
/// All non-fatal — the agent loop falls back to skill-derived
/// prompt rather than aborting.
pub(crate) fn resolve_vertical_for_session(vertical_id: Option<&str>) -> Option<String> {
    let vid = vertical_id?;
    let dir = argo_vertical::default_dir();
    let (registry, _errors) = argo_vertical::load_dir(&dir).ok()?;
    let loaded = registry.get(vid)?;
    let role = loaded.manifest.role.as_ref()?;
    let p = loaded.root.join(&role.system_prompt);
    std::fs::read_to_string(&p).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Cargo runs unit tests in parallel within the same binary,
    /// so the `ENV_VERTICALS_DIR` set/remove pairs in this module
    /// would race without a process-wide lock — one test could
    /// `remove_var` while another expected the value, producing
    /// intermittent assertion flakes. The guard takes the lock,
    /// sets the env var, and restores `unset` on drop so even
    /// panics leave the next test with a clean slate.
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    struct EnvGuard<'a> {
        _lock: std::sync::MutexGuard<'a, ()>,
    }
    impl EnvGuard<'_> {
        fn new(value: &std::path::Path) -> Self {
            let lock = ENV_LOCK
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            std::env::set_var(argo_vertical::ENV_VERTICALS_DIR, value);
            Self { _lock: lock }
        }
    }
    impl Drop for EnvGuard<'_> {
        fn drop(&mut self) {
            std::env::remove_var(argo_vertical::ENV_VERTICALS_DIR);
        }
    }

    #[test]
    fn turn_start_payload_omits_vertical_id_for_unbound() {
        let v = build_turn_start_payload("anthropic", "haiku", None);
        let obj = v.as_object().unwrap();
        assert_eq!(obj.get("provider").and_then(|x| x.as_str()), Some("anthropic"));
        assert_eq!(obj.get("model").and_then(|x| x.as_str()), Some("haiku"));
        assert!(obj.get("vertical_id").is_none());
    }

    #[test]
    fn turn_start_payload_includes_vertical_id_when_bound() {
        let v = build_turn_start_payload("anthropic", "haiku", Some("pr-checker"));
        let obj = v.as_object().unwrap();
        assert_eq!(
            obj.get("vertical_id").and_then(|x| x.as_str()),
            Some("pr-checker")
        );
    }

    fn write_vertical_with_prompt(dir: &std::path::Path, body: &str) {
        let v = dir.join("demo");
        std::fs::create_dir_all(v.join("prompts")).unwrap();
        std::fs::write(v.join("prompts/system.md"), body).unwrap();
        std::fs::write(
            v.join("vertical.toml"),
            "schema_version = \"1\"\n\
             [vertical]\nid = \"demo\"\nname = \"D\"\nversion = \"0.1.0\"\n\
             [role]\nsystem_prompt = \"prompts/system.md\"\n",
        )
        .unwrap();
    }

    fn write_vertical_no_role(dir: &std::path::Path) {
        let v = dir.join("noprompt");
        std::fs::create_dir_all(&v).unwrap();
        std::fs::write(
            v.join("vertical.toml"),
            "schema_version = \"1\"\n\
             [vertical]\nid = \"noprompt\"\nname = \"N\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
    }

    #[test]
    fn resolve_vertical_returns_none_when_session_unbound() {
        assert!(resolve_vertical_for_session(None).is_none());
    }

    #[test]
    fn resolve_vertical_returns_prompt_for_bound_session() {
        let dir = tempfile::tempdir().unwrap();
        write_vertical_with_prompt(dir.path(), "you are demo agent");
        let _g = EnvGuard::new(dir.path());
        let got = resolve_vertical_for_session(Some("demo"));
        assert_eq!(got.as_deref(), Some("you are demo agent"));
    }

    #[test]
    fn resolve_vertical_returns_none_for_unknown_id() {
        let dir = tempfile::tempdir().unwrap();
        write_vertical_with_prompt(dir.path(), "x");
        let _g = EnvGuard::new(dir.path());
        let got = resolve_vertical_for_session(Some("ghost"));
        assert!(got.is_none());
    }

    #[test]
    fn resolve_vertical_returns_none_when_no_role_section() {
        let dir = tempfile::tempdir().unwrap();
        write_vertical_no_role(dir.path());
        let _g = EnvGuard::new(dir.path());
        let got = resolve_vertical_for_session(Some("noprompt"));
        assert!(got.is_none());
    }

    // ---- vertical deny-set (#655) ----------------------------

    fn write_vertical_with_tools(
        dir: &std::path::Path,
        id: &str,
        allowed: &[&str],
        denied: &[&str],
    ) {
        let v = dir.join(id);
        std::fs::create_dir_all(&v).unwrap();
        let allowed_toml = allowed
            .iter()
            .map(|s| format!("\"{s}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let denied_toml = denied
            .iter()
            .map(|s| format!("\"{s}\""))
            .collect::<Vec<_>>()
            .join(", ");
        std::fs::write(
            v.join("vertical.toml"),
            format!(
                "schema_version = \"1\"\n\
                 [vertical]\nid = \"{id}\"\nname = \"X\"\nversion = \"0.1.0\"\n\
                 [tools]\nallowed = [{allowed_toml}]\ndenied = [{denied_toml}]\n"
            ),
        )
        .unwrap();
    }

    #[test]
    fn deny_set_empty_for_unbound_session() {
        let s = build_vertical_deny_set(None);
        assert!(s.is_empty());
    }

    #[test]
    fn deny_set_carries_manifest_denies() {
        let dir = tempfile::tempdir().unwrap();
        write_vertical_with_tools(dir.path(), "ro", &["file_read"], &["file_write", "terminal"]);
        let _g = EnvGuard::new(dir.path());
        let s = build_vertical_deny_set(Some("ro"));
        assert!(s.contains("file_write"));
        assert!(s.contains("terminal"));
        assert!(!s.contains("file_read"));
        assert_eq!(s.len(), 2);
    }

    #[test]
    fn deny_set_empty_for_unknown_id() {
        let dir = tempfile::tempdir().unwrap();
        write_vertical_with_tools(dir.path(), "ro", &[], &["x"]);
        let _g = EnvGuard::new(dir.path());
        let s = build_vertical_deny_set(Some("ghost"));
        assert!(s.is_empty());
    }

    #[test]
    fn deny_set_empty_when_no_tools_section() {
        let dir = tempfile::tempdir().unwrap();
        write_vertical_no_role(dir.path()); // no [tools] either
        let _g = EnvGuard::new(dir.path());
        let s = build_vertical_deny_set(Some("noprompt"));
        assert!(s.is_empty());
    }

    // ---- vertical approval-set (#660) ------------------------

    fn write_vertical_with_approval(
        dir: &std::path::Path,
        id: &str,
        approval: &[&str],
    ) {
        let v = dir.join(id);
        std::fs::create_dir_all(&v).unwrap();
        let approval_toml = approval
            .iter()
            .map(|s| format!("\"{s}\""))
            .collect::<Vec<_>>()
            .join(", ");
        std::fs::write(
            v.join("vertical.toml"),
            format!(
                "schema_version = \"1\"\n\
                 [vertical]\nid = \"{id}\"\nname = \"X\"\nversion = \"0.1.0\"\n\
                 [escalation]\nhuman_approval_required = [{approval_toml}]\n"
            ),
        )
        .unwrap();
    }

    #[test]
    fn approval_set_empty_for_unbound_session() {
        let s = build_vertical_approval_set(None);
        assert!(s.is_empty());
    }

    #[test]
    fn approval_set_carries_manifest_list() {
        let dir = tempfile::tempdir().unwrap();
        write_vertical_with_approval(dir.path(), "gated", &["delegate_task", "terminal"]);
        let _g = EnvGuard::new(dir.path());
        let s = build_vertical_approval_set(Some("gated"));
        assert!(s.contains("delegate_task"));
        assert!(s.contains("terminal"));
        assert_eq!(s.len(), 2);
    }

    #[test]
    fn approval_set_empty_for_unknown_id() {
        let dir = tempfile::tempdir().unwrap();
        write_vertical_with_approval(dir.path(), "gated", &["x"]);
        let _g = EnvGuard::new(dir.path());
        let s = build_vertical_approval_set(Some("ghost"));
        assert!(s.is_empty());
    }

    #[test]
    fn approval_set_empty_when_no_escalation_section() {
        let dir = tempfile::tempdir().unwrap();
        write_vertical_no_role(dir.path()); // no [escalation] either
        let _g = EnvGuard::new(dir.path());
        let s = build_vertical_approval_set(Some("noprompt"));
        assert!(s.is_empty());
    }
}
