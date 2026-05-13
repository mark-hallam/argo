//! Planner-bias wrapper (`mission-planner-bias-from-template`).
//!
//! Composes a [`crate::planner::Planner`] (typically the offline
//! [`crate::TemplatePlanner`]) with a [`LearnedPlanTemplateSource`]
//! so the runtime consults Dreams-curated learned templates before
//! falling through to cold-generate. The "skills get better over
//! time" promise depends on this wire — without it, every planner
//! call is cold and the [`crate::dreams_plan_templates`] pipeline
//! produces templates nobody reads.
//!
//! ## Provenance
//!
//! [`PlannerBias::decompose`] returns the matching
//! [`crate::DecomposeResult`] alongside an
//! [`argo_types::PlanProvenance`] so callers (today: the
//! `argo plan generate` handler) can stamp the audit envelope with
//! why this plan was emitted. Three outcomes:
//!
//! - **`FromLearnedTemplate`** — a promoted learned template matched
//!   the task signature and won the bias gate.
//! - **`BuiltIn`** — bias was disabled, no learned template matched,
//!   or the inner planner produced a shipped-default match.
//! - **`ColdGenerated`** — the inner planner cold-generated (e.g.
//!   [`crate::LlmPlanner`]); no template name was emitted.
//!
//! ## Trust grade
//!
//! v1 gates on `LearnedPlanTemplate.trust_grade = Promoted` only —
//! the source's [`LearnedPlanTemplateSource::list_promoted`] returns
//! exactly the promoted set, so the bias logic does not re-check the
//! grade. The brief's earlier `Bronze/Silver/Gold` vocabulary
//! referred to the older skill TrustGrade ladder and is misaligned
//! with the live `LearnedPlanTemplate` lifecycle (drift-fix
//! amendment 2026-05-13).
//!
//! ## No-DB invariant
//!
//! This module stays free of database deps — the trait is the seam.
//! The SQL-backed impl lives in `argo-cli` next to the
//! `learned_plan_template` reader so this crate's dependency
//! footprint stays tight (matches the pattern in
//! [`crate::dreams_plan_templates`]).

use crate::dreams_plan_templates::goal_tokens;
use crate::planner::{
    apply_context, DecomposeResult, Planner, PlannerContext, PlannerError,
};
use argo_types::PlanProvenance;
use async_trait::async_trait;
use std::collections::HashSet;

/// Stable, redact-safe signature for a planning task. Used to
/// match a task against a learned template's anchor `parent_goal`
/// without leaking operator-specific input values into the
/// provenance trail.
///
/// **Shape**: `vertical=<id>|tokens=<comma-separated-sorted-tokens>`.
/// Tokens are lower-case alphanumeric runs of the goal string, then
/// sorted and deduplicated so whitespace + reordering never change
/// the signature.
///
/// **Privacy**: only the goal-shape (token set, not values) enters
/// the signature; this matches the brief's "field-presence + field-
/// type, not field-values" guidance so operator-specific PII never
/// rides the audit envelope.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TaskSignature(String);

impl TaskSignature {
    /// Build a signature from a coarse goal + planner context.
    /// Falls back to a `vertical=` empty value when the caller
    /// has no vertical bound (legacy CLI path).
    #[must_use]
    pub fn from_task(goal: &str, ctx: &PlannerContext) -> Self {
        let mut tokens: Vec<String> = goal_tokens(goal)
            .into_iter()
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        tokens.sort();
        let vertical = ctx.vertical.as_deref().unwrap_or("");
        Self(format!("vertical={vertical}|tokens={}", tokens.join(",")))
    }

    /// Build a signature from a learned template's anchor goal +
    /// the calling task's vertical. Symmetric with
    /// [`Self::from_task`] so two goals that share token-shape under
    /// the same vertical produce equal signatures.
    #[must_use]
    pub fn from_template(anchor_goal: &str, vertical: Option<&str>) -> Self {
        let mut tokens: Vec<String> = goal_tokens(anchor_goal)
            .into_iter()
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        tokens.sort();
        let v = vertical.unwrap_or("");
        Self(format!("vertical={v}|tokens={}", tokens.join(",")))
    }

