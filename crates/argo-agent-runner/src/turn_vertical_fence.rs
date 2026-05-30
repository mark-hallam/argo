//! Worker-bound least-privilege capability fence for Genesis-hired
//! teammates (DEC-025 Genesis capability fence, Phase 2).
//!
//! The runtime layer that decides, per session, whether the acting
//! principal should be bound to the all-access `owner` role (the
//! historical default) or to a *derived, deny-by-default least-privilege
//! profile* scoped to a Genesis-generated teammate's genuine needs.
//!
//! ## Why this lives here, not in `argo-capability`
//!
//! [`argo_capability::WorkerCapabilityProfile`] is the canonical,
//! manifest-agnostic type (deny-by-default tool allow-set + the
//! irreversibility gate). This module is the thin runtime glue that
//! (a) finds a vertical's manifest — including Genesis verticals under
//! the `.genesis/<tenant>/<id>/` tree that `argo_vertical::load_dir`
//! does NOT walk — and (b) maps its declared `[actions]` / `[tools]` /
//! `[mcp_servers]` into a profile.
//!
//! ## Posture
//!
//! - **Genesis-generated verticals** (resolved under `.genesis/`, or
//!   carrying a Genesis-written `worker.toml`) → derive a least-privilege
//!   profile. This is the hole the fence closes: a generated teammate no
//!   longer inherits the owner's `*` keys.
//! - **Hand-authored catalog verticals** → `None` (no fence). These are
//!   operator-authored + operator-trusted; they keep the existing
//!   behaviour. (A future tightening can fence these too, but that is
//!   out of scope for closing the *generated-teammate* `*` hole.)

use argo_capability::WorkerCapabilityProfile;
use std::path::{Path, PathBuf};

/// Resolve a least-privilege [`WorkerCapabilityProfile`] for a
/// vertical-bound session, IF the bound vertical is Genesis-generated.
///
/// Returns `None` (→ caller keeps the historical `owner` binding) when:
/// - the session isn't vertical-bound,
/// - the vertical can't be found on disk,
/// - or the vertical is a hand-authored catalog vertical (not Genesis).
///
/// Degrades silently to `None` on any IO / parse error so a missing or
/// malformed manifest never aborts the turn — the fence is a *tightening*
/// and its absence must never brick dispatch. (Fail-open on *resolution*
/// is acceptable because the only consequence is the historical
/// behaviour; the fence itself, once derived, is fail-closed.)
#[must_use]
pub(crate) fn resolve_worker_profile_for_session(
    vertical_id: Option<&str>,
) -> Option<WorkerCapabilityProfile> {
    let vid = vertical_id?;
    let root = argo_vertical::default_dir();
    let dir = genesis_vertical_dir_in(&root, vid)?;
    derive_profile_from_dir(&dir)
}

/// Locate a Genesis-generated vertical's on-disk directory under
/// `<verticals_root>/.genesis/<tenant_slug>/<id>/`. Mirrors the
/// gateway hire-gate's resolver (`routes_workers_hire_gate.rs`) so the
/// runtime fence and the hire fence agree on what "is a Genesis
/// vertical" means.
fn genesis_vertical_dir_in(verticals_root: &Path, id: &str) -> Option<PathBuf> {
    let entries = std::fs::read_dir(verticals_root.join(".genesis")).ok()?;
    entries.flatten().find_map(|e| {
        let dir = e.path().join(id);
        dir.join("vertical.toml").is_file().then_some(dir)
    })
}

/// Parse the `vertical.toml` at `dir` and derive a least-privilege
/// profile from its declared `[actions]` / `[tools]` / `[mcp_servers]`.
fn derive_profile_from_dir(dir: &Path) -> Option<WorkerCapabilityProfile> {
    let manifest_path = dir.join("vertical.toml");
    let src = std::fs::read_to_string(&manifest_path).ok()?;
    let manifest = argo_vertical::parse(&src).ok()?;
    Some(derive_profile_from_manifest(&manifest))
}

