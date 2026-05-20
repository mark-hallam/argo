//! Broader-generation path for charters whose archetype classifier
//! confidence is below [`BROAD_ROLE_CONFIDENCE_THRESHOLD`].
//!
//! When the LLM classifier signals it can't confidently pick one of
//! the 12 built-in archetypes, the pipeline switches to this
//! synthesizer instead of forcing a closest-match fall-through. The
//! output is a [`BroadRoleDraft`] tagged with the [`BROAD_SENTINEL`]
//! slug (`"broad"`) so downstream pattern-matchers can distinguish
//! broad-role drafts from the 12 archetypes.
//!
//! Mission `mission-genesis-non-archetype-roles`. The 0.6 confidence
//! threshold is a v1 tunable knob.
//!
//! ```no_run
//! # async fn demo(client: &dyn argo_genesis::BroadRoleClient) -> argo_types::ArgoResult<()> {
//! let draft = argo_genesis::genesis_broad_role(
//!     "be my chief vibes officer, keep the team energy positive",
//!     "vibe",
//!     client,
//! ).await?;
//! assert_eq!(draft.archetype_slug(), "broad");
//! # Ok(()) }
//! ```

use crate::archetype::{Archetype, ClassificationResult, ProviderClient};
use crate::error::GenesisError;
use argo_types::{ArgoError, ArgoResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[path = "broad_role_parse.rs"]
mod broad_role_parse;
#[path = "broad_role_plan.rs"]
mod broad_role_plan;

pub(crate) use broad_role_parse::{
    first_balanced_object, parse_broad_role_response, parse_confidence_response,
};
pub use broad_role_plan::{plan_or_broad, GenesisOutcome};

/// Sentinel slug stamped on every broad-role draft. Distinct from
/// every `Archetype::slug()` so downstream `match` arms can treat
/// `"broad"` as a separate branch without clashing with the 12
/// archetype slugs.
pub const BROAD_SENTINEL: &str = "broad";

/// Classification confidence below which the Genesis pipeline routes
/// the charter to broader generation (instead of forcing the closest
/// archetype match). 0.6 is the v1 starting point.
pub const BROAD_ROLE_CONFIDENCE_THRESHOLD: f32 = 0.6;

/// Output of the broader-generation path. Carries the LLM-synthesised
/// persona description, action vocabulary, success metrics, and
/// onboarding script. The `archetype_slug()` accessor always returns
/// [`BROAD_SENTINEL`] so callers can treat this as a "13th non-
/// archetype slot" without adding it to the [`crate::Archetype`] enum.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BroadRoleDraft {
    /// The original operator charter, byte-identical.
    pub charter: String,
    /// Persona handle (the `--persona` arg in CLI terms).
    pub persona: String,
    /// Model-generated 1-2 sentence persona description. The safety
    /// judge's `AllowWithDisclaimer` verdict prepends to this field.
    pub persona_description: String,
    /// 3-8 action names (snake_case verbs) the model produced.
    pub action_vocabulary: Vec<String>,
    /// 3-5 measurable success metrics the model produced.
    pub success_metrics: Vec<String>,
    /// Single-paragraph onboarding script the model produced.
    pub onboarding_script: String,
}

impl BroadRoleDraft {
    /// Always returns [`BROAD_SENTINEL`]. The accessor exists so
    /// downstream code can pattern-match on a stable string without
    /// reaching into the `Archetype` enum.
    #[must_use]
    pub fn archetype_slug(&self) -> &'static str {
        BROAD_SENTINEL
    }

    /// Prepend a safety-judge disclaimer to [`Self::persona_description`].
    /// Used by the pipeline when the judge returns `AllowWithDisclaimer`.
    pub fn prepend_persona_disclaimer(&mut self, reason: &str) {
        let trimmed = reason.trim();
        if trimmed.is_empty() {
            return;
        }
        self.persona_description = format!("[DISCLAIMER: {trimmed}] {}", self.persona_description);
    }
}

/// Narrow provider abstraction used by [`genesis_broad_role`]. Kept
/// in this crate so `argo-genesis` doesn't have to depend on the
/// larger `argo-provider` crate just to ask one prompt. Implementations
/// MUST set temperature low so the same charter renders consistently.
#[async_trait]
pub trait BroadRoleClient: Send + Sync {
    /// Send a one-shot broad-role synthesis prompt and return the raw
    /// model response text. Implementations should aim for JSON-only
    /// output per the prompt contract, but [`genesis_broad_role`] is
    /// tolerant of preceding prose.
    async fn synthesize(&self, prompt: &str) -> ArgoResult<String>;
}

