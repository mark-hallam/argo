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
use chrono::{Duration, Utc};
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
///     kpis: vec!["Reply within 2h".into()],
///     voice_profile: None,
///     tagline: None,
///     genesis_plan_id: None,
///     fs_root: None,
///     access_fs_root: None,
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
    /// Operator success-measures (KPIs) captured at Genesis intake
    /// (`role_contract.success_criteria`). Persisted onto the role + worker so
    /// the teammate's goals survive the hire — previously DROPPED (the role was
    /// built with `kpis: Vec::new()`), which is the Item-2b delegation fix.
    /// Empty when the operator set none.
    #[serde(default)]
    pub kpis: Vec<String>,
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
    /// Optional explicit filesystem jail root (`worker.fs_root` column)
    /// for the new teammate's file tools. When absent AND the draft
    /// carries Genesis provenance ([`Self::genesis_plan_id`]), the hire
    /// path defaults it to `<home>/SkycotTeammates/<persona-slug>/` so a
    /// Genesis teammate's approved file work lands in the user's own
    /// folder instead of jailing to the gateway process cwd. An explicit
    /// value always wins over the default; a non-Genesis hire with no
    /// explicit value stays `NULL` (unchanged behaviour). See
    /// [`crate::resolved_fs_root`].
    #[serde(default)]
    pub fs_root: Option<String>,
    /// Optional captured-intake Access folder candidate — the local folder
    /// the user NAMED during Genesis intake (e.g. `SkycotDemo/Inbox`, surfaced
    /// in the captured panel as `Access needed (draft-only): "<folder>"`).
    /// Unlike [`Self::fs_root`] (a deliberate, unchecked override), this value
    /// is UNTRUSTED operator input, so the hire path safety-gates it: it is
    /// only bound to `worker.fs_root` when it resolves to a path under the
    /// user's `HOME` with no `..` traversal — otherwise the hire falls back to
    /// the generated Genesis default. Precedence sits BELOW an explicit
    /// `fs_root` and ABOVE the Genesis default. See [`crate::resolved_fs_root`].
    #[serde(default)]
    pub access_fs_root: Option<String>,
}

/// `provenance_chain` prefix that marks a Genesis-plan id entry, e.g.
/// `genesis_plan_id=marketing-intern-maya`. Stable wire format because
/// `provenance_chain` is JSON-encoded on the persisted persona row;
/// read-side parsers (the overview handler) match on this prefix.
pub const GENESIS_PLAN_PROVENANCE_PREFIX: &str = "genesis_plan_id=";

/// `provenance_chain` prefix that marks the END of a Genesis-hired
/// teammate's probation window, e.g.
/// `probation_until=2026-05-31T12:00:00+00:00` (DEC-025 #2 / Genesis P3).
///
/// Stamped at the single hire chokepoint ([`assemble_worker_from_draft`])
/// for every Genesis hire — both the flagship SSE path and
/// `/onboarding/first-hire` route through this one function, so neither
/// can be hired without the marker. Read at runtime by
/// `argo_agent_runner::turn_vertical_fence` off the SAME worker-row read
/// that resolves the capability fence (one session-boot read, never
/// per-action I/O on the hot loop, per the design doc). While the parsed
/// timestamp is in the future, EVERY emitted action is forced
/// approval-gated (held pending, not executed unsupervised) at every
/// dispatch seam; once it passes, the teammate graduates to its normal
/// approval policy. Same JSON-encoded persisted column as
/// [`GENESIS_PLAN_PROVENANCE_PREFIX`].
pub const PROBATION_PROVENANCE_PREFIX: &str = "probation_until=";

/// Default probation window for a freshly-hired Genesis teammate, in
/// hours (DEC-025 #2 — "24h, all actions approval-gated during it, then
/// graduate"). Kept in lock-step with `argo_genesis::DRY_RUN_HOURS` (the
/// `worker.toml` `dry_run_until` writer) by value; `argo-identity` does
/// not depend on `argo-genesis`, so the constant is mirrored here with
/// this cross-reference rather than imported.
pub const PROBATION_HOURS: i64 = 24;

