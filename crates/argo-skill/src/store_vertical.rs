//! Per-vertical extensions to [`SkillStore`] (#678 + #679).
//! Lives in its own file so `store.rs` stays under the
//! 500-LOC budget after the vertical-scoped lookup methods
//! landed.
//!
//! Three additions:
//!
//! - `install_for_vertical(raw, learned_in, vertical_id)` —
//!   tags the new skill row with the bundling vertical so per-
//!   vertical lookups can scope correctly (#678).
//! - `list_skills_bundled_by_vertical(vertical_id)` — returns
//!   only the skill rows tagged for that vertical.
//! - `list_skills_for_vertical(vertical_id)` — UNION of
//!   bundled-by-vertical + learned-in-a-session-of-vertical.
//!   Default skill set the agent runner uses for vertical-bound
//!   sessions (#679 — Rule 6).
use crate::eval_runner::{run_propose_eval_gate, EvalExecutor, EvalGateOutcome};
use crate::parse::parse_skill_md;
use crate::skill::{LifecycleState, Skill};
use crate::store::SkillStore;
use crate::store_vertical_judge::{adjudicate_with_timeout, JudgeOutcome};
use argo_judge::Judge;
use argo_types::{ArgoError, ArgoResult, SessionId, SkillId};

impl SkillStore {
    /// Install a skill, tagging it with the vertical that
    /// bundled it (#678). Used by
    /// `vertical_attach::install_and_attach_bundle` so per-
    /// vertical skill lookups can scope correctly.
    pub async fn install_for_vertical(
        &self,
        raw: &str,
        learned_in: Option<SessionId>,
        vertical_id: Option<&str>,
    ) -> ArgoResult<Skill> {
        let (frontmatter, body) = parse_skill_md(raw)?;
        let existing = self.find_active_by_name(&frontmatter.name).await?;
        if existing.is_some() {
            return Err(ArgoError::Schema(format!(
                "skill '{}' already installed; use supersede() to bump",
                frontmatter.name
            )));
        }
        self.insert_row(frontmatter, body, learned_in, vertical_id).await
    }

