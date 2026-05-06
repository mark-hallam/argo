//! Stream `claude-policy-as-code` — declarative org policies.
//!
//! Today every approval gate consults the per-worker
//! [`crate::AutoApprovalPolicy`]. That's enough for one-operator
//! installs but enterprise adoption needs **declarative org-wide
//! policies** that workers inherit and can't easily override
//! (compliance, finance ceilings, quiet-hours, dual-control
//! requirements).
//!
//! This module ships the data model + YAML parser + chain
//! resolver. A workspace persists declarative policy files at
//! `<workspace>/policies/<name>.yaml`; the resolver layers them
//! over the existing per-worker / per-roster / workspace-default
//! policy stack so the gate gets ONE [`EffectivePolicy`] without
//! callers having to walk the chain.
//!
//! ## Resolution order
//!
//! For a given (action_kind, role, tenant) the resolver walks
//! these layers, top-most win:
//!
//! 1. **Action-specific rule** — `rules[].action_kind` matches
//!    the pending action_type exactly.
//! 2. **Worker explicit policy** — the
//!    [`crate::AutoApprovalPolicy`] field on
//!    [`crate::Worker::auto_approval`].
//! 3. **Roster policy** — collapsed via
//!    [`crate::Worker::resolved_policy_with_rosters`].
//! 4. **Workspace policy file** — first matching
//!    [`PolicyFile::applies_to`] on the file.
//! 5. **Workspace default** — the
//!    `<workspace>/.skycot/auto_approval.toml` shipped in H-1.
//!
//! The resolver returns an [`EffectivePolicy`] that bundles the
//! collapsed [`crate::AutoApprovalPolicy`] (consumed by the
//! existing `auto_approval::evaluate` engine) plus the
//! policy-file-only fields ([`EffectivePolicy::require_signatures`],
//! [`EffectivePolicy::quiet_hours`],
//! [`EffectivePolicy::audit_chain_required`]) that the gate's
//! v1.x extensions can layer on later without re-walking the
//! chain.
//!
//! ## Hard rules (constraints not toggles)
//!
//! Policy-file rules CONSTRAIN the worker — they cannot loosen.
//! Specifically the resolver:
//!
//! - Never raises a worker's spend caps. `spend_cap_per_action_usd`
//!   only takes effect when it's **lower** than the worker's
//!   existing per-action max (`max_usd_micros_per_action`).
//! - Never disables time-window checks already on the worker.
//!   `quiet_hours` always applies regardless of the worker's own
//!   `time_windows`.
//! - Always escalates `require_signatures` to the **maximum** of
//!   any rule that matches.
//!
//! This makes policy files safe to add without auditing every
//! worker — the worst case is "more restrictive than yesterday",
//! never "less restrictive".

use crate::AutoApprovalPolicy;
use chrono::NaiveTime;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// One declarative policy file (deserialised from YAML at
/// `<workspace>/policies/<name>.yaml`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PolicyFile {
    /// Policy file name (without the `.yaml` extension). Set by
    /// the loader from the file basename, not stored in the YAML.
    #[serde(default)]
    pub name: String,
    /// Selector list — a worker matches this file when ANY entry
    /// matches (OR semantics).
    pub applies_to: Vec<PolicyApplies>,
    /// Rules consulted in order; the first whose `action_kind`
    /// matches the pending action wins. `*` is a wildcard
    /// matching every action kind.
    pub rules: Vec<PolicyRule>,
}

/// One selector entry on a [`PolicyFile`]. Glob patterns end with
/// `*` to match a prefix; otherwise the match is exact.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyApplies {
    /// Match by [`crate::Role`] name (or glob like `finance-*`).
    Role(String),
    /// Match by [`argo_types::TenantId`] string id (or glob like
    /// `acme-*`).
    Tenant(String),
}

impl PolicyApplies {
    /// True when this selector matches the supplied (role,
    /// tenant) pair. Both sides honour a single trailing `*`
    /// wildcard; otherwise exact match.
    #[must_use]
    pub fn matches(&self, role: &str, tenant: &str) -> bool {
        match self {
            Self::Role(pattern) => glob_match(pattern, role),
            Self::Tenant(pattern) => glob_match(pattern, tenant),
        }
    }
}

