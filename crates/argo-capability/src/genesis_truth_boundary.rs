//! Conservative default truth-boundary policy for Genesis-generated
//! teammates (DEC-025 #6 / Genesis P5 part (a) — truth-boundary bound at
//! birth).
//!
//! ## Why generated teammates need a policy bound at birth
//!
//! A teammate synthesized by Genesis acts inside a paying customer's
//! business. Until P5 it carried **no** truth-boundary `policy_id`, so the
//! already-wired runner gate (`apply_truth_boundary_gate`) was a no-op for
//! it — activation depended entirely on the process-global
//! `ARGO_TRUTH_BOUNDARY_POLICY_PATH` env var, which is set only for the
//! hand-authored Pitch / GTM agents. A generated teammate that asserted a
//! revenue figure, a legal conclusion, or a medical recommendation to a
//! customer would sail straight through.
//!
//! P5 binds a conservative default policy to **every** Genesis-provenance
//! worker so the SAME runner gate that protects Portia / Grant now protects
//! generated teammates too — `Escalate` (queue an approval, hold the
//! response) on the high-risk claim classes a producer can declare.
//!
//! ## The honest ceiling (carried verbatim from the design doc)
//!
//! The pure evaluator ([`crate::evaluate_claim`]) matches only the claim
//! classes a *producer declares* on the candidate — the promised semantic
//! classifier (design doc P-02) was never built. So an **undeclared**
//! over-claim is still `Allow`-by-default. This binding is therefore a real
//! improvement over a policy-less worker (a declared financial/legal/medical
//! claim now escalates instead of shipping), **not** a complete fabrication
//! guarantee. The claim classes below are the canonical names the
//! canonical-Q&A skill + any future classifier declare, so they fire the
//! moment a class is declared.

use crate::truth_boundary::{
    ChannelScope, ClaimClass, ClaimDisposition, EscalationKind, EscalationTarget,
    TruthBoundaryPolicy,
};
use argo_types::TenantId;

/// The stable `policy_id` bound to every Genesis-generated teammate. Mirrored
/// on the persona `provenance_chain` (prefix `truth_boundary_policy_id=`) by
/// the hire chokepoint so the runner can recognise a worker that should load
/// this policy at session boot.
pub const GENESIS_DEFAULT_POLICY_ID: &str = "genesis-default-conservative";

/// The high-risk claim classes a generated teammate must NOT assert
/// unverified to a customer. Each is `Escalate` (queue an approval, hold the
/// reply) on **every** channel — the conservative posture DEC-025 #6 calls
/// for. These names are the canonical class tokens producers (the
/// canonical-Q&A skill, a future classifier) declare, so a declared claim in
/// any of these classes is gated rather than shipped.
pub const GENESIS_GATED_CLAIM_CLASSES: &[&str] = &[
    // Unverified factual assertions presented as ground truth.
    "factual",
    // Money: revenue, pricing commitments, refunds, account balances.
    "financial",
    // Anything a customer could read as legal advice / a legal position.
    "legal",
    // Anything a customer could read as a medical recommendation.
    "medical",
];

/// Build the conservative default [`TruthBoundaryPolicy`] for a Genesis
/// teammate, scoped to `tenant_id`.
///
/// Every gated claim class (factual / financial / legal / medical) is
/// `Escalate` on all channels, routed to the operator approver. The policy's
/// `tenant_id` is the worker's tenant, so the runner's per-turn tenant guard
/// (`active_policy_for`) applies it only to that tenant's sessions — a
/// cross-tenant session can never pick up another tenant's policy.
///
/// # Examples
///
/// ```
/// use argo_capability::{genesis_default_policy, GENESIS_DEFAULT_POLICY_ID};
/// use argo_capability::{ChannelKind, ClaimCandidate, ClaimDisposition, evaluate_claim};
/// use argo_types::TenantId;
///
/// let tenant = TenantId::new();
/// let policy = genesis_default_policy(tenant);
/// assert_eq!(policy.id, GENESIS_DEFAULT_POLICY_ID);
/// assert_eq!(policy.tenant_id, tenant);
///
/// // A declared financial claim escalates (held for approval), on any channel.
/// let verdict = evaluate_claim(
///     &policy,
///     ChannelKind::Chat,
///     &ClaimCandidate {
///         text: "Our ARR is $4.2M.".into(),
///         declared_classes: vec!["financial".into()],
///     },
/// );
/// assert_eq!(verdict.disposition, ClaimDisposition::Escalate);
///
/// // An undeclared claim is Allow-by-default (the honest ceiling).
/// let undeclared = evaluate_claim(
///     &policy,
///     ChannelKind::Chat,
///     &ClaimCandidate { text: "Our ARR is $4.2M.".into(), declared_classes: vec![] },
/// );
/// assert_eq!(undeclared.disposition, ClaimDisposition::Allow);
/// ```
#[must_use]
pub fn genesis_default_policy(tenant_id: TenantId) -> TruthBoundaryPolicy {
    let claim_classes = GENESIS_GATED_CLAIM_CLASSES
        .iter()
        .map(|name| ClaimClass {
            name: (*name).to_string(),
            disposition: ClaimDisposition::Escalate,
            channels: ChannelScope::AllChannels,
            escalate_to: Some(EscalationTarget {
                // Route to the tenant operator via the approvals subsystem;
                // the runner's `Escalate` path queues an approval request and
                // holds the reply behind it.
                kind: EscalationKind::Approver,
                address: "operator".to_string(),
            }),
        })
        .collect();
    TruthBoundaryPolicy {
        id: GENESIS_DEFAULT_POLICY_ID.to_string(),
        tenant_id,
        claim_classes,
    }
}

#[cfg(test)]
#[path = "genesis_truth_boundary_tests.rs"]
mod tests;
