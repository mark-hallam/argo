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
//! (a) finds a vertical's manifest — whether the Genesis worker's
//! manifest lives under the `.genesis/<tenant>/<id>/` tree that
//! `argo_vertical::load_dir` does NOT walk, OR in the hand-authored
//! catalog (`load_dir`) — and (b) maps its declared `[actions]` /
//! `[tools]` / `[mcp_servers]` into a profile.
//!
//! ## Posture — key off Genesis PROVENANCE, not directory location
//!
//! The fence used to key off where the manifest lived: a profile was
//! derived ONLY for verticals resolved under `.genesis/`. That left a
//! P0 hole — a teammate hired via `POST /api/v1/onboarding/first-hire`
//! (the demo's Scene-1 path) is a genuine Genesis hire (its persona
//! carries `genesis_plan_id` provenance) but is bound to a **catalog**
//! `vertical_id` that is never written to `.genesis/`. Such a worker
//! resolved to `None` → the caller bound it to `owner` (`*` keys). The
//! fence now keys off the worker's **Genesis provenance** (carried on
//! `persona.provenance_chain`), so ANY worker hired through Genesis is
//! fenced regardless of where its manifest lives.
//!
//! - **Worker WITH Genesis provenance** → derive a least-privilege
//!   profile from the bound vertical's manifest, looked up under
//!   `.genesis/` first and then the catalog. If the manifest cannot be
//!   resolved (IO / parse error / not found), the fence **fails CLOSED**
//!   with a deny-all profile — it must NEVER fall back to `owner` for a
//!   known-Genesis session.
//! - **Worker with NO Genesis provenance** (ad-hoc session, or a
//!   hand-authored catalog vertical the operator authored + trusts) →
//!   `None` (no fence; historical `owner` binding). (A future tightening
//!   can fence these too, but that is out of scope here.)

use argo_capability::WorkerCapabilityProfile;
use argo_store::SessionStore;
use argo_types::TenantId;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Genesis-provenance signal for a session, derived from the worker row
/// at the call site (`build_turn_dispatch_ctx`) where the `IdentityStore`
/// is available. Carrying the decision in a tiny enum keeps the
/// store-touching read out of this pure resolver module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GenesisProvenance {
    /// The bound worker's persona carries a `genesis_plan_id=` marker —
    /// it was hired through Genesis and MUST be fenced (fail-closed).
    Genesis,
    /// No worker, or the worker carries no Genesis provenance — keep the
    /// historical `owner` binding (no fence).
    None,
}

/// Decide whether the session's bound worker was hired through Genesis,
/// by reading the persisted persona's `provenance_chain` for the
/// canonical `genesis_plan_id=` marker (the SAME marker both the
/// first-hire seam and the SSE hire stamp). This is the single
/// store-touching read that lets the pure resolver below fence a Genesis
/// worker regardless of where its vertical manifest lives.
///
/// - No `worker_id` (ad-hoc session) → `None` (no fence).
/// - Worker row carries a `genesis_plan_id=` provenance entry →
///   `Genesis` (fence, fail-closed).
/// - Worker row loads but carries no marker → `None` (hand-authored
///   catalog / pre-Genesis worker; historical `owner` binding).
/// - Worker row can't be loaded (transient store error / missing row) →
///   `None`, with a warning. We can't prove it's Genesis from a failed
///   read, and bricking the turn is worse than the historical binding;
///   the manifest-resolution fail-closed path still protects any worker
///   we DO classify as Genesis.
pub(crate) async fn genesis_provenance_for_worker(
    store: &Arc<SessionStore>,
    tenant_id: TenantId,
    worker_id: Option<argo_store::WorkerId>,
) -> GenesisProvenance {
    let Some(worker_id) = worker_id else {
        return GenesisProvenance::None;
    };
    let identities = argo_identity::IdentityStore::new(store);
    match identities.get_worker(tenant_id, worker_id).await {
        Ok(Some(worker)) => {
            if argo_identity::genesis_plan_id_from_provenance(&worker.persona.provenance_chain)
                .is_some()
            {
                GenesisProvenance::Genesis
            } else {
                GenesisProvenance::None
            }
        }
        Ok(None) => GenesisProvenance::None,
        Err(e) => {
            tracing::warn!(
                target: "agent_runner::fence",
                error = %e,
                worker_id = %worker_id.0,
                "could not read worker provenance for capability fence; \
                 falling back to historical binding"
            );
            GenesisProvenance::None
        }
    }
}

