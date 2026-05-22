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
//!
//! ## Workspace-level fallback-chain filter (mission-quiet-hours-channels)
//!
//! [`QuietHoursResolver`] sits one layer above the per-channel
//! [`QuietHours`] silo: it reads the workspace-level
//! `[operator.quiet_hours]` block from `argo.toml` and filters the
//! `FallbackChain` BEFORE any channel `send` runs. Inside the window,
//! channel kinds in `noisy_channels` are removed from the chain so
//! dispatch falls through to the next non-noisy channel
//! (e.g. SMS → WebInbox after 22:00). When filtering empties the chain,
//! the resolver appends `"webinbox"` as the always-on default —
//! operator-decisional, documented in
//! `docs/skycot/operator/quiet-hours-config.md`.
//!
//! Distinct from
//! [`crate::operator_preferences::OperatorQuietHours`] (per-operator
//! window scoped to one `[operator.<id>.preferences]` block) and the
//! per-channel [`QuietHours`] above (each channel's own silo). The
//! workspace block is the simplest operator surface — one window that
//! quiets the noisy channels for everyone on the deployment.

use crate::fallback::FallbackChain;
use chrono::{DateTime, Datelike, Duration, NaiveTime, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use std::path::Path;

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

/// Default `noisy_channels` list used when the operator opts in but
/// does not override the set. Mirrors the brief: SMS, voice, WhatsApp,
/// iMessage, and push are the interrupt channels. Slack is excluded
/// by default because Slack DMs honour the operating system / Slack
/// app DND status separately.
const DEFAULT_NOISY_CHANNELS: &[&str] = &["sms", "voice", "whatsapp", "imessage", "push"];

/// Channel kind appended when the quiet-hours filter empties the
/// chain — operator-decisional, defaulted to the always-on WebInbox.
const FALLTHROUGH_CHANNEL: &str = "webinbox";

/// Workspace-level quiet-hours block parsed off `[operator.quiet_hours]`
/// in `argo.toml`. Distinct shape from the per-channel [`QuietHours`]
/// (fixed-offset, `start`/`end` strings, days filter) and from
/// [`crate::operator_preferences::OperatorQuietHours`] (per-operator,
/// IANA TZ + `honor_for` set keyed on `ChannelId`). The
/// workspace surface is intentionally the smallest possible knob — a
/// single deployment-wide window that quiets a curated list of noisy
/// channel kinds.
///
/// ```
/// use argo_channels::OperatorQuietHoursConfig;
///
/// let cfg: OperatorQuietHoursConfig = toml::from_str(r#"
///     enabled = true
///     start = "22:00"
///     end = "07:00"
///     timezone = "Australia/Melbourne"
///     noisy_channels = ["sms", "voice"]
/// "#).unwrap();
/// assert!(cfg.enabled);
/// assert_eq!(cfg.timezone, "Australia/Melbourne");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct OperatorQuietHoursConfig {
    /// Master enable. When `false` (the default), the workspace
    /// quiet-hours filter is a no-op and the fallback chain reaches
    /// dispatch unchanged.
    pub enabled: bool,
    /// Window start in the operator's local time, `HH:MM`.
    pub start: String,
    /// Window end in the operator's local time, `HH:MM`. When
    /// `end < start` the window wraps midnight (e.g. 22:00 → 07:00).
    pub end: String,
    /// IANA timezone name (e.g. `"Australia/Melbourne"`). DST is
    /// honoured automatically via [`chrono_tz`].
    pub timezone: String,
    /// Channel kinds skipped while the window matches. Channels not
    /// in this set ring through unconditionally.
    pub noisy_channels: Vec<String>,
}

impl Default for OperatorQuietHoursConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            start: "22:00".into(),
            end: "07:00".into(),
            timezone: "UTC".into(),
            noisy_channels: DEFAULT_NOISY_CHANNELS
                .iter()
                .map(|s| (*s).to_string())
                .collect(),
        }
    }
}

/// Resolver consulted by the dispatcher to filter the fallback chain
/// against the workspace `[operator.quiet_hours]` block. Sibling to
/// [`crate::operator_preferences::OperatorPreferencesResolver`] — the
/// preferences resolver picks per-operator primary + fallback steps;
/// this resolver masks noisy channel kinds workspace-wide.
///
/// Construct via [`QuietHoursResolver::new`] when the
/// [`OperatorQuietHoursConfig`] is already in hand, or via
/// [`QuietHoursResolver::from_toml_path`] to read `<cwd>/argo.toml`
/// directly. A parse failure (missing file, malformed TOML, unknown
/// timezone) collapses to a disabled resolver so the dispatcher
/// behaves identically to the unconfigured baseline — quiet-hours is
/// an opt-in override, never a hard requirement.
pub struct QuietHoursResolver {
    config: OperatorQuietHoursConfig,
    timezone: Option<Tz>,
    start: Option<NaiveTime>,
    end: Option<NaiveTime>,
}

