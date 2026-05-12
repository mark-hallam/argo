//! Tomorrow-line computation from `role_duty`.
//!
//! Reads the worker's enabled, non-paused duties and returns the
//! cron-evaluated next-fire that falls inside the next 24 hours.
//! Brief example: `"morning routine at 06:30"`.
//!
//! Sensible deviation: the brief's pseudo-SQL references a
//! `next_fire_at` column on a `worker_duty` table. The actual
//! shipped schema is `role_duty` with a `cron` expression (see
//! `crates/argo-store/migrations/28__role_duties.sql`), so the
//! computation runs cron evaluation in-process per row. Timezone
//! is fixed to UTC because v1 workspace rows do not yet carry a
//! `timezone` column; the i18n / per-workspace-tz roll-up is the
//! sibling hardening brief.

use std::str::FromStr;
use std::sync::Arc;

use argo_identity::WorkerId;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, Duration, Timelike, Utc};
use cron::Schedule;
use sqlx::Row;

/// Tomorrow-line lookahead window.
const TOMORROW_WINDOW: Duration = Duration::hours(24);

/// Return the next-due duty for `worker_id` within the next 24
/// hours, formatted as `"<name> at HH:MM"`. Returns `None` when
/// no duty fires inside the window.
pub async fn next_duty_for_worker(
    store: &Arc<SessionStore>,
    worker_id: WorkerId,
    now: DateTime<Utc>,
) -> ArgoResult<Option<String>> {
    let now_micros = now.timestamp_micros();
    let rows = match sqlx::query(
        "SELECT cron, entry_action, description \
         FROM role_duty \
         WHERE worker_id = ? \
           AND enabled = 1 \
           AND (paused_until IS NULL OR paused_until <= ?)",
    )
    .bind(worker_id.to_string())
    .bind(now_micros)
    .fetch_all(store.pool())
    .await
    {
        Ok(rows) => rows,
        // Table may not exist yet in stores that haven't called
        // `DutyStore::ensure_schema()` — treat that as "no
        // Tomorrow line" rather than a hard error.
        Err(sqlx::Error::Database(db)) if db.message().contains("no such table") => return Ok(None),
        Err(e) => return Err(ArgoError::Provider(format!("team-headline duties: {e}"))),
    };

    let mut earliest: Option<(DateTime<Utc>, String, String)> = None;
    for row in rows {
        let cron_expr: String = row.try_get("cron").unwrap_or_default();
        let entry_action: String = row.try_get("entry_action").unwrap_or_default();
        let description: String = row.try_get("description").unwrap_or_default();
        let Some(next) = next_fire_after(&cron_expr, now) else {
            continue;
        };
        if next > now + TOMORROW_WINDOW {
            continue;
        }
        match &earliest {
            Some((cur, _, _)) if *cur <= next => {}
            _ => earliest = Some((next, description, entry_action)),
        }
    }

    Ok(earliest.map(|(next, description, entry_action)| {
        let name = if description.trim().is_empty() {
            entry_action
        } else {
            description
        };
        format_tomorrow(&name, next)
    }))
}

/// Parse a cron expression and return the next fire strictly
/// after `now`. Returns `None` for unparseable expressions
/// (logged at debug) so a single bad row never blocks the rest
/// of the table.
fn next_fire_after(cron_expr: &str, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
    let schedule = match Schedule::from_str(cron_expr) {
        Ok(s) => s,
        Err(err) => {
            tracing::debug!(
                target: "argo_team_headline",
                cron = %cron_expr,
                error = %err,
                "skipping duty with unparseable cron"
            );
            return None;
        }
    };
    schedule.after(&now).next()
}

/// Format the Tomorrow line.
fn format_tomorrow(name: &str, next: DateTime<Utc>) -> String {
    let trimmed = name.trim();
    let label = if trimmed.is_empty() { "duty" } else { trimmed };
    format!("{label} at {:02}:{:02}", next.hour(), next.minute())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn next_fire_skips_unparseable_cron() {
        assert!(next_fire_after("not a cron", Utc::now()).is_none());
    }

    #[test]
    fn format_tomorrow_pads_time() {
        let when = Utc.with_ymd_and_hms(2026, 5, 12, 6, 30, 0).unwrap();
        assert_eq!(
            format_tomorrow("morning routine", when),
            "morning routine at 06:30"
        );
    }

    #[test]
    fn format_tomorrow_falls_back_when_name_empty() {
        let when = Utc.with_ymd_and_hms(2026, 5, 12, 9, 5, 0).unwrap();
        assert_eq!(format_tomorrow("", when), "duty at 09:05");
    }
}
