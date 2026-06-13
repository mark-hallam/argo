//! Worker-bound least-privilege capability profile (DEC-025 Genesis
//! capability fence, Phase 2).
//!
//! ## The hole this closes
//!
//! Today every agent session binds its acting [`argo_types::Principal`]
//! to the `owner` role, which grants `ToolDispatch "*"`,
//! `NetEgress "**"`, and `SecretRead "*"`. A teammate *hired* through
//! Genesis therefore inherits the human owner's all-access keys and can
//! dispatch any tool. Genesis P0 proved the teammate is competent +
//! judged-safe, but it can still act with FULL owner privileges. That is
//! a live credential-leak hole, flagged by the adversarial bypass-hunt.
//!
//! ## The fence (deny-by-default, bound to the WORKER not the principal)
//!
//! A [`WorkerCapabilityProfile`] is the explicit, minimal set of tool
//! names a generated teammate genuinely needs — derived from its own
//! generated spec (its declared `[actions]` + `[tools].allowed`), with
//! `[tools].denied` always denied. Anything **not** in the allow-set is
//! denied. A novel/off-taxonomy role with no declared needs gets an
//! EMPTY allow-set → it can dispatch nothing autonomously.
//!
//! The profile is enforced by binding a Genesis worker's principal to a
//! role carrying ONLY this profile's grants (instead of `owner`) at the
//! existing dispatch seam: the standard `CapabilityEngine::check`
//! already hard-blocks any tool the role does not grant. Because the
//! grants are built from explicit, non-glob tool names, the fence can
//! only ever be *narrower* than the owner role, never wider — it is
//! immune to an LLM hallucinating an over-broad capability.
//!
//! ## Live-integration posture (DEC-025 locked #6)
//!
//! MCP / connector tool names are **never** auto-bound into the live
//! allow-set at birth. [`WorkerCapabilityProfile::derive`] keeps them in
//! a separate `draft_integrations` list (visible to the operator review
//! screen) but out of the dispatch allow-set, so a day-one teammate can
//! *draft and propose* but physically cannot transmit through a live
//! integration until a deliberate post-hire operator grant promotes it.
//!
//! ## Irreversibility gate (DEC-025 locked #3)
//!
//! [`is_irreversible_action`] is a fixed, deterministic deny-list of
//! money-/data-/identity-destroying verbs (`wire_funds`, `issue_refund`,
//! `drop_table`, …). Such an action ALWAYS requires human approval —
//! regardless of what the profile or the LLM judge say. This is the
//! deterministic floor under the (non-deterministic) semantic judge.

use argo_types::{Capability, CapabilitySet, SpawnKind};
use std::collections::BTreeSet;
use std::path::Path;

/// In-core file-tool names a fenced teammate may hold. When a worker has a
/// scoped `fs_root` AND its allow-set grants one of these, [`WorkerCapabilityProfile::to_capability_set`]
/// emits the matching `Fs*` capability scoped to `<fs_root>/**` — never wider.
/// `file_read` → `FsRead`; `file_write` / `file_patch` → `FsWrite`.
const FILE_READ_TOOL: &str = "file_read";
const FILE_WRITE_TOOLS: &[&str] = &["file_write", "file_patch"];

/// cu1d (DEC-050 G1 follow-up): read-only research tools whose dispatch
/// additionally demands `NetEgress`. When a worker's allow-set permits one
/// of these (deny-wins), [`WorkerCapabilityProfile::to_capability_set`]
/// emits `NetEgress{**}` so the dispatch can actually reach the network.
/// Without this, a manifest that explicitly allows `web_search` still
/// dead-ends at `rbac denied: … lacks NetEgress` — and the #729 evidence
/// gate then blocks every approval-gated proposal the worker makes.
const WEB_RESEARCH_TOOLS: &[&str] = &["web_search", "web_extract"];

#[path = "worker_profile_danger_lists.rs"]
mod danger_lists;
/// Re-exported so the public path (`argo_capability::IRREVERSIBLE_*`) and
/// the in-module references inside [`is_irreversible_action`] are unchanged
/// after the lists moved to a sibling file (500-LOC ceiling).
pub use danger_lists::{IRREVERSIBLE_ACTION_VERBS, IRREVERSIBLE_DANGER_STEMS};

