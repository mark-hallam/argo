//! Per-operator central preferences resolver.
//!
//! Today each channel reads its own config silos — Slack reads
//! `[channels.slack].operator_*`, Email reads `[channels.email].*`,
//! Voice reads its own per-route config. The dispatcher has no
//! single seam to ask "for this operator, route approvals to Signal,
//! fall back to iMessage after 10 min, never SMS, quiet hours
//! 22:00–07:00." This module lands that seam.
//!
//! [`OperatorPreferencesResolver`] is the one-shot lookup the
//! dispatcher consults BEFORE picking a channel. Channel-specific
//! config silos remain authoritative for in-channel rendering
//! (Slack block-kit, Email MIME, etc.).
//!
//! The resolver pulls preferences from a pluggable
//! [`OperatorPreferencesSource`]. Two impls land in this brief:
//!
//! * [`TomlPreferencesSource`] — reads `[operator.<id>.preferences]`
//!   blocks from `argo.toml`. Mtime-poll cached (no inotify).
//! * [`InMemoryPreferencesSource`] — test-only, accepts a
//!   `HashMap<OperatorId, OperatorPreferences>` at construction.
//!
//! See module-level tests in `operator_preferences_tests.rs` for
//! the resolver contract.
//!
//! ## Pilot integration
//!
//! ```no_run
//! use std::sync::Arc;
//! use chrono::Utc;
//! use argo_channels::operator_preferences::{
//!     InMemoryPreferencesSource, OperatorId, OperatorPreferencesResolver,
//! };
//! use argo_channels::RiskLevel;
//!
//! # async fn demo() -> argo_channels::ChannelResult<()> {
//! let source = Arc::new(InMemoryPreferencesSource::default());
//! let resolver = OperatorPreferencesResolver::new(source);
//! let op = OperatorId::new("op-123");
//! let _picked = resolver.resolve_initial(&op, RiskLevel::Medium, Utc::now()).await?;
//! # Ok(()) }
//! ```

use crate::{ChannelError, ChannelId, ChannelResult, RiskLevel};
use async_trait::async_trait;
use chrono::{DateTime, NaiveTime, Utc};
use chrono_tz::Tz;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

/// Stable operator identifier used as the resolver key.
///
/// The brief originally proposed reusing `argo-identity::Operator`
/// but that type does not exist in the workspace today — the
/// `OperatorTarget.operator_id` string is the de-facto identifier.
/// We wrap it in a typed newtype so the resolver signature stays
/// honest. The string form must match the TOML table key
/// (`[operator.<id>.preferences]`).
///
/// ```
/// use argo_channels::operator_preferences::OperatorId;
/// let id = OperatorId::new("alice");
/// assert_eq!(id.as_str(), "alice");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OperatorId(pub String);

impl OperatorId {
    /// Wrap a string into an operator id.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Borrow the underlying string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for OperatorId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// One step in an operator's fallback chain.
///
/// The dispatcher tries [`OperatorPreferences::primary_channel`]
/// first; if no decision arrives within `after_seconds`, it
/// promotes the next step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelChainStep {
    /// Channel kind to escalate to.
    pub channel: ChannelId,
    /// Seconds to wait after the previous step before promoting
    /// this one.
    pub after_seconds: u32,
}

/// Operator quiet-hours window, with per-channel honoring.
///
/// Differs from [`crate::QuietHours`] in two ways: it stores
/// `NaiveTime` (not `String`) and a proper IANA timezone (not a
/// fixed offset), and it carries `honor_for` so high-urgency
/// channels (e.g. voice for critical risk) can ring through the
/// window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorQuietHours {
    /// Window start in the operator's local time.
    pub start_local: NaiveTime,
    /// Window end in the operator's local time. May be earlier
    /// than `start_local` for overnight windows.
    pub end_local: NaiveTime,
    /// IANA zone the operator lives in.
    pub timezone: Tz,
    /// Channels that defer when the window matches. Channels not
    /// in the set are exempt and ring through.
    pub honor_for: HashSet<ChannelId>,
}

impl OperatorQuietHours {
    /// Does `now` fall inside the operator's local window?
    #[must_use]
    pub fn contains(&self, now: DateTime<Utc>) -> bool {
        let local = now.with_timezone(&self.timezone).time();
        if self.start_local <= self.end_local {
            local >= self.start_local && local < self.end_local
        } else {
            local >= self.start_local || local < self.end_local
        }
    }

    /// Should the given channel defer when the window matches?
    #[must_use]
    pub fn honors(&self, channel: &ChannelId) -> bool {
        self.honor_for.contains(channel)
    }
}

/// Central per-operator preferences consulted by the dispatcher.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorPreferences {
    /// Operator the preferences belong to.
    pub operator_id: OperatorId,
    /// Default channel for any approval not matched by an
    /// override.
    pub primary_channel: ChannelId,
    /// Ordered fallback chain. First entry's `after_seconds` is
    /// counted from the primary send.
    pub fallback_chain: Vec<ChannelChainStep>,
    /// Optional quiet hours window.
    pub quiet_hours: Option<OperatorQuietHours>,
    /// Sparse risk-level overrides. Absent risk levels fall
    /// through to `primary_channel`.
    pub channel_overrides: HashMap<RiskLevel, ChannelId>,
}

