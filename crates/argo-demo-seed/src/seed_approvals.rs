//! Canonical demo approval set — the rows `seed::seed_approval_requests`
//! writes into `approval_request` + `approval.asked` audit rows.
//!
//! Extracted from `seed.rs` (mechanical move, no behaviour change) to keep
//! that file under the 500-LOC ceiling. Two demo-data properties live here:
//!
//! * **Role-appropriate ownership** — each row names its **owner persona**
//!   (display name, matched against [`DEMO_SEED_PERSONAS`]) so the
//!   `approval.asked` audit row carries that worker's id. The Memory tab's
//!   "Recent decisions" filters by `worker_id`, so owner-by-name keeps a
//!   *recruiting* "candidate rejection email" on Priya (Recruiter), not the
//!   Marketing teammate a blind round-robin landed it on.
//! * **A trusted-AI ratio + variety** — the banner promises "refunds,
//!   hires, contracts," so the set spans a refund (Jamal/Support), a hire
//!   (Priya/Recruiting), a contract (Grant/GTM), marketing drafts (Marcus),
//!   and marketplace/comms actions. Of the decided rows, 7 approved / 2
//!   refused = ~78% — a healthy approval rate, not a 24% distrust signal.

use argo_store::approvals::ApprovalDecision;

use crate::personas::DEMO_SEED_PERSONAS;

/// One seeded approval: the human label + draft summary, the **owner
/// persona** (display name, matched against [`DEMO_SEED_PERSONAS`]), and
/// an optional pre-decided outcome (`None` = pending in the inbox).
pub(crate) struct ApprovalSeedRow {
    pub(crate) owner: &'static str,
    pub(crate) label: &'static str,
    pub(crate) summary: &'static str,
    pub(crate) decision: Option<ApprovalDecision>,
}

/// The canonical demo approval set: role-appropriate owners, a variety of
/// action types (refund / hire / contract / marketing / marketplace /
/// comms), and a trusted-AI approve:refuse ratio among the decided rows.
///
/// Composition: 5 pending (actionable in the inbox) + 9 decided (feed the
/// 7-day approval-rate ring). The decided split is 7 approved / 2 refused
/// = ~78% approval — a healthy "humans approve most, refuse the rare
/// exception" story, NOT the 24% distrust signal the prior set produced.
pub(crate) fn demo_approval_rows() -> [ApprovalSeedRow; 14] {
    use ApprovalDecision::{Approve, Reject};
    [
        // ── 5 pending — actionable in the demo inbox ─────────────────
        ApprovalSeedRow {
            owner: "Mariah",
            label: "listing repost > $200",
            summary: "Repost listing #4912 at $215 (was $189).",
            decision: None,
        },
        ApprovalSeedRow {
            owner: "Jamal",
            label: "refund > $500",
            summary: "Refund $540 to customer #18293 for the damaged CCM helmet order.",
            decision: None,
        },
        ApprovalSeedRow {
            owner: "Priya",
            label: "candidate interview slot",
            summary: "Book Diego Alvarez interview Thursday 10:30 AM NZST (45 min).",
            decision: None,
        },
        ApprovalSeedRow {
            owner: "Grant",
            label: "enterprise contract terms",
            summary: "Send Northwind Trading the NET-30 renewal contract at the agreed $48k ARR.",
            decision: None,
        },
        ApprovalSeedRow {
            owner: "Marcus",
            label: "marketing reply draft",
            summary: "Post the drafted LinkedIn reply to the \"agents vs. tools\" thread.",
            decision: None,
        },
        // ── 9 decided — feed the 7-day approval-rate ring (~78%) ─────
        ApprovalSeedRow {
            owner: "Priya",
            label: "candidate hire offer",
            summary: "Extend the platform-engineer offer to Maria Garcia at the approved band.",
            decision: Some(Approve),
        },
        ApprovalSeedRow {
            owner: "Jamal",
            label: "refund within policy",
            summary: "Refund $89 to customer #17042 for a late delivery (within Tier-1 cap).",
            decision: Some(Approve),
        },
        ApprovalSeedRow {
            owner: "Mariah",
            label: "buyer counter-offer at $85",
            summary: "Counter CCM helmet offer from $75 to $85 with free local pickup.",
            decision: Some(Approve),
        },
        ApprovalSeedRow {
            owner: "Nori",
            label: "Q2 product update broadcast",
            summary: "Announce audit-log export to 12,400 customers Tuesday 09:00 PST.",
            decision: Some(Approve),
        },
        ApprovalSeedRow {
            owner: "Grant",
            label: "outbound sequence launch",
            summary: "Launch the 3-step .edu enterprise outreach sequence to 84 qualified leads.",
            decision: Some(Approve),
        },
        ApprovalSeedRow {
            owner: "Marcus",
            label: "thought-leadership post",
            summary: "Publish the \"ideas are the moat\" post to Mark's LinkedIn (on-voice draft).",
            decision: Some(Approve),
        },
        ApprovalSeedRow {
            owner: "Peta",
            label: "deadline pull-in",
            summary: "Pull the Q2 launch milestone in by three days to hit the partner demo.",
            decision: Some(Approve),
        },
        // 2 refused — the exceptions that prove the human-oversight loop.
        ApprovalSeedRow {
            owner: "Mariah",
            label: "discount above policy ceiling",
            summary: "Apply 32% discount on Tomás Industries renewal QT-2024-7283.",
            decision: Some(Reject),
        },
        ApprovalSeedRow {
            owner: "Priya",
            label: "candidate rejection email",
            summary: "Send warm rejection to Sara Lin with feedback (held to revise wording).",
            decision: Some(Reject),
        },
    ]
}

/// Resolve a `worker_id` by its persona display name. `worker_ids` is
/// positional-aligned with [`DEMO_SEED_PERSONAS`] (the spine
/// `seed_demo_workers` walked), so the name's index into the persona
/// slice is the matching worker id. Returns `None` when the name is not
/// in the canonical roster.
pub(crate) fn worker_id_for_persona(
    name: &str,
    worker_ids: &[argo_identity::WorkerId],
) -> Option<argo_identity::WorkerId> {
    DEMO_SEED_PERSONAS
        .iter()
        .position(|p| p.name == name)
        .and_then(|idx| worker_ids.get(idx).copied())
}
