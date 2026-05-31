//! Historical-replay seam (DEC-017 phase-2): evaluate a teammate against
//! REAL past outcomes, not only synthetic goldens.
//!
//! [`crate::teammate_runner`] scores hand-authored `(scenario, outcome)`
//! golden cases; [`crate::teammate`] / [`crate::teammate_runner`] /
//! [`crate::teammate_scorecard`] each note that producing a
//! [`ScenarioOutcome`] from a real teammate run is a deferred *phase-2* seam.
//! This module is that seam: a [`ProductionOutcomeSource`] (implemented by the
//! gateway/store layer against argo-store audit/run history; faked in tests)
//! yields the outcomes a teammate actually produced in production, which are
//! paired to their rubric **by name** and scored by the SAME
//! [`run_golden_suite`] + [`Scorecard`] — so the teammate is graded on what it
//! really did, not just on synthetic fixtures.
//!
//! The seam mirrors [`crate::honesty::AuditSampler`]: the live wire (the
//! argo-store query) and the unit tests share one trait, so argo-eval keeps
//! **no direct argo-store dependency**. Pairing + aggregation here are pure and
//! deterministic (no model, no IO), so they run in CI.
//!
//! ```
//! use argo_eval::replay::{pair_replayed, ReplayedInteraction};
//! use argo_eval::teammate::{ScenarioOutcome, TeammateScenario};
//!
//! let scenarios = vec![TeammateScenario {
//!     name: "support/refund-001".into(),
//!     situation: "A customer asks for a refund outside policy.".into(),
//!     expected_substrings: vec!["policy".into()],
//!     should_escalate: true,
//!     required_skills: vec![],
//!     forbidden_claim: None,
//!     budget_ms: 5_000,
//!     budget_usd_micros: 20_000,
//! }];
//! let interactions = vec![ReplayedInteraction {
//!     scenario_name: "support/refund-001".into(),
//!     outcome: ScenarioOutcome {
//!         output: "Our refund policy is 30 days; I'll escalate this exception.".into(),
//!         escalated: true,
//!         skills_used: vec![],
//!         honest: true,
//!         elapsed_ms: 900,
//!         cost_usd_micros: 4_000,
//!     },
//! }];
//! let pairing = pair_replayed(&scenarios, &interactions);
//! assert_eq!(pairing.cases.len(), 1);
//! assert!(pairing.unmatched.is_empty());
//! assert!(pairing.uncovered.is_empty());
//! ```

use crate::teammate::{ScenarioOutcome, TeammateScenario};
use crate::teammate_runner::{run_golden_suite, GoldenCase, TeammateEvalReport};
use crate::teammate_scorecard::{build_scorecard, Scorecard};
use argo_types::ArgoResult;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// A real past interaction to be scored: which rubric it belongs to plus the
/// outcome observed in production.
///
/// The `outcome` carries exactly the signals [`crate::teammate::score_scenario`]
/// grades (response text, escalation, skills used, honesty, latency, cost) —
/// all of which a [`ProductionOutcomeSource`] reads off audit/run history.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReplayedInteraction {
    /// The rubric to score against — matches [`TeammateScenario::name`].
    pub scenario_name: String,
    /// What the teammate actually did, read off the production record.
    pub outcome: ScenarioOutcome,
}

/// Source of a teammate's real past outcomes.
///
/// Implemented by the gateway/store layer against argo-store audit/run
/// history; faked in tests. Same seam pattern as
/// [`crate::honesty::AuditSampler`] — argo-eval keeps no direct argo-store
/// dependency, so the pure scoring path stays CI-runnable.
#[async_trait]
pub trait ProductionOutcomeSource: Send + Sync {
    /// Fetch up to `limit` recent real interactions for `teammate`.
    ///
    /// # Errors
    /// Propagates the underlying store/query error.
    async fn recent_outcomes(
        &self,
        teammate: &str,
        limit: usize,
    ) -> ArgoResult<Vec<ReplayedInteraction>>;
}

/// Result of pairing real interactions to rubrics by name — pure +
/// deterministic. Coverage gaps are surfaced, never silently dropped.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReplayPairing {
    /// Real interactions matched to a rubric, ready for the scorer.
    pub cases: Vec<GoldenCase>,
    /// Interaction `scenario_name`s with no matching rubric, sorted.
    pub unmatched: Vec<String>,
    /// Rubric names that no real interaction exercised, in scenario order.
    pub uncovered: Vec<String>,
}

