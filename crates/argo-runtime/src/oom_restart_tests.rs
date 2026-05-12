//! Unit tests for [`crate::oom_restart`]. Included via `#[path]`
//! from `oom_restart.rs` so the symbols stay private to the
//! parent module while the test file keeps the parent under the
//! 500-LOC ceiling.
//!
//! The brief calls for 8 tests; each one pins one acceptance-
//! criteria row from `streams/mission-oom-restart-handler.md`.

use super::*;
use chrono::TimeZone;
use std::time::Duration;

fn t(offset_secs: i64) -> chrono::DateTime<chrono::Utc> {
    chrono::Utc
        .with_ymd_and_hms(2026, 5, 12, 0, 0, 0)
        .unwrap()
        + chrono::Duration::seconds(offset_secs)
}

#[cfg(unix)]
fn status_from_signal(signal: i32) -> std::process::ExitStatus {
    use std::os::unix::process::ExitStatusExt;
    std::process::ExitStatus::from_raw(signal)
}

#[cfg(unix)]
fn status_from_code(code: i32) -> std::process::ExitStatus {
    use std::os::unix::process::ExitStatusExt;
    // Encode in the upper byte the way `waitpid` does (status &
    // 0xff00 >> 8 == exit code).
    std::process::ExitStatus::from_raw(code << 8)
}

#[cfg(unix)]
#[test]
fn classify_exit_137_as_oom_killed() {
    // Shell-wrapped 137 routes through `code() == Some(137)`.
    let status = status_from_code(EXIT_CODE_OOM_KILL);
    let reason = classify_exit(status, None);
    match reason {
        WorkerExitReason::OomKilled { signal, .. } => assert_eq!(signal, SIGNAL_OOM_KILL),
        other => panic!("expected OomKilled, got {other:?}"),
    }
}

#[cfg(unix)]
#[test]
fn classify_clean_exit_as_clean() {
    let status = status_from_code(0);
    assert_eq!(classify_exit(status, None), WorkerExitReason::Clean { code: 0 });
}

#[cfg(unix)]
#[test]
fn classify_sigsegv_as_other_signal() {
    // SIGSEGV = signal 11.
    let status = status_from_signal(11);
    assert_eq!(
        classify_exit(status, None),
        WorkerExitReason::OtherSignal { signal: 11 }
    );
}

#[test]
fn classify_stale_heartbeat_as_hung() {
    let now = t(600); // 10 min into the day.
    let stale = t(0); // 10 min ago.
    let result = classify_hung(Some(stale), Duration::from_secs(60), now);
    match result {
        Some(WorkerExitReason::Hung { last_heartbeat }) => {
            assert_eq!(last_heartbeat, stale);
        }
        other => panic!("expected Some(Hung), got {other:?}"),
    }
    // Fresh heartbeat → None.
    let fresh = t(590);
    assert!(classify_hung(Some(fresh), Duration::from_secs(60), now).is_none());
}

#[test]
fn restart_coordinator_allows_up_to_max_restarts() {
    let mut coord = OomRestartCoordinator::with_defaults();
    let exit = WorkerExitReason::OomKilled {
        signal: 9,
        cgroup_evidence: None,
    };
    for i in 1..=DEFAULT_MAX_RESTARTS {
        let decision = decide_restart(&mut coord, &exit, t(i64::from(i)));
        match decision {
            RestartDecision::Restart { attempt, .. } => assert_eq!(attempt, i),
            other => panic!("attempt {i}: expected Restart, got {other:?}"),
        }
    }
}

#[test]
fn restart_coordinator_blocks_after_budget_exhausted() {
    let mut coord = OomRestartCoordinator::with_defaults();
    let exit = WorkerExitReason::OomKilled {
        signal: 9,
        cgroup_evidence: None,
    };
    for i in 1..=DEFAULT_MAX_RESTARTS {
        let _ = decide_restart(&mut coord, &exit, t(i64::from(i)));
    }
    let decision = decide_restart(&mut coord, &exit, t(100));
    match decision {
        RestartDecision::GiveUp { salvage_note } => {
            assert!(salvage_note.contains("restarts in window exceeded"));
            assert!(salvage_note.contains("OOM (signal 9)"));
            assert!(salvage_note.contains("oom-restart.md"));
        }
        other => panic!("expected GiveUp, got {other:?}"),
    }
}