impl QuietHoursResolver {
    /// Build a resolver from a pre-parsed [`OperatorQuietHoursConfig`].
    /// Invalid `start` / `end` / `timezone` strings collapse to a
    /// disabled resolver — see the module docs.
    #[must_use]
    pub fn new(config: OperatorQuietHoursConfig) -> Self {
        let timezone = config.timezone.trim().parse::<Tz>().ok();
        let start = parse_naive_hhmm(&config.start);
        let end = parse_naive_hhmm(&config.end);
        Self {
            config,
            timezone,
            start,
            end,
        }
    }

    /// Build a resolver from `<dir>/argo.toml`. The file is parsed for
    /// the `[operator.quiet_hours]` block only; every other section is
    /// ignored. Missing file or any parse error returns a disabled
    /// resolver so the dispatcher falls through unchanged.
    #[must_use]
    pub fn from_toml_path(argo_toml: &Path) -> Self {
        Self::new(load_config_from_toml(argo_toml))
    }

    /// Borrow the parsed config (mostly useful for tests + telemetry).
    #[must_use]
    pub fn config(&self) -> &OperatorQuietHoursConfig {
        &self.config
    }

    /// `true` when `channel` is one of the noisy channels AND `now`
    /// falls inside the operator-local window. Returns `false` when
    /// the resolver is disabled, the window is unparseable, or the
    /// channel is not in `noisy_channels`.
    #[must_use]
    pub fn is_quiet_now(&self, channel: &str, now: DateTime<Utc>) -> bool {
        if !self.config.enabled {
            return false;
        }
        if !self
            .config
            .noisy_channels
            .iter()
            .any(|c| c.eq_ignore_ascii_case(channel))
        {
            return false;
        }
        let (Some(tz), Some(start), Some(end)) = (self.timezone, self.start, self.end) else {
            return false;
        };
        let local = now.with_timezone(&tz).time();
        if start <= end {
            local >= start && local < end
        } else {
            local >= start || local < end
        }
    }

    /// Apply the quiet-hours filter to `chain`. Channel kinds in the
    /// noisy list during the window are removed in-place. When the
    /// filter empties the chain entirely, `"webinbox"` is appended as
    /// the always-on fallthrough — operators rely on the WebInbox
    /// floor never going silent. When the resolver is disabled, the
    /// chain is returned unchanged.
    #[must_use]
    pub fn filter_chain(&self, mut chain: FallbackChain, now: DateTime<Utc>) -> FallbackChain {
        if !self.config.enabled {
            return chain;
        }
        chain.order.retain(|c| !self.is_quiet_now(c, now));
        if chain.order.is_empty() {
            chain.order.push(FALLTHROUGH_CHANNEL.to_string());
        }
        chain
    }
}

fn parse_naive_hhmm(s: &str) -> Option<NaiveTime> {
    NaiveTime::parse_from_str(s.trim(), "%H:%M").ok()
}

fn load_config_from_toml(path: &Path) -> OperatorQuietHoursConfig {
    let Ok(body) = std::fs::read_to_string(path) else {
        return OperatorQuietHoursConfig::default();
    };
    parse_workspace_block(&body).unwrap_or_default()
}

fn parse_workspace_block(body: &str) -> Option<OperatorQuietHoursConfig> {
    // Bare struct intentionally ignores adjacent `[operator.<id>]`
    // entries (e.g. `[operator.alice.preferences]`). Serde without
    // `deny_unknown_fields` drops unknown sibling keys silently —
    // exactly what we want so this resolver coexists with the
    // per-operator preferences block parsed by `TomlPreferencesSource`.
    #[derive(Default, Deserialize)]
    struct File {
        #[serde(default)]
        operator: OperatorTable,
    }
    #[derive(Default, Deserialize)]
    struct OperatorTable {
        #[serde(default)]
        quiet_hours: Option<OperatorQuietHoursConfig>,
    }
    let file: File = toml::from_str(body).ok()?;
    file.operator.quiet_hours
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
