//! Hire-draft helpers for [`crate::IdentityStore::create_worker_from_draft`]
//! (Stream J5). Lives in its own sibling file so [`crate::store`] stays
//! under the 500-LOC ceiling — the brief originally targeted `store.rs`
//! at <400 LOC but recent additions (`update_worker`, `update_level`,
//! `update_charter`) pushed it past that budget. Splitting the hire
//! plumbing into a sibling is the same pattern `store_role.rs` /
//! `store_channels.rs` / `store_skills.rs` follow today.
//!
//! Public surface: [`HireDraft`], [`HireDutyDraft`],
//! [`ApprovalPolicyChoice`], plus the crate-private
//! [`validate_hire_draft`] + [`assemble_worker_from_draft`] used by
//! the [`crate::IdentityStore::create_worker_from_draft`] method.
//!
//! Field shapes mirror the frontend `WorkerDraft` (E2-e
//! `web/lib/hire-flow/types.ts`) so the gateway can deserialise the
//! hire request body directly with `#[serde(rename_all = "camelCase")]`.

use crate::{
    AutoApprovalPolicy, CircuitState, Duty, MemoryNs, MemoryScope, Persona, Role, VoiceProfile,
    Worker, WorkerId, WorkerLevel,
};
use argo_types::{ArgoError, ArgoResult, TenantId};
use serde::{Deserialize, Serialize};

/// Caller-supplied hire context for the J5 hire endpoint.
///
/// Field names are camelCase on the wire so the JSON envelope from
/// E2-e rides straight through deserialisation.
///
/// ```
/// use argo_identity::{ApprovalPolicyChoice, HireDraft, HireDutyDraft};
/// let draft = HireDraft {
///     suggested_name: "Maya".into(),
///     charter: "Own outbound marketplace follow-up end-to-end.".into(),
///     suggested_vertical: "marketplace-seller".into(),
///     approval_policy: ApprovalPolicyChoice { level: "balanced".into() },
///     duties: vec![HireDutyDraft {
///         cron: "0 9 * * 1-5".into(),
///         entry_action: "follow_up".into(),
///         description: "Send approved follow-up.".into(),
///         default_enabled: true,
///     }],
///     voice_profile: None,
///     tagline: None,
///     genesis_plan_id: None,
/// };
/// assert_eq!(draft.suggested_name, "Maya");
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HireDraft {
    /// Persona display name (`worker.persona.name`).
    pub suggested_name: String,
    /// Operating charter (markdown, persisted as the role charter).
    pub charter: String,
    /// Vertical template id; must resolve in the vertical catalog.
    pub suggested_vertical: String,
    /// Approval-policy bucket; `aggressive` enables auto-approval,
    /// `conservative` / `balanced` leave it disabled (ask everything).
    pub approval_policy: ApprovalPolicyChoice,
    /// Scheduled duties; at least one must have `default_enabled = true`.
    pub duties: Vec<HireDutyDraft>,
    /// Optional voice-profile override; falls back to a generated
    /// `"You are <name>, an autonomous SKYCOT worker."` preamble.
    #[serde(default)]
    pub voice_profile: Option<VoiceProfile>,
    /// Optional one-line bio / tagline persisted on the persona row.
    #[serde(default)]
    pub tagline: Option<String>,
    /// Optional Genesis plan id that generated this draft (e.g.
    /// `marketing-intern-maya`). When present, the hire path stamps a
    /// [`GENESIS_PLAN_PROVENANCE_PREFIX`] entry on the persona's
    /// `provenance_chain` so the read-side (`/workers/{id}/overview`)
    /// can surface a "Plan provenance" chip backing the charter line
    /// `Genesis plan id: <id>`. Without this carrier, the plan id is
    /// dropped between [`crate::IdentityStore::create_worker_from_draft`]
    /// and the persisted worker row.
    #[serde(default)]
    pub genesis_plan_id: Option<String>,
}

/// `provenance_chain` prefix that marks a Genesis-plan id entry, e.g.
/// `genesis_plan_id=marketing-intern-maya`. Stable wire format because
/// `provenance_chain` is JSON-encoded on the persisted persona row;
/// read-side parsers (the overview handler) match on this prefix.
pub const GENESIS_PLAN_PROVENANCE_PREFIX: &str = "genesis_plan_id=";

/// Approval-policy bucket the frontend hire flow surfaces. Richer
/// per-policy caps land later via the existing [`AutoApprovalPolicy`]
/// fields once the UX exposes them.
///
/// ```
/// use argo_identity::ApprovalPolicyChoice;
/// let choice = ApprovalPolicyChoice { level: "balanced".into() };
/// assert_eq!(choice.level, "balanced");
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalPolicyChoice {
    /// `"conservative"`, `"balanced"`, or `"aggressive"`.
    pub level: String,
}