#[test]
fn restart_window_resets_after_full_window() {
    let mut coord = OomRestartCoordinator::with_defaults();
    let exit = WorkerExitReason::OomKilled {
        signal: 9,
        cgroup_evidence: None,
    };
    for i in 1..=DEFAULT_MAX_RESTARTS {
        let _ = decide_restart(&mut coord, &exit, t(i64::from(i)));
    }
    // Step well past the window — old entries should age out
    // and the coordinator should accept a fresh restart.
    let later = t(2 * (DEFAULT_RESTART_WINDOW.as_secs() as i64));
    let decision = decide_restart(&mut coord, &exit, later);
    match decision {
        RestartDecision::Restart { attempt, .. } => assert_eq!(attempt, 1),
        other => panic!("expected Restart after window roll, got {other:?}"),
    }
}

#[test]
fn backoff_doubles_and_clamps_at_max() {
    let config = OomRestartConfig {
        max_restarts: 10,
        base_backoff: Duration::from_secs(1),
        max_backoff: Duration::from_secs(8),
        restart_window: Duration::from_secs(3600),
    };
    let mut coord = OomRestartCoordinator::new(config);
    let exit = WorkerExitReason::OomKilled {
        signal: 9,
        cgroup_evidence: None,
    };
    let mut backoffs = Vec::new();
    for i in 1..=6 {
        let decision = decide_restart(&mut coord, &exit, t(i));
        if let RestartDecision::Restart { after_backoff, .. } = decision {
            backoffs.push(after_backoff.as_secs());
        }
    }
    // 1s, 2s, 4s, then clamped at 8s for the remaining attempts.
    assert_eq!(backoffs, vec![1, 2, 4, 8, 8, 8]);
}

#[test]
fn audit_row_shape_matches_spec() {
    // Pure-shape check on the payload-building branch. Asserts
    // every key documented in the brief is present + serialises.
    let exit = WorkerExitReason::OomKilled {
        signal: 9,
        cgroup_evidence: Some("oom_kill 3".to_string()),
    };
    let decision = RestartDecision::Restart {
        after_backoff: Duration::from_secs(4),
        attempt: 2,
    };
    let payload = serde_json::json!({
        "worker_id": "wkr-test",
        "exit_reason": exit,
        "decision": decision,
        "attempt_count": 2,
        "recorded_at": chrono::Utc::now().to_rfc3339(),
    });
    assert_eq!(payload["worker_id"], "wkr-test");
    assert_eq!(payload["exit_reason"]["kind"], "oom_killed");
    assert_eq!(payload["exit_reason"]["signal"], 9);
    assert_eq!(payload["exit_reason"]["cgroup_evidence"], "oom_kill 3");
    assert_eq!(payload["decision"]["kind"], "restart");
    assert_eq!(payload["decision"]["attempt"], 2);
    assert_eq!(payload["attempt_count"], 2);
    assert!(payload["recorded_at"].is_string());
}

#[test]
fn clean_exit_yields_ignore_decision() {
    let mut coord = OomRestartCoordinator::with_defaults();
    let exit = WorkerExitReason::Clean { code: 0 };
    let decision = decide_restart(&mut coord, &exit, t(0));
    assert_eq!(decision, RestartDecision::IgnoreCleanExit);
    // Clean exits must not consume a restart-budget slot.
    assert_eq!(coord.attempts_in_window(t(0)), 0);
}

#[cfg(not(unix))]
#[test]
fn windows_fallthrough_does_not_panic() {
    // The classifier must compile + run on non-unix even though
    // the OOM detection path is unix-only. On Windows
    // `ExitStatus::code()` is always `Some`, so any non-137 code
    // lands as Clean.
    let status = std::process::ExitStatus::default();
    let reason = classify_exit(status, None);
    assert!(matches!(reason, WorkerExitReason::Clean { .. }));
}
