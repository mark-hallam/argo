//! `CapabilitySpec` — the typed capability IR (DEC-025 Genesis P1).
//!
//! A `CapabilitySpec` is the single intermediate representation between a
//! plain-English role description and a loadable vertical. It **supersedes**
//! the bare six-field [`crate::BroadRoleDraft`] (the shape that today can be
//! judged and slug-read but never *materialized* — there is no
//! `write_broad_vertical` anywhere in the workspace). Where `BroadRoleDraft`
//! carries only a flat action-name list, `CapabilitySpec` carries each
//! capability as a typed [`CapabilityDecl`] (risk + reversibility + a
//! proposed executor) plus the tool / secret / egress scopes and escalation
//! routing the materializer needs to emit a real least-privilege
//! `vertical.toml`.
//!
//! ## Hybrid synthesis (DEC-025 locked decision #3)
//!
//! The spec is synthesized *from scratch* for ANY role, but always carries a
//! [`CapabilitySpec::nearest_archetype`] — the nearest of the 12 built-in
//! archetypes, used as a cheap deterministic prior + low-confidence fallback.
//! It is NOT a taxonomy cap: the actions, prompts, and scopes come from the
//! spec, not from the archetype's seed template. The nearest archetype only
//! (a) gives the materialized [`crate::GenesisPlan`] a stable `archetype`
//! slug for the SSE payload + audit, and (b) seeds a sensible default when a
//! field is otherwise empty.
//!
//! ```
//! use argo_genesis::{Archetype, CapabilitySpec};
//! // A novel role with no good archetype fit still synthesizes a spec; the
//! // nearest archetype is just the cheap prior.
//! let draft = argo_genesis::BroadRoleDraft {
//!     charter: "schedule grape harvests and coordinate trucking".into(),
//!     persona: "vinny".into(),
//!     persona_description: "Coordinates vineyard logistics.".into(),
//!     action_vocabulary: vec!["schedule_harvest".into(), "send_truck_dispatch".into()],
//!     success_metrics: vec!["zero missed harvest windows".into()],
//!     onboarding_script: "Hi, I keep the harvest calendar tight.".into(),
//! };
//! let spec = CapabilitySpec::from_broad_draft(draft, Archetype::CalendarCoordinator);
//! assert_eq!(spec.persona, "vinny");
//! assert_eq!(spec.capabilities.len(), 2);
//! // The sensitive verb `send` was forced to require approval.
//! let dispatch = spec.capabilities.iter().find(|c| c.action_name == "send_truck_dispatch").unwrap();
//! assert!(dispatch.requires_approval);
//! ```

use crate::archetype::Archetype;
use crate::broad_role::BroadRoleDraft;
use crate::safety_judge;
use argo_vertical::{ActionRiskLevel, KpiComparator, KpiKind};
use serde::{Deserialize, Serialize};

/// Where the materializer proposes an action should execute. Mirrors the
/// argo-vertical [`argo_vertical::ExecutorDeclaration`] kinds, but stays in
/// the spec layer so synthesis is transport-free. Per DEC-025 locked #6 the
/// materializer **never auto-binds a live integration at birth**: an `Mcp`
/// intent records the server name as a *draft* integration (surfaced to the
/// operator review screen, kept OUT of the live dispatch allow-set) and the
/// emitted action still runs `human`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ExecutorIntent {
    /// A human runs this action (draft → operator approves → operator acts).
    /// The conservative default for every generated capability.
    #[default]
    Human,
    /// The role *wants* this MCP server/tool eventually. Recorded as a
    /// draft integration; NOT live at birth.
    Mcp {
        /// MCP server name the role referenced (e.g. `"gmail"`).
        server: String,
        /// Optional tool within the server.
        #[serde(default)]
        tool: Option<String>,
    },
    /// A built-in, side-effect-free capability (read/draft/clarify).
    Builtin,
}

