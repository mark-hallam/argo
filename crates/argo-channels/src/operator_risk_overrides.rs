//! Per-operator risk-level overrides.
//!
//! Today every action's risk classification flows from the vertical
//! manifest's `[actions.<name>].risk` declaration. Operators have no
//! way to say "for me, `book_travel` is Low" or "for me, every action
//! is Critical, treat me as paranoid". This module lands the seam.
//!
//! [`RiskResolver`] wraps any [`DefaultRiskPolicy`] (the existing
//! manifest-driven classifier) and consults an
//! [`OperatorRiskOverridesSource`] before falling through. Per-vertical
//! overrides win over `all-verticals` overrides; downgrades below the
//! default are refused unless the override row carries
//! `allow_downgrade_below_default = true`. Upgrades are always allowed
//! — paranoia is free.
//!
//! [`RiskResolutionTrace`] explains the resolution to forensics: was
//! the default kept, was an override applied, or was a downgrade
//! refused? Callers persist the trace into the audit-chain row.
//!
//! ```no_run
//! # async fn demo() -> argo_channels::ChannelResult<()> {
//! use std::sync::Arc;
//! use argo_channels::operator_preferences::OperatorId;
//! use argo_channels::operator_risk_overrides::{
//!     DefaultRiskPolicy, InMemoryOperatorRiskOverridesSource,
//!     RiskResolver,
//! };
//! use argo_channels::RiskLevel;
//!
//! struct AlwaysMedium;
//! #[async_trait::async_trait]
//! impl DefaultRiskPolicy for AlwaysMedium {
//!     async fn classify(&self, _: &str, _: &str)
//!         -> argo_channels::ChannelResult<RiskLevel> {
//!         Ok(RiskLevel::Medium)
//!     }
//! }
//!
//! let source = Arc::new(InMemoryOperatorRiskOverridesSource::default());
//! let resolver = RiskResolver::new(source, Arc::new(AlwaysMedium));
//! let (_risk, _trace) = resolver
//!     .resolve(&OperatorId::new("op-1"), "send_email", "support")
//!     .await?;
//! # Ok(()) }
//! ```

use crate::operator_preferences::OperatorId;
use crate::{ChannelError, ChannelResult, RiskLevel};
use async_trait::async_trait;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;

/// Per-operator risk-level override record.
///
/// Keys are `(action_kind, Option<vertical_id>)`. `None` in the
/// vertical position means "apply for every vertical this operator
/// touches"; `Some(v)` is per-vertical. Per-vertical wins over
/// all-verticals on the same `action_kind`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorRiskOverrides {
    /// Operator the overrides belong to.
    pub operator_id: OperatorId,
    /// Map from `(action_kind, vertical_id)` to the risk level the
    /// operator wants. `vertical_id == None` matches every vertical.
    pub overrides: HashMap<(String, Option<String>), RiskLevel>,
    /// When false (default), the resolver refuses any override that
    /// would lower the risk below the manifest default. Operators
    /// must opt in with `--allow-downgrade` per
    /// `argo operator risk-overrides set`.
    pub allow_downgrade_below_default: bool,
}

impl OperatorRiskOverrides {
    /// Build an empty override record for `operator_id`.
    #[must_use]
    pub fn empty(operator_id: OperatorId) -> Self {
        Self {
            operator_id,
            overrides: HashMap::new(),
            allow_downgrade_below_default: false,
        }
    }
}

/// Scope an applied override matched at.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OverrideScope {
    /// Override row keyed on `(action_kind, vertical_id)`.
    PerVertical(String),
    /// Override row keyed on `(action_kind, None)` — applies to
    /// every vertical this operator touches.
    AllVerticals,
}

/// Forensic explanation of how a risk level was resolved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RiskResolutionTrace {
    /// Risk level returned by the default policy before overrides.
    pub default: RiskLevel,
    /// Risk level applied by an override, if any. `None` means the
    /// default was kept either because no override matched or
    /// because the matched override was refused as a downgrade.
    pub override_applied: Option<RiskLevel>,
    /// Scope of the applied override.
    pub override_scope: Option<OverrideScope>,
    /// When the resolver saw a matching override but refused it
    /// because it would downgrade below the default without the
    /// `allow_downgrade_below_default` flag, this carries the level
    /// the override *would* have produced. Auditors use this to
    /// detect attempted-but-blocked downgrade attacks.
    pub downgrade_blocked: Option<RiskLevel>,
}