/// Pluggable preferences source.
#[async_trait]
pub trait OperatorPreferencesSource: Send + Sync {
    /// Fetch the latest preferences for `op`. Implementations are
    /// expected to cache; the resolver calls this once per
    /// approval-send decision.
    async fn for_operator(&self, op: &OperatorId) -> ChannelResult<OperatorPreferences>;
}

/// Central resolver. Wraps any [`OperatorPreferencesSource`].
pub struct OperatorPreferencesResolver {
    source: Arc<dyn OperatorPreferencesSource>,
}

impl OperatorPreferencesResolver {
    /// Build a resolver over `source`.
    #[must_use]
    pub fn new(source: Arc<dyn OperatorPreferencesSource>) -> Self {
        Self { source }
    }

    /// Resolve the initial channel for `op` at `now`.
    ///
    /// Precedence: `channel_overrides[risk]` > primary, then quiet
    /// hours skips the picked channel and walks the fallback chain
    /// until a non-quiet channel is found. Returns the last chain
    /// entry when every step is in quiet — the caller still needs
    /// to send somewhere.
    pub async fn resolve_initial(
        &self,
        op: &OperatorId,
        risk: RiskLevel,
        now: DateTime<Utc>,
    ) -> ChannelResult<ChannelId> {
        let prefs = self.source.for_operator(op).await?;
        let candidate = prefs
            .channel_overrides
            .get(&risk)
            .cloned()
            .unwrap_or_else(|| prefs.primary_channel.clone());
        if !is_quiet(&prefs.quiet_hours, &candidate, now) {
            return Ok(candidate);
        }
        for step in &prefs.fallback_chain {
            if !is_quiet(&prefs.quiet_hours, &step.channel, now) {
                return Ok(step.channel.clone());
            }
        }
        Ok(candidate)
    }

    /// Promote the next fallback step when `elapsed_since_send`
    /// crosses the step's `after_seconds` threshold.
    ///
    /// `current` is the channel last attempted. The next step is
    /// the first chain entry whose channel is not `current` and
    /// whose `after_seconds <= elapsed_since_send`, walking from
    /// the chain head. Returns `None` when the chain is exhausted
    /// or `elapsed_since_send` has not crossed the next step.
    pub async fn next_fallback(
        &self,
        op: &OperatorId,
        current: ChannelId,
        elapsed_since_send: Duration,
        now: DateTime<Utc>,
    ) -> ChannelResult<Option<ChannelId>> {
        let prefs = self.source.for_operator(op).await?;
        let elapsed_secs =
            u32::try_from(elapsed_since_send.as_secs()).unwrap_or(u32::MAX);
        for step in &prefs.fallback_chain {
            if step.channel == current {
                continue;
            }
            if elapsed_secs < step.after_seconds {
                return Ok(None);
            }
            if is_quiet(&prefs.quiet_hours, &step.channel, now) {
                continue;
            }
            return Ok(Some(step.channel.clone()));
        }
        Ok(None)
    }
}

fn is_quiet(
    quiet: &Option<OperatorQuietHours>,
    channel: &ChannelId,
    now: DateTime<Utc>,
) -> bool {
    match quiet {
        Some(q) => q.honors(channel) && q.contains(now),
        None => false,
    }
}

/// Test-only [`OperatorPreferencesSource`] backed by an in-memory
/// map. Missing operator ids return [`ChannelError::Config`].
#[derive(Default, Clone)]
pub struct InMemoryPreferencesSource {
    inner: Arc<Mutex<HashMap<OperatorId, OperatorPreferences>>>,
}

impl InMemoryPreferencesSource {
    /// Build a fresh empty source.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Build a source pre-populated from `map`.
    #[must_use]
    pub fn with_map(map: HashMap<OperatorId, OperatorPreferences>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(map)),
        }
    }

    /// Insert or replace preferences for one operator.
    pub fn insert(&self, prefs: OperatorPreferences) {
        let mut guard = self.inner.lock();
        guard.insert(prefs.operator_id.clone(), prefs);
    }
}

#[async_trait]
impl OperatorPreferencesSource for InMemoryPreferencesSource {
    async fn for_operator(&self, op: &OperatorId) -> ChannelResult<OperatorPreferences> {
        let guard = self.inner.lock();
        guard
            .get(op)
            .cloned()
            .ok_or_else(|| ChannelError::Config(format!("no preferences for operator {op}")))
    }
}

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
            let quiet_hours = prefs
                .quiet_hours
                .map(parse_quiet_hours)
                .transpose()?;
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

#[cfg(test)]
#[path = "operator_preferences_tests.rs"]
mod tests;