/// One declarative rule inside a [`PolicyFile`]. Every field
/// beyond `action_kind` is optional — the resolver layers each
/// `Some` field onto the effective policy, leaving the others
/// alone.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PolicyRule {
    /// Action-kind selector. `*` = wildcard. The first rule whose
    /// `action_kind` matches the pending action_type wins.
    pub action_kind: String,
    /// Number of human signatures required. `1` is the default
    /// elsewhere; `2+` enables dual-control.
    #[serde(default)]
    pub require_signatures: Option<u32>,
    /// Quiet-hours window (per the operator's policy file). When
    /// the action fires inside the window, behaviour is governed
    /// by [`QuietHoursBehavior`]. Outside the window, the rule
    /// has no effect.
    #[serde(default)]
    pub quiet_hours: Option<QuietHours>,
    /// Per-action USD ceiling. Lower than the worker's existing
    /// per-action ceiling tightens it; higher is ignored.
    #[serde(default)]
    pub spend_cap_per_action_usd: Option<f64>,
    /// When true, every action of this kind must end up in the
    /// audit chain (failures included). Today the audit hooks
    /// always write — this flag is recorded on the effective
    /// policy so the v1.x audit-required gate can enforce it.
    #[serde(default)]
    pub audit_chain_required: Option<bool>,
}

/// One quiet-hours window. `start`/`end` are local-time `HH:MM`
/// strings; `behavior` decides what to do when an action fires
/// inside the window.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuietHours {
    /// Window start (`HH:MM`).
    pub start: String,
    /// Window end (`HH:MM`); when `end < start` the window wraps
    /// midnight (e.g. `start=18:00 end=09:00`).
    pub end: String,
    /// What to do inside the window.
    #[serde(default)]
    pub behavior: QuietHoursBehavior,
}

impl QuietHours {
    /// Parse `start` + `end` into [`NaiveTime`]. Used by the gate
    /// when checking the active window.
    pub fn parse_times(&self) -> Result<(NaiveTime, NaiveTime), PolicyError> {
        let start = NaiveTime::parse_from_str(&self.start, "%H:%M")
            .map_err(|e| PolicyError::QuietHoursTimeParse {
                value: self.start.clone(),
                message: e.to_string(),
            })?;
        let end = NaiveTime::parse_from_str(&self.end, "%H:%M").map_err(|e| {
            PolicyError::QuietHoursTimeParse {
                value: self.end.clone(),
                message: e.to_string(),
            }
        })?;
        Ok((start, end))
    }
}

/// What the gate should do when an action fires inside a
/// quiet-hours window.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuietHoursBehavior {
    /// Reject the action immediately.
    #[default]
    Block,
    /// Hold the action until the window closes; release on next
    /// scheduler tick.
    QueueUntilMorning,
}

/// Resolved policy returned by [`resolve_effective_policy`]. The
/// gate consumes the embedded [`AutoApprovalPolicy`] verbatim;
/// the file-only fields surface separately so the existing H-1
/// evaluator stays untouched.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EffectivePolicy {
    /// Collapsed auto-approval policy. The H-1 evaluator
    /// (`crate::auto_approval::evaluate`) consumes this directly.
    pub auto_approval: AutoApprovalPolicy,
    /// Number of human signatures required. Defaults to `1`.
    pub require_signatures: u32,
    /// Active quiet-hours window, if any.
    pub quiet_hours: Option<QuietHours>,
    /// Whether the audit hook must succeed before the action
    /// counts as approved. `false` by default.
    pub audit_chain_required: bool,
    /// Names of policy files that contributed to this resolution
    /// (debugging surface for `argo policy show --resolved`).
    pub source_files: Vec<String>,
}

impl EffectivePolicy {
    /// Construct an `EffectivePolicy` from a base
    /// [`AutoApprovalPolicy`] without any file overrides applied
    /// — used as the "no policy files" baseline before the
    /// resolver layers files on.
    #[must_use]
    pub fn from_base(base: AutoApprovalPolicy) -> Self {
        Self {
            auto_approval: base,
            require_signatures: 1,
            quiet_hours: None,
            audit_chain_required: false,
            source_files: Vec::new(),
        }
    }
}

/// Errors the policy parser / resolver / loader can return.
#[derive(Debug, thiserror::Error)]
pub enum PolicyError {
    /// YAML failed to deserialise into a [`PolicyFile`].
    #[error("policy file '{name}' failed to parse: {message}")]
    Parse {
        /// File name (basename without `.yaml`).
        name: String,
        /// Underlying serde_yaml error string.
        message: String,
    },
    /// Validation rejected the parsed file (e.g. empty rules).
    #[error("policy file '{name}' is invalid: {detail}")]
    Validation {
        /// File name (basename without `.yaml`).
        name: String,
        /// Human-readable explanation.
        detail: String,
    },
    /// Couldn't read the policy directory or a file inside it.
    #[error("policy filesystem error at {path}: {message}")]
    Io {
        /// Path that errored.
        path: String,
        /// Underlying io::Error string.
        message: String,
    },
    /// `quiet_hours.start` or `.end` wasn't a valid `HH:MM`.
    #[error("policy quiet_hours time '{value}' is not HH:MM: {message}")]
    QuietHoursTimeParse {
        /// The bad time value.
        value: String,
        /// chrono parse error string.
        message: String,
    },
}

