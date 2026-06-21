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
use crate::spec::SuccessCriterion;
use argo_types::ArgoResult;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[path = "broad_role_parse.rs"]
mod broad_role_parse;
#[path = "broad_role_plan.rs"]
mod broad_role_plan;
#[path = "broad_role_richen.rs"]
pub(crate) mod broad_role_richen;

pub(crate) use broad_role_parse::{
    first_balanced_object, is_non_json_refusal, parse_broad_role_response, parse_confidence_response,
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
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
    /// Concrete day-to-day responsibilities the model produced. Populates
    /// [`crate::spec::CapabilitySpec::responsibilities`] so a from-scratch
    /// teammate's system prompt reads like a real hire instead of falling back
    /// to the literal `(derive your responsibilities from the charter above)`
    /// line. Additive + `#[serde(default)]`: drafts serialized before this
    /// field still deserialize (empty responsibilities).
    #[serde(default)]
    pub responsibilities: Vec<String>,
    /// A firing cadence as a validated 5/6-field cron expression (or `None`
    /// when the model emitted none / free text). Threaded into
    /// [`crate::spec::CapabilitySpec::cadence`] in the exact shape the hire→duty
    /// bridge's `normalize_stored_cron` accepts, so a richened broad role
    /// actually yields a *firing* duty rather than a skipped one.
    #[serde(default)]
    pub cadence: Option<String>,
    /// Typed, measurable success criteria (KPIs) the model produced — reuses
    /// the manifest KPI vocabulary (kind / target / comparator). Threaded into
    /// [`crate::spec::CapabilitySpec::success_criteria`] so the captured goals
    /// are enforceable, and their names ride to the hire draft's non-empty
    /// `kpis`. Additive + `#[serde(default)]`.
    #[serde(default)]
    pub success_criteria: Vec<SuccessCriterion>,
    /// Least-privilege access set — only the wired connectors the model named
    /// (anything outside the whitelist dropped). Threaded into
    /// [`crate::spec::CapabilitySpec::access_integrations`] as draft-only
    /// "to be connected" needs (never live bindings at birth). Additive +
    /// `#[serde(default)]`.
    #[serde(default)]
    pub access_integrations: Vec<String>,
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

/// Build the deterministic broad-role synthesis prompt. Asks the model to
/// FULLY author a from-scratch teammate in one JSON envelope: persona, action
/// vocabulary, concrete responsibilities, a firing cadence (cron), typed
/// success criteria (KPIs), least-privilege access, and an onboarding script.
///
/// The richer fields (`responsibilities` / `cadence_cron` / `success_criteria`
/// / `access_integrations`) are what turn a one-line off-taxonomy role into a
/// real teammate instead of one whose system prompt says
/// "(derive your responsibilities from the charter above)". The cadence is
/// requested AS A CRON so it threads through the hire→duty bridge unchanged;
/// access is held to the wired connector list so the model cannot invent
/// integrations. All are best-effort — the parser tolerates a model that omits
/// any of them — but the prompt asks for them so a capable model fills them in.
#[must_use]
pub fn build_broad_role_prompt(charter: &str, persona: &str) -> String {
    let connectors = broad_role_richen::WIRED_CONNECTORS.join(", ");
    format!(
        "You are designing an AI worker role for an operator. Read the charter \
         and return ONE JSON object with these fields (no extra prose, no markdown):\n\
         {{\n  \
         \"persona_description\": <1-2 sentence persona description, third person>,\n  \
         \"responsibilities\": [3 to 5 concrete day-to-day duties, plain language — NOT \"derive from the charter\"],\n  \
         \"action_vocabulary\": [3 to 8 action names as snake_case verbs],\n  \
         \"cadence_cron\": <a 5-field cron expression for WHEN this role runs, e.g. \"0 9 * * 1-5\" for weekday mornings; omit or use \"\" if purely event-driven>,\n  \
         \"success_metrics\": [3 to 5 measurable outcome statements],\n  \
         \"success_criteria\": [up to 5 typed KPIs, each {{\"name\": <metric>, \"kind\": one of percentage|count|ratio|duration_ms|score, \"target\": <number>, \"comparator\": one of gte|lte|eq}}],\n  \
         \"access_integrations\": [systems this role needs — ONLY from: {connectors}; omit any not on that list, never invent one],\n  \
         \"onboarding_script\": <one paragraph introducing the worker>\n\
         }}\n\n\
         Rules: be specific to THIS role; do not invent access outside the allowed connector list; \
         sensitive or irreversible actions are escalated to a human, never done autonomously.\n\n\
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
/// - [`GenesisError::EmptyCharter`] if the charter is whitespace-only.
/// - [`GenesisError::InvalidPersona`] if the persona is whitespace-only.
/// - [`GenesisError::RoleRefused`] (graceful + recoverable) when the model
///   returns prose with no JSON object at all — typically a refusal to
///   synthesize a destructive / unsafe role.
/// - [`GenesisError::Validation`] on a transport failure or a transient
///   malformed-JSON glitch (a `{...}` that won't parse / is missing fields).
pub async fn genesis_broad_role(
    charter: &str,
    persona: &str,
    client: &dyn BroadRoleClient,
) -> Result<BroadRoleDraft, GenesisError> {
    if charter.trim().is_empty() {
        return Err(GenesisError::EmptyCharter);
    }
    if persona.trim().is_empty() {
        return Err(GenesisError::InvalidPersona(persona.to_string()));
    }
    let prompt = build_broad_role_prompt(charter, persona);
    let raw = client
        .synthesize(&prompt)
        .await
        .map_err(|e| GenesisError::Validation {
            stage: "broad_role.transport",
            detail: e.to_string(),
        })?;
    // Refusal vs glitch: a response with NO JSON object at all is the model
    // returning prose (commonly declining a destructive role). Degrade to a
    // graceful, recoverable RoleRefused instead of a hard "no JSON object"
    // provider error that pre-empts the safety gate. A response that DID carry
    // a `{...}` but failed to parse stays a (retryable) Validation glitch.
    if is_non_json_refusal(&raw) {
        return Err(GenesisError::RoleRefused);
    }
    let parsed = parse_broad_role_response(&raw).map_err(|e| GenesisError::Validation {
        stage: "broad_role.synthesize",
        detail: e.to_string(),
    })?;
    Ok(BroadRoleDraft {
        charter: charter.to_string(),
        persona: persona.to_string(),
        persona_description: parsed.persona_description,
        action_vocabulary: parsed.action_vocabulary,
        success_metrics: parsed.success_metrics,
        onboarding_script: parsed.onboarding_script,
        responsibilities: parsed.responsibilities,
        cadence: parsed.cadence,
        success_criteria: parsed.success_criteria,
        access_integrations: parsed.access_integrations,
    })
}

#[cfg(test)]
#[path = "broad_role_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "broad_role_richen_integration_tests.rs"]
mod richen_integration_tests;
