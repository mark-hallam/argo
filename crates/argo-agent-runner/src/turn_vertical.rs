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
        std::env::set_var(argo_vertical::ENV_VERTICALS_DIR, dir.path());
        let got = resolve_vertical_for_session(Some("demo"));
        std::env::remove_var(argo_vertical::ENV_VERTICALS_DIR);
        assert_eq!(got.as_deref(), Some("you are demo agent"));
    }

    #[test]
    fn resolve_vertical_returns_none_for_unknown_id() {
        let dir = tempfile::tempdir().unwrap();
        write_vertical_with_prompt(dir.path(), "x");
        std::env::set_var(argo_vertical::ENV_VERTICALS_DIR, dir.path());
        let got = resolve_vertical_for_session(Some("ghost"));
        std::env::remove_var(argo_vertical::ENV_VERTICALS_DIR);
        assert!(got.is_none());
    }

    #[test]
    fn resolve_vertical_returns_none_when_no_role_section() {
        let dir = tempfile::tempdir().unwrap();
        write_vertical_no_role(dir.path());
        std::env::set_var(argo_vertical::ENV_VERTICALS_DIR, dir.path());
        let got = resolve_vertical_for_session(Some("noprompt"));
        std::env::remove_var(argo_vertical::ENV_VERTICALS_DIR);
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
        std::env::set_var(argo_vertical::ENV_VERTICALS_DIR, dir.path());
        let s = build_vertical_deny_set(Some("ro"));
        std::env::remove_var(argo_vertical::ENV_VERTICALS_DIR);
        assert!(s.contains("file_write"));
        assert!(s.contains("terminal"));
        assert!(!s.contains("file_read"));
        assert_eq!(s.len(), 2);
    }

    #[test]
    fn deny_set_empty_for_unknown_id() {
        let dir = tempfile::tempdir().unwrap();
        write_vertical_with_tools(dir.path(), "ro", &[], &["x"]);
        std::env::set_var(argo_vertical::ENV_VERTICALS_DIR, dir.path());
        let s = build_vertical_deny_set(Some("ghost"));
        std::env::remove_var(argo_vertical::ENV_VERTICALS_DIR);
        assert!(s.is_empty());
    }

    #[test]
    fn deny_set_empty_when_no_tools_section() {
        let dir = tempfile::tempdir().unwrap();
        write_vertical_no_role(dir.path()); // no [tools] either
        std::env::set_var(argo_vertical::ENV_VERTICALS_DIR, dir.path());
        let s = build_vertical_deny_set(Some("noprompt"));
        std::env::remove_var(argo_vertical::ENV_VERTICALS_DIR);
        assert!(s.is_empty());
    }
}
