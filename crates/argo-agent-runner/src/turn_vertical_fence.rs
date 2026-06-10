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

use argo_capability::{TruthBoundaryPolicy, WorkerCapabilityProfile};
use argo_store::SessionStore;
use argo_types::TenantId;
use std::path::{Path, PathBuf};

/// Filename the synthesized boundaries policy is written under, next to the
/// generated vertical. Mirrors `argo_genesis::policy_synth::
/// BOUNDARIES_POLICY_FILENAME` BY VALUE — this crate does not depend on
/// argo-genesis (same posture as [`sanitize_tenant`] below mirroring the
/// argo-cli copy). The two MUST stay byte-identical: the gateway writes the
/// policy under this exact name, and this fence reads it back under the same
/// name. A drift would silently unbind every operator-configured policy.
const BOUNDARIES_POLICY_FILENAME: &str = "truth-boundary-policy.toml";
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

/// Genesis P3 probation signal for a session (DEC-025 #2). Read off the
/// SAME worker-row fetch as [`GenesisProvenance`] so the hot loop pays
/// for one session-boot store read, not per-action I/O (the design doc's
/// explicit requirement). While `InProbation`, EVERY emitted action is
/// forced approval-gated (held pending) at the dispatch seam regardless
/// of the worker's normal policy; once the stamped window passes, the
/// teammate graduates and this resolves to `Normal`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProbationStatus {
    /// The bound worker carries a `probation_until=` marker still in the
    /// future — gate every action.
    InProbation,
    /// No worker, no marker, or the window has passed — normal policy.
    Normal,
}

impl ProbationStatus {
    /// `true` when the worker is still inside its probation window.
    pub(crate) fn is_active(self) -> bool {
        matches!(self, ProbationStatus::InProbation)
    }
}

/// Genesis P5 (a) truth-boundary binding signal for a session (DEC-025 #6).
/// Read off the SAME worker-row fetch as [`GenesisProvenance`] +
/// [`ProbationStatus`]. When `Bound`, the runner loads the conservative
/// default truth-boundary policy (`argo_capability::genesis_default_policy`)
/// scoped to the worker's tenant, replacing the env-var-only activation —
/// so a generated teammate's declared financial/legal/medical/factual claims
/// escalate (held for operator approval) instead of shipping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TruthBoundaryBinding {
    /// The worker carries a `truth_boundary_policy_id=` marker → bind the
    /// conservative default policy at session boot.
    Bound,
    /// No worker / no marker → keep the historical (env-var) activation.
    None,
}

/// Combined Genesis signals for a session, all derived from a single
/// worker-row read. Returned by [`genesis_signals_for_worker`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GenesisSignals {
    pub(crate) provenance: GenesisProvenance,
    pub(crate) probation: ProbationStatus,
    pub(crate) truth_boundary: TruthBoundaryBinding,
}