/// Pure mapping from a parsed manifest to a least-privilege profile.
///
/// - declared actions ← `[actions]` keys (the teammate's vocabulary),
/// - allowed tools ← `[tools].allowed`,
/// - denied tools ← `[tools].denied` (deny wins),
/// - integration servers ← `[mcp_servers]` keys (recorded as draft-only,
///   NOT promoted into the live allow-set per locked decision #6).
#[must_use]
pub(crate) fn derive_profile_from_manifest(
    manifest: &argo_vertical::VerticalManifest,
) -> WorkerCapabilityProfile {
    let declared_actions: Vec<String> = manifest
        .actions
        .as_ref()
        .map(|a| a.actions.keys().cloned().collect())
        .unwrap_or_default();
    let (allowed, denied): (Vec<String>, Vec<String>) = manifest
        .tools
        .as_ref()
        .map(|t| (t.allowed.clone(), t.denied.clone()))
        .unwrap_or_default();
    let integration_servers: Vec<String> = manifest
        .mcp_servers
        .as_ref()
        .map(|m| m.servers.keys().cloned().collect())
        .unwrap_or_default();
    WorkerCapabilityProfile::derive(declared_actions, allowed, denied, integration_servers)
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_vertical::{
        ActionDeclaration, ActionsSection, McpServerDeclaration, McpServersSection, ToolsSection,
        VerticalIdentity, VerticalManifest, SCHEMA_VERSION,
    };
    use std::collections::BTreeMap;

    fn manifest_with(
        actions: &[&str],
        allowed: &[&str],
        denied: &[&str],
        mcp_servers: &[&str],
    ) -> VerticalManifest {
        let mut action_map: BTreeMap<String, ActionDeclaration> = BTreeMap::new();
        for a in actions {
            action_map.insert((*a).to_string(), ActionDeclaration::default());
        }
        let mut servers: BTreeMap<String, McpServerDeclaration> = BTreeMap::new();
        for s in mcp_servers {
            servers.insert(
                (*s).to_string(),
                McpServerDeclaration {
                    transport: "stdio".into(),
                    command: "npx".into(),
                    args: vec![],
                },
            );
        }
        VerticalManifest {
            schema_version: SCHEMA_VERSION.to_string(),
            vertical: VerticalIdentity {
                id: "demo".into(),
                name: "Demo".into(),
                version: "0.0.1".into(),
                ..VerticalIdentity::default()
            },
            actions: Some(ActionsSection {
                actions: action_map,
            }),
            tools: Some(ToolsSection {
                allowed: allowed.iter().map(|s| (*s).to_string()).collect(),
                denied: denied.iter().map(|s| (*s).to_string()).collect(),
            }),
            mcp_servers: (!servers.is_empty()).then_some(McpServersSection { servers }),
            ..VerticalManifest::default()
        }
    }

    #[test]
    fn derive_maps_actions_and_allowed_tools() {
        let m = manifest_with(
            &["draft_reply", "summarize_ticket"],
            &["web_search"],
            &[],
            &[],
        );
        let profile = derive_profile_from_manifest(&m);
        assert!(profile.permits_tool("draft_reply"));
        assert!(profile.permits_tool("summarize_ticket"));
        assert!(profile.permits_tool("web_search"));
        assert!(!profile.permits_tool("terminal"));
    }

    #[test]
    fn derive_keeps_mcp_servers_draft_only() {
        let m = manifest_with(&["draft_reply"], &[], &[], &["gmail", "stripe"]);
        let profile = derive_profile_from_manifest(&m);
        // Recorded as draft integrations, NOT live dispatchable tools.
        assert_eq!(profile.draft_integrations(), vec!["gmail", "stripe"]);
        assert!(!profile.permits_tool("gmail"));
        assert!(!profile.permits_tool("stripe"));
    }

    #[test]
    fn derive_respects_deny_list() {
        let m = manifest_with(&["send_email"], &["send_email"], &["send_email"], &[]);
        let profile = derive_profile_from_manifest(&m);
        assert!(!profile.permits_tool("send_email"));
    }

    #[test]
    fn empty_manifest_yields_deny_all_profile() {
        let m = VerticalManifest {
            schema_version: SCHEMA_VERSION.to_string(),
            vertical: VerticalIdentity {
                id: "bare".into(),
                name: "Bare".into(),
                version: "0.0.1".into(),
                ..VerticalIdentity::default()
            },
            ..VerticalManifest::default()
        };
        let profile = derive_profile_from_manifest(&m);
        assert!(profile.allowed_tools().is_empty());
        assert!(!profile.permits_tool("anything"));
    }

    #[test]
    fn resolve_returns_none_for_unbound_session() {
        assert!(resolve_worker_profile_for_session(None).is_none());
    }
}
