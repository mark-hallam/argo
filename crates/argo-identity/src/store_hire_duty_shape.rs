//! Dual-shape deserialiser for [`super::HireDutyDraft`].
//!
//! Extracted from `store_hire.rs` to keep that file under the 500-LOC
//! ceiling; declared via `#[path] mod duty_shape;`. Holds the custom
//! `Deserialize` impl that accepts BOTH the canonical hire-duty wire shape
//! and the SPA hire flow's `WorkerDuty` shape, plus the `entry_action`
//! slugifier they share.

use super::HireDutyDraft;
use serde::Deserialize;

/// Wire-shape adapter for [`HireDutyDraft`] deserialisation.
///
/// Two clients send a hire duty under two DIFFERENT shapes:
///
/// 1. **Canonical** (Genesis SSE / `/onboarding/first-hire` / curl):
///    `{cron, entryAction, description, defaultEnabled}`.
/// 2. **SPA hire flow** (`web/lib/hire/draft-types.ts` `WorkerDuty`):
///    `{label, schedule:{kind,spec}, defaultEnabled}` — no `cron`, no
///    `entryAction`.
///
/// Before this adapter the SPA shape failed `serde_json::from_value` (its
/// required `cron`/`entryAction` were absent), so the gateway rejected the
/// whole `worker_draft` and the teammate's schedule dropped on the floor —
/// ZERO `role_duty` rows fired, blank goals, no error. This intermediate
/// accepts BOTH and normalises onto the canonical [`HireDutyDraft`]:
/// `schedule.spec` becomes the `cron` (cron-kind passes through verbatim for
/// the bridge's `normalize_stored_cron` to validate; event-kind keeps its
/// expression, which the bridge will skip+warn rather than poison the
/// scheduler), `label` becomes the `description`, and `entry_action`
/// (when absent) is slugified from the label.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct HireDutyWire {
    cron: Option<String>,
    entry_action: Option<String>,
    description: Option<String>,
    label: Option<String>,
    schedule: Option<DutyScheduleWire>,
    #[serde(default)]
    default_enabled: bool,
}

/// SPA `WorkerDuty.schedule` — `{kind: "cron"|"event", spec}`.
#[derive(Deserialize)]
struct DutyScheduleWire {
    spec: String,
}

impl<'de> Deserialize<'de> for HireDutyDraft {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = HireDutyWire::deserialize(deserializer)?;
        // Cron precedence: an explicit `cron` (canonical shape) always wins;
        // otherwise the SPA `schedule.spec` carries it. Missing both is an
        // error — a duty with no schedule can never fire.
        let cron = wire
            .cron
            .filter(|c| !c.trim().is_empty())
            .or_else(|| wire.schedule.map(|s| s.spec))
            .filter(|c| !c.trim().is_empty())
            .ok_or_else(|| {
                serde::de::Error::custom(
                    "hire duty needs `cron` or `schedule.spec` (got neither)",
                )
            })?;
        // Description precedence: explicit `description`, else the SPA `label`,
        // else a generic fallback so the row is never blank.
        let description = wire
            .description
            .filter(|d| !d.trim().is_empty())
            .or_else(|| wire.label.clone().filter(|l| !l.trim().is_empty()))
            .unwrap_or_else(|| "Scheduled duty.".to_string());
        // Entry-action precedence: explicit `entryAction`, else slugify the
        // label/description into a snake_case action name (matching the
        // Genesis convention), else a generic `scheduled_duty`.
        let entry_action = wire
            .entry_action
            .filter(|a| !a.trim().is_empty())
            .unwrap_or_else(|| slugify_entry_action(&description));
        Ok(HireDutyDraft {
            cron,
            entry_action,
            description,
            default_enabled: wire.default_enabled,
        })
    }
}

/// Slugify a duty label into a snake_case `entry_action` name (lowercase
/// alphanumerics, runs of other chars collapse to a single `_`). Empty or
/// all-punctuation input yields the stable fallback `scheduled_duty`, so the
/// returned action name is always non-empty.
pub(super) fn slugify_entry_action(label: &str) -> String {
    let mut slug = String::new();
    let mut prev_us = false;
    for ch in label.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            slug.extend(ch.to_lowercase());
            prev_us = false;
        } else if !prev_us && !slug.is_empty() {
            slug.push('_');
            prev_us = true;
        }
    }
    let trimmed = slug.trim_matches('_');
    if trimmed.is_empty() {
        "scheduled_duty".to_string()
    } else {
        trimmed.to_string()
    }
}