/// Read the worker row ONCE and derive both the Genesis capability-fence
/// provenance signal AND the Genesis P3 probation signal. The fence and
/// the probation gate both need the same persisted `provenance_chain`, so
/// folding them into one read keeps the hot loop's session-boot cost at a
/// single store fetch (the design doc's "load once at session boot, not
/// per-action I/O" requirement).
///
/// Failure / missing-row semantics:
/// - No `worker_id` (ad-hoc session) → `{ None, Normal }` (no fence, no
///   probation).
/// - Worker row carries `genesis_plan_id=` → `provenance = Genesis`
///   (fence, fail-closed).
/// - Worker row carries `probation_until=` still in the future →
///   `probation = InProbation` (gate every action).
/// - Worker row can't be loaded (transient store error / missing row) →
///   `{ None, Normal }` with a warning. We can't prove probation from a
///   failed read; bricking the turn is worse than the historical
///   binding, and the fence's manifest-resolution fail-closed path still
///   protects any worker we DO classify as Genesis. Note this is a
///   deliberate fail-OPEN on probation for an unreadable row — flagged
///   for the conductor's adversarial pass; a future tightening could
///   fail closed here once the read is proven non-flaky under load.
pub(crate) async fn genesis_signals_for_worker(
    store: &Arc<SessionStore>,
    tenant_id: TenantId,
    worker_id: Option<argo_store::WorkerId>,
) -> GenesisSignals {
    let normal = GenesisSignals {
        provenance: GenesisProvenance::None,
        probation: ProbationStatus::Normal,
        truth_boundary: TruthBoundaryBinding::None,
    };
    let Some(worker_id) = worker_id else {
        return normal;
    };
    let identities = argo_identity::IdentityStore::new(store);
    match identities.get_worker(tenant_id, worker_id).await {
        Ok(Some(worker)) => {
            let chain = &worker.persona.provenance_chain;
            let provenance = if argo_identity::genesis_plan_id_from_provenance(chain).is_some() {
                GenesisProvenance::Genesis
            } else {
                GenesisProvenance::None
            };
            let probation = if argo_identity::is_in_probation(chain, chrono::Utc::now()) {
                ProbationStatus::InProbation
            } else {
                ProbationStatus::Normal
            };
            // P5 (a): a `truth_boundary_policy_id=` marker → bind the
            // conservative default claim policy. We pin the marker value to
            // the canonical constant so a future drift between the two
            // mirrored constants surfaces in debug builds rather than
            // silently unbinding the policy.
            let truth_boundary = match argo_identity::truth_boundary_policy_id_from_provenance(chain)
            {
                Some(id) => {
                    debug_assert_eq!(
                        id, argo_capability::GENESIS_DEFAULT_POLICY_ID,
                        "provenance truth-boundary policy id drifted from the canonical constant"
                    );
                    TruthBoundaryBinding::Bound
                }
                None => TruthBoundaryBinding::None,
            };
            GenesisSignals {
                provenance,
                probation,
                truth_boundary,
            }
        }
        Ok(None) => normal,
        Err(e) => {
            tracing::warn!(
                target: "agent_runner::fence",
                error = %e,
                worker_id = %worker_id.0,
                "could not read worker provenance for capability fence + probation; \
                 falling back to historical binding (no fence, no probation)"
            );
            normal
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
    tenant_id: TenantId,
    vertical_id: Option<&str>,
    provenance: GenesisProvenance,
) -> Option<WorkerCapabilityProfile> {
    resolve_worker_profile_in(
        &argo_vertical::default_dir(),
        tenant_id,
        vertical_id,
        provenance,
    )
}

/// Root-parameterized core of [`resolve_worker_profile_for_session`].
/// Split out so tests can point at a fixture verticals tree without
/// mutating the process-global `$ARGO_VERTICALS_DIR` (which would race
/// other tests in the same binary). Production callers go through the
/// public wrapper, which passes [`argo_vertical::default_dir()`].
///
/// `tenant_id` scopes the `.genesis/<tenant>/` lookup to the worker's OWN
/// tenant (P5 (b) tenant-match enforcement): a manifest another tenant wrote
/// under the same `vertical_id` is NOT resolved, so a Genesis worker can
/// never derive its capability profile from a foreign tenant's manifest. A
/// cross-tenant-only id falls through to the catalog lookup and then to the
/// deny-all fail-closed profile.
#[must_use]
pub(crate) fn resolve_worker_profile_in(
    verticals_root: &Path,
    tenant_id: TenantId,
    vertical_id: Option<&str>,
    provenance: GenesisProvenance,
) -> Option<WorkerCapabilityProfile> {
    match provenance {
        GenesisProvenance::None => None,
        // Fail CLOSED for a known-Genesis worker: an unresolved /
        // malformed manifest yields a deny-all profile, NEVER `owner`.
        GenesisProvenance::Genesis => Some(
            vertical_id
                .and_then(|vid| resolve_genesis_manifest(verticals_root, tenant_id, vid))
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
/// lives: first under the worker's OWN `.genesis/<tenant>/<id>/vertical.toml`
/// (SSE-hired workers; tenant-scoped per P5 (b)), then in the hand-authored
/// catalog via `load_dir` (first-hire / catalog-bound Genesis workers).
/// Returns `None` only when neither lookup yields a parseable manifest. A
/// manifest another tenant wrote under the same `vertical_id` is invisible
/// here.
#[must_use]
fn resolve_genesis_manifest(
    verticals_root: &Path,
    tenant_id: TenantId,
    vertical_id: &str,
) -> Option<argo_vertical::VerticalManifest> {
    if let Some(dir) = genesis_vertical_dir_in(verticals_root, tenant_id, vertical_id) {
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

/// Genesis P5 (a) per-worker RUNTIME loading: resolve the operator's
/// SYNTHESIZED truth-boundary policy persisted next to the worker's Genesis
/// vertical (`.genesis/<tenant_slug>/<vertical_id>/truth-boundary-policy.toml`
/// — the artifact the access-control panel writes). This is what makes the
/// operator's per-teammate guardrails actually bite at runtime instead of
/// being a stored artifact a process-global env path would have to point at.
///
/// Keyed on the SERVER-AUTHORITATIVE `tenant_id` + `vertical_id` (never a
/// client/spec field). Returns `None` when there is no per-vertical policy
/// file (or it can't be read/parsed) — the caller then falls back to the
/// conservative default. Production callers go through this wrapper, which
/// passes [`argo_vertical::default_dir()`].
#[must_use]
pub(crate) fn load_synthesized_policy_for_worker(
    tenant_id: TenantId,
    vertical_id: Option<&str>,
) -> Option<TruthBoundaryPolicy> {
    load_synthesized_policy_in(&argo_vertical::default_dir(), tenant_id, vertical_id)
}

/// Root-parameterized core of [`load_synthesized_policy_for_worker`], so tests
/// can point at a fixture verticals tree without mutating the process-global
/// `$ARGO_VERTICALS_DIR`.
///
/// The dir lookup is already tenant-scoped (`genesis_vertical_dir_in` only
/// resolves under THIS tenant's `.genesis/<slug>/` segment), so a worker can
/// never reach another tenant's vertical dir. The trailing `tenant_id` match
/// on the loaded policy is defense-in-depth: even a hand-edited file carrying
/// a foreign `tenant_id` is refused rather than applied to the wrong tenant.
#[must_use]
pub(crate) fn load_synthesized_policy_in(
    verticals_root: &Path,
    tenant_id: TenantId,
    vertical_id: Option<&str>,
) -> Option<TruthBoundaryPolicy> {
    let vid = vertical_id?;
    let dir = genesis_vertical_dir_in(verticals_root, tenant_id, vid)?;
    let path = dir.join(BOUNDARIES_POLICY_FILENAME);
    let policy = argo_capability::load_policy(&path).ok()?;
    (policy.tenant_id == tenant_id).then_some(policy)
}

/// Locate a Genesis-generated vertical's on-disk directory under the
/// worker's OWN tenant segment `<verticals_root>/.genesis/<tenant_slug>/<id>/`
/// (P5 (b) tenant-match enforcement). Mirrors the gateway hire-gate's
/// tenant-scoped resolver (`genesis_tenant::genesis_vertical_dir_for_tenant`)
/// so the runtime fence and the hire fence agree on what "is THIS tenant's
/// Genesis vertical" means — neither scans other tenants' segments.
fn genesis_vertical_dir_in(
    verticals_root: &Path,
    tenant_id: TenantId,
    id: &str,
) -> Option<PathBuf> {
    // Reject any id that could escape this tenant's `.genesis/<slug>/` segment
    // before it is joined onto the path. `Path::join` does NOT normalize `..`,
    // so a raw join of an attacker-shaped `id` (`../<victim>/<id>`, an absolute
    // path, a NUL) physically resolves into a FOREIGN tenant's segment. The
    // policy loader's `tenant_id` backstop catches the synthesized-policy read,
    // but the manifest/profile path (`resolve_genesis_manifest`) has no such
    // backstop — so the guard belongs HERE, at the shared resolver, closing
    // every caller at once (security audit 2026-06-10).
    if !is_safe_vertical_id(id) {
        return None;
    }
    let slug = sanitize_tenant(&tenant_id.to_string());
    let dir = verticals_root.join(".genesis").join(slug).join(id);
    dir.join("vertical.toml").is_file().then_some(dir)
}

/// A Genesis vertical id is a server-derived slug. Accept only
/// `[A-Za-z0-9_-]` (every real catalog + synthesized id is of this shape) and
/// reject anything that could traverse out of the tenant segment — a path
/// separator, a `.` (and therefore `..`), an absolute prefix, or a NUL. Strict
/// allow-list, not a blocklist, so a novel escape sequence cannot slip through.
#[must_use]
fn is_safe_vertical_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Canonical tenant-id → `.genesis/<slug>/` segment map. Mirrors
/// `argo_cli::handlers::gateway::genesis_tenant::sanitize_tenant` by value
/// (this crate does not depend on `argo-cli`); the two MUST stay
/// byte-identical so the slug the gateway writes a vertical under matches
/// the slug this fence reads it under.
#[must_use]
fn sanitize_tenant(tenant_id: &str) -> String {
    let cleaned: String = tenant_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = cleaned.trim_matches('-');
    if trimmed.is_empty() {
        "anonymous".to_string()
    } else {
        trimmed.to_string()
    }
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
    // DEC-049: grant child-teammate spawn (`Spawn{Subagent}`) only when the
    // manifest opts in (lists `delegate_task` in `[tools].allowed`) AND the
    // global `ARGO_WORKER_SWARM` flag is on. Both required → doubly default-off.
    let can_spawn_subagent =
        swarm_enabled() && allowed.iter().any(|t| t.trim() == "delegate_task");
    WorkerCapabilityProfile::derive(declared_actions, allowed, denied, integration_servers)
        .with_subagent_spawn(can_spawn_subagent)
}

/// DEC-049 master switch for worker child-teammate spawning. Reads
/// `ARGO_WORKER_SWARM`; default OFF. Accepts `1`/`true`/`yes`/`on`
/// (case-insensitive) as on, everything else (including unset) as off —
/// mirrors the `ARGO_DUTY_AUTOFIRE` posture. Operator-only flip after a
/// blast-radius review.
#[must_use]
pub(crate) fn swarm_enabled() -> bool {
    matches!(
        std::env::var("ARGO_WORKER_SWARM")
            .ok()
            .as_deref()
            .map(|v| v.trim().to_ascii_lowercase())
            .as_deref(),
        Some("1" | "true" | "yes" | "on")
    )
}

#[cfg(test)]
#[path = "turn_vertical_fence_probation_tests.rs"]
mod probation_tests;

#[cfg(test)]
#[path = "turn_vertical_fence_derive_tests.rs"]
mod derive_tests;

#[cfg(test)]
#[path = "turn_vertical_fence_policy_load_tests.rs"]
mod policy_load_tests;
