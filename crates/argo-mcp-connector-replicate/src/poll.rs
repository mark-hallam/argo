//! Exponential-backoff polling schedule for terminal-state waits.
//!
//! Production schedule is `[1, 2, 4, 8, 15]` seconds (ceiling 15s). When
//! the schedule is exhausted before the prediction reaches a terminal
//! state, the polling helper continues at the ceiling delay until the
//! caller-supplied timeout fires.
//!
//! The schedule mirrors `argo-mcp-connector-clearbit`'s established
//! pattern. Tests inject `[0, 0, 0, 0, 0]` so polling completes instantly.

use std::time::Duration;

/// Production polling schedule (millis between polls): `[1s, 2s, 4s, 8s, 15s]`.
pub const POLL_BACKOFF_MS: &[u64] = &[1_000, 2_000, 4_000, 8_000, 15_000];

/// Return the next poll delay given how many polls have already happened.
///
/// `poll_count` is zero-indexed (`0` = "we haven't polled yet, this is the
/// first wait"). Once the schedule is exhausted the helper returns the
/// last entry as the ceiling — Replicate predictions can run for minutes
/// (e.g. SDXL with high step counts) so a ceiling lets the loop continue
/// without unbounded backoff growth.
pub(crate) fn next_delay(schedule: &[u64], poll_count: usize) -> Duration {
    let idx = poll_count.min(schedule.len().saturating_sub(1));
    let ms = schedule.get(idx).copied().unwrap_or(0);
    Duration::from_millis(ms)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schedule_progresses_then_holds_at_ceiling() {
        let schedule = POLL_BACKOFF_MS;
        assert_eq!(next_delay(schedule, 0), Duration::from_millis(1_000));
        assert_eq!(next_delay(schedule, 4), Duration::from_millis(15_000));
        // Past the schedule, hold at the ceiling.
        assert_eq!(next_delay(schedule, 100), Duration::from_millis(15_000));
    }

    #[test]
    fn empty_schedule_returns_zero() {
        assert_eq!(next_delay(&[], 0), Duration::ZERO);
        assert_eq!(next_delay(&[], 999), Duration::ZERO);
    }

    #[test]
    fn tests_can_inject_zero_schedule() {
        let zeros = vec![0u64; 5];
        for i in 0..10 {
            assert_eq!(next_delay(&zeros, i), Duration::ZERO);
        }
    }
}