/// One typed capability the teammate may perform. The materializer wraps
/// each of these into an `[actions.<action_name>]` declaration carrying the
/// risk + approval requirement + a real executor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityDecl {
    /// `snake_case` action verb (e.g. `schedule_harvest`). Becomes the
    /// `[actions]` table key.
    pub action_name: String,
    /// One-line human description of the intent.
    pub intent: String,
    /// Coarse risk tier.
    pub risk: ActionRiskLevel,
    /// Whether the action is reversible. Feeds the deterministic
    /// irreversibility gate AND the conservative approval default.
    pub reversible: bool,
    /// Proposed execution surface (draft-only at birth per DEC-025 #6).
    pub proposed_executor: ExecutorIntent,
    /// Whether the action requires human approval before it transitions
    /// past `pending`. The safety judge force-sets this for sensitive
    /// verbs regardless of what synthesis proposed.
    pub requires_approval: bool,
}

impl CapabilityDecl {
    /// Build a capability from a raw action name, deriving a conservative
    /// risk + approval default. Sensitive verbs (per the [`safety_judge`])
    /// AND irreversible verbs are forced to `requires_approval = true` and
    /// `High` risk; everything else starts `Low` / no-approval. This is the
    /// deterministic floor the LLM cannot relax.
    #[must_use]
    pub fn from_action_name(action_name: &str, intent: impl Into<String>) -> Self {
        let name = sanitize_action_name(action_name);
        let sensitive = safety_judge::matches_sensitive_verb(&name);
        let irreversible = argo_capability::is_irreversible_action(&name);
        let must_approve = sensitive || irreversible;
        Self {
            action_name: name,
            intent: intent.into(),
            risk: if must_approve {
                ActionRiskLevel::High
            } else {
                ActionRiskLevel::Low
            },
            reversible: !irreversible,
            proposed_executor: ExecutorIntent::Human,
            requires_approval: must_approve,
        }
    }
}

/// Typed escalation routing. The materializer emits this as the manifest's
/// `[escalation]` section.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct EscalationSpec {
    /// Action names that ALWAYS require human approval (the conservative
    /// gated set). Mapped to `[escalation].human_approval_required`.
    pub human_approval_required: Vec<String>,
    /// Escalate to a human after this many consecutive failures.
    pub escalate_after_failures: Option<u32>,
    /// Who to escalate to (free-text contact, e.g. `"the operator"`).
    pub escalate_to: Option<String>,
}

/// One measurable success criterion captured at intake — HOW the teammate's
/// performance is judged. Each carries a [`KpiKind`] (the unit/format) + a
/// numeric `target` + a [`KpiComparator`] (pass direction), reusing the
/// manifest's KPI vocabulary rather than defining a parallel comparator enum.
///
/// This is an **intake record**, not a live-evaluated KPI: it pins WHAT the
/// operator said "good" looks like at hire time. Wiring these into the
/// manifest's `[[kpi]]` block + live enforcement is a follow-up
/// (`mission-kpi-enforcement-gate`); here they are captured + surfaced for the
/// `vertical_ready` review so the intake is honest about how success is
/// measured.
///
/// `Eq` is intentionally NOT derived — `target` is `f64`, mirroring
/// [`argo_vertical::KpiDefinition`], which omits `Eq` for the same reason.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SuccessCriterion {
    /// Operator-facing label for the metric (e.g. `"first response time"`).
    pub name: String,
    /// The metric's type tag — controls unit/format downstream (percentage,
    /// duration, count, ratio, score). Reuses the manifest KPI vocabulary.
    pub kind: KpiKind,
    /// The pass/fail line the operator chose, interpreted via `comparator`.
    pub target: f64,
    /// How an observed value is compared against `target` (`gte` / `lte` /
    /// `eq`). Reuses the manifest KPI comparator.
    pub comparator: KpiComparator,
}

