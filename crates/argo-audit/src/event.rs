//! Typed audit-event kinds.
//!
//! Phase-1 kinds (turn boundaries + tool dispatch) plus the 11 v0.2
//! kinds called out in §17.1: routing, judge, RBAC, forge,
//! marketplace, replay, lock, steer.
use serde::{Deserialize, Serialize};

/// Stable `kind` label persisted in the `audit_record.kind` column.
///
/// Adding a variant is API-stable; renaming or removing one is not —
/// a rename means a schema migration so old records still verify.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditKind {
    // ── Phase-1 kinds (the agent loop) ────────────────────────────
    /// `run_turn` started.
    TurnStart,
    /// Assistant produced a message (text or tool calls).
    AssistantMessage,
    /// A tool call was dispatched.
    ToolCall,
    /// A tool result came back.
    ToolResult,
    /// `run_turn` ended (with a final answer or halt).
    TurnEnd,
    /// A halt event (max iterations, error, etc.).
    Halt,
    // ── v0.2 kinds (§17.1) ────────────────────────────────────────
    /// Cost-router decision (Amendment 1, §24).
    RoutingDecision,
    /// Judge verdict (Amendment 2, §25).
    JudgeVerdict,
    /// RBAC denial — principal lacked the required capability.
    RbacDeny,
    /// Forge optimization run started.
    ForgeRunStart,
    /// Forge optimization run ended.
    ForgeRunEnd,
    /// Workspace bundle pushed to the OCI marketplace.
    MarketplacePush,
    /// Workspace bundle pulled from the OCI marketplace.
    MarketplacePull,
    /// Time-travel session replay started.
    SessionReplay,
    /// Lock set or cleared on a memory entry / skill.
    LockSet,
    /// Lock violated (denied agent edit).
    LockViolate,
    /// Steer enqueued onto a running session.
    SteerEnqueued,
    /// A tool dispatch was halted because the bound vertical's
    /// `[escalation].human_approval_required` list named the tool
    /// (#660). The dispatch did NOT run; an operator must approve
    /// (or refuse) before the agent can re-issue the call.
    EscalationRequired,
    /// An operator granted human approval for a previously-
    /// blocked tool on a specific session via `argo session
    /// approve` or the gateway equivalent (#664).
    ApprovalGranted,
    /// An operator revoked a previously-granted approval (#664).
    ApprovalRevoked,
    /// A one-shot approval grant was consumed by a successful
    /// gate-passing dispatch (#667). The corresponding grant
    /// row's `consumed_at` column is now populated; subsequent
    /// dispatches of the same tool re-block unless another grant
    /// is in place.
    ApprovalConsumed,
    /// A capability-gap signal was detected (#682, work order
    /// #4): repeated failure / manual correction / weak
    /// evidence / missing knowledge / policy denial / repeated
    /// similar task. Payload carries the gap kind + the
    /// underlying signal (e.g. `{kind:"policy_denial",
    /// tool:"...", count: N}`). The detector NEVER auto-bypasses
    /// policy — it only records the signal so a human (or the
    /// learned-skill review gate, #681) can decide what to do.
    CapabilityGap,
    /// A learned skill in `pending_review` was promoted to
    /// `active` by a human reviewer (#684). Distinct from
    /// `ApprovalGranted` (which is for tool capability grants).
    /// Payload: `{skill_id, name, vertical_id?}`.
    SkillApproved,
    /// A learned skill in `pending_review` was rejected by a
    /// human reviewer (#684). The skill row stays in the table
    /// for audit but the lookup never returns it.
    SkillRejected,
    /// A memory-backed skill candidate (JSON in MEMORY.md body
    /// matching the `learned_skill_candidates` schema_key
    /// convention) was materialised into a `pending_review`
    /// skill row by an operator (#697). Distinct from
    /// `SkillApproved` because nothing is activated yet — the
    /// promote step just moves the artefact from memory into
    /// the skills table for review.
    /// Payload: `{skill_id, name, vertical_id, memory_entry_id}`.
    SkillCandidatePromoted,
    /// A skill version was rolled back to a prior active-eligible
    /// row by an operator. Payload carries `name`, from/to
    /// versions, skill ids, and optional reason.
    SkillRolledBack,
    /// One inbound HTTP request to the gateway. Recorded after the
    /// route runs so the payload can carry response status +
    /// wall-clock duration alongside method, path (query string
    /// stripped — may carry secrets), and the resolved principal.
    /// Liveness probes (`/health*`, `/metrics*`) are skipped by the
    /// gateway wrapper to keep the chain free of probe noise.
    HttpRequest,
    /// Whole-process worker subprocess died and the supervisor
    /// reacted (mission-oom-restart-handler). Payload carries the
    /// typed `WorkerExitReason` (Clean / OomKilled / OtherSignal /
    /// Hung) + the supervisor's `RestartDecision` (Restart with
    /// backoff / GiveUp with salvage note / IgnoreCleanExit) + the
    /// attempt counter within the rolling restart window. Distinct
    /// from `Halt` (per-turn iteration cap) and from the executor
    /// `RestartStrategy` paths (per-action retry, not whole-process
    /// death). One row per non-clean exit.
    WorkerOomRestart,
}

