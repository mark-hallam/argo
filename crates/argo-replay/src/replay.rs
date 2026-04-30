//! Replayer — walks a recorded session's turns and re-emits them,
//! optionally with model/tools/workspace overrides.
use crate::source::{SessionSource, TurnRecord};
use argo_types::{ArgoResult, SessionId};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Replay configuration — what session to replay, and what (if
/// anything) to swap out before re-running.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayPlan {
    /// Session to replay.
    pub source_session: SessionId,
    /// Optional model override (e.g. `"claude-sonnet-4-6"`). When
    /// set, the replayer rebuilds requests against this model.
    pub model_override: Option<String>,
    /// Optional toolset override.
    pub tools_override: Option<Vec<String>>,
    /// Optional workspace override (alternate workspace state to
    /// replay against).
    pub workspace_override: Option<PathBuf>,
}

impl ReplayPlan {
    /// Verbatim plan — no overrides; replay reproduces the original.
    #[must_use]
    pub fn verbatim(source_session: SessionId) -> Self {
        Self {
            source_session,
            model_override: None,
            tools_override: None,
            workspace_override: None,
        }
    }

    /// `true` when no overrides are set — the §28.BB gate "replay
    /// reproduces original verbatim when no overrides given" only
    /// holds in this case.
    #[must_use]
    pub fn is_verbatim(&self) -> bool {
        self.model_override.is_none()
            && self.tools_override.is_none()
            && self.workspace_override.is_none()
    }
}

/// One replayed turn — the original alongside the replayed output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnTrace {
    /// Original turn record.
    pub original: TurnRecord,
    /// Re-emitted assistant output.
    pub replayed_assistant: String,
    /// `true` when `replayed_assistant != original.assistant_text`.
    pub diverged: bool,
}

/// Replay result — the plan + per-turn traces.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayResult {
    /// The plan that was executed.
    pub plan: ReplayPlan,
    /// Per-turn traces, in turn order.
    pub turns: Vec<TurnTrace>,
}

impl ReplayResult {
    /// Number of turns where the replayed assistant diverged from
    /// the recorded one. Always 0 for a verbatim replay (gate 1).
    #[must_use]
    pub fn divergence_count(&self) -> usize {
        self.turns.iter().filter(|t| t.diverged).count()
    }
}

/// Replayer abstraction.
///
/// Production impl plugs into `argo-provider` + `argo-judge`. The
/// shipped [`VerbatimReplayer`] is the reference impl: it copies the
/// original assistant text through unchanged. With `is_verbatim()`
/// plans, this is the §28.BB "reproduces original verbatim" gate.
#[async_trait]
pub trait Replayer: Send + Sync {
    /// Execute the plan against `source` and return per-turn traces.
    async fn execute(
        &self,
        plan: &ReplayPlan,
        source: &dyn SessionSource,
    ) -> ArgoResult<ReplayResult>;
}

/// Verbatim replayer — re-emits the recorded assistant text byte
/// for byte. Used as the "no overrides" reference + as the test
/// double for the rest of the replay machinery.
#[derive(Debug, Default, Clone, Copy)]
pub struct VerbatimReplayer;

#[async_trait]
impl Replayer for VerbatimReplayer {
    async fn execute(
        &self,
        plan: &ReplayPlan,
        source: &dyn SessionSource,
    ) -> ArgoResult<ReplayResult> {
        let turns = source.fetch_turns(plan.source_session).await?;
        let traces = turns
            .into_iter()
            .map(|t| TurnTrace {
                replayed_assistant: t.assistant_text.clone(),
                diverged: false,
                original: t,
            })
            .collect();
        Ok(ReplayResult {
            plan: plan.clone(),
            turns: traces,
        })
    }
}
