//! Turn-taking policy and state transitions.

use serde::{Deserialize, Serialize};

/// Speaker identity for a turn event.
///
/// # Examples
///
/// ```
/// use argo_voice_io::Speaker;
/// assert_ne!(Speaker::User, Speaker::Agent);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Speaker {
    /// End user or meeting participant.
    User,
    /// Argo agent.
    Agent,
}

/// Turn-taking policy loaded from a voice manifest.
///
/// # Examples
///
/// ```
/// use argo_voice_io::TurnTakingPolicy;
/// assert!(TurnTakingPolicy::default().allow_user_interrupt);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnTakingPolicy {
    /// Silence duration that ends a user turn.
    pub endpoint_silence_ms: u32,
    /// Whether user speech interrupts agent speech.
    pub allow_user_interrupt: bool,
    /// Whether agent speech can barge over user speech.
    pub allow_agent_barge: bool,
}

impl Default for TurnTakingPolicy {
    fn default() -> Self {
        Self {
            endpoint_silence_ms: 600,
            allow_user_interrupt: true,
            allow_agent_barge: false,
        }
    }
}

/// A high-level turn-taking event.
///
/// # Examples
///
/// ```
/// use argo_voice_io::{Speaker, TurnEvent};
/// let event = TurnEvent::InterruptionDetected { who_was_speaking: Speaker::Agent };
/// assert!(matches!(event, TurnEvent::InterruptionDetected { .. }));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnEvent {
    /// User speech started.
    UserBeganSpeaking,
    /// User speech stopped.
    UserStoppedSpeaking,
    /// Agent speech started.
    AgentBeganSpeaking,
    /// Agent speech finished or was cancelled.
    AgentFinishedSpeaking,
    /// One speaker interrupted another.
    InterruptionDetected {
        /// Speaker who was interrupted.
        who_was_speaking: Speaker,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TurnState {
    Idle,
    UserSpeaking,
    AgentSpeaking,
}

impl TurnState {
    pub(crate) fn speaker(self) -> Option<Speaker> {
        match self {
            Self::Idle => None,
            Self::UserSpeaking => Some(Speaker::User),
            Self::AgentSpeaking => Some(Speaker::Agent),
        }
    }
}