/// `true` when `action_name` denotes an irreversible / business-ruining
/// action that must ALWAYS go through human approval.
///
/// Two-tier match (deny-by-default biases toward flagging):
/// - a [`IRREVERSIBLE_ACTION_VERBS`] entry appearing as a whole token
///   (or as a contiguous phrase for the multi-token entries), OR
/// - a [`IRREVERSIBLE_DANGER_STEMS`] entry appearing **anywhere** in the
///   name (so dangerous families inside a compound name are still caught).
///
/// ```
/// use argo_capability::is_irreversible_action;
/// assert!(is_irreversible_action("wire_funds"));
/// assert!(is_irreversible_action("issue_refund"));
/// assert!(is_irreversible_action("drop_table"));
/// assert!(is_irreversible_action("terminate_employee"));
/// // Synonyms / compounds the old token-exact list missed:
/// assert!(is_irreversible_action("process_payment"));
/// assert!(is_irreversible_action("transfer_funds"));
/// assert!(is_irreversible_action("extract_customer_records"));
/// assert!(is_irreversible_action("authenticate_portal"));
/// assert!(is_irreversible_action("empty_archived_storage"));
/// // Not irreversible — a draft / read action:
/// assert!(!is_irreversible_action("draft_summary"));
/// assert!(!is_irreversible_action("rewire_config"));
/// ```
#[must_use]
pub fn is_irreversible_action(action_name: &str) -> bool {
    let lowered = action_name.to_lowercase();
    let tokens: Vec<&str> = lowered.split('_').collect();
    let token_or_phrase = IRREVERSIBLE_ACTION_VERBS.iter().any(|verb| {
        if verb.contains('_') {
            // Multi-token verb (e.g. "wire_funds"): require the whole
            // phrase to appear as a contiguous run in the name.
            lowered.contains(verb)
        } else {
            tokens.iter().any(|tok| tok == verb)
        }
    });
    token_or_phrase
        || IRREVERSIBLE_DANGER_STEMS
            .iter()
            .any(|stem| lowered.contains(stem))
}

/// Always-on, non-side-effecting tools every generated teammate needs
/// to function (escalate, clarify, read its own memory/skills). None of
/// these transmit externally, charge money, or destroy data, so they
/// are safe to grant unconditionally inside the least-privilege fence.
pub const MINIMAL_TEAMMATE_TOOLS: &[&str] = &["clarify", "memory", "skill_view", "skills_list"];

/// A deny-by-default, worker-bound least-privilege capability profile.
///
/// The profile is the explicit allow-set of tool names a generated
/// teammate may dispatch. It is bound to the *worker* (derived from the
/// worker's generated spec / bound vertical manifest), NOT to the human
/// principal — so it cannot widen the principal's grant, only narrow it.
///
/// ```
/// use argo_capability::WorkerCapabilityProfile;
/// // A teammate whose spec declares two actions + one allowed tool.
/// let profile = WorkerCapabilityProfile::derive(
///     ["draft_reply".into(), "summarize_ticket".into()], // declared actions
///     ["web_search".into()],                              // [tools].allowed
///     [],                                                  // [tools].denied
///     [],                                                  // mcp/connector servers (draft-only)
/// );
/// assert!(profile.permits_tool("draft_reply"));
/// assert!(profile.permits_tool("web_search"));
/// // Anything not granted is denied — including the owner's `*` reach:
/// assert!(!profile.permits_tool("terminal"));
/// assert!(!profile.permits_tool("wire_funds"));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkerCapabilityProfile {
    /// Tool names explicitly allowed (deny-by-default: empty = deny all).
    allowed_tools: BTreeSet<String>,
    /// Tool names explicitly denied. Deny wins over allow.
    denied_tools: BTreeSet<String>,
    /// Integration (MCP / connector) names the spec referenced but that
    /// are NOT live at birth (DEC-025 locked #6 — "never auto-bind
    /// integrations at birth"). Surfaced to the operator review screen;
    /// kept OUT of the dispatch allow-set until a deliberate post-hire
    /// grant promotes one.
    draft_integrations: BTreeSet<String>,
    /// DEC-049: when true, [`Self::to_capability_set`] emits
    /// `Spawn{Subagent}` so this worker may `delegate_task` (spawn a child
    /// teammate). Default false (deny). Set ONLY via
    /// [`Self::with_subagent_spawn`], which the agent-runner gates behind the
    /// manifest opt-in AND the `ARGO_WORKER_SWARM` flag. NEVER grants
    /// `Spawn{Subprocess}`/`{Container}` — those stay owner-only (host escape).
    can_spawn_subagent: bool,
    /// cu1 (DEC-050 G1): when `Some`, this worker has a scoped filesystem
    /// sandbox root. [`Self::to_capability_set`] then emits `FsRead`/`FsWrite`
    /// caps scoped to `<fs_root>/**` — but ONLY for the file tools the
    /// allow-set already grants (`file_read` → `FsRead`; `file_write` /
    /// `file_patch` → `FsWrite`). `None` (the default) emits no `Fs*` caps at
    /// all, so a worker without a sandbox keeps the historical default-deny:
    /// it cannot touch the filesystem. Access is NEVER granted outside
    /// `fs_root`, and a sandbox NEVER grants `Spawn` (terminal stays
    /// owner-only — DEC-049). Set via [`Self::with_fs_root`].
    fs_root: Option<String>,
}

