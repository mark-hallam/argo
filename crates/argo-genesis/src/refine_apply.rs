//! Request-prompt builder + response-application helpers for the
//! refinement pass.
//!
//! Split out of `refine.rs` to keep that file under the 500-LOC
//! ceiling. The orchestration entry points
//! ([`super::refine_plan`] / [`super::refine_plan_with_cap`])
//! call [`build_prompt`] before the LLM hop and
//! [`apply_refinement_with_cap`] after the response parses.

use super::{ProposedAction, RefineConfig, RefinementResponse};
use crate::error::GenesisError;
use crate::pipeline::GenesisPlan;
use crate::safety_judge;
use argo_vertical::{ActionDeclaration, ActionRiskLevel, ActionsSection, ExecutorDeclaration};

/// Apply the validated refinement using the default vocabulary
/// cap. Retained for the existing public API; new callers should
/// prefer [`apply_refinement_with_cap`].
pub fn apply_refinement(
    plan: GenesisPlan,
    response: RefinementResponse,
) -> Result<GenesisPlan, GenesisError> {
    apply_refinement_with_cap(plan, response, RefineConfig::DEFAULT_MAX_VOCAB_SIZE)
        .map(|(plan, _)| plan)
}

/// Cap-aware refinement application. Merges new actions (under
/// `max_vocab_size`), force-promotes Medium/High-risk newly-added
/// actions to `requires_approval = true`, then runs the safety
/// judge. Returns the refined plan alongside the names of
/// action_types newly added.
pub fn apply_refinement_with_cap(
    mut plan: GenesisPlan,
    response: RefinementResponse,
    max_vocab_size: usize,
) -> Result<(GenesisPlan, Vec<String>), GenesisError> {
    let RefinementResponse {
        additional_actions,
        system_prompt_addendum,
        ..
    } = response;
    let mut evolved = Vec::new();
    if let Some(actions) = plan.manifest.actions.as_mut() {
        evolved = merge_proposed_actions(actions, additional_actions, max_vocab_size);
        // Defense-in-depth: a malicious-or-confused model may
        // propose `delete_all_data` with requires_approval=false.
        // Force the flag back on for Medium/High-risk
        // newly-added actions BEFORE the safety judge runs so the
        // override survives auto-fix.
        force_requires_approval_for_risky(actions, &evolved);
        safety_judge::auto_fix(actions);
        safety_judge::judge(actions)?;
    }
    if !system_prompt_addendum.is_empty() {
        plan.manifest.vertical.description = format!(
            "{}\n\n## Refined addendum\n{}",
            plan.manifest.vertical.description, system_prompt_addendum
        );
    }
    Ok((plan, evolved))
}

fn merge_proposed_actions(
    actions: &mut ActionsSection,
    proposed: Vec<ProposedAction>,
    max_vocab_size: usize,
) -> Vec<String> {
    let mut evolved = Vec::new();
    for p in proposed {
        if actions.actions.contains_key(&p.action_type) {
            continue;
        }
        if actions.actions.len() >= max_vocab_size {
            tracing::debug!(
                target: "argo_genesis::refine",
                action_type = %p.action_type,
                cap = max_vocab_size,
                "vocabulary at cap; dropping proposed action_type",
            );
            continue;
        }
        let ProposedAction {
            action_type,
            risk,
            requires_approval,
            description,
        } = p;
        actions.actions.insert(
            action_type.clone(),
            ActionDeclaration {
                risk,
                requires_approval,
                description,
                executor: ExecutorDeclaration::default(),
                ..ActionDeclaration::default()
            },
        );
        evolved.push(action_type);
    }
    evolved
}

fn force_requires_approval_for_risky(actions: &mut ActionsSection, evolved: &[String]) {
    for name in evolved {
        if let Some(decl) = actions.actions.get_mut(name) {
            if matches!(decl.risk, ActionRiskLevel::Medium | ActionRiskLevel::High)
                && !decl.requires_approval
            {
                decl.requires_approval = true;
            }
        }
    }
}

pub(super) fn build_prompt(plan: &GenesisPlan, max_new_actions: usize) -> String {
    let archetype_label = plan.archetype.label();
    let action_names: Vec<&str> = plan
        .manifest
        .actions
        .as_ref()
        .map(|a| a.actions.keys().map(String::as_str).collect())
        .unwrap_or_default();
    format!(
        "You're refining a draft Skycot vertical generated from a charter.\n\n\
         Charter: {charter}\n\n\
         Deterministic archetype: {archetype_label}\n\
         Deterministic actions: [{actions}]\n\n\
         Respond with a JSON object exactly matching this schema:\n\
         {{\n\
           \"archetype_confirmed\": <bool>,\n\
           \"archetype_suggested\": <string or null>,\n\
           \"additional_actions\": [\n\
             {{ \"action_type\": <snake_case>, \"risk\": \"low\"|\"medium\"|\"high\",\n\
                \"requires_approval\": <bool>, \"description\": <string> }}\n\
           ] (propose up to {max_new_actions} new actions),\n\
           \"system_prompt_addendum\": <string up to 4096 chars>\n\
         }}\n\n\
         Do not propose actions that already exist. Do not propose actions \
         that touch external systems autonomously — sensitive verbs (send, \
         post, pay, publish, delete, share, message) MUST be \
         requires_approval=true.",
        charter = plan.charter,
        archetype_label = archetype_label,
        actions = action_names.join(", "),
        max_new_actions = max_new_actions,
    )
}
