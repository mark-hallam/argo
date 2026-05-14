//! Self-healing proposals for eval fixtures.
//!
//! This module is deliberately transport-agnostic: an outer layer can
//! ask an LLM or judge service why a fixture failed, then pass the
//! structured judgment here. Only confident environmental failures
//! produce update proposals, and applying a proposal requires an
//! explicit operator approval decision.

use crate::{EvalMetric, EvalRecord, EvalTask};
use argo_types::{ArgoError, ArgoResult};
use serde::{Deserialize, Serialize};

/// Minimum-confidence gate for self-healing proposals.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SelfHealingConfig {
    /// Lowest acceptable confidence for an environmental judgment.
    pub min_confidence: f32,
}

impl Default for SelfHealingConfig {
    fn default() -> Self {
        Self {
            min_confidence: 0.70,
        }
    }
}

/// The judged reason an eval fixture failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FixtureFailureKind {
    /// External UI/API/document shape changed; the fixture may be stale.
    Environmental,
    /// The evaluated skill or model produced the wrong answer.
    Logical,
    /// The judge could not classify the failure confidently.
    Unknown,
}

/// Structured judgment for one failed fixture.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FixtureFailureJudgment {
    /// Failure class.
    pub kind: FixtureFailureKind,
    /// Human-readable rationale from the judge.
    pub reason: String,
    /// Confidence in `[0.0, 1.0]`.
    pub confidence: f32,
    /// Replacement expected output when the environment changed.
    pub proposed_expected: Option<String>,
}

impl FixtureFailureJudgment {
    /// Build a confident environmental judgment with a replacement
    /// expected value.
    ///
    /// ```
    /// use argo_eval::FixtureFailureJudgment;
    ///
    /// let judgment = FixtureFailureJudgment::environmental(
    ///     "button label changed",
    ///     "Approve request",
    ///     0.91,
    /// );
    /// assert_eq!(judgment.proposed_expected.as_deref(), Some("Approve request"));
    /// ```
    #[must_use]
    pub fn environmental(
        reason: impl Into<String>,
        expected: impl Into<String>,
        confidence: f32,
    ) -> Self {
        Self {
            kind: FixtureFailureKind::Environmental,
            reason: reason.into(),
            confidence,
            proposed_expected: Some(expected.into()),
        }
    }

    /// Build a logical-failure judgment.
    #[must_use]
    pub fn logical(reason: impl Into<String>, confidence: f32) -> Self {
        Self {
            kind: FixtureFailureKind::Logical,
            reason: reason.into(),
            confidence,
            proposed_expected: None,
        }
    }
}

/// Proposed patch for one fixture.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixtureUpdateProposal {
    /// Fixture id.
    pub fixture: String,
    /// Judge rationale shown to the operator.
    pub reason: String,
    /// Judgment confidence.
    pub confidence: f32,
    /// Existing fixture.
    pub before: EvalTask,
    /// Replacement fixture to apply after approval.
    pub after: EvalTask,
}

/// Operator decision for a fixture-update proposal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "decision")]
pub enum OperatorFixtureApproval {
    /// Operator approved the proposal.
    Approved {
        /// Stable operator identifier for audit trails.
        approver: String,
    },
    /// Operator rejected the proposal.
    Rejected {
        /// Optional rejection reason.
        reason: Option<String>,
    },
}

/// Receipt returned after an approved fixture update is applied.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixtureUpdateReceipt {
    /// Fixture id.
    pub fixture: String,
    /// Operator who approved the update.
    pub approver: String,
    /// Previous fixture.
    pub before: EvalTask,
    /// Applied replacement fixture.
    pub after: EvalTask,
}

/// Propose a fixture update for one failed eval record.
///
/// Returns `None` when the fixture already passed, the judgment is not
/// environmental, confidence is below the configured floor, or no
/// replacement expected value is available.
#[must_use]
pub fn propose_fixture_update(
    task: &EvalTask,
    record: &EvalRecord,
    judgment: &FixtureFailureJudgment,
    config: SelfHealingConfig,
) -> Option<FixtureUpdateProposal> {
    if record.passed
        || record.task != task.name
        || judgment.kind != FixtureFailureKind::Environmental
        || judgment.confidence < config.min_confidence
    {
        return None;
    }
    let expected = judgment
        .proposed_expected
        .as_ref()
        .filter(|v| !v.trim().is_empty())?;
    let mut after = task.clone();
    after.expected = expected.clone();
    if matches!(after.metric, EvalMetric::SubstringContains { .. }) {
        after.metric = EvalMetric::SubstringContains {
            must_contain: vec![expected.clone()],
        };
    }
    Some(FixtureUpdateProposal {
        fixture: task.name.clone(),
        reason: judgment.reason.clone(),
        confidence: judgment.confidence.clamp(0.0, 1.0),
        before: task.clone(),
        after,
    })
}

/// Apply an approved fixture update to an in-memory fixture set.
///
/// # Errors
/// Returns [`ArgoError::Provider`] when the proposal's fixture id is
/// absent from `tasks`.
pub fn apply_fixture_update(
    tasks: &mut [EvalTask],
    proposal: &FixtureUpdateProposal,
    approval: OperatorFixtureApproval,
) -> ArgoResult<Option<FixtureUpdateReceipt>> {
    let approver = match approval {
        OperatorFixtureApproval::Approved { approver } => approver,
        OperatorFixtureApproval::Rejected { .. } => return Ok(None),
    };
    let task = tasks
        .iter_mut()
        .find(|task| task.name == proposal.fixture)
        .ok_or_else(|| ArgoError::Provider(format!("fixture '{}' not found", proposal.fixture)))?;
    let before = task.clone();
    *task = proposal.after.clone();
    Ok(Some(FixtureUpdateReceipt {
        fixture: proposal.fixture.clone(),
        approver,
        before,
        after: proposal.after.clone(),
    }))
}

#[cfg(test)]
#[path = "self_healing_tests.rs"]
mod tests;