impl WorkerCapabilityProfile {
    /// Derive a least-privilege profile from a generated teammate's
    /// declared needs.
    ///
    /// - `declared_actions`: the `[actions]` action-type names from the
    ///   generated manifest — the teammate's genuine action vocabulary.
    /// - `allowed_tools`: the manifest's `[tools].allowed` list.
    /// - `denied_tools`: the manifest's `[tools].denied` list (deny wins).
    /// - `integration_servers`: declared MCP / connector server names.
    ///   These are recorded as `draft_integrations` (NOT live) per the
    ///   locked "never auto-bind integrations at birth" decision.
    ///
    /// A teammate that declares no actions and no allowed tools gets an
    /// EMPTY allow-set — it can dispatch nothing autonomously (the
    /// safest possible default for a novel role).
    #[must_use]
    pub fn derive(
        declared_actions: impl IntoIterator<Item = String>,
        allowed_tools: impl IntoIterator<Item = String>,
        denied_tools: impl IntoIterator<Item = String>,
        integration_servers: impl IntoIterator<Item = String>,
    ) -> Self {
        let mut allowed: BTreeSet<String> = BTreeSet::new();
        for a in declared_actions {
            let t = a.trim();
            if !t.is_empty() {
                allowed.insert(t.to_string());
            }
        }
        for t in allowed_tools {
            let t = t.trim();
            if !t.is_empty() {
                allowed.insert(t.to_string());
            }
        }
        let denied: BTreeSet<String> = denied_tools
            .into_iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        let draft_integrations: BTreeSet<String> = integration_servers
            .into_iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        Self {
            allowed_tools: allowed,
            denied_tools: denied,
            draft_integrations,
            can_spawn_subagent: false,
            fs_root: None,
        }
    }

