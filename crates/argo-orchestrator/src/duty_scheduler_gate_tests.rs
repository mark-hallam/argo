//! Per-duty `GateConfig` matrix coverage for `ScheduleGate`,
//! `QuietHoursGate`, and the `gate_from_config` factory
//! (mission-duty-gate-per-duty-config).
//!
//! Loaded via `#[path]` from `duty_scheduler_gate.rs` so the tests
//! sit in a sibling file without touching `lib.rs`. The other
//! gates (`OpenGate`, `ClosedGate`) and `FireDecision::label`
//! shape are covered by the in-file `mod tests` block.

use super::*;
use argo_identity::WorkerId;
use argo_types::TenantId;
use chrono::Timelike;

fn record(cron: &str) -> DutyRecord {
    DutyRecord::new(TenantId::new(), WorkerId::new(), cron, "ping", "test duty")
}

#[tokio::test]
async fn schedule_gate_allows_when_current_hour_in_window() {
    let now_hour = u8::try_from(Utc::now().hour()).unwrap();
    let g = ScheduleGate::new(vec![now_hour]);
    assert_eq!(
        g.evaluate(&record("0 * * * * *")).await.unwrap(),
        FireDecision::AutoApprove,
    );
}

#[tokio::test]
async fn schedule_gate_blocks_when_current_hour_outside_window() {
    let now_hour = u8::try_from(Utc::now().hour()).unwrap();
    // Build an allow-list that explicitly excludes the current hour.
    let allow: Vec<u8> = (0u8..24).filter(|h| *h != now_hour).collect();
    let g = ScheduleGate::new(allow);
    assert_eq!(
        g.evaluate(&record("0 * * * * *")).await.unwrap(),
        FireDecision::DeniedGate,
    );
}

#[tokio::test]
async fn quiet_hours_gate_blocks_during_deny_window() {
    let local_hour =
        u8::try_from(Utc::now().with_timezone(&chrono_tz::UTC).hour()).unwrap();
    let g = QuietHoursGate::new(vec![local_hour], chrono_tz::UTC);
    assert_eq!(
        g.evaluate(&record("0 * * * * *")).await.unwrap(),
        FireDecision::DeniedGate,
    );
}

#[tokio::test]
async fn quiet_hours_gate_allows_outside_deny_window() {
    let local_hour =
        u8::try_from(Utc::now().with_timezone(&chrono_tz::UTC).hour()).unwrap();
    let deny: Vec<u8> = (0u8..24).filter(|h| *h != local_hour).collect();
    let g = QuietHoursGate::new(deny, chrono_tz::UTC);
    assert_eq!(
        g.evaluate(&record("0 * * * * *")).await.unwrap(),
        FireDecision::AutoApprove,
    );
}

#[tokio::test]
async fn quiet_hours_gate_respects_per_zone_local_hour() {
    // Pick a tz whose local hour is guaranteed-different from UTC
    // (Pacific/Kiritimati is UTC+14, the largest civil offset).
    let utc_now = Utc::now();
    let kir_hour =
        u8::try_from(utc_now.with_timezone(&chrono_tz::Pacific::Kiritimati).hour()).unwrap();
    let utc_hour = u8::try_from(utc_now.hour()).unwrap();
    if kir_hour == utc_hour {
        // Skip the assertion in the rare case the hour-shift wraps
        // back to coincide; the intent of the test is the tz lookup
        // path, which we exercise unconditionally below.
        let g = QuietHoursGate::new(vec![kir_hour], chrono_tz::Pacific::Kiritimati);
        assert_eq!(
            g.evaluate(&record("0 * * * * *")).await.unwrap(),
            FireDecision::DeniedGate,
        );
        return;
    }
    // Deny-list contains Kiritimati's hour but not UTC's hour.
    let g = QuietHoursGate::new(vec![kir_hour], chrono_tz::Pacific::Kiritimati);
    assert_eq!(
        g.evaluate(&record("0 * * * * *")).await.unwrap(),
        FireDecision::DeniedGate,
    );
    // Same deny-list bound to UTC would NOT block at the same instant.
    let g_utc = QuietHoursGate::new(vec![kir_hour], chrono_tz::UTC);
    assert_eq!(
        g_utc.evaluate(&record("0 * * * * *")).await.unwrap(),
        FireDecision::AutoApprove,
    );
}

#[tokio::test]
async fn gate_from_config_dispatches_each_variant() {
    // Open → OpenGate auto-approves.
    let open = gate_from_config(&GateConfig::Open).unwrap();
    assert_eq!(
        open.evaluate(&record("0 * * * * *")).await.unwrap(),
        FireDecision::AutoApprove,
    );
    // Closed → ClosedGate denies.
    let closed = gate_from_config(&GateConfig::Closed).unwrap();
    assert_eq!(
        closed.evaluate(&record("0 * * * * *")).await.unwrap(),
        FireDecision::DeniedGate,
    );
    // Schedule → ScheduleGate path. Allow-list covering all 24
    // hours unconditionally approves.
    let sched = gate_from_config(&GateConfig::Schedule {
        allow_hours: (0u8..24).collect(),
    })
    .unwrap();
    assert_eq!(
        sched.evaluate(&record("0 * * * * *")).await.unwrap(),
        FireDecision::AutoApprove,
    );
    // QuietHours → QuietHoursGate path. Empty deny-list never blocks.
    let quiet = gate_from_config(&GateConfig::QuietHours {
        deny_hours: vec![],
        tz: "UTC".into(),
    })
    .unwrap();
    assert_eq!(
        quiet.evaluate(&record("0 * * * * *")).await.unwrap(),
        FireDecision::AutoApprove,
    );
    // Bad tz surfaces at config-load (factory call), not at tick.
    let bad = gate_from_config(&GateConfig::QuietHours {
        deny_hours: vec![],
        tz: "Not/A_Real_Zone".into(),
    });
    assert!(matches!(bad, Err(ArgoError::Schema(_))));
}
