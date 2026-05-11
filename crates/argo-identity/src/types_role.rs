//! Persona and Role primitives split from `types.rs` (file-LOC ceiling).
//!
//! Holds the delegation-contract types — how a worker presents to humans
//! (`VoiceProfile`, `Persona`) and what they're delegated to do
//! (`Duty`, `Role`). Re-exported via `types::` so call sites are unchanged.
//!
//! ## `VoiceProfile::compose_preamble` ordering
//!
//! Structured-fields-first, freeform-preamble-last. The freeform
//! `preamble` is the last instruction the model reads so operator
//! overrides clearly win over structured defaults (per the
//! `mission-voice-profile-richer-fields` brief).

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// High-level voice register applied as a tone instruction.
///
/// Default = [`Tone::Neutral`], which produces no prepended tone
/// instruction — legacy preambles compose byte-identical when every
/// structured field is at its default.
///
/// ```
/// use argo_identity::Tone;
/// assert_eq!(Tone::default(), Tone::Neutral);
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tone {
    /// No tone bias — preamble composes verbatim.
    #[default]
    Neutral,
    /// Warm, empathetic register.
    Warm,
    /// Direct, concise register.
    Direct,
    /// Playful, lighter register.
    Playful,
    /// Formal, measured register.
    Formal,
}

impl Tone {
    fn instruction(self) -> Option<&'static str> {
        match self {
            Self::Neutral => None,
            Self::Warm => Some("Speak with a warm, empathetic tone."),
            Self::Direct => Some("Speak directly and concisely."),
            Self::Playful => Some("Speak with a light, playful tone."),
            Self::Formal => Some("Speak with a formal, measured tone."),
        }
    }
}

/// Formality register on a 5-step ladder.
///
/// Default = [`Formality::Conversational`], which produces no
/// prepended formality instruction.
///
/// ```
/// use argo_identity::Formality;
/// assert_eq!(Formality::default(), Formality::Conversational);
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Formality {
    /// Casual, peer-to-peer register.
    Casual,
    /// Default conversational register — no prepended instruction.
    #[default]
    Conversational,
    /// Professional workplace register.
    Professional,
    /// Executive briefing register.
    Executive,
    /// Legal / contractual register.
    Legal,
}

impl Formality {
    fn instruction(self) -> Option<&'static str> {
        match self {
            Self::Conversational => None,
            Self::Casual => Some("Use a casual, peer-to-peer register."),
            Self::Professional => Some("Use a professional workplace register."),
            Self::Executive => Some("Use a concise executive-briefing register."),
            Self::Legal => Some("Use a precise, legal/contractual register."),
        }
    }
}

/// Public-facing voice profile for a persona.
///
/// The freeform `preamble` is preserved verbatim for back-compat;
/// structured fields layer additional instructions on top via
/// [`VoiceProfile::compose_preamble`]. Personas serialised before
/// this struct gained the structured fields deserialise unchanged
/// (every new field is `#[serde(default)]`).
///
/// ```
/// use argo_identity::VoiceProfile;
/// let voice = VoiceProfile { preamble: "Be concise.".into(), ..Default::default() };
/// assert_eq!(voice.compose_preamble(), "Be concise.");
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoiceProfile {
    /// System-prompt preamble that defines the worker's voice.
    pub preamble: String,
    /// High-level tone register.
    #[serde(default)]
    pub tone: Tone,
    /// Formality register.
    #[serde(default)]
    pub formality: Formality,
    /// Domain vocabulary the persona may use (e.g. `"MTTR"`, `"DAU"`).
    /// Composed as a hint, not a mandate.
    #[serde(default)]
    pub lexicon: Vec<String>,
    /// Phrases the persona must avoid. Best-effort: the model may
    /// still produce them; downstream output filtering is the real
    /// enforcement layer.
    #[serde(default)]
    pub forbidden_phrases: Vec<String>,
}