    /// List skills bundled by a specific vertical (#678). Returns
    /// only rows where `skill.vertical_id = ?` AND not superseded.
    /// Used by per-vertical skill discovery + by lint to confirm
    /// the bundled skill set.
    pub async fn list_skills_bundled_by_vertical(
        &self,
        vertical_id: &str,
    ) -> ArgoResult<Vec<Skill>> {
        let rows: Vec<crate::store::RawSkill> = sqlx::query_as::<_, crate::store::RawSkill>(
            "SELECT s.id, s.name, s.version, s.body, s.frontmatter, s.source, s.hash, \
                    s.learned_in, s.superseded_by, s.created_at, l.lock, \
                    s.vertical_id, s.lifecycle_state \
             FROM skill s LEFT JOIN skill_lock l ON l.skill_id = s.id \
             WHERE s.vertical_id = ? AND s.superseded_by IS NULL \
               AND s.lifecycle_state = 'active' \
             ORDER BY s.created_at ASC",
        )
        .bind(vertical_id)
        .fetch_all(self.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("list_skills_bundled_by_vertical: {e}")))?;
        rows.into_iter().map(Skill::try_from).collect()
    }

    /// All active skills visible to a vertical-bound session
    /// (#679 — Rule 6). Returns the union of:
    ///
    /// - bundled skills tagged with `skill.vertical_id = ?`
    /// - learned skills installed via a session whose
    ///   `session.vertical_id = ?`
    ///
    /// Excludes everything else by default — workspace-global
    /// skills (no vertical context), other verticals' bundled
    /// skills, and other verticals' learned skills are NOT
    /// implicitly visible. Cross-vertical sharing requires the
    /// shared-asset surface (Rule 11, future PR).
    ///
    /// This is the default skill set the agent runner uses
    /// when no per-session attachments exist (#653 attachment
    /// path takes precedence when present).
    pub async fn list_skills_for_vertical(
        &self,
        vertical_id: &str,
    ) -> ArgoResult<Vec<Skill>> {
        let rows = sqlx::query_as::<_, crate::store::RawSkill>(
            "SELECT s.id, s.name, s.version, s.body, s.frontmatter, s.source, s.hash, \
                    s.learned_in, s.superseded_by, s.created_at, l.lock, s.vertical_id, \
                    s.lifecycle_state \
             FROM skill s \
             LEFT JOIN skill_lock l ON l.skill_id = s.id \
             LEFT JOIN session sess ON sess.id = s.learned_in \
             WHERE s.superseded_by IS NULL \
               AND s.lifecycle_state = 'active' \
               AND ( \
                 s.vertical_id = ? \
                 OR sess.vertical_id = ? \
               ) \
             ORDER BY s.name ASC",
        )
        .bind(vertical_id)
        .bind(vertical_id)
        .fetch_all(self.pool())
        .await
        .map_err(|e| {
            ArgoError::Provider(format!("list_skills_for_vertical: {e}"))
        })?;
        rows.into_iter().map(Skill::try_from).collect()
    }

    /// Propose a new learned skill for human review (#681 —
    /// work order #3). The skill is installed in the
    /// `pending_review` state — the agent's runtime lookup
    /// (`list_skills_for_vertical`) excludes non-active rows
    /// so the proposal can't influence agent decisions until
    /// a human approves via `transition_lifecycle`.
    ///
    /// `learned_in` is required (the agent that proposed it);
    /// `vertical_id` is required (default-deny: a learned
    /// skill must belong to a vertical).
    ///
    /// Back-compat shim: delegates to [`Self::propose_for_review_judged`]
    /// with `judge = None`. Existing callers continue to work
    /// unchanged. New callers that want the judge gate should call
    /// [`Self::propose_for_review_judged`] directly with a
    /// `Some(&dyn Judge)` argument (mission-wire-judge-on-skill-creation).
    pub async fn propose_for_review(
        &self,
        raw: &str,
        learned_in: SessionId,
        vertical_id: &str,
    ) -> ArgoResult<Skill> {
        self.propose_for_review_judged(raw, learned_in, vertical_id, None)
            .await
    }

    /// Propose a new learned skill for human review with an optional
    /// judge gate (mission-wire-judge-on-skill-creation).
    ///
    /// When `judge` is `Some`, the candidate is sent to
    /// [`Judge::adjudicate`] before the lifecycle transition. If the
    /// verdict falls below the configured floor (env
    /// `ARGO_JUDGE_SKILL_FLOOR`, default `0.6`), the candidate is
    /// installed in [`LifecycleState::Rejected`] with a stamped
    /// `judge_low_confidence: <reason>` provenance and `Ok(skill)` is
    /// returned (the row exists; reviewers can see the rejection in
    /// the audit trail).
    ///
    /// On judge timeout (30s) or transport error the candidate
    /// **fails OPEN** — proceeds to `PendingReview` and a
    /// `tracing::warn!` is emitted with target
    /// `argo_skill::judge::unavailable` so the operator can spot a
    /// degraded judge. We never block all skill learning on a
    /// flapping judge.
    ///
    /// `judge = None` is exactly the legacy `propose_for_review`
    /// behaviour: no gate, no audit, candidate goes straight to
    /// `PendingReview`.
    pub async fn propose_for_review_judged(
        &self,
        raw: &str,
        learned_in: SessionId,
        vertical_id: &str,
        judge: Option<&dyn Judge>,
    ) -> ArgoResult<Skill> {
        self.propose_for_review_judged_with_evals(raw, learned_in, vertical_id, judge, None)
            .await
    }

    /// Propose with both judge gate AND optional eval-set runner
    /// (`claude-skill-test-execution-engine`).
    ///
    /// When the parsed frontmatter declares a non-empty `evals` array
    /// AND `evals_executor` is `Some`, the runner walks every case
    /// before the lifecycle decision. If the resulting pass-rate is
    /// strictly below the [`DEFAULT_EVAL_PASS_RATE_THRESHOLD`] floor,
    /// the candidate is installed in [`LifecycleState::Rejected`] with
    /// an `eval_low_confidence: <rate>` reason — the row exists for
    /// audit but the agent never sees it.
    ///
    /// Eval-runner failure modes (executor errors, transport panics)
    /// already count as fails inside [`run_evals`]; this layer simply
    /// applies the floor. Empty evals OR `evals_executor = None`
    /// short-circuits and falls through to the existing judge gate
    /// — behaviour is unchanged.
    ///
    /// Eval gate runs BEFORE the judge: a skill that fails its own
    /// declared eval-set should never spend judge tokens. The judge
    /// gate keeps its own fail-OPEN semantics (degraded judge → still
    /// promotes to PendingReview).
    pub async fn propose_for_review_judged_with_evals(
        &self,
        raw: &str,
        learned_in: SessionId,
        vertical_id: &str,
        judge: Option<&dyn Judge>,
        evals_executor: Option<&dyn EvalExecutor>,
    ) -> ArgoResult<Skill> {
        let (frontmatter, body) = parse_skill_md(raw)?;
        let existing = self.find_active_by_name(&frontmatter.name).await?;
        if existing.is_some() {
            return Err(ArgoError::Schema(format!(
                "skill '{}' already installed; use supersede() to bump",
                frontmatter.name
            )));
        }
        let eval_outcome = run_propose_eval_gate(&frontmatter, &body, evals_executor).await;
        let lifecycle = match eval_outcome {
            EvalGateOutcome::Reject(reason) => {
                tracing::info!(
                    target: "argo_skill::eval::reject",
                    skill = %frontmatter.name,
                    reason = %reason,
                    "eval-runner rejected skill candidate"
                );
                LifecycleState::Rejected
            }
            EvalGateOutcome::Pass => match judge {
                None => LifecycleState::PendingReview,
                Some(j) => match adjudicate_with_timeout(j, learned_in, &frontmatter.name, &body)
                    .await
                {
                    JudgeOutcome::Allow => LifecycleState::PendingReview,
                    JudgeOutcome::Reject(reason) => {
                        tracing::info!(
                            target: "argo_skill::judge::reject",
                            skill = %frontmatter.name,
                            reason = %reason,
                            "judge rejected skill candidate"
                        );
                        LifecycleState::Rejected
                    }
                    JudgeOutcome::Unavailable(why) => {
                        tracing::warn!(
                            target: "argo_skill::judge::unavailable",
                            skill = %frontmatter.name,
                            why = %why,
                            "judge unavailable; failing OPEN to PendingReview"
                        );
                        LifecycleState::PendingReview
                    }
                },
            },
        };
        self.insert_row_with_state(
            frontmatter,
            body,
            Some(learned_in),
            Some(vertical_id),
            lifecycle,
        )
        .await
    }

    /// Transition a skill's lifecycle state. Caller is the
    /// human reviewer (CLI / gateway approve flow). Enforces
    /// the legal transition graph:
    ///
    /// - `Draft` → `PendingReview`
    /// - `PendingReview` → `Active` | `Rejected`
    /// - `Active` → `Archived`
    ///
    /// Rejects every other transition with `ArgoError::Schema`.
    pub async fn transition_lifecycle(
        &self,
        skill_id: SkillId,
        next: LifecycleState,
    ) -> ArgoResult<LifecycleState> {
        let row: Option<String> = sqlx::query_scalar(
            "SELECT lifecycle_state FROM skill WHERE id = ?",
        )
        .bind(skill_id.0.as_bytes().as_slice())
        .fetch_optional(self.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("transition lookup: {e}")))?;
        let current = row
            .as_deref()
            .and_then(LifecycleState::from_label)
            .ok_or_else(|| {
                ArgoError::Schema(format!("skill '{skill_id}' not found"))
            })?;
        if !is_legal_transition(current, next) {
            return Err(ArgoError::Schema(format!(
                "illegal lifecycle transition: {} → {}",
                current.label(),
                next.label()
            )));
        }
        sqlx::query("UPDATE skill SET lifecycle_state = ? WHERE id = ?")
            .bind(next.label())
            .bind(skill_id.0.as_bytes().as_slice())
            .execute(self.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("transition update: {e}")))?;
        Ok(next)
    }
}