/// The typed capability IR. Produced by a `SpecSynthesizer` (the archetype
/// prior or the LLM synthesizer), consumed by `materialize_spec`.
///
/// `Eq` is intentionally NOT derived — [`Self::success_criteria`] carries an
/// `f64` target (see [`SuccessCriterion`]).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapabilitySpec {
    /// The operator's original role description, byte-identical.
    pub charter: String,
    /// Persona handle (validated `snake_case`/dash slug).
    pub persona: String,
    /// One-to-two sentence persona description (third person).
    pub persona_description: String,
    /// What the teammate is responsible for, in plain language.
    pub responsibilities: Vec<String>,
    /// The teammate's typed capability set.
    pub capabilities: Vec<CapabilityDecl>,
    /// Measurable success metrics (free-text; mapped to KPI hints).
    pub success_metrics: Vec<String>,
    /// Single-paragraph onboarding script.
    pub onboarding_script: String,
    /// Typed escalation routing.
    pub escalation: EscalationSpec,
    /// The nearest of the 12 archetypes — a cheap deterministic prior +
    /// low-confidence fallback (DEC-025 #3). NOT a taxonomy cap.
    pub nearest_archetype: Archetype,
    /// `false` when synthesis was confident a known archetype fit (the
    /// archetype-prior path); `true` when this is a genuinely from-scratch
    /// synthesis for an off-taxonomy role. Surfaced to the operator as the
    /// honest `low-fit` signal (replaces the dishonest `confidence=1.0`).
    pub from_scratch: bool,
    /// WHEN the teammate runs — a free-text cadence/schedule the operator
    /// stated at intake (e.g. `"every weekday at 9am"`, `"on inbound ticket"`).
    /// Additive + `#[serde(default)]`: specs authored before this field still
    /// deserialize (cadence `None`).
    #[serde(default)]
    pub cadence: Option<String>,
    /// WHAT exceptions / carve-outs apply — situations the teammate must NOT
    /// act on, or must hand off (e.g. `"never touch legal escalations"`).
    /// Additive + `#[serde(default)]` for back-compat.
    #[serde(default)]
    pub exceptions: Vec<String>,
    /// HOW success is measured — typed, measurable criteria captured at intake.
    /// Reuses the manifest [`KpiKind`] / [`KpiComparator`] vocabulary.
    /// Additive + `#[serde(default)]` for back-compat.
    #[serde(default)]
    pub success_criteria: Vec<SuccessCriterion>,
    /// WHAT systems/accounts the teammate will need (DEC-048 intake slot).
    /// Free-text names captured at intake, surfaced on the role contract as
    /// draft-only integrations — honest "to be connected" items, never live
    /// bindings (integrations stay draft-only at birth). Additive +
    /// `#[serde(default)]` for back-compat.
    #[serde(default)]
    pub access_integrations: Vec<String>,
}

impl CapabilitySpec {
    /// Build a [`CapabilitySpec`] from the legacy [`BroadRoleDraft`] plus the
    /// nearest-archetype prior. This is the bridge that makes the existing
    /// LLM broad-role synthesis path materializable: the draft's flat
    /// `action_vocabulary` becomes a typed [`CapabilityDecl`] set (with the
    /// safety/irreversibility approval floor applied), and the conservative
    /// escalation set is the union of every approval-gated action.
    #[must_use]
    pub fn from_broad_draft(draft: BroadRoleDraft, nearest_archetype: Archetype) -> Self {
        // Each capability gets a grammatical last-resort description anchored
        // in its own action verb. This is ONLY the fallback floor — the LLM
        // authoring pass (`author::apply_authored`) overwrites each `intent`
        // with a specific, role-aware one-liner when it runs, and
        // `GenesisPlan::sync_action_descriptions_from_spec` plumbs that authored
        // text into the operator-visible manifest. The eval floor (DEC-025 #7)
        // still validates real domain vocabulary: the charter is embedded
        // verbatim in the behaviour contract and drives the eval-seed `primary`
        // keyword (`hire_gate::behaviour_contract_text` /
        // `eval_seed::generate(&plan.charter, …)`), so the floor no longer
        // depends on the per-action description carrying a role descriptor.
        let capabilities: Vec<CapabilityDecl> = draft
            .action_vocabulary
            .iter()
            .map(|name| CapabilityDecl::from_action_name(name, synthesized_intent(name)))
            .collect();
        let human_approval_required: Vec<String> = capabilities
            .iter()
            .filter(|c| c.requires_approval)
            .map(|c| c.action_name.clone())
            .collect();
        Self {
            charter: draft.charter,
            persona: draft.persona,
            persona_description: draft.persona_description,
            responsibilities: Vec::new(),
            capabilities,
            success_metrics: draft.success_metrics,
            onboarding_script: draft.onboarding_script,
            escalation: EscalationSpec {
                human_approval_required,
                // Conservative defaults: escalate after 3 failures, to the
                // operator. The operator can tune these on the review screen.
                escalate_after_failures: Some(3),
                escalate_to: Some("the operator".to_string()),
            },
            nearest_archetype,
            // A draft produced by the broad-role path is by definition the
            // off-taxonomy / from-scratch case.
            from_scratch: true,
            // The legacy broad-draft path carries no cadence / exceptions /
            // typed success-criteria; these are populated by the richer intake
            // (the capture panel) or a later author pass.
            cadence: None,
            exceptions: Vec::new(),
            success_criteria: Vec::new(),
            access_integrations: Vec::new(),
        }
    }

