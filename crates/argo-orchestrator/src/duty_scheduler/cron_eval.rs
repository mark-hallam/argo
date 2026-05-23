//! Cron-window evaluation for a [`super::DutyRecord`] plus the
//! micros-truncation helper used by [`super::DutyRecord::new`].
//!
//! Extracted from `duty_scheduler.rs` to keep that file under the
//! 500-LOC ceiling once PR #2088 landed.

use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, TimeZone, Utc};
use cron::Schedule;
use std::str::FromStr;

use super::DutyRecord;

impl DutyRecord {
    /// `true` iff the cron window has elapsed since the last fire
    /// origin (`last_fired_at` if set, else `created_at`).
    pub fn should_fire(&self, now: DateTime<Utc>) -> ArgoResult<bool> {
        if !self.enabled {
            return Ok(false);
        }
        if matches!(self.paused_until, Some(until) if until > now) {
            return Ok(false);
        }
        let schedule = Schedule::from_str(&self.cron)
            .map_err(|e| ArgoError::Schema(format!("duty.cron '{}': {e}", self.cron)))?;
        let origin = self.last_fired_at.unwrap_or(self.created_at);
        match schedule.after(&origin).next() {
            Some(next) => Ok(next <= now),
            None => Ok(false),
        }
    }
}

/// Truncate a `DateTime<Utc>` to whole microseconds so the SQLite
/// round-trip (which stores `timestamp_micros`) is exact.
pub(super) fn trunc_to_micros(d: DateTime<Utc>) -> DateTime<Utc> {
    Utc.timestamp_micros(d.timestamp_micros())
        .single()
        .unwrap_or(d)
}