/// One duty entry inside [`HireDraft`]. Maps to a `duty` table row
/// written by the existing transactional [`crate::IdentityStore::create_worker`]
/// path.
///
/// ```
/// use argo_identity::HireDutyDraft;
/// let duty = HireDutyDraft {
///     cron: "0 9 * * 1-5".into(),
///     entry_action: "follow_up".into(),
///     description: "Send approved follow-up.".into(),
///     default_enabled: true,
/// };
/// assert!(duty.default_enabled);
/// ```
///
/// See [`GENESIS_PLAN_PROVENANCE_PREFIX`] for the provenance-chain
/// marker the hire path uses to forward a Genesis plan id from the
/// gateway into the persisted persona row.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HireDutyDraft {
    /// Cron expression that drives the duty.
    pub cron: String,
    /// Entry action invoked when the duty fires.
    pub entry_action: String,
    /// Human-readable description.
    pub description: String,
    /// Whether the duty is enabled at creation; at least one duty in
    /// the draft must carry `true` so the worker isn't inert at hire.
    #[serde(default)]
    pub default_enabled: bool,
}

/// Extract the Genesis plan id (if any) from a persona's
/// `provenance_chain`. Reads the canonical
/// [`GENESIS_PLAN_PROVENANCE_PREFIX`] marker the hire path stamps.
/// Returns the first non-empty value or `None` when no marker is
/// present. Tolerant of pre-existing rows that never carried a plan
/// id (returns `None`, no panic).
///
/// ```
/// use argo_identity::genesis_plan_id_from_provenance;
/// let chain = vec!["genesis_plan_id=marketing-intern-maya".into()];
/// assert_eq!(
///     genesis_plan_id_from_provenance(&chain).as_deref(),
///     Some("marketing-intern-maya"),
/// );
/// assert!(genesis_plan_id_from_provenance(&[]).is_none());
/// ```
#[must_use]
pub fn genesis_plan_id_from_provenance(chain: &[String]) -> Option<String> {
    chain
        .iter()
        .find_map(|entry| entry.strip_prefix(GENESIS_PLAN_PROVENANCE_PREFIX))
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

pub(crate) fn validate_hire_draft(draft: &HireDraft) -> ArgoResult<()> {
    if draft.suggested_name.trim().is_empty() {
        return Err(ArgoError::Schema(
            "worker_draft.suggestedName must be non-empty".into(),
        ));
    }
    if draft.charter.trim().len() < 20 {
        return Err(ArgoError::Schema(
            "worker_draft.charter must be at least 20 chars".into(),
        ));
    }
    if draft.suggested_vertical.trim().is_empty() {
        return Err(ArgoError::Schema(
            "worker_draft.suggestedVertical must be non-empty".into(),
        ));
    }
    if draft.duties.is_empty() {
        return Err(ArgoError::Schema(
            "worker_draft.duties must contain at least one duty".into(),
        ));
    }
    if !draft.duties.iter().any(|d| d.default_enabled) {
        return Err(ArgoError::Schema(
            "worker_draft.duties: at least one duty must have defaultEnabled=true".into(),
        ));
    }
    match draft.approval_policy.level.as_str() {
        "conservative" | "balanced" | "aggressive" => Ok(()),
        other => Err(ArgoError::Schema(format!(
            "worker_draft.approvalPolicy.level '{other}' must be one of \
             conservative|balanced|aggressive"
        ))),
    }
}

pub(crate) fn assemble_worker_from_draft(
    draft: &HireDraft,
    tenant_id: TenantId,
    principal_id: &str,
) -> Worker {
    let voice = draft.voice_profile.clone().unwrap_or_else(|| VoiceProfile {
        preamble: format!(
            "You are {}, an autonomous SKYCOT worker.",
            draft.suggested_name
        ),
        ..Default::default()
    });
    let mut provenance_chain = Vec::new();
    if let Some(plan_id) = draft.genesis_plan_id.as_ref() {
        let trimmed = plan_id.trim();
        if !trimmed.is_empty() {
            provenance_chain.push(format!("{GENESIS_PLAN_PROVENANCE_PREFIX}{trimmed}"));
        }
    }
    let persona = Persona {
        name: draft.suggested_name.clone(),
        voice_profile: voice,
        avatar: None,
        bio: draft.tagline.clone(),
        builder_attribution: principal_id.to_string(),
        provenance_chain,
    };
    let duties: Vec<Duty> = draft
        .duties
        .iter()
        .filter(|d| d.default_enabled)
        .map(|d| {
            Duty::new(
                d.cron.clone(),
                d.entry_action.clone(),
                d.description.clone(),
            )
        })
        .collect();
    let role = Role {
        charter: draft.charter.clone(),
        duties,
        kpis: Vec::new(),
        escalation: Some(format!(
            "approval_policy_level={}",
            draft.approval_policy.level
        )),
    };
    let id = WorkerId::new();
    Worker {
        id,
        tenant_id,
        vertical_id: draft.suggested_vertical.clone(),
        persona,
        role,
        memory_namespace: MemoryNs::new(tenant_id, id, MemoryScope::WorkerSelf),
        auto_approval: AutoApprovalPolicy {
            enabled: draft.approval_policy.level == "aggressive",
            ..Default::default()
        },
        circuit_state: CircuitState::Closed,
        channels: Vec::new(),
        charter: Some(draft.charter.clone()),
        kpis: None,
        level: WorkerLevel::Entry,
        trust_grade: 0,
        last_used_at: None,
        role_name: None,
    }
}