    /// Borrow the underlying string for audit serialization.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Candidate learned template the bias logic considers. Trimmed
/// down to the fields the runtime actually consults (id, name,
/// anchor goal, YAML body, recency); the full row payload stays
/// with the persistence layer.
///
/// This trait-facing struct intentionally mirrors a subset of
/// `argo_store::learned_plan_template::LearnedPlanTemplateRow` —
/// the SQL-backed adapter living in argo-cli maps the row to this
/// shape so argo-runtime keeps its zero-db dependency.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LearnedTemplateCandidate {
    /// Stable UUIDv7 id; mirrors the `learned_plan_template.id`
    /// column.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Anchor goal seeded the template — used to derive a
    /// signature that matches against the calling task's.
    pub anchor_goal: String,
    /// Workflow YAML body.
    pub yaml: String,
    /// Microseconds since epoch of the most-recent activity
    /// (promoted_at if set, else created_at). The bias logic
    /// compares against the now-clock to gate on
    /// `require_recent_within_days`.
    pub recency_micros: i64,
}

/// Source of Dreams-curated learned templates the bias logic
/// consults. Concrete impls:
///
/// - [`InMemoryTemplateSource`] — test fixture.
/// - `argo_cli::planner_bias_source::StoreTemplateSource` — SQL
///   adapter that reads `learned_plan_template` rows where
///   `trust_grade = 'promoted'`. Lives in argo-cli to preserve
///   argo-runtime's no-db invariant.
#[async_trait]
pub trait LearnedPlanTemplateSource: Send + Sync {
    /// Return every promoted template the source knows about.
    /// Bias-side filtering (signature match, recency, vertical
    /// scope) happens in [`PlannerBias::decompose`]; sources stay
    /// dumb so the policy lives in one place.
    ///
    /// # Errors
    /// Sources surface persistence / transport failures here. The
    /// bias path treats any error as "no learned template
    /// available" and falls through to cold-generate (see the
    /// `source_error_fallthrough` test).
    async fn list_promoted(&self) -> Result<Vec<LearnedTemplateCandidate>, PlannerError>;
}

/// In-memory test fixture for [`LearnedPlanTemplateSource`].
#[derive(Debug, Clone, Default)]
pub struct InMemoryTemplateSource {
    rows: Vec<LearnedTemplateCandidate>,
    fail: bool,
}

impl InMemoryTemplateSource {
    /// Build from an explicit candidate list.
    #[must_use]
    pub fn new(rows: Vec<LearnedTemplateCandidate>) -> Self {
        Self { rows, fail: false }
    }

    /// Produce a source that errors on every lookup — used to
    /// verify the bias path falls through to cold-generate when
    /// persistence is degraded.
    #[must_use]
    pub fn failing() -> Self {
        Self {
            rows: Vec::new(),
            fail: true,
        }
    }
}

#[async_trait]
impl LearnedPlanTemplateSource for InMemoryTemplateSource {
    async fn list_promoted(&self) -> Result<Vec<LearnedTemplateCandidate>, PlannerError> {
        if self.fail {
            return Err(PlannerError::Llm("simulated source failure".into()));
        }
        Ok(self.rows.clone())
    }
}

/// Operator knobs that gate the bias path. Mirrors
/// `argo_config::PlannerBiasConfig` (cross-crate copy kept here so
/// argo-runtime stays free of an argo-config dep — argo-config
/// already mirrors a handful of in-memory shapes for the same
/// reason, see [`crate::context_compress`]). The CLI maps the TOML
/// section to this struct at AgentDeps wiring time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BiasConfig {
    /// Master enable. `false` ⇒ always cold-generate.
    pub enabled: bool,
    /// Prefer per-vertical templates when multiple match. v1
    /// no-op (LearnedPlanTemplate has no vertical_id column yet);
    /// kept forward-compatible so when the column lands the
    /// behaviour flips on without a config break.
    pub prefer_per_vertical: bool,
    /// Recency window (days). `0` disables recency gating.
    pub require_recent_within_days: u32,
}

impl Default for BiasConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            prefer_per_vertical: true,
            require_recent_within_days: 30,
        }
    }
}

/// Bias-aware planner wrapper. Composes an inner [`Planner`] with a
/// [`LearnedPlanTemplateSource`]; consults the source first and
/// falls through to the inner planner on miss / error / disabled.
pub struct PlannerBias<P: Planner> {
    inner: P,
    source: std::sync::Arc<dyn LearnedPlanTemplateSource>,
    config: BiasConfig,
    now_micros: fn() -> i64,
}