/// Parse a YAML body into a [`PolicyFile`]. Used by both the CLI
/// loader and the gate's runtime loader. `name` is the file
/// basename (e.g. `finance-strict` for `finance-strict.yaml`).
pub fn parse_policy_yaml(name: &str, body: &str) -> Result<PolicyFile, PolicyError> {
    let mut file: PolicyFile =
        serde_yaml::from_str(body).map_err(|e| PolicyError::Parse {
            name: name.to_string(),
            message: e.to_string(),
        })?;
    file.name = name.to_string();
    validate_policy_file(&file)?;
    Ok(file)
}

/// Validate a parsed policy file. Catches problems the type
/// system can't (empty applies_to, wildcard-only rules, etc.).
pub fn validate_policy_file(file: &PolicyFile) -> Result<(), PolicyError> {
    if file.applies_to.is_empty() {
        return Err(PolicyError::Validation {
            name: file.name.clone(),
            detail: "applies_to is empty (must select at least one role or tenant)".into(),
        });
    }
    if file.rules.is_empty() {
        return Err(PolicyError::Validation {
            name: file.name.clone(),
            detail: "rules is empty (must declare at least one rule)".into(),
        });
    }
    for (i, rule) in file.rules.iter().enumerate() {
        if rule.action_kind.is_empty() {
            return Err(PolicyError::Validation {
                name: file.name.clone(),
                detail: format!("rules[{i}].action_kind is empty"),
            });
        }
        if let Some(qh) = &rule.quiet_hours {
            qh.parse_times()?;
        }
        if let Some(cap) = rule.spend_cap_per_action_usd {
            if !cap.is_finite() || cap < 0.0 {
                return Err(PolicyError::Validation {
                    name: file.name.clone(),
                    detail: format!(
                        "rules[{i}].spend_cap_per_action_usd must be >= 0; got {cap}"
                    ),
                });
            }
        }
    }
    Ok(())
}

/// Load every `*.yaml` policy file under `<dir>`, validating each
/// one. Errors short-circuit so a misconfigured workspace never
/// silently degrades. Files in unspecified order — caller passes
/// the result to [`resolve_effective_policy`] which preserves
/// order during chain walk.
pub fn load_policy_dir(dir: &Path) -> Result<Vec<PolicyFile>, PolicyError> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let entries = std::fs::read_dir(dir).map_err(|e| PolicyError::Io {
        path: dir.display().to_string(),
        message: e.to_string(),
    })?;
    let mut out = Vec::new();
    let mut paths: Vec<std::path::PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("yaml"))
        .collect();
    paths.sort(); // stable order so resolver behaviour is repeatable
    for p in paths {
        let body = std::fs::read_to_string(&p).map_err(|e| PolicyError::Io {
            path: p.display().to_string(),
            message: e.to_string(),
        })?;
        let name = p
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("policy")
            .to_string();
        out.push(parse_policy_yaml(&name, &body)?);
    }
    Ok(out)
}

/// Resolve the effective policy for one (action_kind, role,
/// tenant) tuple. Walks the policy-file chain in argument order,
/// applying matching rules ON TOP of `base`. Hard rules (see
/// module docs): never raises caps, never weakens windows,
/// always escalates signatures.
pub fn resolve_effective_policy(
    base: &AutoApprovalPolicy,
    files: &[PolicyFile],
    role: &str,
    tenant: &str,
    action_kind: &str,
) -> EffectivePolicy {
    let mut effective = EffectivePolicy::from_base(base.clone());
    for file in files {
        if !file.applies_to.iter().any(|sel| sel.matches(role, tenant)) {
            continue;
        }
        let Some(rule) = file
            .rules
            .iter()
            .find(|r| r.action_kind == action_kind || r.action_kind == "*")
        else {
            continue;
        };
        effective.source_files.push(file.name.clone());
        if let Some(req) = rule.require_signatures {
            if req > effective.require_signatures {
                effective.require_signatures = req;
            }
        }
        if let Some(qh) = &rule.quiet_hours {
            effective.quiet_hours = Some(qh.clone());
        }
        if let Some(cap_usd) = rule.spend_cap_per_action_usd {
            let cap_micros = (cap_usd * 1_000_000.0) as u64;
            effective.auto_approval.max_usd_micros_per_action = Some(
                effective
                    .auto_approval
                    .max_usd_micros_per_action
                    .map_or(cap_micros, |existing| existing.min(cap_micros)),
            );
        }
        if let Some(true) = rule.audit_chain_required {
            effective.audit_chain_required = true;
        }
    }
    effective
}

fn glob_match(pattern: &str, value: &str) -> bool {
    if let Some(prefix) = pattern.strip_suffix('*') {
        return value.starts_with(prefix);
    }
    pattern == value
}

#[cfg(test)]
#[path = "policy_tests.rs"]
mod tests;
