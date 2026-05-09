//! Unit tests for the restart layer. Cover the brief's seven cases:
//!
//! 1. `should_restart` exhausts retry budget then bails.
//! 2. `should_restart` distinguishes retryable vs fatal error kinds.
//! 3. `take_snapshot` captures the right step index.
//! 4. `restore_from` re-seeds the prompt with goal summary.
//! 5. SnapshotResetRetry with `snapshot_every_n_steps = 3` snapshots
//!    at the right cadence.
//! 6. RetryWithContext (no snapshot) preserves the trajectory log
//!    across attempts (i.e. snapshots don't appear).
//! 7. None strategy is a no-op.

use super::*;
use crate::CuError;

fn http_err() -> CuError {
    CuError::Protocol("transient model parse failure".into())
}

fn fatal_err() -> CuError {
    CuError::BudgetExhausted {
        step: 4,
        reason: "tokens".into(),
    }
}

#[test]
fn case_1_exhausts_budget_then_bails() {
    let mut c = RestartCoordinator::new(RestartStrategy::RetryWithContext {
        max_attempts: 2,
        backoff_ms: 0,
    });
    // 2 attempts allowed.
    assert_eq!(
        c.should_restart(&http_err()),
        RestartDecision::Retry { with_context: true }
    );
    assert_eq!(c.attempts(), 1);
    assert_eq!(
        c.should_restart(&http_err()),
        RestartDecision::Retry { with_context: true }
    );
    assert_eq!(c.attempts(), 2);
    // 3rd attempt: budget exhausted → Bail.
    assert_eq!(c.should_restart(&http_err()), RestartDecision::Bail);
    assert_eq!(c.attempts(), 2, "Bail does not charge a fresh attempt");
}

#[test]
fn case_2_retryable_vs_fatal_error_kinds() {
    let mut c = RestartCoordinator::new(RestartStrategy::SnapshotResetRetry {
        max_attempts: 5,
        snapshot_every_n_steps: 1,
    });
    // Fatal error: Continue (no charge).
    assert_eq!(c.should_restart(&fatal_err()), RestartDecision::Continue);
    assert_eq!(c.attempts(), 0);
    // Retryable: Retry.
    let dec = c.should_restart(&CuError::HttpStatus {
        stage: "anthropic/messages",
        status: 502,
        body: "bad gateway".into(),
    });
    assert_eq!(dec, RestartDecision::Retry { with_context: false });
    assert_eq!(c.attempts(), 1);
    // Another fatal kind: still Continue.
    let unknown_profile = CuError::UnknownProfile {
        requested: "x".into(),
        registered: "y".into(),
    };
    assert_eq!(
        c.should_restart(&unknown_profile),
        RestartDecision::Continue
    );
    assert_eq!(c.attempts(), 1);
}

#[test]
fn case_3_take_snapshot_captures_step_index_and_payload() {
    let traj = vec!["step 1: click".to_string(), "step 2: type".to_string()];
    let png = vec![0xDEu8, 0xAD, 0xBE, 0xEF];
    let snap = take_snapshot(&png, 2, "Find marketplace listing", &traj);
    assert_eq!(snap.last_action_idx, 2);
    assert_eq!(snap.last_screenshot, png);
    assert!(
        snap.goal_summary.starts_with("Find marketplace listing"),
        "got: {}",
        snap.goal_summary
    );
    assert!(
        snap.goal_summary.contains("step 2: type"),
        "should fold trajectory tail into goal summary; got: {}",
        snap.goal_summary
    );
}

#[test]
fn case_4_restore_from_reseeds_prompt_with_goal_summary() {
    let snap = Snapshot {
        last_screenshot: vec![1, 2, 3],
        last_action_idx: 4,
        goal_summary: "Find the first marketplace listing".into(),
    };
    let mut prompt = String::from("base prompt");
    let mut screenshot = vec![0u8; 0];
    restore_from(&snap, &mut prompt, &mut screenshot).expect("restore_from must succeed");
    assert!(prompt.contains("base prompt"), "preserves base; got {prompt}");
    assert!(
        prompt.contains("RESTART CONTEXT"),
        "stamps restart marker; got {prompt}"
    );
    assert!(
        prompt.contains("Find the first marketplace listing"),
        "carries goal summary; got {prompt}"
    );
    assert!(
        prompt.contains("step 4"),
        "carries resume index; got {prompt}"
    );
    assert_eq!(screenshot, vec![1, 2, 3]);
}

