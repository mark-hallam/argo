//! V1.3 — Declarative handoff routing rules.
//!
//! Operators describe routing intent in YAML/TOML; the
//! [`HandoffWalker`] consults [`RoutingEngine::route`] before
//! falling through to the explicit escalation chain target.
//!
//! ## DSL shape
//!
//! ```yaml
//! rules:
//!   - name: copy-review
//!     priority: 10
//!     active: true
//!     when:
//!       expected_outcome_contains: "copy review"
//!     then:
//!       skill_match: "marketing-editor"
//!   - name: after-hours-escalation
//!     priority: 20
//!     active: true
//!     when:
//!       hour_of_day_utc_in: [22, 23, 0, 1, 2, 3, 4, 5]
//!     then:
//!       role_match: "on-call-sre"
//! ```
//!
//! Conditions are combined with AND semantics — every set field
//! must match. Empty `when` means "always". Rules fire in
//! ascending `priority` order; the first match wins.
//!
//! ## Selectors
//!
//! v1 implements `SpecificWorker` (literal `WorkerId`) and
//! `RoundRobin` (counter mod len) end-to-end. `SkillMatch` and
//! `RoleMatch` are reserved variants — the DSL parses + persists
//! them, but [`RoutingEngine::route`] returns `None` for those
//! variants until the skill/role registry exposes the lookup.
//! Walker falls back to the explicit chain target in that case.

use crate::EscalationStep;
use argo_identity::WorkerId;
use chrono::{DateTime, Datelike, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use uuid::Uuid;

pub use crate::routing_store::RoutingRuleStore;

/// Stable rule id (UUIDv7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RuleId(pub Uuid);

impl RuleId {
    /// Generate a fresh id.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for RuleId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for RuleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl std::str::FromStr for RuleId {
    type Err = uuid::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(s).map(Self)
    }
}

/// A single declarative routing rule.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HandoffRoutingRule {
    /// Stable identity.
    pub id: RuleId,
    /// Operator-friendly label (unique per tenant).
    pub name: String,
    /// Match conditions (AND-combined). Empty = always-match.
    pub when: WhenClause,
    /// Target selector to invoke when the conditions match.
    pub target: TargetSelector,
    /// Lower fires first.
    pub priority: u32,
    /// Soft-disable flag — inactive rules are skipped without
    /// being deleted.
    pub active: bool,
}

impl HandoffRoutingRule {
    /// Build a fresh rule with a generated id.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        when: WhenClause,
        target: TargetSelector,
        priority: u32,
    ) -> Self {
        Self {
            id: RuleId::new(),
            name: name.into(),
            when,
            target,
            priority,
            active: true,
        }
    }
}

/// Conjunctive match conditions. Every set field must hold.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WhenClause {
    /// Substring (case-insensitive) the handoff's
    /// `expected_outcome` must contain.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_outcome_contains: Option<String>,
    /// UTC hours-of-day during which the rule is eligible. Empty
    /// = no time constraint. Hours are `0..24`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hour_of_day_utc_in: Vec<u32>,
    /// UTC weekdays during which the rule is eligible (`0` =
    /// Monday … `6` = Sunday). Empty = no weekday constraint.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub weekday_utc_in: Vec<u32>,
}

impl WhenClause {
    /// Check whether the conjunctive match conditions hold for
    /// the given context.
    #[must_use]
    pub fn matches(&self, ctx: &RoutingContext) -> bool {
        if let Some(needle) = &self.expected_outcome_contains {
            let outcome = ctx.expected_outcome.to_ascii_lowercase();
            if !outcome.contains(&needle.to_ascii_lowercase()) {
                return false;
            }
        }
        if !self.hour_of_day_utc_in.is_empty() {
            let h = ctx.now.hour();
            if !self.hour_of_day_utc_in.contains(&h) {
                return false;
            }
        }
        if !self.weekday_utc_in.is_empty() {
            let wd = ctx.now.weekday().num_days_from_monday();
            if !self.weekday_utc_in.contains(&wd) {
                return false;
            }
        }
        true
    }
}

/// How the rule picks the target worker once it matches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetSelector {
    /// Literal worker id — always picks the same target.
    SpecificWorker(WorkerId),
    /// Cycle through the listed workers, picking position
    /// `count % len()` per call.
    RoundRobin(Vec<WorkerId>),
    /// Workers matching the named skill. Reserved — v1 returns
    /// `None` until the skill registry exposes the lookup.
    SkillMatch(String),
    /// Workers matching the named role. Reserved — v1 returns
    /// `None` until the role registry exposes the lookup.
    RoleMatch(String),
}

/// Inputs the routing engine inspects per evaluation.
#[derive(Debug, Clone)]
pub struct RoutingContext {
    /// `now` in UTC — drives time-based conditions.
    pub now: DateTime<Utc>,
    /// `expected_outcome` text the handoff payload carried.
    pub expected_outcome: String,
}

impl RoutingContext {
    /// Convenience constructor for tests + walker callers.
    #[must_use]
    pub fn new(now: DateTime<Utc>, expected_outcome: impl Into<String>) -> Self {
        Self {
            now,
            expected_outcome: expected_outcome.into(),
        }
    }
}

/// Holds the current rule set + a round-robin counter. Cheap to
/// `clone` — the counter is shared via `Arc<AtomicUsize>` so the
/// walker sees consistent rotation across ticks.
#[derive(Debug, Clone, Default)]
pub struct RoutingEngine {
    rules: Vec<HandoffRoutingRule>,
    round_robin_counter: Arc<AtomicUsize>,
}

impl RoutingEngine {
    /// Build an engine with the given rules. Order doesn't matter
    /// — [`RoutingEngine::route`] sorts by priority each call so
    /// hot-reloads work without re-sorting.
    #[must_use]
    pub fn new(rules: Vec<HandoffRoutingRule>) -> Self {
        Self {
            rules,
            round_robin_counter: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// View the loaded rules.
    #[must_use]
    pub fn rules(&self) -> &[HandoffRoutingRule] {
        &self.rules
    }

    /// Evaluate every active rule in priority order; return the
    /// first match's resolved target. `None` means "no rule
    /// matched" — the caller should fall back to the explicit
    /// escalation chain.
    #[must_use]
    pub fn route(&self, ctx: &RoutingContext) -> Option<EscalationStep> {
        let mut sorted: Vec<&HandoffRoutingRule> =
            self.rules.iter().filter(|r| r.active).collect();
        sorted.sort_by_key(|r| r.priority);
        for rule in sorted {
            if !rule.when.matches(ctx) {
                continue;
            }
            if let Some(step) = self.resolve_target(&rule.target) {
                return Some(step);
            }
        }
        None
    }

    fn resolve_target(&self, target: &TargetSelector) -> Option<EscalationStep> {
        match target {
            TargetSelector::SpecificWorker(id) => Some(EscalationStep::Worker(*id)),
            TargetSelector::RoundRobin(workers) => {
                if workers.is_empty() {
                    return None;
                }
                let idx = self.round_robin_counter.fetch_add(1, Ordering::SeqCst);
                Some(EscalationStep::Worker(workers[idx % workers.len()]))
            }
            // Reserved — see module docs.
            TargetSelector::SkillMatch(_) | TargetSelector::RoleMatch(_) => None,
        }
    }
}

#[cfg(test)]
#[path = "routing_tests.rs"]
mod tests;