/// `provenance_chain` prefix that marks the truth-boundary `policy_id`
/// bound to a Genesis-hired teammate (DEC-025 #6 / Genesis P5 part (a)),
/// e.g. `truth_boundary_policy_id=genesis-default-conservative`.
///
/// Stamped at the single hire chokepoint ([`build_provenance_chain`]) for
/// every Genesis hire so BOTH the flagship SSE path and
/// `/onboarding/first-hire` carry it. Read at runtime by
/// `argo_agent_runner` off the SAME session-boot worker-row read that
/// resolves the capability fence + probation; when present, the runner
/// loads the conservative default truth-boundary policy
/// (`argo_capability::genesis_default_policy`) bound to the worker's
/// tenant, replacing the env-var-only activation. Same JSON-encoded
/// persisted column as [`GENESIS_PLAN_PROVENANCE_PREFIX`].
pub const TRUTH_BOUNDARY_PROVENANCE_PREFIX: &str = "truth_boundary_policy_id=";

/// The policy id stamped for every Genesis teammate. Mirrors
/// `argo_capability::GENESIS_DEFAULT_POLICY_ID` by value; `argo-identity`
/// does not depend on `argo-capability`, so the constant is mirrored here
/// with this cross-reference (a `debug_assert` in the runner pins them in
/// lock-step). Changing one without the other would silently unbind the
/// policy at session boot.
pub const GENESIS_DEFAULT_TRUTH_BOUNDARY_POLICY_ID: &str = "genesis-default-conservative";

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

/// Extract the probation-window end (if any) from a persona's
/// `provenance_chain`, parsed as a UTC instant (DEC-025 #2 / Genesis P3).
/// Reads the canonical [`PROBATION_PROVENANCE_PREFIX`] marker the hire
/// path stamps for Genesis teammates.
///
/// Returns `None` when no marker is present (a pre-Genesis / pre-P3
/// worker, or one hired without a Genesis plan id) OR when the stamped
/// value fails to parse as RFC-3339 — the caller treats a `None` here as
/// "no probation window known". The runtime enforcement layer combines
/// this with [`is_in_probation`] to decide whether to gate.
///
/// ```
/// use argo_identity::probation_until_from_provenance;
/// let chain = vec!["probation_until=2999-01-01T00:00:00+00:00".into()];
/// assert!(probation_until_from_provenance(&chain).is_some());
/// assert!(probation_until_from_provenance(&[]).is_none());
/// // A malformed stamp parses to None (never panics).
/// assert!(probation_until_from_provenance(&["probation_until=not-a-date".into()]).is_none());
/// ```
#[must_use]
pub fn probation_until_from_provenance(
    chain: &[String],
) -> Option<chrono::DateTime<chrono::Utc>> {
    chain
        .iter()
        .find_map(|entry| entry.strip_prefix(PROBATION_PROVENANCE_PREFIX))
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc))
}

/// `true` when `now` is still inside the worker's probation window —
/// i.e. the chain carries a [`PROBATION_PROVENANCE_PREFIX`] marker whose
/// parsed instant is strictly in the future relative to `now`. Genesis
/// P3 enforcement gates every action while this holds; once `now`
/// reaches the stamped instant the teammate graduates to its normal
/// approval policy. A chain with no (or a malformed) marker is NOT in
/// probation — enforcement is keyed off the explicit stamp, and a worker
/// the hire path never marked is governed by its ordinary policy.
///
/// ```
/// use argo_identity::is_in_probation;
/// use chrono::{Duration, Utc};
/// let now = Utc::now();
/// let future = vec![format!("probation_until={}", (now + Duration::hours(1)).to_rfc3339())];
/// let past = vec![format!("probation_until={}", (now - Duration::hours(1)).to_rfc3339())];
/// assert!(is_in_probation(&future, now));
/// assert!(!is_in_probation(&past, now));
/// assert!(!is_in_probation(&[], now));
/// ```
#[must_use]
pub fn is_in_probation(chain: &[String], now: chrono::DateTime<chrono::Utc>) -> bool {
    probation_until_from_provenance(chain).is_some_and(|until| until > now)
}

