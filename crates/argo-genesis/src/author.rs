//! LLM-authoring pass — the "bottle what a frontier model does" step.
//!
//! Genesis classifies a role into an archetype then TEMPLATE-FILLS the
//! teammate's persona + responsibilities (`materialize_writer::*`), which reads
//! thin/generic. This module adds a pass that asks a frontier model to richly
//! AUTHOR the persona description, the concrete responsibilities, and the
//! onboarding script from the operator's role text — so a generated teammate's
//! system prompt + persona read like a hand-authored hire. These three fields
//! already exist on [`CapabilitySpec`] and already flow into
//! `prompts/system.md` + `prompts/persona.md` via the unchanged
//! `materialize_spec` → `write_spec_artefacts` path, so no schema change is
//! needed for the quality win. (Structured great-at / escalate-when /
//! signature-behaviors for the worker-detail card are a separate Phase-2 slice.)
//!
//! Pure half — NO transport edge (mirrors `rubric.rs` / `genesis_judge.rs`).
//! The gateway supplies the provider via [`AuthorClient`]. Degrade-fail-safe:
//! [`author_teammate`] returns `None` on ANY transport or parse failure, and
//! the caller keeps the templated spec untouched — never a regression. The
//! deterministic safety floor (capability risk/approval) is NEVER read or
//! relaxed here.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use argo_types::ArgoResult;

use crate::spec::CapabilitySpec;

/// Narrow provider trait — same shape as [`crate::rubric::RubricClient`]. The
/// gateway wraps the shared dispatcher; tests inject a deterministic mock.
/// Implementations MUST use a low temperature so the same role authors stably.
#[async_trait]
pub trait AuthorClient: Send + Sync {
    /// Send the authoring prompt; return the raw model response (JSON expected).
    async fn author(&self, prompt: &str) -> ArgoResult<String>;
}

/// The richly-authored teammate fields. Each maps onto an EXISTING
/// [`CapabilitySpec`] field, so the authored output flows through the unchanged
/// `materialize_spec` → `write_spec_artefacts` path with no schema change.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthoredTeammate {
    /// Rich 1-2 sentence, third-person persona description.
    pub persona_description: String,
    /// 3-5 concrete responsibilities in plain language.
    pub responsibilities: Vec<String>,
    /// A warm, specific single-paragraph onboarding script.
    pub onboarding_script: String,
}

/// Build the authoring prompt. Reads ONLY safe spec fields (charter, persona,
/// declared capability action-names) — never the risk/approval floor. One
/// inline gold example sets the quality bar: specific, truth-bounded,
/// approval-aware — not generic filler.
#[must_use]
pub fn build_author_prompt(spec: &CapabilitySpec) -> String {
    let actions = if spec.capabilities.is_empty() {
        "(none declared yet)".to_string()
    } else {
        spec.capabilities
            .iter()
            .map(|c| c.action_name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    };
    format!(
        "You are an expert at designing AI teammates for SKYCOT — an enterprise \
         platform where companies HIRE persistent, accountable AI workers. Given an \
         operator's plain-English role description, author a teammate that reads like \
         a real, well-briefed hire — NOT a generic bot.\n\n\
         Rules:\n\
         - Be SPECIFIC to THIS role. No filler like 'reliable', 'helpful', 'efficient'.\n\
         - Truth-bounded: the teammate is clear about what it can and cannot do.\n\
         - Approval-aware: sensitive or irreversible actions are escalated to a human, never done autonomously.\n\
         - Third person, warm, concrete.\n\n\
         GOLD EXAMPLE (role: 'a marketplace lister who lists and re-prices items for sale'):\n\
         {{\"persona_description\": \"A meticulous marketplace lister who writes tight, \
         buyer-ready listings and keeps prices sharp against the live market. Patient with \
         detail, fast on buyer questions, and never posts without a clean source photo.\", \
         \"responsibilities\": [\"Draft and publish listings with accurate specs and photos\", \
         \"Re-price stale listings against current competitor data\", \"Answer buyer questions \
         on active listings within scope\", \"Flag any item dropping below the seller's floor price\"], \
         \"onboarding_script\": \"Welcome aboard. I'll list and re-price your inventory, keep buyers \
         answered, and surface anything that needs your call — like a price below your floor or an \
         offer outside my scope. I draft; you approve anything irreversible.\"}}\n\n\
         Now author the teammate for THIS role.\n\
         Role (operator's words): {charter}\n\
         Persona handle: {persona}\n\
         Declared actions: {actions}\n\n\
         Reply with EXACTLY ONE JSON object and nothing else:\n\
         {{\"persona_description\": \"<1-2 sentences>\", \"responsibilities\": [\"<3-5 items>\"], \
         \"onboarding_script\": \"<one paragraph>\"}}",
        charter = spec.charter,
        persona = spec.persona,
    )
}

/// Run the authoring pass. Degrade-fail-safe: returns `None` on transport error
/// OR any response the parser cannot turn into the three fields, so the caller
/// keeps the templated spec untouched (never a regression).
pub async fn author_teammate(
    spec: &CapabilitySpec,
    client: &dyn AuthorClient,
) -> Option<AuthoredTeammate> {
    let prompt = build_author_prompt(spec);
    let raw = client.author(&prompt).await.ok()?;
    parse_authored(&raw)
}

/// Tolerant parse — reuses the shared balanced-object scanner; returns `None`
/// (→ fall back to templates) on no JSON / parse error / empty persona.
fn parse_authored(raw: &str) -> Option<AuthoredTeammate> {
    let json_text = crate::broad_role::first_balanced_object(raw)?;
    let authored: AuthoredTeammate = serde_json::from_str(json_text).ok()?;
    if authored.persona_description.trim().is_empty() {
        return None;
    }
    Some(authored)
}

/// Fold authored fields into the spec, WITHOUT touching the charter, persona,
/// or any capability's risk/approval (the deterministic safety floor). An empty
/// authored field leaves the templated value in place.
#[must_use]
pub fn apply_authored(mut spec: CapabilitySpec, authored: AuthoredTeammate) -> CapabilitySpec {
    if !authored.persona_description.trim().is_empty() {
        spec.persona_description = authored.persona_description;
    }
    if !authored.responsibilities.is_empty() {
        spec.responsibilities = authored.responsibilities;
    }
    if !authored.onboarding_script.trim().is_empty() {
        spec.onboarding_script = authored.onboarding_script;
    }
    spec
}

#[cfg(test)]
#[path = "author_tests.rs"]
mod tests;
