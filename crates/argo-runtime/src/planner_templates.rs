//! Offline keyword-matching planner: [`PlanTemplate`] +
//! [`TemplatePlanner`] plus the shipped default templates. Sibling
//! of [`crate::planner`], split out to keep planner.rs under the
//! 500-LOC ceiling. Public API is re-exported via the parent
//! `planner` module so external imports
//! (`use argo_runtime::TemplatePlanner`) continue to resolve.

use crate::planner::{apply_context, DecomposeResult, Planner, PlannerContext, PlannerError};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// One template the offline [`TemplatePlanner`] can match.
/// Matching is keyword-based and case-insensitive.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanTemplate {
    /// Stable name, used for ranking + reasoning.
    pub name: String,
    /// One-line description shown to the operator.
    pub description: String,
    /// Lower-case keywords any of which trigger the template.
    pub keywords: Vec<String>,
    /// Workflow YAML body. Parsed lazily on match.
    pub yaml: String,
}

/// Offline planner: matches goal keywords to a saved template.
/// Ships with three sensible defaults (research, draft, approval)
/// so `argo plan generate` always has something to emit.
#[derive(Debug, Clone)]
pub struct TemplatePlanner {
    templates: Vec<PlanTemplate>,
}

impl TemplatePlanner {
    /// Build from an explicit template list. Tests use this to
    /// inject a deterministic set; the CLI uses
    /// [`Self::shipped_defaults`].
    #[must_use]
    pub fn new(templates: Vec<PlanTemplate>) -> Self {
        Self { templates }
    }

    /// Prepend operator-promoted Dreams-distilled templates ahead
    /// of the existing in-memory list. Templates are matched in
    /// insertion order, so callers should supply
    /// `learned_plan_template` rows from
    /// `argo_store::learned_plan_template::list_promoted` first
    /// and let the shipped defaults remain as the fallback. This
    /// is the seam by which Dreams output reaches the planner —
    /// the runtime stays free of database deps; argo-cli does
    /// the SQL read and hands the converted templates here.
    #[must_use]
    pub fn with_learned_templates(mut self, mut learned: Vec<PlanTemplate>) -> Self {
        learned.append(&mut self.templates);
        self.templates = learned;
        self
    }

    /// Three shipped templates that cover most "first plan"
    /// scenarios. Operators can extend by passing their own list
    /// to [`Self::new`] — disk-loaded templates are a follow-up.
    #[must_use]
    pub fn shipped_defaults() -> Self {
        let templates = vec![
            PlanTemplate {
                name: "research-then-summarise".into(),
                description: "Two-step research + summary template.".into(),
                keywords: vec!["research".into(), "investigate".into(), "summary".into(), "summarize".into()],
                yaml: SHIPPED_RESEARCH_YAML.into(),
            },
            PlanTemplate {
                name: "marketing-launch".into(),
                description: "Promote / launch / announce a piece of work.".into(),
                keywords: vec![
                    "promote".into(),
                    "launch".into(),
                    "marketing".into(),
                    "announce".into(),
                    "publish".into(),
                ],
                yaml: SHIPPED_MARKETING_YAML.into(),
            },
            PlanTemplate {
                name: "approval-gate".into(),
                description: "Single-step plan that always pauses for operator approval.".into(),
                keywords: vec!["approve".into(), "review".into(), "gate".into()],
                yaml: SHIPPED_APPROVAL_YAML.into(),
            },
        ];
        Self::new(templates)
    }

    fn template_names(&self) -> Vec<String> {
        self.templates.iter().map(|t| t.name.clone()).collect()
    }

    fn find_match(&self, goal: &str) -> Option<&PlanTemplate> {
        let lower = goal.to_lowercase();
        self.templates
            .iter()
            .find(|t| t.keywords.iter().any(|kw| lower.contains(kw)))
    }
}

#[async_trait]
impl Planner for TemplatePlanner {
    async fn decompose(
        &self,
        goal: &str,
        ctx: &PlannerContext,
    ) -> Result<DecomposeResult, PlannerError> {
        let template = self.find_match(goal).ok_or_else(|| PlannerError::NoTemplateMatch {
            goal: goal.to_string(),
            available: self.template_names(),
        })?;
        let workflow =
            argo_workflow::parse_yaml_str(&template.yaml).map_err(|e| match e {
                argo_workflow::YamlError::Invalid(v) => {
                    PlannerError::YamlParse(format!("validate: {v}"))
                }
                other => PlannerError::YamlParse(format!("{other}")),
            })?;
        let workflow = apply_context(workflow, ctx);
        Ok(DecomposeResult {
            workflow,
            reasoning: format!(
                "Matched template '{}': {}",
                template.name, template.description
            ),
            planner_kind: "template".into(),
            template_name: Some(template.name.clone()),
        })
    }
}

// Shipped templates use depends_on edges (no `${tasks.X.out.Y}`
// expression refs) so the bundled MockRunner can execute them
// without bespoke output schemas. Real-deployment workflows can
// override these via `argo plan generate --output ...` and edit
// the YAML to add expression-based input wiring.
const SHIPPED_RESEARCH_YAML: &str = r#"
id: shipped-research
description: Research a topic, then summarise.
tasks:
  - id: gather
    worker: ""
  - id: summarise
    worker: ""
    depends_on:
      - gather
"#;

const SHIPPED_MARKETING_YAML: &str = r#"
id: shipped-marketing-launch
description: Draft, review, then publish a marketing message.
tasks:
  - id: draft
    worker: ""
  - id: review
    worker: ""
    depends_on:
      - draft
  - id: publish
    worker: ""
    depends_on:
      - review
"#;

const SHIPPED_APPROVAL_YAML: &str = r#"
id: shipped-approval-gate
description: Single-step plan that pauses for operator approval.
tasks:
  - id: gate
    worker: ""
"#;