/// Extract the truth-boundary `policy_id` (if any) bound to a teammate from
/// its persona `provenance_chain` (DEC-025 #6 / Genesis P5 part (a)). Reads
/// the canonical [`TRUTH_BOUNDARY_PROVENANCE_PREFIX`] marker the hire path
/// stamps for Genesis teammates. Returns `None` for a non-Genesis worker (no
/// marker) so the runner falls back to its historical (env-var) activation.
///
/// ```
/// use argo_identity::truth_boundary_policy_id_from_provenance;
/// let chain = vec!["truth_boundary_policy_id=genesis-default-conservative".into()];
/// assert_eq!(
///     truth_boundary_policy_id_from_provenance(&chain).as_deref(),
///     Some("genesis-default-conservative"),
/// );
/// assert!(truth_boundary_policy_id_from_provenance(&[]).is_none());
/// ```
#[must_use]
pub fn truth_boundary_policy_id_from_provenance(chain: &[String]) -> Option<String> {
    chain
        .iter()
        .find_map(|entry| entry.strip_prefix(TRUTH_BOUNDARY_PROVENANCE_PREFIX))
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

/// Build the persona `provenance_chain` for a hire. When the draft
/// carries a (non-empty) Genesis plan id, the worker is a Genesis hire:
/// it gets THREE markers — the [`GENESIS_PLAN_PROVENANCE_PREFIX`] marker
/// (for the `/workers/{id}/overview` provenance chip + the capability
/// fence), a [`PROBATION_PROVENANCE_PREFIX`] marker pinned at `now +
/// PROBATION_HOURS` (DEC-025 #2 / Genesis P3 — every action approval-gated
/// until it passes), AND a [`TRUTH_BOUNDARY_PROVENANCE_PREFIX`] marker
/// binding the conservative default truth-boundary policy (DEC-025 #6 /
/// Genesis P5 — declared financial/legal/medical/factual claims escalate).
/// A non-Genesis hire (no plan id) gets an empty chain: it is not a
/// generated teammate, so it is governed by its ordinary approval policy
/// with no probation window and no auto-bound claim policy. `now` is a
/// parameter so the stamp is deterministically testable.
///
/// This is the SINGLE chokepoint both Genesis hire paths funnel through
/// (`assemble_worker_from_draft` ← `create_worker_from_draft` ← flagship
/// SSE hire AND `/onboarding/first-hire`), so no Genesis hire can reach
/// the persisted worker row without ALL THREE markers.
#[must_use]
fn build_provenance_chain(
    genesis_plan_id: Option<&str>,
    now: chrono::DateTime<Utc>,
) -> Vec<String> {
    let mut chain = Vec::new();
    let Some(plan_id) = genesis_plan_id.map(str::trim).filter(|s| !s.is_empty()) else {
        return chain;
    };
    chain.push(format!("{GENESIS_PLAN_PROVENANCE_PREFIX}{plan_id}"));
    chain.push(format!(
        "{TRUTH_BOUNDARY_PROVENANCE_PREFIX}{GENESIS_DEFAULT_TRUTH_BOUNDARY_POLICY_ID}"
    ));
    let until = (now + Duration::hours(PROBATION_HOURS)).to_rfc3339();
    chain.push(format!("{PROBATION_PROVENANCE_PREFIX}{until}"));
    chain
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
    let provenance_chain = build_provenance_chain(draft.genesis_plan_id.as_deref(), Utc::now());
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
        kpis: draft.kpis.clone(),
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
        kpis: if draft.kpis.is_empty() {
            None
        } else {
            Some(serde_json::json!(draft.kpis))
        },
        level: WorkerLevel::Entry,
        trust_grade: 0,
        last_used_at: None,
        role_name: None,
    }
}

#[cfg(test)]
#[path = "store_hire_provenance_tests.rs"]
mod probation_tests;