impl RiskResolutionTrace {
    fn passthrough(default: RiskLevel) -> Self {
        Self {
            default,
            override_applied: None,
            override_scope: None,
            downgrade_blocked: None,
        }
    }
}

/// Default risk policy abstraction.
///
/// The existing classifier — today, the vertical manifest's
/// `[actions.<name>].risk` declaration — implements this trait so the
/// resolver can fall through to it whenever no override matches.
#[async_trait]
pub trait DefaultRiskPolicy: Send + Sync {
    /// Classify `action_kind` under `vertical_id` using the
    /// manifest-driven default.
    async fn classify(&self, action_kind: &str, vertical_id: &str) -> ChannelResult<RiskLevel>;
}

/// Pluggable override source.
#[async_trait]
pub trait OperatorRiskOverridesSource: Send + Sync {
    /// Fetch the latest overrides for `op`. Implementations are
    /// expected to cache and return [`OperatorRiskOverrides::empty`]
    /// (not an error) when the operator has nothing on file.
    async fn for_operator(&self, op: &OperatorId) -> ChannelResult<OperatorRiskOverrides>;
}

/// Composed default + override resolver.
pub struct RiskResolver {
    overrides_source: Arc<dyn OperatorRiskOverridesSource>,
    default_policy: Arc<dyn DefaultRiskPolicy>,
}

impl RiskResolver {
    /// Build a resolver. Both inputs are arc-shared so the resolver
    /// can be cloned (or held under a shared `Arc`) by multiple
    /// dispatch paths.
    #[must_use]
    pub fn new(
        overrides_source: Arc<dyn OperatorRiskOverridesSource>,
        default_policy: Arc<dyn DefaultRiskPolicy>,
    ) -> Self {
        Self {
            overrides_source,
            default_policy,
        }
    }

    /// Resolve the risk level for `(op, action_kind, vertical_id)`.
    ///
    /// Precedence:
    /// 1. Compute the manifest default via the wrapped policy.
    /// 2. Look up `(action_kind, Some(vertical_id))` then
    ///    `(action_kind, None)`. Per-vertical wins.
    /// 3. If the matched override would downgrade below the default
    ///    and `allow_downgrade_below_default` is false, REFUSE the
    ///    override — record it in `trace.downgrade_blocked` so the
    ///    audit chain shows the attempt. Upgrade-only is the default
    ///    so that paranoia is free but looseness requires explicit
    ///    operator acknowledgement.
    pub async fn resolve(
        &self,
        op: &OperatorId,
        action_kind: &str,
        vertical_id: &str,
    ) -> ChannelResult<(RiskLevel, RiskResolutionTrace)> {
        let default = self
            .default_policy
            .classify(action_kind, vertical_id)
            .await?;
        let overrides = self.overrides_source.for_operator(op).await?;
        let per_vertical = overrides
            .overrides
            .get(&(action_kind.to_string(), Some(vertical_id.to_string())))
            .copied();
        let all_verticals = overrides
            .overrides
            .get(&(action_kind.to_string(), None))
            .copied();
        let (matched, scope) = match (per_vertical, all_verticals) {
            (Some(r), _) => (Some(r), Some(OverrideScope::PerVertical(vertical_id.into()))),
            (None, Some(r)) => (Some(r), Some(OverrideScope::AllVerticals)),
            (None, None) => (None, None),
        };
        let Some(target) = matched else {
            return Ok((default, RiskResolutionTrace::passthrough(default)));
        };
        if rank(target) < rank(default) && !overrides.allow_downgrade_below_default {
            let trace = RiskResolutionTrace {
                default,
                override_applied: None,
                override_scope: None,
                downgrade_blocked: Some(target),
            };
            return Ok((default, trace));
        }
        let trace = RiskResolutionTrace {
            default,
            override_applied: Some(target),
            override_scope: scope,
            downgrade_blocked: None,
        };
        Ok((target, trace))
    }
}

fn rank(r: RiskLevel) -> u8 {
    match r {
        RiskLevel::Low => 0,
        RiskLevel::Medium => 1,
        RiskLevel::High => 2,
    }
}

/// Test-only [`OperatorRiskOverridesSource`] backed by an in-memory
/// map. Missing operators get an empty record rather than an error so
/// the resolver can use the default policy without special-casing.
#[derive(Default, Clone)]
pub struct InMemoryOperatorRiskOverridesSource {
    inner: Arc<Mutex<HashMap<OperatorId, OperatorRiskOverrides>>>,
}

