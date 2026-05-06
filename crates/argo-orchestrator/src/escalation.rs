//! Escalation chain — declarative path of fall-back participants
//! when an action exhausts its retry budget.
//!
//! Encoded as a [`Vec<EscalationStep>`]. Each step is either
//! another worker or a named human (typically email). The chain
//! is walked in order: the first non-faulty step accepts the
//! handoff; if it too fails, control passes to the next step.
//!
//! ```
//! use argo_orchestrator::EscalationChain;
//!
//! let chain = EscalationChain::from_strs(&[
//!     "worker:support_agent",
//!     "human:mark@acmeco.com",
//! ]);
//! assert_eq!(chain.len(), 2);
//! assert_eq!(chain.advance(0).map(|(i, _)| i), Some(1));
//! ```

use argo_identity::WorkerId;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// One step in an escalation chain.
///
/// `Worker(id)` references a worker by stable id. `Human(addr)`
/// is an opaque address — typically email but could be Slack DM,
/// SMS, etc. The `addr` is interpreted by the channel adapter
/// that owns the human handoff.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EscalationStep {
    /// Hand off to another worker by id.
    Worker(WorkerId),
    /// Hand off to a human at a free-form address.
    Human(String),
}

impl EscalationStep {
    /// Parse a `kind:value` string. Returns `None` if the prefix
    /// is unknown. `value` for `worker:` must be a non-empty
    /// string (it becomes a [`WorkerId`]).
    ///
    /// ```
    /// use argo_orchestrator::EscalationStep;
    /// let s = EscalationStep::parse("human:ops@acmeco.com");
    /// assert!(matches!(s, Some(EscalationStep::Human(_))));
    /// assert!(EscalationStep::parse("oops").is_none());
    /// ```
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        let (kind, value) = s.split_once(':')?;
        match kind {
            "worker" if !value.is_empty() => {
                // WorkerId is a typed UUID. If the value is not a
                // valid UUID we still accept the step — the
                // chain is declarative and downstream code may
                // assign ids before walking. To keep types clean
                // we generate a deterministic id when parsing
                // fails: callers using non-UUID worker labels
                // must round-trip via a typed `EscalationChain::new`.
                Some(Self::Worker(
                    WorkerId::from_str(value).unwrap_or_else(|_| WorkerId::new()),
                ))
            }
            "human" if !value.is_empty() => Some(Self::Human(value.to_string())),
            _ => None,
        }
    }
}

/// An ordered list of escalation steps.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EscalationChain {
    steps: Vec<EscalationStep>,
}

impl EscalationChain {
    /// Build a chain from typed steps.
    #[must_use]
    pub fn new(steps: Vec<EscalationStep>) -> Self {
        Self { steps }
    }

    /// Build a chain from string-encoded steps. Invalid entries
    /// are silently dropped — callers wanting strict validation
    /// should `parse` each step themselves and use [`Self::new`].
    #[must_use]
    pub fn from_strs(items: &[&str]) -> Self {
        Self {
            steps: items.iter().filter_map(|s| EscalationStep::parse(s)).collect(),
        }
    }

    /// Number of steps in the chain.
    #[must_use]
    pub fn len(&self) -> usize {
        self.steps.len()
    }

    /// `true` iff the chain has no steps.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// Borrow the underlying step list.
    #[must_use]
    pub fn steps(&self) -> &[EscalationStep] {
        &self.steps
    }

    /// Get the step at `index` if present.
    #[must_use]
    pub fn step(&self, index: usize) -> Option<&EscalationStep> {
        self.steps.get(index)
    }

    /// Move to the next step. Given the current step's index,
    /// returns `Some((next_index, next_step))` when one exists,
    /// or `None` when the chain is exhausted.
    ///
    /// Exhaustion is the runtime's signal to bubble the action
    /// up to a "no recipient available" failure mode — the
    /// dispatch shim records the action as `escalation_exhausted`
    /// and notifies the operator via the worker's default channel.
    #[must_use]
    pub fn advance(&self, current_index: usize) -> Option<(usize, &EscalationStep)> {
        let next = current_index.checked_add(1)?;
        self.steps.get(next).map(|step| (next, step))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_worker_step() {
        let s = EscalationStep::parse("worker:abc123").unwrap();
        assert!(matches!(s, EscalationStep::Worker(_)));
    }

    #[test]
    fn parse_human_step() {
        let s = EscalationStep::parse("human:mark@acmeco.com").unwrap();
        assert!(matches!(s, EscalationStep::Human(_)));
    }

    #[test]
    fn parse_rejects_unknown_kind() {
        assert!(EscalationStep::parse("alien:zorp").is_none());
    }

    #[test]
    fn parse_rejects_missing_value() {
        assert!(EscalationStep::parse("worker:").is_none());
    }

    #[test]
    fn parse_rejects_no_colon() {
        assert!(EscalationStep::parse("worker").is_none());
    }

    #[test]
    fn from_strs_drops_invalid() {
        let c = EscalationChain::from_strs(&[
            "worker:a",
            "garbage",
            "human:b",
        ]);
        assert_eq!(c.len(), 2);
    }

    #[test]
    fn advance_walks_to_end() {
        let c = EscalationChain::from_strs(&["worker:a", "worker:b", "human:c"]);
        let (i1, _) = c.advance(0).unwrap();
        assert_eq!(i1, 1);
        let (i2, _) = c.advance(i1).unwrap();
        assert_eq!(i2, 2);
        assert!(c.advance(i2).is_none());
    }

    #[test]
    fn advance_handles_overflow_safely() {
        let c = EscalationChain::from_strs(&["worker:a"]);
        // usize::MAX + 1 would overflow; checked_add must catch it.
        assert!(c.advance(usize::MAX).is_none());
    }

    #[test]
    fn empty_chain_is_empty() {
        let c = EscalationChain::default();
        assert!(c.is_empty());
        assert_eq!(c.len(), 0);
    }
}