    /// cu1 (DEC-050 G1): bind (or clear) this worker's scoped filesystem
    /// sandbox root. When `fs_root` is `Some`, [`Self::to_capability_set`]
    /// emits `FsRead`/`FsWrite` caps scoped to `<fs_root>/**` for whichever
    /// file tools the allow-set already permits (`file_read` → `FsRead`;
    /// `file_write` / `file_patch` → `FsWrite`). When `None` (the default),
    /// no `Fs*` cap is emitted and the worker cannot touch the filesystem —
    /// the historical default-deny posture is preserved.
    ///
    /// The grant is doubly fenced: a `file_*` tool that is NOT in the
    /// allow-set (or is explicitly denied) yields no `Fs*` cap even with a
    /// sandbox set, and the glob is ALWAYS confined to `<fs_root>/**` so a
    /// worker can never read or write outside its own folder. A sandbox NEVER
    /// emits `Spawn` of any kind.
    ///
    /// ```
    /// use argo_capability::WorkerCapabilityProfile;
    /// use argo_types::Capability;
    /// use std::path::Path;
    /// let profile = WorkerCapabilityProfile::derive(
    ///     [], ["file_write".into()], [], [],
    /// )
    /// .with_fs_root(Some(Path::new("/tmp/w")));
    /// let set = profile.to_capability_set();
    /// assert!(set.covers(&Capability::FsWrite { path_glob: "/tmp/w/notes.txt".into() }));
    /// // Never outside the sandbox:
    /// assert!(!set.covers(&Capability::FsWrite { path_glob: "/tmp/other/x".into() }));
    /// ```
    #[must_use]
    pub fn with_fs_root(mut self, fs_root: Option<&Path>) -> Self {
        // Defense-in-depth (DEC-050 G1 security review): an empty or
        // `..`-containing root would mint a `/**` or traversal glob. The
        // capability `covers()` check is LEXICAL (the file tools' `confine_path`
        // is the real traversal guard), so refuse such roots at the type
        // boundary too — they resolve to `None` (no Fs grant) rather than a
        // dangerous glob. The runtime reader already filters empty; this closes
        // the same footgun for every caller of the type.
        // Normalize the separator to `/` (no-op on unix). The stored root
        // becomes a globset GRANT pattern via `format!("{root}/**")`, and
        // globset matches on `/` while treating `\` as an escape — so a
        // Windows `C:\…\maya` root would compile to a broken pattern and the
        // worker would be denied access to its own sandbox. Mirrors the
        // workspace-glob fix in `crate::lib` (win-capability-fs-cap-glob-separator).
        self.fs_root = fs_root
            .map(|p| p.display().to_string().replace('\\', "/"))
            .filter(|s| !s.trim().is_empty() && !s.contains(".."));
        self
    }

    /// The worker's scoped filesystem sandbox root, if one is bound.
    #[must_use]
    pub fn fs_root(&self) -> Option<&str> {
        self.fs_root.as_deref()
    }

    /// DEC-049: grant (or revoke) `Spawn{Subagent}` on this profile. The
    /// agent-runner sets this to `swarm_enabled() && manifest allows
    /// delegate_task`, so the grant is doubly default-off. Subagent ONLY —
    /// there is no path to `Spawn{Subprocess}`/`{Container}` from a profile.
    #[must_use]
    pub fn with_subagent_spawn(mut self, yes: bool) -> Self {
        self.can_spawn_subagent = yes;
        self
    }

    /// `true` if this profile grants child-teammate spawning (DEC-049).
    #[must_use]
    pub fn can_spawn_subagent(&self) -> bool {
        self.can_spawn_subagent
    }

    /// `true` when `tool_name` is permitted by this profile.
    ///
    /// Deny-by-default with deny-wins precedence: a tool fires only if it
    /// is in `allowed_tools` and NOT in `denied_tools`.
    #[must_use]
    pub fn permits_tool(&self, tool_name: &str) -> bool {
        !self.denied_tools.contains(tool_name) && self.allowed_tools.contains(tool_name)
    }

    /// The allow-set, sorted, for the operator review screen + audit.
    #[must_use]
    pub fn allowed_tools(&self) -> Vec<String> {
        self.allowed_tools.iter().cloned().collect()
    }

    /// The explicit deny-set, sorted, for the operator review screen.
    #[must_use]
    pub fn denied_tools(&self) -> Vec<String> {
        self.denied_tools.iter().cloned().collect()
    }

    /// Integration server names the spec referenced that are NOT live
    /// at birth (draft-only until a deliberate post-hire grant).
    #[must_use]
    pub fn draft_integrations(&self) -> Vec<String> {
        self.draft_integrations.iter().cloned().collect()
    }

