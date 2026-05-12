//! Unit tests for [`crate::sandbox_prewarm`].
//!
//! All six tests run offline against the pure decision surface
//! (`record_spawn`, `should_burst_prewarm`, `should_hourly_prewarm`,
//! `try_admit`, `try_consume`). No live E2B endpoint is touched
//! and no async runtime is needed — the prewarmer is in-process
//! synchronous state.

use super::*;
use chrono::TimeZone;
use std::time::Duration;

fn enabled_config() -> SandboxPrewarmConfig {
    SandboxPrewarmConfig {
        enabled: true,
        ..SandboxPrewarmConfig::default()
    }
}

fn sandbox(id: &str) -> E2bSandbox {
    E2bSandbox { id: id.into() }
}

fn entry(template: &str, id: &str, prewarmed_at: Instant) -> PrewarmedSandbox {
    PrewarmedSandbox {
        template: template.into(),
        sandbox: sandbox(id),
        prewarmed_at,
    }
}

#[test]
fn sliding_window_evicts_events_older_than_configured_width() {
    let mut p = SandboxPrewarmer::new(SandboxPrewarmConfig {
        window_seconds: 60,
        ..enabled_config()
    });
    let t0 = Instant::now();
    // Two events well inside the window.
    p.record_spawn(t0);
    p.record_spawn(t0 + Duration::from_secs(10));
    // One event from before the window opens.
    p.record_spawn(t0 - Duration::from_secs(120));
    // The pre-window event was just pushed; record_spawn evicts
    // only entries older than the window relative to `now`. So we
    // observe it from a much-later `now` to force eviction.
    let now = t0 + Duration::from_secs(30);
    let count_pre_eviction = p.spawn_count_in_window(now);
    // The 120-s-old event is outside the 60-s window from `now`,
    // so the count should be 2 (the two recent events).
    assert_eq!(count_pre_eviction, 2);
    // Pushing a new event at `now` should evict the stale entry
    // from the deque so internal state stays bounded.
    p.record_spawn(now);
    assert_eq!(p.spawn_count_in_window(now), 3);
    // Now advance past the window so all original entries fall
    // out and confirm the count drops back to 1 (the fresh push).
    let later = now + Duration::from_secs(90);
    p.record_spawn(later);
    let count_after = p.spawn_count_in_window(later);
    assert_eq!(count_after, 1, "stale events should be evicted");
}

#[test]
fn burst_does_not_trigger_at_or_below_threshold() {
    let mut p = SandboxPrewarmer::new(SandboxPrewarmConfig {
        spawn_rate_threshold: 3,
        burst_size: 2,
        max_prewarm: 5,
        ..enabled_config()
    });
    let t = Instant::now();
    // Exactly threshold: 3 events. Should NOT trigger.
    p.record_spawn(t);
    p.record_spawn(t);
    p.record_spawn(t);
    assert_eq!(
        p.should_burst_prewarm(t),
        0,
        "exactly at threshold must not trigger burst pre-warm"
    );
    // Below threshold: 2 events.
    let mut p2 = SandboxPrewarmer::new(SandboxPrewarmConfig {
        spawn_rate_threshold: 3,
        burst_size: 2,
        max_prewarm: 5,
        ..enabled_config()
    });
    p2.record_spawn(t);
    p2.record_spawn(t);
    assert_eq!(p2.should_burst_prewarm(t), 0);
}

#[test]
fn burst_triggers_burst_size_strictly_above_threshold() {
    let mut p = SandboxPrewarmer::new(SandboxPrewarmConfig {
        spawn_rate_threshold: 3,
        burst_size: 2,
        max_prewarm: 5,
        ..enabled_config()
    });
    let t = Instant::now();
    for _ in 0..4 {
        p.record_spawn(t);
    }
    assert_eq!(p.should_burst_prewarm(t), 2);
    // Disabled config returns 0 regardless of count.
    let mut p_off = SandboxPrewarmer::new(SandboxPrewarmConfig {
        enabled: false,
        spawn_rate_threshold: 3,
        burst_size: 2,
        ..SandboxPrewarmConfig::default()
    });
    for _ in 0..10 {
        p_off.record_spawn(t);
    }
    assert_eq!(
        p_off.should_burst_prewarm(t),
        0,
        "disabled prewarmer never triggers a burst"
    );
}

#[test]
fn try_admit_caps_reservoir_at_max_prewarm() {
    let mut p = SandboxPrewarmer::new(SandboxPrewarmConfig {
        max_prewarm: 3,
        ..enabled_config()
    });
    let t = Instant::now();
    assert!(p.try_admit(entry("browser", "sb-1", t)));
    assert!(p.try_admit(entry("browser", "sb-2", t)));
    assert!(p.try_admit(entry("browser", "sb-3", t)));
    assert_eq!(p.occupancy(), 3);
    // Fourth admit must be rejected.
    assert!(
        !p.try_admit(entry("browser", "sb-4", t)),
        "at cap, try_admit returns false so caller can close the sandbox"
    );
    assert_eq!(p.occupancy(), 3, "rejected admit must not grow reservoir");
    // Burst headroom is zero at cap, even with plenty of spawn
    // demand — caller would just close the extras.
    for _ in 0..10 {
        p.record_spawn(t);
    }
    assert_eq!(p.should_burst_prewarm(t), 0);
}