    /// The action-type names this spec declares, in declaration order.
    #[must_use]
    pub fn action_names(&self) -> Vec<String> {
        self.capabilities
            .iter()
            .map(|c| c.action_name.clone())
            .collect()
    }
}

/// Normalise an arbitrary action label to a manifest-legal `snake_case`
/// action_type: lowercase, ASCII alphanumerics + `_`, collapse runs of
/// separators, trim leading/trailing `_`. A label that sanitises to empty
/// falls back to `"perform_task"` so the materializer never emits an
/// unnameable action.
#[must_use]
pub(crate) fn sanitize_action_name(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut last_underscore = true; // suppress a leading `_`
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            out.extend(ch.to_lowercase());
            last_underscore = false;
        } else if !last_underscore {
            out.push('_');
            last_underscore = true;
        }
    }
    while out.ends_with('_') {
        out.pop();
    }
    if out.is_empty() {
        "perform_task".to_string()
    } else {
        out
    }
}

/// Build the **last-resort** one-line capability description — a grammatical
/// sentence built from the action's own de-snake-cased verb phrase
/// (`schedule_harvest` → "Schedule harvest."). This is the fallback floor ONLY:
/// the LLM authoring pass overwrites each `intent` with a specific, role-aware
/// description when it runs (`author::apply_authored` +
/// `GenesisPlan::sync_action_descriptions_from_spec`).
///
/// The prior version appended an "as the {role_descriptor}" tail
/// (`"Archive article as the assistant named Remy reads"`) — ungrammatical,
/// repetitive, and camera-visible in the Genesis review panel. A plain
/// capitalized verb phrase reads far better. The eval floor (DEC-025 #7) is
/// unaffected: the charter is embedded verbatim in the behaviour contract and
/// drives the eval-seed `primary` keyword, so the domain vocabulary no longer
/// has to ride on the per-action description. The text stays plain English
/// (spaces only) so it tokenizes byte-identically on the eval-seed and
/// behaviour-contract sides (no embedded `-`/`_` to diverge on).
fn synthesized_intent(action_name: &str) -> String {
    format!("{}.", capitalize_first(&humanize_action_name(action_name)))
}

/// Turn a `snake_case` action name into a lower-case space-separated verb
/// phrase (`draft_truck_plan` → "draft truck plan"). Falls back to "perform a
/// task" for an empty/degenerate name so the description is never blank.
fn humanize_action_name(action_name: &str) -> String {
    let words: Vec<&str> = action_name
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|s| !s.is_empty())
        .collect();
    if words.is_empty() {
        "perform a task".to_string()
    } else {
        words.join(" ").to_lowercase()
    }
}

/// Capitalize the first ASCII letter of a phrase, leaving the rest untouched.
fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

#[cfg(test)]
#[path = "spec_tests.rs"]
mod tests;
