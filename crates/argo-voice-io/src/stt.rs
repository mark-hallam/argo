//! The vendor-agnostic speech-to-text contract.

use crate::types::Transcript;
use argo_types::ArgoResult;
use async_trait::async_trait;

/// Turns spoken audio into text.
///
/// Implementors wrap a real vendor (Deepgram, OpenAI) or a deterministic
/// in-crate fake ([`crate::MockStt`]). Audio is passed as opaque bytes; the
/// adapter owns any knowledge of the wire format. Failures map to
/// [`argo_types::ArgoError::Provider`] for real vendors.
///
/// # Examples
///
/// ```
/// use argo_voice_io::{MockStt, SpeechToText};
/// # async fn demo() -> argo_types::ArgoResult<()> {
/// let stt = MockStt::echo();
/// let transcript = stt.transcribe(b"hello-bytes").await?;
/// assert!(!transcript.text().is_empty());
/// # Ok(()) }
/// ```
#[async_trait]
pub trait SpeechToText: Send + Sync {
    /// Transcribe a complete audio buffer into a single [`Transcript`].
    ///
    /// # Errors
    ///
    /// Returns [`argo_types::ArgoError::Provider`] when the backing vendor
    /// rejects the request, or [`argo_types::ArgoError::Schema`] when the
    /// audio buffer is unusable (e.g. empty).
    ///
    /// # Examples
    ///
    /// ```
    /// use argo_voice_io::{MockStt, SpeechToText};
    /// # async fn demo() -> argo_types::ArgoResult<()> {
    /// let stt = MockStt::canned("yes please");
    /// assert_eq!(stt.transcribe(b"any").await?.text(), "yes please");
    /// # Ok(()) }
    /// ```
    async fn transcribe(&self, audio: &[u8]) -> ArgoResult<Transcript>;
}