impl<P: Planner> std::fmt::Debug for PlannerBias<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlannerBias")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl<P: Planner> PlannerBias<P> {
    /// Build a bias wrapper over `inner` + `source` with operator
    /// `config`. Uses the wall clock for recency.
    pub fn new(
        inner: P,
        source: std::sync::Arc<dyn LearnedPlanTemplateSource>,
        config: BiasConfig,
    ) -> Self {
        Self {
            inner,
            source,
            config,
            now_micros: || chrono::Utc::now().timestamp_micros(),
        }
    }

    /// Override the wall-clock used for recency gating. Used by
    /// the `recency_skip` tests so the assertions don't depend on
    /// real time.
    #[must_use]
    #[cfg(test)]
    pub fn with_now_micros(mut self, now: fn() -> i64) -> Self {
        self.now_micros = now;
        self
    }

    /// Decompose `goal` under `ctx`. Returns the result + a stamp
    /// describing how it was emitted so the caller can write the
    /// provenance into its audit envelope.
    ///
    /// Fall-through order:
    /// 1. `config.enabled == false` ⇒ inner.decompose + provenance
    ///    derived from inner's `template_name`.
    /// 2. Source returns Err ⇒ inner.decompose, provenance derived.
    /// 3. No candidate matches signature ⇒ inner.decompose, derived.
    /// 4. Candidate matches but YAML body fails to parse ⇒
    ///    inner.decompose, derived. The bias never crashes the
    ///    worker — broken templates demote to cold-generate.
    /// 5. Candidate matches + parses ⇒ DecomposeResult + provenance
    ///    `FromLearnedTemplate { template_id, signature }`.
    ///
    /// # Errors
    /// Propagates inner planner errors verbatim when bias falls
    /// through. Bias-internal errors never surface — they degrade
    /// to cold-generate.
    pub async fn decompose(
        &self,
        goal: &str,
        ctx: &PlannerContext,
    ) -> Result<(DecomposeResult, PlanProvenance), PlannerError> {
        if !self.config.enabled {
            return self.cold(goal, ctx).await;
        }
        let candidates = match self.source.list_promoted().await {
            Ok(rows) => rows,
            Err(_) => return self.cold(goal, ctx).await,
        };
        let task_sig = TaskSignature::from_task(goal, ctx);
        let now = (self.now_micros)();
        let window_micros = recency_window_micros(self.config.require_recent_within_days);
        let mut matches: Vec<LearnedTemplateCandidate> = candidates
            .into_iter()
            .filter(|c| {
                let cand_sig =
                    TaskSignature::from_template(&c.anchor_goal, ctx.vertical.as_deref());
                cand_sig == task_sig
            })
            .filter(|c| match window_micros {
                Some(w) => now.saturating_sub(c.recency_micros) <= w,
                None => true,
            })
            .collect();
        if matches.is_empty() {
            return self.cold(goal, ctx).await;
        }
        // v1 selection — newest-first (highest recency_micros). The
        // `prefer_per_vertical` knob is a forward-compat no-op until
        // LearnedPlanTemplate carries vertical_id.
        matches.sort_by_key(|c| std::cmp::Reverse(c.recency_micros));
        let pick = &matches[0];
        match parse_template_body(&pick.yaml, ctx, &pick.name) {
            Ok(result) => Ok((
                result,
                PlanProvenance::FromLearnedTemplate {
                    template_id: pick.id.clone(),
                    signature: task_sig.0,
                },
            )),
            Err(_) => self.cold(goal, ctx).await,
        }
    }

    async fn cold(
        &self,
        goal: &str,
        ctx: &PlannerContext,
    ) -> Result<(DecomposeResult, PlanProvenance), PlannerError> {
        let result = self.inner.decompose(goal, ctx).await?;
        let provenance = derive_inner_provenance(&result);
        Ok((result, provenance))
    }
}

fn derive_inner_provenance(result: &DecomposeResult) -> PlanProvenance {
    match (result.planner_kind.as_str(), &result.template_name) {
        ("template", Some(name)) => PlanProvenance::BuiltIn {
            template_name: name.clone(),
        },
        _ => PlanProvenance::ColdGenerated,
    }
}

fn parse_template_body(
    yaml: &str,
    ctx: &PlannerContext,
    template_name: &str,
) -> Result<DecomposeResult, PlannerError> {
    let workflow = argo_workflow::parse_yaml_str(yaml).map_err(|e| match e {
        argo_workflow::YamlError::Invalid(v) => {
            PlannerError::YamlParse(format!("validate: {v}"))
        }
        other => PlannerError::YamlParse(format!("{other}")),
    })?;
    let workflow = apply_context(workflow, ctx);
    Ok(DecomposeResult {
        workflow,
        reasoning: format!("Biased toward learned template '{template_name}'"),
        planner_kind: "template_bias".into(),
        template_name: Some(template_name.to_string()),
    })
}

fn recency_window_micros(days: u32) -> Option<i64> {
    if days == 0 {
        return None;
    }
    let secs = i64::from(days).saturating_mul(86_400);
    Some(secs.saturating_mul(1_000_000))
}

#[cfg(test)]
#[path = "planner_bias_tests.rs"]
mod tests;