#[test]
fn try_consume_returns_fifo_oldest_matching_template() {
    let mut p = SandboxPrewarmer::new(enabled_config());
    let t = Instant::now();
    // Admit in this order: A1, B1, A2. FIFO take of "A" should
    // return A1 first, then A2.
    assert!(p.try_admit(entry("template-A", "A1", t)));
    assert!(p.try_admit(entry("template-B", "B1", t)));
    assert!(p.try_admit(entry("template-A", "A2", t)));
    let first = p
        .try_consume("template-A")
        .expect("first A consume should succeed");
    assert_eq!(first.id, "A1", "FIFO must yield oldest first");
    let second = p
        .try_consume("template-A")
        .expect("second A consume should succeed");
    assert_eq!(second.id, "A2");
    // Now reservoir holds only B1; consuming A again misses.
    assert!(p.try_consume("template-A").is_none());
    // Mismatched-template consume against a non-empty reservoir
    // also misses.
    assert!(p.try_consume("template-Z").is_none());
    let last = p
        .try_consume("template-B")
        .expect("B consume should succeed");
    assert_eq!(last.id, "B1");
    assert_eq!(p.occupancy(), 0);
}

#[test]
fn hourly_trigger_fires_once_per_active_hour_then_quiets() {
    let mut p = SandboxPrewarmer::new(SandboxPrewarmConfig {
        active_hours: vec![9, 10, 11],
        ..enabled_config()
    });
    let in_hour_9 = Utc.with_ymd_and_hms(2026, 5, 12, 9, 5, 0).unwrap();
    let later_in_hour_9 = Utc.with_ymd_and_hms(2026, 5, 12, 9, 45, 0).unwrap();
    let in_hour_10 = Utc.with_ymd_and_hms(2026, 5, 12, 10, 0, 0).unwrap();
    let in_quiet_hour_3 = Utc.with_ymd_and_hms(2026, 5, 12, 3, 0, 0).unwrap();

    // First call inside an active hour fires.
    assert!(p.should_hourly_prewarm(in_hour_9));
    // Second call in the same hour does NOT re-fire.
    assert!(!p.should_hourly_prewarm(later_in_hour_9));
    // Crossing into the next active hour re-fires.
    assert!(p.should_hourly_prewarm(in_hour_10));
    // A non-active hour does not fire even on a fresh prewarmer.
    let mut p_quiet = SandboxPrewarmer::new(SandboxPrewarmConfig {
        active_hours: vec![9, 10, 11],
        ..enabled_config()
    });
    assert!(!p_quiet.should_hourly_prewarm(in_quiet_hour_3));
    // Empty active_hours means "every hour is active" — the
    // operator-active-default for tenants with no quiet-hours
    // configured.
    let mut p_always = SandboxPrewarmer::new(SandboxPrewarmConfig {
        active_hours: Vec::new(),
        ..enabled_config()
    });
    assert!(p_always.should_hourly_prewarm(in_quiet_hour_3));
    // Hourly trigger respects max_prewarm cap — once full, no
    // more fires until reservoir drains.
    let mut p_full = SandboxPrewarmer::new(SandboxPrewarmConfig {
        max_prewarm: 2,
        active_hours: Vec::new(),
        ..enabled_config()
    });
    let t = Instant::now();
    assert!(p_full.try_admit(entry("browser", "x1", t)));
    assert!(p_full.try_admit(entry("browser", "x2", t)));
    assert!(!p_full.should_hourly_prewarm(in_hour_9));
}

#[test]
fn drain_idle_returns_entries_past_ttl_and_keeps_recent() {
    let mut p = SandboxPrewarmer::new(SandboxPrewarmConfig {
        idle_ttl_seconds: 60,
        max_prewarm: 5,
        ..enabled_config()
    });
    let now = Instant::now();
    let old = now - Duration::from_secs(120);
    let recent = now - Duration::from_secs(10);
    assert!(p.try_admit(entry("browser", "old-1", old)));
    assert!(p.try_admit(entry("browser", "recent-1", recent)));
    assert!(p.try_admit(entry("browser", "old-2", old)));
    let drained = p.drain_idle(now);
    let drained_ids: Vec<&str> = drained.iter().map(|e| e.sandbox.id.as_str()).collect();
    assert_eq!(drained.len(), 2);
    assert!(drained_ids.contains(&"old-1"));
    assert!(drained_ids.contains(&"old-2"));
    assert_eq!(p.occupancy(), 1, "recent entry should survive reaper");
    assert_eq!(
        p.try_consume("browser").map(|s| s.id),
        Some("recent-1".to_owned()),
        "remaining entry must be the not-yet-stale one"
    );
}
