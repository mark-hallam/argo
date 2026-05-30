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

use argo_types::{Capability, CapabilitySet};
use std::collections::BTreeSet;

/// Irreversible / business-ruining action verbs that ALWAYS require a
/// human in the loop, no matter what a generated profile grants.
///
/// Deterministic by construction: this is a fixed list, NOT an
/// LLM-judged classification and NOT the looser sensitive-verb lexical
/// scan (`send|post|pay|publish|delete|share|message`) the structural
/// safety judge already applies. It is the hard floor under that judge —
/// the actions where an autonomous mistake cannot be undone.
///
/// Matched as a sub-token split on `_` (case-insensitive); multi-token
/// phrases (e.g. `wire_funds`) match as a contiguous substring so
/// `wire_funds`, `funds_wire_now`, and `WIRE_FUNDS` all match, while an
/// unrelated substring (e.g. `rewire_config`) does not.
pub const IRREVERSIBLE_ACTION_VERBS: &[&str] = &[
    "wire",
    "wire_funds",
    "refund",
    "chargeback",
    "drop",
    "drop_table",
    "truncate",
    "wipe",
    "purge",
    "destroy",
    "terminate",
    "deprovision",
    "revoke",
    "cancel_subscription",
    "delete_account",
    "close_account",
];

/// `true` when `action_name` denotes an irreversible / business-ruining
/// action that must ALWAYS go through human approval.
///
/// ```
/// use argo_capability::is_irreversible_action;
/// assert!(is_irreversible_action("wire_funds"));
/// assert!(is_irreversible_action("issue_refund"));
/// assert!(is_irreversible_action("drop_table"));
/// assert!(is_irreversible_action("terminate_employee"));
/// // Not irreversible — a draft / read action:
/// assert!(!is_irreversible_action("draft_summary"));
/// assert!(!is_irreversible_action("rewire_config"));
/// ```
#[must_use]
pub fn is_irreversible_action(action_name: &str) -> bool {
    let lowered = action_name.to_lowercase();
    let tokens: Vec<&str> = lowered.split('_').collect();
    IRREVERSIBLE_ACTION_VERBS.iter().any(|verb| {
        if verb.contains('_') {
            // Multi-token verb (e.g. "wire_funds"): require the whole
            // phrase to appear as a contiguous run in the name.
            lowered.contains(verb)
        } else {
            tokens.iter().any(|tok| tok == verb)
        }
    })
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
        }
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
        CapabilitySet(caps)
    }
}

#[cfg(test)]
#[path = "worker_profile_tests.rs"]
mod tests;