/// Resolve a least-privilege [`WorkerCapabilityProfile`] for a session,
/// keyed off whether the bound worker was hired through Genesis.
///
/// - `provenance = None` → returns `None` (→ caller keeps the historical
///   `owner` binding) for ad-hoc / hand-authored-catalog sessions.
/// - `provenance = Genesis` → ALWAYS returns `Some(profile)`. It derives
///   the profile from the bound vertical's manifest (looked up under
///   `.genesis/` first, then the catalog). If the manifest can't be
///   resolved or parsed, it returns a **deny-all** profile — the fence
///   **fails CLOSED** rather than leaking the owner's `*` keys to a
///   known-Genesis worker.
#[must_use]
pub(crate) fn resolve_worker_profile_for_session(
    vertical_id: Option<&str>,
    provenance: GenesisProvenance,
) -> Option<WorkerCapabilityProfile> {
    resolve_worker_profile_in(&argo_vertical::default_dir(), vertical_id, provenance)
}

/// Root-parameterized core of [`resolve_worker_profile_for_session`].
/// Split out so tests can point at a fixture verticals tree without
/// mutating the process-global `$ARGO_VERTICALS_DIR` (which would race
/// other tests in the same binary). Production callers go through the
/// public wrapper, which passes [`argo_vertical::default_dir()`].
#[must_use]
pub(crate) fn resolve_worker_profile_in(
    verticals_root: &Path,
    vertical_id: Option<&str>,
    provenance: GenesisProvenance,
) -> Option<WorkerCapabilityProfile> {
    match provenance {
        GenesisProvenance::None => None,
        // Fail CLOSED for a known-Genesis worker: an unresolved /
        // malformed manifest yields a deny-all profile, NEVER `owner`.
        GenesisProvenance::Genesis => Some(
            vertical_id
                .and_then(|vid| resolve_genesis_manifest(verticals_root, vid))
                .map_or_else(deny_all_profile, |m| derive_profile_from_manifest(&m)),
        ),
    }
}

/// The deny-all least-privilege profile: an empty allow-set permits
/// nothing autonomously (only the always-safe minimal teammate tools
/// survive into the capability set). This is the fail-closed posture for
/// a known-Genesis worker whose manifest can't be resolved.
#[must_use]
fn deny_all_profile() -> WorkerCapabilityProfile {
    WorkerCapabilityProfile::derive(
        Vec::<String>::new(),
        Vec::<String>::new(),
        Vec::<String>::new(),
        Vec::<String>::new(),
    )
}

/// Locate + parse a Genesis worker's bound vertical manifest, wherever it
/// lives: first under `.genesis/<tenant>/<id>/vertical.toml` (SSE-hired
/// workers), then in the hand-authored catalog via `load_dir`
/// (first-hire / catalog-bound Genesis workers). Returns `None` only when
/// neither lookup yields a parseable manifest.
#[must_use]
fn resolve_genesis_manifest(
    verticals_root: &Path,
    vertical_id: &str,
) -> Option<argo_vertical::VerticalManifest> {
    if let Some(dir) = genesis_vertical_dir_in(verticals_root, vertical_id) {
        if let Some(manifest) = parse_manifest_in_dir(&dir) {
            return Some(manifest);
        }
    }
    manifest_from_catalog(verticals_root, vertical_id)
}