impl AuditKind {
    /// Wire label persisted in the `kind` column.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::TurnStart => "turn_start",
            Self::AssistantMessage => "assistant_message",
            Self::ToolCall => "tool_call",
            Self::ToolResult => "tool_result",
            Self::TurnEnd => "turn_end",
            Self::Halt => "halt",
            Self::RoutingDecision => "routing.decision",
            Self::JudgeVerdict => "judge.verdict",
            Self::RbacDeny => "rbac.deny",
            Self::ForgeRunStart => "forge.run.start",
            Self::ForgeRunEnd => "forge.run.end",
            Self::MarketplacePush => "marketplace.push",
            Self::MarketplacePull => "marketplace.pull",
            Self::SessionReplay => "session.replay",
            Self::LockSet => "lock.set",
            Self::LockViolate => "lock.violate",
            Self::SteerEnqueued => "steer.enqueued",
            Self::EscalationRequired => "escalation.required",
            Self::ApprovalGranted => "approval.granted",
            Self::ApprovalRevoked => "approval.revoked",
            Self::ApprovalConsumed => "approval.consumed",
            Self::CapabilityGap => "capability.gap",
            Self::SkillApproved => "skill.approved",
            Self::SkillRejected => "skill.rejected",
            Self::SkillCandidatePromoted => "skill.candidate.promoted",
            Self::SkillRolledBack => "skill.rollback",
            Self::HttpRequest => "http.request",
            Self::WorkerOomRestart => "worker.oom_restart",
        }
    }

    /// Parse from a `kind` column value. Returns `None` for unknown
    /// labels — `argo audit verify` flags those rows so an out-of-band
    /// migration didn't introduce silent corruption.
    #[must_use]
    pub fn from_label(s: &str) -> Option<Self> {
        Some(match s {
            "turn_start" => Self::TurnStart,
            "assistant_message" => Self::AssistantMessage,
            "tool_call" => Self::ToolCall,
            "tool_result" => Self::ToolResult,
            "turn_end" => Self::TurnEnd,
            "halt" => Self::Halt,
            "routing.decision" => Self::RoutingDecision,
            "judge.verdict" => Self::JudgeVerdict,
            "rbac.deny" => Self::RbacDeny,
            "forge.run.start" => Self::ForgeRunStart,
            "forge.run.end" => Self::ForgeRunEnd,
            "marketplace.push" => Self::MarketplacePush,
            "marketplace.pull" => Self::MarketplacePull,
            "session.replay" => Self::SessionReplay,
            "lock.set" => Self::LockSet,
            "lock.violate" => Self::LockViolate,
            "steer.enqueued" => Self::SteerEnqueued,
            "escalation.required" => Self::EscalationRequired,
            "approval.granted" => Self::ApprovalGranted,
            "approval.revoked" => Self::ApprovalRevoked,
            "approval.consumed" => Self::ApprovalConsumed,
            "capability.gap" => Self::CapabilityGap,
            "skill.approved" => Self::SkillApproved,
            "skill.rejected" => Self::SkillRejected,
            "skill.candidate.promoted" => Self::SkillCandidatePromoted,
            "skill.rollback" => Self::SkillRolledBack,
            "http.request" => Self::HttpRequest,
            "worker.oom_restart" => Self::WorkerOomRestart,
            _ => return None,
        })
    }

    /// `true` for kinds added in v0.2 (§17.1).
    #[must_use]
    pub fn is_v02(self) -> bool {
        matches!(
            self,
            Self::RoutingDecision
                | Self::JudgeVerdict
                | Self::RbacDeny
                | Self::ForgeRunStart
                | Self::ForgeRunEnd
                | Self::MarketplacePush
                | Self::MarketplacePull
                | Self::SessionReplay
                | Self::LockSet
                | Self::LockViolate
                | Self::SteerEnqueued
        )
    }

    /// `true` for kinds added by the vertical-template surface
    /// (Phase 6+). Today nine: `EscalationRequired` (#660 gate),
    /// `ApprovalGranted` (#664 grant), `ApprovalRevoked` (#664
    /// revoke), `ApprovalConsumed` (#667 one-shot dispatch),
    /// `CapabilityGap` (#682 detection), `SkillApproved` (#684),
    /// `SkillRejected` (#684), `SkillCandidatePromoted` (#697
    /// memory→pending_review materialisation), and
    /// `SkillRolledBack`. The gateway / CLI
    /// rollups treat these the same as any other audit kind.
    #[must_use]
    pub fn is_vertical(self) -> bool {
        matches!(
            self,
            Self::EscalationRequired
                | Self::ApprovalGranted
                | Self::ApprovalRevoked
                | Self::ApprovalConsumed
                | Self::CapabilityGap
                | Self::SkillApproved
                | Self::SkillRejected
                | Self::SkillCandidatePromoted
                | Self::SkillRolledBack
        )
    }

    /// All known kinds, sorted by label. Used by tests + `argo audit
    /// verify` to enumerate expected events.
    #[must_use]
    pub fn all() -> Vec<Self> {
        let mut v = vec![
            Self::TurnStart,
            Self::AssistantMessage,
            Self::ToolCall,
            Self::ToolResult,
            Self::TurnEnd,
            Self::Halt,
            Self::RoutingDecision,
            Self::JudgeVerdict,
            Self::RbacDeny,
            Self::ForgeRunStart,
            Self::ForgeRunEnd,
            Self::MarketplacePush,
            Self::MarketplacePull,
            Self::SessionReplay,
            Self::LockSet,
            Self::LockViolate,
            Self::SteerEnqueued,
            Self::EscalationRequired,
            Self::ApprovalGranted,
            Self::ApprovalRevoked,
            Self::ApprovalConsumed,
            Self::CapabilityGap,
            Self::SkillApproved,
            Self::SkillRejected,
            Self::SkillCandidatePromoted,
            Self::SkillRolledBack,
            Self::HttpRequest,
            Self::WorkerOomRestart,
        ];
        v.sort_by_key(|k| k.label());
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_round_trip_for_every_kind() {
        for k in AuditKind::all() {
            assert_eq!(AuditKind::from_label(k.label()), Some(k), "{:?}", k);
        }
    }

    #[test]
    fn all_eleven_v02_kinds_present() {
        let v02_count = AuditKind::all().iter().filter(|k| k.is_v02()).count();
        assert_eq!(v02_count, 11);
    }

    #[test]
    fn unknown_label_yields_none() {
        assert!(AuditKind::from_label("not_a_kind").is_none());
    }

    #[test]
    fn phase1_kinds_marked_not_v02() {
        for k in [
            AuditKind::TurnStart,
            AuditKind::AssistantMessage,
            AuditKind::ToolCall,
            AuditKind::ToolResult,
            AuditKind::TurnEnd,
            AuditKind::Halt,
        ] {
            assert!(!k.is_v02(), "{:?} should not be v02", k);
        }
    }

    #[test]
    fn escalation_required_is_known_vertical_kind() {
        // Locked: gateway rollups, CLI filters, and the verifier
        // all walk `AuditKind::all()` — escalation.required must
        // be enumerable there.
        let labels: Vec<&'static str> = AuditKind::all().iter().map(|k| k.label()).collect();
        assert!(
            labels.contains(&"escalation.required"),
            "got: {labels:?}"
        );
        assert!(AuditKind::EscalationRequired.is_vertical());
        assert!(!AuditKind::EscalationRequired.is_v02());
        assert_eq!(
            AuditKind::from_label("escalation.required"),
            Some(AuditKind::EscalationRequired)
        );
    }

    #[test]
    fn approval_kinds_round_trip_and_are_vertical() {
        for (label, kind) in [
            ("approval.granted", AuditKind::ApprovalGranted),
            ("approval.revoked", AuditKind::ApprovalRevoked),
            ("approval.consumed", AuditKind::ApprovalConsumed),
        ] {
            assert_eq!(AuditKind::from_label(label), Some(kind));
            assert_eq!(kind.label(), label);
            assert!(kind.is_vertical(), "{kind:?} should be vertical");
            assert!(!kind.is_v02(), "{kind:?} must not be v02");
        }
        // all three must appear in all()
        let labels: Vec<&'static str> = AuditKind::all().iter().map(|k| k.label()).collect();
        assert!(labels.contains(&"approval.granted"));
        assert!(labels.contains(&"approval.revoked"));
        assert!(labels.contains(&"approval.consumed"));
    }

    #[test]
    fn capability_gap_round_trips_and_is_vertical() {
        let kind = AuditKind::CapabilityGap;
        assert_eq!(AuditKind::from_label("capability.gap"), Some(kind));
        assert_eq!(kind.label(), "capability.gap");
        assert!(kind.is_vertical());
        assert!(!kind.is_v02());
        let labels: Vec<&'static str> = AuditKind::all().iter().map(|k| k.label()).collect();
        assert!(labels.contains(&"capability.gap"));
    }

    #[test]
    fn worker_oom_restart_round_trips_and_listed() {
        let kind = AuditKind::WorkerOomRestart;
        assert_eq!(AuditKind::from_label("worker.oom_restart"), Some(kind));
        assert_eq!(kind.label(), "worker.oom_restart");
        assert!(!kind.is_v02());
        assert!(!kind.is_vertical());
        let labels: Vec<&'static str> = AuditKind::all().iter().map(|k| k.label()).collect();
        assert!(labels.contains(&"worker.oom_restart"));
    }

    #[test]
    fn skill_review_kinds_round_trip_and_are_vertical() {
        for (label, kind) in [
            ("skill.approved", AuditKind::SkillApproved),
            ("skill.rejected", AuditKind::SkillRejected),
            ("skill.candidate.promoted", AuditKind::SkillCandidatePromoted),
            ("skill.rollback", AuditKind::SkillRolledBack),
        ] {
            assert_eq!(AuditKind::from_label(label), Some(kind));
            assert_eq!(kind.label(), label);
            assert!(kind.is_vertical());
            assert!(!kind.is_v02());
        }
        let labels: Vec<&'static str> = AuditKind::all().iter().map(|k| k.label()).collect();
        assert!(labels.contains(&"skill.approved"));
        assert!(labels.contains(&"skill.rejected"));
        assert!(labels.contains(&"skill.candidate.promoted"));
        assert!(labels.contains(&"skill.rollback"));
    }
}
