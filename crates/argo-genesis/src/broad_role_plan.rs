use super::{
    classify_llm_with_confidence, genesis_broad_role, should_use_broad_role, BroadRoleClient,
    BroadRoleDraft,
};
use crate::archetype::{Archetype, ClassificationResult, ProviderClient};
use crate::error::GenesisError;
use crate::genesis_judge::{
    judge_genesis_draft, GenesisJudgeClient, GenesisJudgeVerdict, JudgeInput,
};
use crate::pipeline::{assemble_plan, validate_persona, GenesisConfig, GenesisPlan};

/// Output of `plan_or_broad` — either an archetype-path plan or a
/// broader-generation draft.
#[derive(Debug, Clone)]
pub enum GenesisOutcome {
    /// Standard archetype Genesis plan.
    Plan(Box<GenesisPlan>),
    /// Broad-role draft for low-confidence non-archetype charters.
    Broad(Box<BroadRoleDraft>),
}

impl GenesisOutcome {
    /// `"broad"` for broad-role drafts, else the archetype slug.
    #[must_use]
    pub fn archetype_slug(&self) -> String {
        match self {
            Self::Plan(p) => p.archetype.slug().to_string(),
            Self::Broad(d) => d.archetype_slug().to_string(),
        }
    }

    fn judge_input(&self) -> JudgeInput<'_> {
        match self {
            Self::Plan(p) => JudgeInput {
                charter: &p.charter,
                persona_description: &p.persona,
                action_vocabulary: p
                    .manifest
                    .actions
                    .as_ref()
                    .map(|a| a.actions.keys().cloned().collect())
                    .unwrap_or_default(),
            },
            Self::Broad(d) => JudgeInput::from_broad(d),
        }
    }

    fn apply_disclaimer(&mut self, reason: &str) {
        match self {
            Self::Plan(p) => {
                p.charter = format!("[DISCLAIMER: {reason}] {}", p.charter);
            }
            Self::Broad(d) => d.prepend_persona_disclaimer(reason),
        }
    }
}

/// Confidence-aware Genesis entry.
#[allow(clippy::too_many_arguments)]
pub async fn plan_or_broad(
    charter: &str,
    persona: &str,
    config: &GenesisConfig,
    llm: Option<&dyn ProviderClient>,
    broad: Option<&dyn BroadRoleClient>,
    judge: Option<&dyn GenesisJudgeClient>,
    allow_broad_role: bool,
) -> Result<GenesisOutcome, GenesisError> {
    validate_persona(persona)?;
    let classification = resolve_classification(charter, config, llm).await;
    let mut outcome = if should_use_broad_role(&classification) {
        build_broad_outcome(charter, persona, broad, allow_broad_role).await?
    } else {
        let archetype = classification
            .archetype
            .ok_or_else(|| GenesisError::UnknownArchetype {
                available: Archetype::ALL
                    .iter()
                    .map(|a| a.label())
                    .collect::<Vec<_>>()
                    .join(", "),
            })?;
        GenesisOutcome::Plan(Box::new(assemble_plan(charter, persona, archetype)?))
    };
    if let Some(judge_client) = judge {
        apply_judge(&mut outcome, judge_client).await?;
    }
    Ok(outcome)
}

async fn resolve_classification(
    charter: &str,
    config: &GenesisConfig,
    llm: Option<&dyn ProviderClient>,
) -> ClassificationResult {
    match (config.use_llm_classifier, llm) {
        (true, Some(client)) => classify_llm_with_confidence(charter, client)
            .await
            .unwrap_or_else(|_| ClassificationResult::keyword_fallback(charter)),
        _ => ClassificationResult::keyword_fallback(charter),
    }
}

async fn build_broad_outcome(
    charter: &str,
    persona: &str,
    broad: Option<&dyn BroadRoleClient>,
    allow_broad_role: bool,
) -> Result<GenesisOutcome, GenesisError> {
    if !allow_broad_role {
        return Err(GenesisError::Validation {
            stage: "archetype.classify",
            detail: "no archetype confident match; use --allow-broad-role to enable \
                     LLM-driven broader generation"
                .into(),
        });
    }
    let client = broad.ok_or_else(|| GenesisError::Validation {
        stage: "broad_role.client",
        detail: "broad-role generation requires a BroadRoleClient".into(),
    })?;
    let draft = genesis_broad_role(charter, persona, client)
        .await
        .map_err(|e| GenesisError::Validation {
            stage: "broad_role.synthesize",
            detail: e.to_string(),
        })?;
    Ok(GenesisOutcome::Broad(Box::new(draft)))
}

async fn apply_judge(
    outcome: &mut GenesisOutcome,
    judge_client: &dyn GenesisJudgeClient,
) -> Result<(), GenesisError> {
    let input = outcome.judge_input();
    let verdict = judge_genesis_draft(&input, judge_client)
        .await
        .map_err(|e| GenesisError::Validation {
            stage: "safety_judge",
            detail: e.to_string(),
        })?;
    match verdict {
        GenesisJudgeVerdict::Block { reason } => Err(GenesisError::Validation {
            stage: "safety_judge",
            detail: format!("judge blocked draft: {reason}"),
        }),
        GenesisJudgeVerdict::AllowWithDisclaimer { reason } => {
            outcome.apply_disclaimer(&reason);
            Ok(())
        }
        GenesisJudgeVerdict::Allow => Ok(()),
    }
}