impl InMemoryOperatorRiskOverridesSource {
    /// Build a fresh empty source.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Build a source pre-populated from `map`.
    #[must_use]
    pub fn with_map(map: HashMap<OperatorId, OperatorRiskOverrides>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(map)),
        }
    }

    /// Insert or replace overrides for one operator.
    pub fn insert(&self, overrides: OperatorRiskOverrides) {
        let mut guard = self.inner.lock();
        guard.insert(overrides.operator_id.clone(), overrides);
    }
}

#[async_trait]
impl OperatorRiskOverridesSource for InMemoryOperatorRiskOverridesSource {
    async fn for_operator(&self, op: &OperatorId) -> ChannelResult<OperatorRiskOverrides> {
        let guard = self.inner.lock();
        Ok(guard
            .get(op)
            .cloned()
            .unwrap_or_else(|| OperatorRiskOverrides::empty(op.clone())))
    }
}

/// TOML-backed override source. Reads
/// `[operator.<id>.risk_overrides]` blocks from `argo.toml`. Reload
/// is mtime-poll-cached (same shape as
/// [`crate::operator_preferences::TomlPreferencesSource`]) so multiple
/// dispatcher calls within a 5-second window see the same parsed
/// snapshot without re-statting the file.
pub struct TomlOperatorRiskOverridesSource {
    path: PathBuf,
    cache: Arc<Mutex<TomlCache>>,
}

#[derive(Default)]
struct TomlCache {
    mtime: Option<SystemTime>,
    operators: HashMap<OperatorId, OperatorRiskOverrides>,
}

impl TomlOperatorRiskOverridesSource {
    /// Build a source over `path`. The file is not opened until the
    /// first [`OperatorRiskOverridesSource::for_operator`] call.
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
                    "risk_overrides file {} unreadable: {e}",
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
                "risk_overrides file {} read failed: {e}",
                self.path.display()
            ))
        })?;
        let parsed: TomlFileShape = toml::from_str(&body)
            .map_err(|e| ChannelError::Parse(format!("risk_overrides TOML: {e}")))?;
        let operators = parsed.into_operators()?;
        let mut guard = self.cache.lock();
        guard.mtime = Some(mtime);
        guard.operators = operators;
        Ok(())
    }
}

#[async_trait]
impl OperatorRiskOverridesSource for TomlOperatorRiskOverridesSource {
    async fn for_operator(&self, op: &OperatorId) -> ChannelResult<OperatorRiskOverrides> {
        self.reload_if_stale()?;
        let guard = self.cache.lock();
        Ok(guard
            .operators
            .get(op)
            .cloned()
            .unwrap_or_else(|| OperatorRiskOverrides::empty(op.clone())))
    }
}

#[derive(Debug, Deserialize)]
struct TomlFileShape {
    #[serde(default)]
    operator: HashMap<String, TomlOperatorEntry>,
}

#[derive(Debug, Deserialize)]
struct TomlOperatorEntry {
    risk_overrides: Option<TomlRiskOverrides>,
}

#[derive(Debug, Deserialize)]
struct TomlRiskOverrides {
    #[serde(default)]
    allow_downgrade_below_default: bool,
    #[serde(default)]
    entries: Vec<TomlEntry>,
}

#[derive(Debug, Deserialize)]
struct TomlEntry {
    action_kind: String,
    risk_level: String,
    #[serde(default)]
    vertical_id: Option<String>,
}

impl TomlFileShape {
    fn into_operators(self) -> ChannelResult<HashMap<OperatorId, OperatorRiskOverrides>> {
        let mut out = HashMap::with_capacity(self.operator.len());
        for (id, entry) in self.operator {
            let Some(over) = entry.risk_overrides else {
                continue;
            };
            let operator_id = OperatorId::new(id);
            let mut overrides = HashMap::new();
            for e in over.entries {
                let level = parse_risk(&e.risk_level)?;
                overrides.insert((e.action_kind, e.vertical_id), level);
            }
            out.insert(
                operator_id.clone(),
                OperatorRiskOverrides {
                    operator_id,
                    overrides,
                    allow_downgrade_below_default: over.allow_downgrade_below_default,
                },
            );
        }
        Ok(out)
    }
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
#[path = "operator_risk_overrides_tests.rs"]
mod tests;