/// True when the [`ClassificationResult`] is low-confidence or signals
/// no archetype match — i.e., the pipeline should route to
/// [`genesis_broad_role`] (when `--allow-broad-role` is set) instead of
/// the archetype path.
#[must_use]
pub fn should_use_broad_role(classification: &ClassificationResult) -> bool {
    classification.archetype.is_none()
        || classification.confidence < BROAD_ROLE_CONFIDENCE_THRESHOLD
}

/// Build the deterministic confidence-classifier prompt. Asks the
/// LLM for `<slug> <confidence>` in two tokens — simpler to parse
/// than JSON and small enough to fit any token cap. Sits in
/// `broad_role.rs` to keep `archetype.rs` under the 500-LOC ceiling.
#[must_use]
pub fn build_confidence_classifier_prompt(charter: &str) -> String {
    let slugs = Archetype::ALL
        .iter()
        .map(|a| a.slug())
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "You are an archetype classifier for a worker-agent charter. \
         Respond with EXACTLY two whitespace-separated tokens and nothing else: \
         <slug> <confidence>. The slug is one of [{slugs}], or the literal \
         \"none\" if no archetype is a good fit. The confidence is a decimal \
         in [0.0, 1.0].\n\nCharter: {charter}",
    )
}

/// LLM-driven classifier that returns a [`ClassificationResult`]
/// instead of unwrapping to a single `Archetype`. Sibling of
/// [`crate::classify_llm`]; the two coexist so the legacy
/// `Result<Archetype, _>` callers don't have to adapt.
///
/// On any provider error, falls back to the deterministic keyword
/// classifier with `confidence = 1.0`. If the model returns `"none"`
/// or an unknown slug, returns `archetype: None` (confidence whatever
/// the model reported) so callers can route to broader generation.
///
/// # Errors
///
/// Only when the charter is empty / whitespace-only.
pub async fn classify_llm_with_confidence(
    charter: &str,
    llm: &dyn ProviderClient,
) -> ArgoResult<ClassificationResult> {
    if charter.trim().is_empty() {
        return Err(GenesisError::EmptyCharter.into());
    }
    let prompt = build_confidence_classifier_prompt(charter);
    match llm.classify(&prompt).await {
        Ok(raw) => Ok(parse_confidence_response(&raw)),
        Err(_) => Ok(ClassificationResult::keyword_fallback(charter)),
    }
}

/// Build the deterministic broad-role synthesis prompt. Asks the
/// model for a persona description, 3-8 action names, 3-5 success
/// metrics, and a 1-paragraph onboarding script — all in one JSON
/// envelope to keep parsing simple.
#[must_use]
pub fn build_broad_role_prompt(charter: &str, persona: &str) -> String {
    format!(
        "You are designing an AI worker role for an operator. Read the charter \
         and return ONE JSON object with these fields (no extra prose, no markdown):\n\
         {{\n  \
         \"persona_description\": <1-2 sentence persona description, third person>,\n  \
         \"action_vocabulary\": [3 to 8 action names as snake_case verbs],\n  \
         \"success_metrics\": [3 to 5 measurable outcome statements],\n  \
         \"onboarding_script\": <one paragraph introducing the worker>\n\
         }}\n\n\
         Persona handle: {persona}\n\
         Charter: {charter}",
    )
}

/// Synthesise a [`BroadRoleDraft`] for a charter that didn't match a
/// confident archetype. The model returns a JSON envelope; we parse
/// it leniently (tolerant of prose around the object). Empty charters
/// error early without calling the provider.
///
/// # Errors
///
/// - `EmptyCharter` if the charter is whitespace-only.
/// - `ArgoError::Provider` on transport failure or unparseable response.
pub async fn genesis_broad_role(
    charter: &str,
    persona: &str,
    client: &dyn BroadRoleClient,
) -> ArgoResult<BroadRoleDraft> {
    if charter.trim().is_empty() {
        return Err(ArgoError::from(GenesisError::EmptyCharter));
    }
    if persona.trim().is_empty() {
        return Err(ArgoError::from(GenesisError::InvalidPersona(
            persona.to_string(),
        )));
    }
    let prompt = build_broad_role_prompt(charter, persona);
    let raw = client.synthesize(&prompt).await?;
    let parsed = parse_broad_role_response(&raw)?;
    Ok(BroadRoleDraft {
        charter: charter.to_string(),
        persona: persona.to_string(),
        persona_description: parsed.persona_description,
        action_vocabulary: parsed.action_vocabulary,
        success_metrics: parsed.success_metrics,
        onboarding_script: parsed.onboarding_script,
    })
}

#[cfg(test)]
#[path = "broad_role_tests.rs"]
mod tests;
