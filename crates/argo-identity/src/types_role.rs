//! Persona and Role primitives split from `types.rs` (file-LOC ceiling).
//!
//! Holds the delegation-contract types — how a worker presents to humans
//! (`VoiceProfile`, `Persona`) and what they're delegated to do
//! (`Duty`, `Role`). Re-exported via `types::` so call sites are unchanged.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Public-facing voice profile for a persona.
///
/// ```
/// use argo_identity::VoiceProfile;
/// let voice = VoiceProfile { preamble: "Be concise.".into() };
/// assert!(voice.preamble.contains("concise"));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoiceProfile {
    /// System-prompt preamble that defines the worker's voice.
    pub preamble: String,
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