/// Pair real interactions to rubrics by name.
///
/// Each interaction whose `scenario_name` equals a [`TeammateScenario::name`]
/// becomes a [`GoldenCase`] (in interaction order); the rest are reported in
/// `unmatched` (sorted). Rubrics no interaction referenced are reported in
/// `uncovered` (scenario order). Pure + deterministic — no model, no IO.
#[must_use]
pub fn pair_replayed(
    scenarios: &[TeammateScenario],
    interactions: &[ReplayedInteraction],
) -> ReplayPairing {
    let mut cases = Vec::new();
    let mut unmatched = Vec::new();
    let mut covered: BTreeSet<&str> = BTreeSet::new();
    for it in interactions {
        match scenarios.iter().find(|s| s.name == it.scenario_name) {
            Some(scenario) => {
                covered.insert(scenario.name.as_str());
                cases.push(GoldenCase {
                    scenario: scenario.clone(),
                    outcome: it.outcome.clone(),
                });
            }
            None => unmatched.push(it.scenario_name.clone()),
        }
    }
    unmatched.sort();
    let uncovered: Vec<String> = scenarios
        .iter()
        .filter(|s| !covered.contains(s.name.as_str()))
        .map(|s| s.name.clone())
        .collect();
    ReplayPairing {
        cases,
        unmatched,
        uncovered,
    }
}

/// A teammate's evaluation against REAL past outcomes: the per-scenario
/// report + aggregate scorecard over the real interactions, plus replay
/// coverage so a caller can see how much of the rubric set was exercised.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReplayReport {
    /// The teammate evaluated.
    pub teammate: String,
    /// Per-scenario verdicts + pass-rate over the REAL interactions.
    pub report: TeammateEvalReport,
    /// Aggregate per-axis scorecard over the same real interactions.
    pub scorecard: Scorecard,
    /// Number of real interactions that were scored (matched a rubric).
    pub scored: usize,
    /// Interaction names with no matching rubric (coverage gap), sorted.
    pub unmatched: Vec<String>,
    /// Rubric names no real interaction exercised.
    pub uncovered: Vec<String>,
}

/// Evaluate `teammate` against its REAL past outcomes.
///
/// Pulls up to `limit` recent interactions from `source`, pairs them to
/// `scenarios` by name, and scores with the existing golden-suite scorer +
/// scorecard at `threshold`. Closes the DEC-017 phase-2 "live-outcome
/// producer" seam — the teammate is graded on production behaviour, not only
/// synthetic goldens.
///
/// # Errors
/// Propagates the source's store/query error.
pub async fn replay_report(
    source: &dyn ProductionOutcomeSource,
    teammate: &str,
    scenarios: &[TeammateScenario],
    limit: usize,
    threshold: f64,
) -> ArgoResult<ReplayReport> {
    let interactions = source.recent_outcomes(teammate, limit).await?;
    let pairing = pair_replayed(scenarios, &interactions);
    let report = run_golden_suite(&pairing.cases, threshold);
    let scorecard = build_scorecard(teammate, &pairing.cases, threshold);
    Ok(ReplayReport {
        teammate: teammate.to_string(),
        scored: pairing.cases.len(),
        unmatched: pairing.unmatched,
        uncovered: pairing.uncovered,
        report,
        scorecard,
    })
}

/// In-memory [`ProductionOutcomeSource`] for tests and for local replay from
/// an already-loaded set of interactions (e.g. a JSONL export of audit/run
/// history). The live path uses the store-backed impl instead.
#[derive(Debug, Clone, Default)]
pub struct VecOutcomeSource {
    interactions: Vec<ReplayedInteraction>,
}

impl VecOutcomeSource {
    /// Build from an explicit set of interactions.
    #[must_use]
    pub fn new(interactions: Vec<ReplayedInteraction>) -> Self {
        Self { interactions }
    }
}

#[async_trait]
impl ProductionOutcomeSource for VecOutcomeSource {
    async fn recent_outcomes(
        &self,
        _teammate: &str,
        limit: usize,
    ) -> ArgoResult<Vec<ReplayedInteraction>> {
        Ok(self.interactions.iter().take(limit).cloned().collect())
    }
}

#[cfg(test)]
#[path = "replay_tests.rs"]
mod tests;
