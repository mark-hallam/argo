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
    /// (Phase 6+). Today there is one (`EscalationRequired`,
    /// #660) — the gateway / CLI rollups treat these the same as
    /// any other audit kind.
    #[must_use]
    pub fn is_vertical(self) -> bool {
        matches!(self, Self::EscalationRequired)
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
}
