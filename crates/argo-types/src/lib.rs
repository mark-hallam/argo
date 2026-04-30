//! `argo-types` — shared canonical types for Argo.
//!
//! This crate has **zero dependencies on other Argo crates**. It is the
//! foundation that every other crate consumes.
//!
//! See `docs/plan/argo-build-plan.md` §5 for the normative spec.
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod budget;
mod capability;
mod cost_ceilings;
mod error;
mod ids;
mod judge;
mod lock;
mod messages;
mod routing;

pub use budget::{Budget, BudgetExhausted, BudgetKind, BudgetNode};
pub use capability::{Capability, CapabilitySet, Principal, Role, RoleBinding, SpawnKind};
pub use cost_ceilings::{
    enforce_pre_spend, CapBlocked, CapScope, CostCeilings, EnforceContext, NotifyEvent,
    SpendCounters,
};
pub use error::{ArgoError, ArgoResult};
pub use ids::{
    AgentId, AgentName, ForgeRunId, JudgeRecordId, MessageId, RequestId, RoutingDecisionId,
    SessionId, SkillId, TenantId, ToolCallId, WorkspaceId,
};
pub use judge::{JudgeRecord, JudgeSubject, JudgeVerdict};
pub use lock::Lock;
pub use messages::{ContentBlock, Message, MessageRole, ToolCallRequest, ToolCallResult};
pub use routing::{ProviderModel, RoutingDecision, RoutingPolicy, TieredRouting};

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn id_types_are_uuidv7() {
        let s = SessionId::new();
        let m = MessageId::new();
        assert_eq!(s.0.get_version_num(), 7);
        assert_eq!(m.0.get_version_num(), 7);
    }

    #[test]
    fn capability_glob_covers_basic() {
        let g = CapabilitySet(vec![Capability::FsRead {
            path_glob: "/tmp/**".into(),
        }]);
        assert!(g.covers(&Capability::FsRead {
            path_glob: "/tmp/foo/bar.txt".into()
        }));
        assert!(!g.covers(&Capability::FsRead {
            path_glob: "/etc/passwd".into()
        }));
    }

    #[test]
    fn capability_kind_must_match() {
        let g = CapabilitySet(vec![Capability::FsRead {
            path_glob: "**".into(),
        }]);
        assert!(!g.covers(&Capability::FsWrite {
            path_glob: "/tmp/x".into()
        }));
    }

    #[test]
    fn net_egress_port_semantics() {
        let unrestricted = CapabilitySet(vec![Capability::NetEgress {
            host_glob: "*.example.com".into(),
            port: None,
        }]);
        let restricted = CapabilitySet(vec![Capability::NetEgress {
            host_glob: "*.example.com".into(),
            port: Some(443),
        }]);
        let want_port_443 = Capability::NetEgress {
            host_glob: "api.example.com".into(),
            port: Some(443),
        };
        let want_no_port = Capability::NetEgress {
            host_glob: "api.example.com".into(),
            port: None,
        };
        assert!(unrestricted.covers(&want_port_443));
        assert!(unrestricted.covers(&want_no_port));
        assert!(restricted.covers(&want_port_443));
        assert!(!restricted.covers(&want_no_port));
    }

    #[test]
    fn budget_charges_propagate() {
        let root = BudgetNode::root(&Budget {
            iterations: Some(5),
            ..Default::default()
        });
        let child = root.fork(&Budget {
            iterations: Some(10),
            ..Default::default()
        });
        assert!(child.charge(BudgetKind::Iterations, 1).is_ok());
        assert!(child.charge(BudgetKind::Iterations, 1).is_ok());
        assert!(child.charge(BudgetKind::Iterations, 1).is_ok());
        assert!(child.charge(BudgetKind::Iterations, 1).is_ok());
        assert!(child.charge(BudgetKind::Iterations, 1).is_ok());
        assert!(child.charge(BudgetKind::Iterations, 1).is_err());
    }

    #[test]
    fn budget_independent_kinds() {
        let n = BudgetNode::root(&Budget {
            iterations: Some(2),
            tokens_in: Some(100),
            ..Default::default()
        });
        assert!(n.charge(BudgetKind::TokensIn, 50).is_ok());
        assert!(n.charge(BudgetKind::Iterations, 1).is_ok());
        assert!(n.charge(BudgetKind::Iterations, 1).is_ok());
        assert!(n.charge(BudgetKind::Iterations, 1).is_err());
        assert!(n.charge(BudgetKind::TokensIn, 50).is_ok());
        assert!(n.charge(BudgetKind::TokensIn, 1).is_err());
    }

    fn enforce_ctx() -> EnforceContext {
        EnforceContext {
            tenant: TenantId::new(),
            session: SessionId::new(),
        }
    }

    #[test]
    fn cost_ceilings_block_at_session_cap() {
        let c = CostCeilings {
            per_session_usd_micros: Some(1_000_000),
            ..CostCeilings::default()
        };
        let used = SpendCounters {
            session_used: 900_000,
            ..SpendCounters::default()
        };
        let err = enforce_pre_spend(&c, used, 200_000, enforce_ctx()).unwrap_err();
        assert_eq!(err.scope, CapScope::Session);
        assert_eq!(err.cap_usd_micros, 1_000_000);
    }

    #[test]
    fn cost_ceilings_block_at_day_cap_even_if_session_ok() {
        let c = CostCeilings {
            per_session_usd_micros: Some(1_000_000_000),
            per_day_usd_micros: Some(10_000_000),
            ..CostCeilings::default()
        };
        let used = SpendCounters {
            session_used: 0,
            day_used: 9_500_000,
            month_used: 0,
        };
        let err = enforce_pre_spend(&c, used, 1_000_000, enforce_ctx()).unwrap_err();
        assert_eq!(err.scope, CapScope::Day);
    }

    #[test]
    fn cost_ceilings_unlimited_never_blocks() {
        let c = CostCeilings::unlimited();
        let used = SpendCounters {
            session_used: u64::MAX / 2,
            day_used: u64::MAX / 2,
            month_used: u64::MAX / 2,
        };
        let events = enforce_pre_spend(&c, used, 1_000_000, enforce_ctx()).unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn cost_ceilings_emit_80_pct_notification_on_first_crossing() {
        let c = CostCeilings {
            per_session_usd_micros: Some(1_000_000),
            notify_at_pct: vec![80, 95],
            ..CostCeilings::default()
        };
        let used = SpendCounters {
            session_used: 700_000,
            ..SpendCounters::default()
        };
        let events = enforce_pre_spend(&c, used, 150_000, enforce_ctx()).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].scope, CapScope::Session);
        assert_eq!(events[0].threshold_pct, 80);
    }

    #[test]
    fn cost_ceilings_emit_both_80_and_95_when_jump_crosses_both() {
        let c = CostCeilings {
            per_session_usd_micros: Some(1_000_000),
            notify_at_pct: vec![80, 95],
            ..CostCeilings::default()
        };
        let used = SpendCounters {
            session_used: 100_000,
            ..SpendCounters::default()
        };
        let events = enforce_pre_spend(&c, used, 900_000, enforce_ctx()).unwrap();
        let pcts: Vec<u8> = events.iter().map(|e| e.threshold_pct).collect();
        assert!(pcts.contains(&80));
        assert!(pcts.contains(&95));
    }

    #[test]
    fn cost_ceilings_no_notification_on_subsequent_charge_after_crossing() {
        // First charge crosses 80% → emits one event.
        // Second charge (still under cap) is past 80% already →
        // emits no event.
        let c = CostCeilings {
            per_session_usd_micros: Some(1_000_000),
            notify_at_pct: vec![80, 95],
            ..CostCeilings::default()
        };
        let after_first = SpendCounters {
            session_used: 850_000,
            ..SpendCounters::default()
        };
        let events = enforce_pre_spend(&c, after_first, 50_000, enforce_ctx()).unwrap();
        assert!(events.iter().all(|e| e.threshold_pct != 80));
    }

    #[test]
    fn cost_ceilings_block_uses_pre_spend_state() {
        // CapBlocked is returned BEFORE spend is mutated externally.
        // This is the contract: callers must not record a charge
        // when an Err comes back.
        let c = CostCeilings {
            per_session_usd_micros: Some(1_000),
            ..CostCeilings::default()
        };
        let used = SpendCounters {
            session_used: 500,
            ..SpendCounters::default()
        };
        let result = enforce_pre_spend(&c, used, 600, enforce_ctx());
        assert!(result.is_err());
        // `used` still reads as 500 — this module does not mutate it.
        assert_eq!(used.session_used, 500);
    }

    proptest! {
        #[test]
        fn capability_self_covers(path in "[a-z/]{1,20}") {
            let cap = Capability::FsRead { path_glob: path.clone() };
            let set = CapabilitySet(vec![cap.clone()]);
            prop_assert!(set.covers(&cap));
        }

        #[test]
        fn budget_never_exceeds_root(
            charges in proptest::collection::vec(1u32..10, 1..30),
            cap in 5u32..50,
        ) {
            let root = BudgetNode::root(&Budget {
                iterations: Some(cap),
                ..Default::default()
            });
            let mut allowed = 0u32;
            for c in charges {
                if root.charge(BudgetKind::Iterations, u64::from(c)).is_ok() {
                    allowed = allowed.saturating_add(c);
                }
            }
            prop_assert!(allowed <= cap);
        }
    }
}
