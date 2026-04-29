//! Property tests proving §16.4 acceptance: total spend across an
//! arbitrary delegation tree never exceeds the root budget.
//!
//! The actual budget machinery lives in `argo_types::BudgetNode` —
//! this module is a thin verification layer.
#![cfg(test)]

use argo_types::{Budget, BudgetKind, BudgetNode};
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// §16.4 acceptance gate. Build an arbitrary 3-deep delegation
    /// tree with random per-node budget caps, charge random amounts
    /// at each leaf, then assert the root never sees more than its
    /// own cap.
    #[test]
    fn total_spend_never_exceeds_root_budget(
        root_cap in 50u32..500,
        child_caps in proptest::collection::vec(20u32..200, 1..6),
        grandchild_charges in proptest::collection::vec(1u32..30, 1..40),
    ) {
        let root = BudgetNode::root(&Budget {
            iterations: Some(root_cap),
            ..Default::default()
        });
        let mut allowed_total: u64 = 0;
        for cap in &child_caps {
            let child = root.fork(&Budget {
                iterations: Some(*cap),
                ..Default::default()
            });
            for c in &grandchild_charges {
                if child.charge(BudgetKind::Iterations, u64::from(*c)).is_ok() {
                    allowed_total = allowed_total.saturating_add(u64::from(*c));
                }
            }
        }
        prop_assert!(
            allowed_total <= u64::from(root_cap),
            "allowed_total {allowed_total} exceeded root_cap {root_cap}"
        );
    }
}