impl SkillStore {
    /// List skills awaiting human review (#684 — driven by
    /// `argo skill review list`). Returns rows in `pending_review`
    /// AND `superseded_by IS NULL`. Empty when the queue is
    /// drained.
    pub async fn list_pending_review(&self) -> ArgoResult<Vec<Skill>> {
        let rows = sqlx::query_as::<_, crate::store::RawSkill>(
            "SELECT s.id, s.name, s.version, s.body, s.frontmatter, s.source, s.hash, \
                    s.learned_in, s.superseded_by, s.created_at, l.lock, s.vertical_id, \
                    s.lifecycle_state \
             FROM skill s LEFT JOIN skill_lock l ON l.skill_id = s.id \
             WHERE s.superseded_by IS NULL \
               AND s.lifecycle_state = 'pending_review' \
             ORDER BY s.created_at ASC",
        )
        .fetch_all(self.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("list_pending_review: {e}")))?;
        rows.into_iter().map(Skill::try_from).collect()
    }
}

fn is_legal_transition(from: LifecycleState, to: LifecycleState) -> bool {
    use LifecycleState::*;
    matches!(
        (from, to),
        (Draft, PendingReview)
            | (PendingReview, Active)
            | (PendingReview, Rejected)
            | (Active, Archived)
    )
}

#[cfg(test)]
#[path = "store_vertical_judge_tests.rs"]
mod judge_tests;

#[cfg(test)]
#[path = "store_vertical_eval_tests.rs"]
mod eval_tests;