/// Look the id up in the on-disk catalog (`argo_vertical::load_dir`,
/// which does NOT walk `.genesis/`) and clone its parsed manifest.
#[must_use]
fn manifest_from_catalog(root: &Path, id: &str) -> Option<argo_vertical::VerticalManifest> {
    let (registry, _errors) = argo_vertical::load_dir(root).ok()?;
    registry.get(id).map(|loaded| loaded.manifest.clone())
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

/// Parse the `vertical.toml` at `dir`. `None` on any IO / parse error.
fn parse_manifest_in_dir(dir: &Path) -> Option<argo_vertical::VerticalManifest> {
    let src = std::fs::read_to_string(dir.join("vertical.toml")).ok()?;
    argo_vertical::parse(&src).ok()
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
    fn non_genesis_worker_is_not_fenced() {
        // A session whose worker carries NO Genesis provenance keeps the
        // historical owner binding — `None` regardless of vertical id.
        // (Uses the public wrapper; `default_dir()` is irrelevant on the
        // `None` arm — it short-circuits before any disk read.)
        assert!(resolve_worker_profile_for_session(None, GenesisProvenance::None).is_none());
        assert!(resolve_worker_profile_for_session(
            Some("ai-receptionist"),
            GenesisProvenance::None
        )
        .is_none());
    }

    #[test]
    fn genesis_worker_bound_to_catalog_id_is_fenced() {
        // The bypass this closes: a Genesis-provenance worker bound to a
        // CATALOG vertical id (not under `.genesis/`) MUST resolve to a
        // least-privilege profile derived from that catalog manifest — not
        // `None` (which the caller would bind to owner). Race-free: points
        // the resolver at a tempdir catalog via the root-parameterized
        // inner fn instead of mutating the process-global env var.
        let tmp = tempfile::tempdir().expect("tmpdir");
        let vdir = tmp.path().join("intern");
        std::fs::create_dir_all(&vdir).expect("mkdir");
        std::fs::write(
            vdir.join("vertical.toml"),
            "schema_version = \"1\"\n\
             [vertical]\nid = \"intern\"\nname = \"Intern\"\nversion = \"0.1.0\"\n\
             [actions.draft_reply]\nrisk = \"low\"\nrequires_approval = false\n\
             description = \"Draft a reply.\"\n\
             [tools]\nallowed = [\"web_search\"]\ndenied = []\n",
        )
        .expect("write manifest");

        let profile =
            resolve_worker_profile_in(tmp.path(), Some("intern"), GenesisProvenance::Genesis)
                .expect("Genesis worker must always yield a profile, never None/owner");
        assert!(
            profile.permits_tool("draft_reply"),
            "declared action allowed"
        );
        assert!(profile.permits_tool("web_search"), "allowed tool allowed");
        assert!(
            !profile.permits_tool("terminal"),
            "undeclared tool must stay denied under the fence"
        );
    }

    #[test]
    fn genesis_worker_with_unresolvable_manifest_fails_closed() {
        // A known-Genesis worker whose manifest can't be resolved (no
        // matching .genesis/ dir, not in the catalog) must NOT fall back
        // to owner — it gets a deny-all profile so the fence fails CLOSED.
        let tmp = tempfile::tempdir().expect("tmpdir");
        let profile = resolve_worker_profile_in(
            tmp.path(),
            Some("a-vertical-that-does-not-exist-anywhere"),
            GenesisProvenance::Genesis,
        )
        .expect("known-Genesis session must always yield a profile, never None");
        assert!(
            profile.allowed_tools().is_empty(),
            "fail-closed profile must have an empty allow-set"
        );
        assert!(!profile.permits_tool("terminal"));
        assert!(!profile.permits_tool("send_email"));
    }

    #[test]
    fn genesis_worker_with_no_vertical_id_fails_closed() {
        // Defensive: a Genesis worker with a missing vertical binding is
        // still fenced (deny-all), never owner.
        let profile = resolve_worker_profile_for_session(None, GenesisProvenance::Genesis)
            .expect("known-Genesis session must always yield a profile");
        assert!(profile.allowed_tools().is_empty());
    }
}
