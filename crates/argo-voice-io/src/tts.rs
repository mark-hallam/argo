//! The vendor-agnostic text-to-speech contract.

use crate::types::{AudioClip, VoiceId};
use argo_types::ArgoResult;
use async_trait::async_trait;

/// Turns text into spoken audio.
///
/// Implementors wrap a real vendor (ElevenLabs, OpenAI) or a deterministic
/// in-crate fake ([`crate::MockTts`]). The [`VoiceId`] selects which voice to
/// speak in — it is a manifest parameter, so Portia's voice differs from
/// Grant's. Output is a self-describing [`AudioClip`].
///
/// # Examples
///
/// ```
/// use argo_voice_io::{MockTts, TextToSpeech, VoiceId};
/// # async fn demo() -> argo_types::ArgoResult<()> {
/// let tts = MockTts::new();
/// let clip = tts.synthesize("hello", &VoiceId::new("portia")).await?;
/// assert!(!clip.is_empty());
/// # Ok(()) }
/// ```
#[async_trait]
pub trait TextToSpeech: Send + Sync {
    /// Synthesize `text` into an [`AudioClip`] spoken in `voice`.
    ///
    /// # Errors
    ///
    /// Returns [`argo_types::ArgoError::Provider`] when the backing vendor
    /// rejects the request, or [`argo_types::ArgoError::Schema`] when `text`
    /// is unusable (e.g. empty).
    ///
    /// # Examples
    ///
    /// ```
    /// use argo_voice_io::{MockTts, TextToSpeech, VoiceId};
    /// # async fn demo() -> argo_types::ArgoResult<()> {
    /// let clip = MockTts::new().synthesize("hi", &VoiceId::new("v")).await?;
    /// assert_eq!(clip.format(), "mock/raw");
    /// # Ok(()) }
    /// ```
    async fn synthesize(&self, text: &str, voice: &VoiceId) -> ArgoResult<AudioClip>;
}
