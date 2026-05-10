//! Operator quiet-hours window shared across all notification channels.
//!
//! Originally lived inside [`crate::email`]; lifted here so that Slack,
//! SMS, voice, WhatsApp, iMessage, push, and webinbox can opt into the
//! same defer-when-quiet discipline. When a channel's `send` lands
//! inside a configured quiet window it must return a `DeliveryStatus::
//! Deferred` receipt with `deferred_reason = Some("quiet_hours".into())`
//! — the literal `"quiet_hours"` string is grepped across telemetry by
//! operators and must stay byte-identical across channels.
//!
//! ```
//! use argo_channels::QuietHours;
//! use chrono::{TimeZone, Utc};
//!
//! let quiet = QuietHours {
//!     enabled: true,
//!     start: "22:00".into(),
//!     end: "07:00".into(),
//!     timezone_offset_minutes: 0,
//!     days: Vec::new(),
//! };
//! assert!(quiet.contains(Utc.with_ymd_and_hms(2026, 5, 11, 23, 30, 0).unwrap()));
//! assert!(!quiet.contains(Utc.with_ymd_and_hms(2026, 5, 11, 12, 0, 0).unwrap()));
//! ```

use chrono::{DateTime, Datelike, Duration, Utc};
use serde::{Deserialize, Serialize};

/// Quiet-hours preference for an operator.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct QuietHours {
    /// Whether quiet hours are active.
    pub enabled: bool,
    /// Start time, `HH:MM`, in the configured fixed offset.
    pub start: String,
    /// End time, `HH:MM`, in the configured fixed offset.
    pub end: String,
    /// Fixed offset from UTC in minutes.
    pub timezone_offset_minutes: i32,
    /// Days where quiet hours apply. Empty means every day.
    pub days: Vec<String>,
}

impl Default for QuietHours {
    fn default() -> Self {
        Self {
            enabled: false,
            start: "18:00".into(),
            end: "08:00".into(),
            timezone_offset_minutes: 0,
            days: Vec::new(),
        }
    }
}

impl QuietHours {
    /// Return true when `now` lands inside the configured window.
    #[must_use]
    pub fn contains(&self, now: DateTime<Utc>) -> bool {
        if !self.enabled {
            return false;
        }
        let Some(start) = parse_hhmm(&self.start) else {
            return false;
        };
        let Some(end) = parse_hhmm(&self.end) else {
            return false;
        };
        let local = now + Duration::minutes(i64::from(self.timezone_offset_minutes));
        if !self.days.is_empty() {
            let day = format!("{:?}", local.weekday()).to_lowercase();
            let short = day.chars().take(3).collect::<String>();
            if !self.days.iter().any(|d| d.eq_ignore_ascii_case(&short)) {
                return false;
            }
        }
        let mins = local.hour_minute();
        if start <= end {
            mins >= start && mins < end
        } else {
            mins >= start || mins < end
        }
    }
}

trait HourMinute {
    fn hour_minute(&self) -> u32;
}

impl HourMinute for DateTime<Utc> {
    fn hour_minute(&self) -> u32 {
        use chrono::Timelike;
        self.hour() * 60 + self.minute()
    }
}

fn parse_hhmm(s: &str) -> Option<u32> {
    let (h, m) = s.split_once(':')?;
    let h = h.parse::<u32>().ok()?;
    let m = m.parse::<u32>().ok()?;
    if h < 24 && m < 60 {
        Some(h * 60 + m)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn quiet_hours_handles_overnight_window() {
        let q = QuietHours {
            enabled: true,
            start: "18:00".into(),
            end: "08:00".into(),
            timezone_offset_minutes: 0,
            days: Vec::new(),
        };
        assert!(q.contains(Utc.with_ymd_and_hms(2026, 5, 6, 22, 0, 0).unwrap()));
        assert!(q.contains(Utc.with_ymd_and_hms(2026, 5, 6, 7, 59, 0).unwrap()));
        assert!(!q.contains(Utc.with_ymd_and_hms(2026, 5, 6, 12, 0, 0).unwrap()));
    }

    #[test]
    fn disabled_window_never_matches() {
        let q = QuietHours::default();
        assert!(!q.contains(Utc.with_ymd_and_hms(2026, 5, 6, 22, 0, 0).unwrap()));
    }

    #[test]
    fn day_filter_skips_excluded_weekdays() {
        let q = QuietHours {
            enabled: true,
            start: "00:00".into(),
            end: "23:59".into(),
            timezone_offset_minutes: 0,
            days: vec!["mon".into()],
        };
        // 2026-05-06 is a Wednesday — outside the mon-only filter.
        assert!(!q.contains(Utc.with_ymd_and_hms(2026, 5, 6, 12, 0, 0).unwrap()));
        // 2026-05-04 is a Monday — inside the filter.
        assert!(q.contains(Utc.with_ymd_and_hms(2026, 5, 4, 12, 0, 0).unwrap()));
    }
}
