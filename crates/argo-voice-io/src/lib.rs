//! Provider-agnostic agent-side voice I/O for live channels.
//!
//! This is the **L1** layer of the live-voice plan
//! (`docs/skycot/design/2026-05-29-gap-closure-plans/01-live-voice.md`): a clean,
//! vendor-neutral seam for turning the agent's words into speech and a
//! listener's speech into text. It defines two async traits — [`SpeechToText`]
//! and [`TextToSpeech`] — plus the tiny value types they exchange
//! ([`Transcript`], [`AudioClip`], [`VoiceId`]).
//!
//! Real vendors (ElevenLabs, Deepgram, OpenAI) are **out of scope here**: they
//! need an operator-held API key and land at V2. To unblock building and
//! testing the rest of the stack today, the crate ships deterministic,
//! key-free mock adapters ([`MockStt`], [`MockTts`]) and a [`voice_turn`]
//! composition helper that wires `STT -> an injected answer source -> TTS`.
//! That answer source is where the live truth-bounded pitch-ask plugs in at V2;
//! it is fully injectable so no pitch logic is hard-wired into this library.
//!
//! Which adapter and which voice to use is selected by a [`VoiceManifest`]
//! (TOML, not Rust — per the "provider manifests, not adapters" invariant).
//!
//! # Examples
//!
//! ```
//! use argo_voice_io::{voice_turn, MockStt, MockTts, VoiceId};
//! # async fn demo() -> argo_types::ArgoResult<()> {
//! let stt = MockStt::canned("what makes skycot different");
//! let tts = MockTts::new();
//! let voice = VoiceId::new("portia");
//! let outcome = voice_turn(&stt, &tts, &voice, b"audio", |heard| async move {
//!     Ok::<_, argo_types::ArgoError>(format!("answering: {}", heard.text()))
//! })
//! .await?;
//! assert_eq!(outcome.answer_text, "answering: what makes skycot different");
//! # Ok(()) }
//! ```
#![warn(missing_docs)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

pub mod manifest;
pub mod mock;
pub mod stt;
pub mod tts;
pub mod types;
pub mod voice_turn;

#[cfg(test)]
mod tests;

pub use manifest::{SttConfig, TtsConfig, VoiceManifest};
pub use mock::{MockStt, MockTts, MOCK_AUDIO_FORMAT};
pub use stt::SpeechToText;
pub use tts::TextToSpeech;
pub use types::{AudioClip, Transcript, VoiceId};
pub use voice_turn::{voice_turn, VoiceTurnOutcome};
