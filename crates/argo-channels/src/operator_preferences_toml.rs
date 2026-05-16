//! TOML-backed [`OperatorPreferencesSource`] implementation.
//!
//! Split out of `operator_preferences.rs` to keep that file under
//! the 500-LOC ceiling. Re-exported as
//! `operator_preferences::TomlPreferencesSource`; external callers
//! must not import from this module directly.

use super::{
    ChannelChainStep, OperatorId, OperatorPreferences, OperatorPreferencesSource,
    OperatorQuietHours,
};
use crate::{ChannelError, ChannelId, ChannelResult, RiskLevel};
use async_trait::async_trait;
use chrono::NaiveTime;
use chrono_tz::Tz;
use parking_lot::Mutex;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;

/// TOML-backed preferences source. Reads
/// `[operator.<id>.preferences]` blocks from `argo.toml`. Reload
/// is mtime-poll-cached: each `for_operator` call stats the file
/// and re-parses only when the mtime advances.
pub struct TomlPreferencesSource {
    path: PathBuf,
    cache: Arc<Mutex<TomlCache>>,
}

#[derive(Default)]
struct TomlCache {
    mtime: Option<SystemTime>,
    operators: HashMap<OperatorId, OperatorPreferences>,
}

impl TomlPreferencesSource {
    /// Build a source reading from `path`. The file is not opened
    /// until the first [`OperatorPreferencesSource::for_operator`]
    /// call.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            cache: Arc::new(Mutex::new(TomlCache::default())),
        }
    }

    fn reload_if_stale(&self) -> ChannelResult<()> {
        let mtime = std::fs::metadata(&self.path)
            .and_then(|m| m.modified())
            .map_err(|e| {
                ChannelError::Config(format!(
                    "preferences file {} unreadable: {e}",
                    self.path.display()
                ))
            })?;
        {
            let guard = self.cache.lock();
            if guard.mtime == Some(mtime) {
                return Ok(());
            }
        }
        let body = std::fs::read_to_string(&self.path).map_err(|e| {
            ChannelError::Config(format!(
                "preferences file {} read failed: {e}",
                self.path.display()
            ))
        })?;
        let parsed: TomlFileShape = toml::from_str(&body)
            .map_err(|e| ChannelError::Parse(format!("preferences TOML: {e}")))?;
        let operators = parsed.into_operators()?;
        let mut guard = self.cache.lock();
        guard.mtime = Some(mtime);
        guard.operators = operators;
        Ok(())
    }
}

#[async_trait]
impl OperatorPreferencesSource for TomlPreferencesSource {
    async fn for_operator(&self, op: &OperatorId) -> ChannelResult<OperatorPreferences> {
        self.reload_if_stale()?;
        let guard = self.cache.lock();
        guard
            .operators
            .get(op)
            .cloned()
            .ok_or_else(|| ChannelError::Config(format!("no preferences for operator {op}")))
    }
}

/// TOML shape: `[operator.<id>.preferences]` blocks under a top
/// level `operator` table.
#[derive(Debug, Deserialize)]
struct TomlFileShape {
    #[serde(default)]
    operator: HashMap<String, TomlOperatorEntry>,
}

#[derive(Debug, Deserialize)]
struct TomlOperatorEntry {
    preferences: Option<TomlPreferences>,
}

#[derive(Debug, Deserialize)]
struct TomlPreferences {
    primary_channel: String,
    #[serde(default)]
    fallback_chain: Vec<TomlChainStep>,
    quiet_hours: Option<TomlQuietHours>,
    #[serde(default)]
    channel_overrides: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct TomlChainStep {
    channel: String,
    after_seconds: u32,
}

#[derive(Debug, Deserialize)]
struct TomlQuietHours {
    start_local: String,
    end_local: String,
    timezone: String,
    #[serde(default)]
    honor_for: Vec<String>,
}

impl TomlFileShape {
    fn into_operators(self) -> ChannelResult<HashMap<OperatorId, OperatorPreferences>> {
        let mut out = HashMap::with_capacity(self.operator.len());
        for (id, entry) in self.operator {
            let Some(prefs) = entry.preferences else {
                continue;
            };
            let operator_id = OperatorId::new(id);
            let fallback_chain = prefs
                .fallback_chain
                .into_iter()
                .map(|s| ChannelChainStep {
                    channel: ChannelId::new(s.channel),
                    after_seconds: s.after_seconds,
                })
                .collect();
            let quiet_hours = prefs.quiet_hours.map(parse_quiet_hours).transpose()?;
            let channel_overrides = prefs
                .channel_overrides
                .into_iter()
                .map(|(k, v)| parse_risk(&k).map(|r| (r, ChannelId::new(v))))
                .collect::<ChannelResult<HashMap<_, _>>>()?;
            out.insert(
                operator_id.clone(),
                OperatorPreferences {
                    operator_id,
                    primary_channel: ChannelId::new(prefs.primary_channel),
                    fallback_chain,
                    quiet_hours,
                    channel_overrides,
                },
            );
        }
        Ok(out)
    }
}

fn parse_quiet_hours(q: TomlQuietHours) -> ChannelResult<OperatorQuietHours> {
    let start_local = NaiveTime::parse_from_str(&q.start_local, "%H:%M")
        .map_err(|e| ChannelError::Parse(format!("quiet_hours.start_local: {e}")))?;
    let end_local = NaiveTime::parse_from_str(&q.end_local, "%H:%M")
        .map_err(|e| ChannelError::Parse(format!("quiet_hours.end_local: {e}")))?;
    let timezone: Tz = q
        .timezone
        .parse()
        .map_err(|e| ChannelError::Parse(format!("quiet_hours.timezone {}: {e}", q.timezone)))?;
    let honor_for = q.honor_for.into_iter().map(ChannelId::new).collect();
    Ok(OperatorQuietHours {
        start_local,
        end_local,
        timezone,
        honor_for,
    })
}

fn parse_risk(s: &str) -> ChannelResult<RiskLevel> {
    match s {
        "low" => Ok(RiskLevel::Low),
        "medium" => Ok(RiskLevel::Medium),
        "high" => Ok(RiskLevel::High),
        other => Err(ChannelError::Parse(format!(
            "unknown risk level {other:?} (expected low/medium/high)"
        ))),
    }
}
