//! Provider-agnostic agent-side voice I/O for live channels.
//!
//! The crate owns the stable STT, TTS, manifest, provider-registry,
//! and turn-taking contracts. Real providers are loaded through
//! follow-up subprocess plugins; this crate includes only a deterministic
//! in-memory stub provider for tests and local wiring.
//!
//! # Examples
//!
//! ```
//! use argo_voice_io::{VoiceManifest, VoiceProviderRegistry};
//! let manifest = VoiceManifest::from_toml_str(
//!     r#"id = "stub"
//! provider = "stub"
//! [stt]
//! language = "en-AU"
//! interim_results = true
//! [tts]
//! voice_id = "demo"
//! speed = 1.0
//! [turn_taking]
//! endpoint_silence_ms = 600
//! allow_user_interrupt = true
//! allow_agent_barge = false
//! "#,
//! ).unwrap();
//! assert!(manifest.validate_provider(&VoiceProviderRegistry::with_stub()).is_ok());
//! ```
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

pub mod manifest;
pub mod provider;
pub mod providers;
pub mod session;
pub mod turn_taking;

#[cfg(test)]
mod lib_tests;

pub use manifest::{ProviderConfig, SttConfig, TtsConfig, VoiceManifest};
pub use provider::{
    SpeakHandle, SpeakOptions, SpeakPriority, SpeakStatus, TranscriptEvent, VoiceCapabilities,
    VoiceProvider, VoiceProviderRegistry, VoiceTransport,
};
pub use providers::stub::StubProvider;
pub use session::VoiceSession;
pub use turn_taking::{Speaker, TurnEvent, TurnTakingPolicy};
