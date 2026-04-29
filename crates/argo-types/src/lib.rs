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
mod error;
mod ids;
mod judge;
mod lock;
mod messages;
mod routing;

pub use budget::{Budget, BudgetExhausted, BudgetKind, BudgetNode};
pub use capability::{Capability, CapabilitySet, Principal, Role, RoleBinding, SpawnKind};
pub use error::{ArgoError, ArgoResult};
pub use ids::{
    AgentId, AgentName, ForgeRunId, JudgeRecordId, MessageId, RoutingDecisionId, SessionId,
    SkillId, TenantId, ToolCallId, WorkspaceId,
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
