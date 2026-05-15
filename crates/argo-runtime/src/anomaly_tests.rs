use super::anomaly::{AnomalyPolicy, SessionAnomalyState, StepSignal};

fn policy() -> AnomalyPolicy {
    AnomalyPolicy {
        auto_pause: true,
        threshold: 0.85,
        repeated_step_limit: 3,
        max_step_latency_ms: 10_000,
    }
}

#[test]
fn repeated_outputs_cross_pause_threshold() {
    let mut state = SessionAnomalyState::default();
    let p = policy();
    let mut decision = state.observe(
        &StepSignal {
            step: 1,
            text: "same loop body [CONTINUE]".into(),
            latency_ms: 10,
        },
        &p,
    );
    assert!(!decision.should_pause);
    for step in 2..=3 {
        decision = state.observe(
            &StepSignal {
                step,
                text: "same loop body [CONTINUE]".into(),
                latency_ms: 10,
            },
            &p,
        );
    }
    assert!(decision.should_pause);
    assert!(decision.score >= 0.9);
    assert!(decision.reasons[0].contains("repeated"));
}

#[test]
fn auto_pause_false_scores_without_pausing() {
    let mut state = SessionAnomalyState::default();
    let mut p = policy();
    p.auto_pause = false;
    let mut decision = state.observe(
        &StepSignal {
            step: 1,
            text: "repeat [CONTINUE]".into(),
            latency_ms: 10,
        },
        &p,
    );
    for step in 2..=3 {
        decision = state.observe(
            &StepSignal {
                step,
                text: "repeat [CONTINUE]".into(),
                latency_ms: 10,
            },
            &p,
        );
    }
    assert!(decision.score >= 0.9);
    assert!(!decision.should_pause);
}

#[test]
fn latency_spike_can_pause_when_threshold_is_low_enough() {
    let mut state = SessionAnomalyState::default();
    let p = AnomalyPolicy {
        threshold: 0.8,
        ..policy()
    };
    let decision = state.observe(
        &StepSignal {
            step: 1,
            text: "slow but unique".into(),
            latency_ms: 20_000,
        },
        &p,
    );
    assert!(decision.should_pause);
    assert!(decision.reasons.iter().any(|r| r.contains("latency")));
}

#[test]
fn tool_budget_repetition_scores_below_default_pause_threshold() {
    let mut state = SessionAnomalyState::default();
    let p = policy();
    let text = "_[step boundary: tool-round budget exhausted; continuing in next step]_";
    let _ = state.observe(
        &StepSignal {
            step: 1,
            text: text.into(),
            latency_ms: 10,
        },
        &p,
    );
    let decision = state.observe(
        &StepSignal {
            step: 2,
            text: text.into(),
            latency_ms: 10,
        },
        &p,
    );
    assert!(decision.score >= 0.82);
    assert!(!decision.should_pause);
}