impl VoiceProfile {
    /// Materialise the final preamble that the agent-prompt path
    /// sends to the model.
    ///
    /// Order (load-bearing): tone instruction → formality instruction
    /// → lexicon hint → forbidden-phrase guard → freeform `preamble`.
    /// The freeform preamble is last so operator overrides clearly
    /// win over structured defaults.
    ///
    /// When every structured field is at its default and `preamble`
    /// is set, the return value is byte-identical to `preamble`.
    /// When every field is empty/default, the return value is empty.
    ///
    /// ```
    /// use argo_identity::{Tone, VoiceProfile};
    /// let voice = VoiceProfile {
    ///     preamble: "Be Maya.".into(),
    ///     tone: Tone::Warm,
    ///     ..Default::default()
    /// };
    /// let composed = voice.compose_preamble();
    /// assert!(composed.starts_with("Speak with a warm"));
    /// assert!(composed.ends_with("Be Maya."));
    /// ```
    #[must_use]
    pub fn compose_preamble(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        if let Some(line) = self.tone.instruction() {
            parts.push(line.to_string());
        }
        if let Some(line) = self.formality.instruction() {
            parts.push(line.to_string());
        }
        if !self.lexicon.is_empty() {
            parts.push(format!(
                "Vocabulary you may use: {}.",
                self.lexicon.join(", ")
            ));
        }
        if !self.forbidden_phrases.is_empty() {
            parts.push(format!(
                "Never use these phrases: {}.",
                self.forbidden_phrases.join(", ")
            ));
        }
        if !self.preamble.is_empty() {
            parts.push(self.preamble.clone());
        }
        parts.join("\n\n")
    }
}

/// Persona presented to humans.
///
/// ```
/// use argo_identity::Persona;
/// let persona = Persona::named("Maya", "operator@example.com");
/// assert_eq!(persona.name, "Maya");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Persona {
    /// Human-facing name.
    pub name: String,
    /// Voice and tone instructions.
    pub voice_profile: VoiceProfile,
    /// Optional avatar URL or asset id.
    pub avatar: Option<String>,
    /// Optional public biography.
    pub bio: Option<String>,
    /// Operator email, handle, or id that built the persona.
    pub builder_attribution: String,
    /// Skill ids that contributed to this persona over time.
    pub provenance_chain: Vec<String>,
}

impl Persona {
    /// Create a minimal named persona.
    #[must_use]
    pub fn named(name: impl Into<String>, builder_attribution: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            voice_profile: VoiceProfile {
                preamble: format!("You are {name}, an autonomous SKYCOT worker."),
                ..Default::default()
            },
            name,
            avatar: None,
            bio: None,
            builder_attribution: builder_attribution.into(),
            provenance_chain: Vec::new(),
        }
    }
}

/// A scheduled or callable duty assigned to a role
/// (Stream `claude-worker-duty-scheduler`). `id`/`enabled`/
/// `paused_until`/`last_fired_at` are `#[serde(default)]` so role
/// JSON written before V1.3 still deserialises unchanged.
///
/// ```
/// use argo_identity::Duty;
/// let duty = Duty::new("0 9 * * 1-5", "daily_check", "Check the inbox.");
/// assert_eq!(duty.entry_action, "daily_check");
/// assert!(duty.enabled && duty.paused_until.is_none());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Duty {
    /// Stable id (UUIDv7-stringified for sort order).
    #[serde(default = "Duty::default_id")]
    pub id: String,
    /// Cron schedule that drives the duty.
    pub cron: String,
    /// Entry action invoked when the duty runs.
    pub entry_action: String,
    /// Human-readable description.
    pub description: String,
    /// Scheduler considers this duty when `true`.
    #[serde(default = "Duty::default_enabled")]
    pub enabled: bool,
    /// Scheduler skips this duty until the `DateTime` passes.
    #[serde(default)]
    pub paused_until: Option<chrono::DateTime<chrono::Utc>>,
    /// Last successful fire (powers the per-window idempotency).
    #[serde(default)]
    pub last_fired_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl Duty {
    /// Build an enabled, never-fired duty with a fresh UUIDv7 id.
    #[must_use]
    pub fn new(
        cron: impl Into<String>,
        entry_action: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            id: Self::default_id(),
            cron: cron.into(),
            entry_action: entry_action.into(),
            description: description.into(),
            enabled: true,
            paused_until: None,
            last_fired_at: None,
        }
    }

    fn default_id() -> String { Uuid::now_v7().to_string() }
    fn default_enabled() -> bool { true }
}

/// Delegated role: charter, duties, KPIs, and escalation path.
///
/// ```
/// use argo_identity::Role;
/// let role = Role::new("Follow up with approved leads.");
/// assert!(role.duties.is_empty());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Role {
    /// Free-text operating charter.
    pub charter: String,
    /// Duties that can be scheduled or invoked.
    pub duties: Vec<Duty>,
    /// Key performance indicators.
    pub kpis: Vec<String>,
    /// Escalation path for blocked work.
    pub escalation: Option<String>,
}

impl Role {
    /// Create a role with no duties yet.
    ///
    /// ```
    /// use argo_identity::Role;
    /// let role = Role::new("Own qualified support triage.");
    /// assert!(role.charter.starts_with("Own"));
    /// ```
    #[must_use]
    pub fn new(charter: impl Into<String>) -> Self {
        Self {
            charter: charter.into(),
            duties: Vec::new(),
            kpis: Vec::new(),
            escalation: None,
        }
    }
}