#[test]
fn case_4b_restore_from_rejects_empty_screenshot() {
    let snap = Snapshot {
        last_screenshot: vec![],
        last_action_idx: 1,
        goal_summary: "x".into(),
    };
    let mut prompt = String::new();
    let mut screenshot = vec![];
    let err = restore_from(&snap, &mut prompt, &mut screenshot)
        .expect_err("empty screenshot must be rejected");
    assert!(
        format!("{err:?}").contains("must not be empty"),
        "got: {err:?}"
    );
}

#[test]
fn case_5_snapshot_cadence_every_n_steps() {
    let c = RestartCoordinator::new(RestartStrategy::SnapshotResetRetry {
        max_attempts: 3,
        snapshot_every_n_steps: 3,
    });
    // Step 0: never (no useful state).
    assert!(!c.should_snapshot(0));
    // 1, 2: skip; 3: snapshot; 4, 5: skip; 6: snapshot.
    assert!(!c.should_snapshot(1));
    assert!(!c.should_snapshot(2));
    assert!(c.should_snapshot(3));
    assert!(!c.should_snapshot(4));
    assert!(!c.should_snapshot(5));
    assert!(c.should_snapshot(6));
    // 9, 12 also fire.
    assert!(c.should_snapshot(9));
    assert!(c.should_snapshot(12));
}

#[test]
fn case_5b_snapshot_cadence_zero_disables_snapshots() {
    // snapshot_every_n_steps = 0 must not snapshot at any step
    // (avoid div-by-zero & noise).
    let c = RestartCoordinator::new(RestartStrategy::SnapshotResetRetry {
        max_attempts: 3,
        snapshot_every_n_steps: 0,
    });
    for step in 0..10u32 {
        assert!(!c.should_snapshot(step), "step {step} must not snapshot");
    }
}

#[test]
fn case_6_retry_with_context_does_not_snapshot() {
    let c = RestartCoordinator::new(RestartStrategy::RetryWithContext {
        max_attempts: 3,
        backoff_ms: 50,
    });
    // RetryWithContext mode never snapshots — the trajectory log is
    // preserved by the caller's own `prior_actions` vector.
    for step in 0..10u32 {
        assert!(
            !c.should_snapshot(step),
            "RetryWithContext never snapshots; got snapshot at step {step}"
        );
    }
    // last_snapshot is None: there's nothing to restore from, but
    // that's fine — RetryWithContext re-prompts the same session.
    assert!(c.last_snapshot().is_none());
    // Backoff is exposed so the executor can sleep between attempts.
    assert_eq!(c.backoff_ms(), 50);
}

#[test]
fn case_7_none_strategy_is_no_op() {
    let mut c = RestartCoordinator::new(RestartStrategy::None);
    // Every error → Continue (existing flow), zero attempts charged.
    assert_eq!(c.should_restart(&http_err()), RestartDecision::Continue);
    assert_eq!(c.should_restart(&fatal_err()), RestartDecision::Continue);
    assert_eq!(c.attempts(), 0);
    // No backoff, no snapshot cadence.
    assert_eq!(c.backoff_ms(), 0);
    for step in 0..10u32 {
        assert!(!c.should_snapshot(step));
    }
    assert!(c.last_snapshot().is_none());
}

#[test]
fn record_snapshot_stores_latest() {
    let mut c = RestartCoordinator::new(RestartStrategy::SnapshotResetRetry {
        max_attempts: 1,
        snapshot_every_n_steps: 1,
    });
    let snap1 = Snapshot {
        last_screenshot: vec![1],
        last_action_idx: 1,
        goal_summary: "g1".into(),
    };
    let snap2 = Snapshot {
        last_screenshot: vec![2],
        last_action_idx: 2,
        goal_summary: "g2".into(),
    };
    c.record_snapshot(snap1);
    c.record_snapshot(snap2.clone());
    assert_eq!(c.last_snapshot(), Some(&snap2));
}

#[test]
fn restart_strategy_serde_roundtrip_via_toml() {
    // The strategy embeds in the profile TOML — round-trip a
    // SnapshotResetRetry block + a None.
    let toml_src = r#"
kind = "snapshot_reset_retry"
max_attempts = 4
snapshot_every_n_steps = 5
"#;
    let parsed: RestartStrategy = toml::from_str(toml_src).expect("parse");
    assert_eq!(
        parsed,
        RestartStrategy::SnapshotResetRetry {
            max_attempts: 4,
            snapshot_every_n_steps: 5
        }
    );
    let none_src = r#"kind = "none""#;
    let parsed_none: RestartStrategy = toml::from_str(none_src).expect("parse none");
    assert_eq!(parsed_none, RestartStrategy::None);
}
