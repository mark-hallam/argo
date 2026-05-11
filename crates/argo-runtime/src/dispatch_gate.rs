//! Decompose-or-dispatch gate. Lets the agent dispatch loop ask
//! "should I run the planner on this goal?" before falling through
//! to single-task dispatch
//! (mission-planner-dispatch-loop-wire).
//!
//! v1 ships [`HeuristicGate`] — a deterministic, dependency-free
//! check based on simple list markers and length. v2 (separate
//! brief, `mission-planner-llm-gate`) replaces it with an
//! LLM-backed gate.
//!
//! Errors are non-authoritative: callers MUST treat any
//! [`GateError`] as "do not decompose" so a flaky gate never
//! crashes a turn. The agent-runner side enforces this by
//! `tracing::warn!`-ing and falling through to the existing
//! single-task path.

use crate::PlannerContext;
use async_trait::async_trait;
use thiserror::Error;

/// Errors a [`DecomposeGate`] may emit. Today only one variant is
/// possible, but the trait keeps room for future LLM-gate failure
/// modes (timeout, parse, auth). Callers are required to treat
/// any error as a "no-decompose" answer.
#[derive(Debug, Error)]
pub enum GateError {
    /// Generic gate failure (LLM timeout, parse, auth, etc).
    #[error("decompose gate failed: {0}")]
    Failed(String),
}

/// Pluggable "should we decompose?" hook. Called once per turn at
/// the dispatch-decision point before the worker's coarse goal is
/// executed as a single task.
///
/// ```ignore
/// use argo_runtime::{DecomposeGate, HeuristicGate, PlannerContext};
/// # async fn doc() -> Result<(), Box<dyn std::error::Error>> {
/// let gate = HeuristicGate::new();
/// let ctx = PlannerContext::for_worker("wkr-1");
/// assert!(gate.should_decompose("research X and then summarise", &ctx).await?);
/// # Ok(()) }
/// ```
#[async_trait]
pub trait DecomposeGate: Send + Sync {
    /// Decide whether `goal` warrants a [`crate::Planner::decompose`]
    /// pass. Returns `Ok(true)` → call planner; `Ok(false)` → skip
    /// and dispatch as a single task. Errors → caller treats as
    /// `false` (see module-level docs).
    async fn should_decompose(
        &self,
        goal: &str,
        ctx: &PlannerContext,
    ) -> Result<bool, GateError>;
}

/// Length threshold above which the heuristic gate decides a goal
/// warrants decomposition regardless of explicit list markers.
/// Empirically 200 chars catches multi-paragraph delegations
/// without firing on a sentence-long question.
pub const HEURISTIC_LENGTH_THRESHOLD: usize = 200;

/// Substrings the heuristic treats as explicit list markers. Any
/// hit short-circuits the decision to "decompose". Order is not
/// significant; matching is case-sensitive on purpose so we don't
/// false-positive on prose like "Andthen…" or "Then,".
pub const HEURISTIC_LIST_MARKERS: &[&str] = &["\n- ", "; ", " and then ", " then "];

/// Deterministic v1 gate. Returns `true` when:
/// - The goal contains an explicit list marker
///   (see [`HEURISTIC_LIST_MARKERS`]), OR
/// - The trimmed goal exceeds [`HEURISTIC_LENGTH_THRESHOLD`].
///
/// Empty / whitespace-only goals always return `false` so the
/// existing single-task path runs unchanged for trivial inputs.
/// The implementation is intentionally dumb: v2 swaps this out
/// for an LLM-backed gate (`mission-planner-llm-gate`).
#[derive(Debug, Default, Clone)]
pub struct HeuristicGate;

impl HeuristicGate {
    /// Build a default heuristic gate. Equivalent to
    /// `HeuristicGate::default()`.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl DecomposeGate for HeuristicGate {
    async fn should_decompose(
        &self,
        goal: &str,
        _ctx: &PlannerContext,
    ) -> Result<bool, GateError> {
        let trimmed = goal.trim();
        if trimmed.is_empty() {
            return Ok(false);
        }
        if trimmed.len() > HEURISTIC_LENGTH_THRESHOLD {
            return Ok(true);
        }
        for marker in HEURISTIC_LIST_MARKERS {
            if goal.contains(marker) {
                return Ok(true);
            }
        }
        Ok(false)
    }
}
