//! Tests for `seed_team_headline_activity`.

use super::ACTIVITY_PLAN;
use crate::personas::DEMO_SEED_PERSONAS;

#[test]
fn activity_plan_has_one_nonzero_entry_per_persona() {
    assert_eq!(
        ACTIVITY_PLAN.len(),
        DEMO_SEED_PERSONAS.len(),
        "ACTIVITY_PLAN must carry one (assistant_message, tool_call) entry per hero persona"
    );
    for (i, (messages, tool_calls)) in ACTIVITY_PLAN.iter().enumerate() {
        assert!(
            messages + tool_calls > 0,
            "persona index {i} would seed zero activity -> its card would still read 'No activity yet.'"
        );
    }
}
