//! Composition helper wiring STT -> an injected answer source -> TTS.
//!
//! This is the seam the live truth-bounded pitch-ask plugs into at V2: the
//! `answer` closure is where `POST /api/v1/pitch/ask` (LLM + capability
//! truth-boundary + audit) will be called. L1 keeps that source fully
//! injectable so no pitch logic is hard-wired into the voice library.

use crate::stt::SpeechToText;
use crate::tts::TextToSpeech;
use crate::types::{AudioClip, Transcript, VoiceId};
use argo_types::ArgoResult;
use std::future::Future;

/// The result of one full voice turn: what was heard and what was spoken.
///
/// # Examples
///
/// ```
/// use argo_voice_io::VoiceTurnOutcome;
/// use argo_voice_io::{AudioClip, Transcript};
/// let outcome = VoiceTurnOutcome {
///     heard: Transcript::new("hi"),
///     answer_text: "hello".into(),
///     spoken: AudioClip::new("mock/raw", vec![1]),
/// };
/// assert_eq!(outcome.heard.text(), "hi");
/// assert_eq!(outcome.answer_text, "hello");
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct VoiceTurnOutcome {
    /// The transcript the [`SpeechToText`] adapter produced from the input.
    pub heard: Transcript,
    /// The answer text the injected source returned (pre-synthesis).
    pub answer_text: String,
    /// The synthesized audio for `answer_text`.
    pub spoken: AudioClip,
}

/// Run one voice turn: transcribe `audio`, ask `answer`, synthesize the reply.
///
/// The pipeline is `stt.transcribe -> answer(transcript) -> tts.synthesize`.
/// The `answer` closure is the injection point for the real (truth-bounded)
/// brain; this helper never inspects or constrains its output.
///
/// # Errors
///
/// Propagates the first error from any stage: STT, the `answer` closure, or
/// TTS. A failing stage short-circuits the rest of the turn.
///
/// # Examples
///
/// ```
/// use argo_voice_io::{voice_turn, MockStt, MockTts, VoiceId};
/// # async fn demo() -> argo_types::ArgoResult<()> {
/// let stt = MockStt::canned("what does skycot do");
/// let tts = MockTts::new();
/// let voice = VoiceId::new("portia");
/// let outcome = voice_turn(&stt, &tts, &voice, b"question-audio", |heard| async move {
///     Ok::<_, argo_types::ArgoError>(format!("you asked: {}", heard.text()))
/// })
/// .await?;
/// assert_eq!(outcome.answer_text, "you asked: what does skycot do");
/// assert!(!outcome.spoken.is_empty());
/// # Ok(()) }
/// ```
pub async fn voice_turn<S, T, F, Fut>(
    stt: &S,
    tts: &T,
    voice: &VoiceId,
    audio: &[u8],
    answer: F,
) -> ArgoResult<VoiceTurnOutcome>
where
    S: SpeechToText + ?Sized,
    T: TextToSpeech + ?Sized,
    F: FnOnce(Transcript) -> Fut,
    Fut: Future<Output = ArgoResult<String>>,
{
    let heard = stt.transcribe(audio).await?;
    let answer_text = answer(heard.clone()).await?;
    let spoken = tts.synthesize(&answer_text, voice).await?;
    Ok(VoiceTurnOutcome {
        heard,
        answer_text,
        spoken,
    })
}