    /// Compile this profile into a [`CapabilitySet`] of
    /// [`Capability::ToolDispatch`] grants — one per allowed tool name,
    /// plus the always-safe [`MINIMAL_TEAMMATE_TOOLS`].
    ///
    /// This is the bridge into the existing RBAC engine: binding a
    /// Genesis worker to a role carrying THIS set (instead of `owner`)
    /// makes the standard `CapabilityEngine::check` at the dispatch seam
    /// enforce the fence with no new runtime code. Because the set is
    /// built from explicit, non-glob tool names, it can never grant the
    /// `*` reach the owner role does.
    ///
    /// ```
    /// use argo_capability::WorkerCapabilityProfile;
    /// use argo_types::Capability;
    /// let profile = WorkerCapabilityProfile::derive(
    ///     ["draft_reply".into()],
    ///     [],
    ///     [],
    ///     [],
    /// );
    /// let set = profile.to_capability_set();
    /// assert!(set.covers(&Capability::ToolDispatch {
    ///     tool_name_glob: "draft_reply".into(),
    /// }));
    /// assert!(!set.covers(&Capability::ToolDispatch {
    ///     tool_name_glob: "terminal".into(),
    /// }));
    /// ```
    #[must_use]
    pub fn to_capability_set(&self) -> CapabilitySet {
        let mut caps: Vec<Capability> = Vec::new();
        for tool in &self.allowed_tools {
            if self.denied_tools.contains(tool) {
                continue;
            }
            caps.push(Capability::ToolDispatch {
                tool_name_glob: tool.clone(),
            });
        }
        // Generated teammates always need to escalate / clarify / read
        // their own memory + skills to do real work, and these are
        // never side-effecting transmit paths. Granting them keeps the
        // fence from bricking the teammate while still denying every
        // transmit / destructive tool by default.
        for safe in MINIMAL_TEAMMATE_TOOLS {
            if self.denied_tools.contains(*safe) {
                continue;
            }
            caps.push(Capability::ToolDispatch {
                tool_name_glob: (*safe).to_string(),
            });
        }
        // DEC-049: a worker may spawn CHILD TEAMMATES (delegate_task) only when
        // explicitly granted. `Subagent` ONLY — the host-escape kinds
        // (`Subprocess`/`Container`) are never reachable from a worker profile;
        // they stay owner/admin-only. The grant is doubly gated upstream (the
        // manifest must allow `delegate_task` AND `ARGO_WORKER_SWARM` must be
        // on), so this stays deny-by-default.
        if self.can_spawn_subagent {
            caps.push(Capability::Spawn {
                spawn_kind: SpawnKind::Subagent,
            });
        }
        // cu1 (DEC-050 G1): a worker with a scoped sandbox gets `Fs*` caps
        // confined to `<fs_root>/**`, but ONLY for the file tools its allow-set
        // already permits (deny-wins via `permits_tool`). No sandbox → no `Fs*`
        // cap (default-deny preserved). The glob is always rooted at `fs_root`,
        // so a worker can never read or write outside its own folder; `Spawn`
        // is never emitted from here (terminal stays owner-only — DEC-049).
        if let Some(root) = &self.fs_root {
            let glob = format!("{}/**", root.trim_end_matches('/'));
            if self.permits_tool(FILE_READ_TOOL) {
                caps.push(Capability::FsRead {
                    path_glob: glob.clone(),
                });
            }
            if FILE_WRITE_TOOLS.iter().any(|t| self.permits_tool(t)) {
                caps.push(Capability::FsWrite { path_glob: glob });
            }
        }
        // cu1d (DEC-050 G1 follow-up): the read-only web research tools
        // (`web_search` / `web_extract`) additionally require `NetEgress` at
        // dispatch. Emit it ONLY on explicit manifest opt-in (deny-wins via
        // `permits_tool`); a worker without web tools keeps zero egress
        // (default-deny preserved). The glob is `**` because search backends
        // and extraction targets are operator-unknown hosts — but the grant
        // is only exercisable through the two read-only web tools: no
        // transmit, shell, or write tool in a worker profile declares
        // `NetEgress`, and `Spawn`/`LocalCodeExecution` are never emitted
        // here, so this cannot become a data-exfiltration primitive beyond
        // what the web tools themselves return into the audited transcript.
        if WEB_RESEARCH_TOOLS.iter().any(|t| self.permits_tool(t)) {
            caps.push(Capability::NetEgress {
                host_glob: "**".into(),
                port: None,
            });
        }
        CapabilitySet(caps)
    }
}

#[cfg(test)]
#[path = "worker_profile_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "worker_profile_net_egress_tests.rs"]
mod net_egress_tests;
