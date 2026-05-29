//! Deterministic, key-free mock adapters for tests and local wiring.
//!
//! These let the rest of the voice stack (gateway routes at V2, the browser UI
//! at V3) be built and tested *before* any real vendor API key exists. Every
//! output is a pure function of the input, so assertions are stable.

use crate::stt::SpeechToText;
use crate::tts::TextToSpeech;
use crate::types::{AudioClip, Transcript, VoiceId};
use argo_types::{ArgoError, ArgoResult};
use async_trait::async_trait;
use sha2::{Digest, Sha256};

/// The format tag every [`MockTts`] clip carries.
pub const MOCK_AUDIO_FORMAT: &str = "mock/raw";

/// Deterministic [`TextToSpeech`] that derives audio bytes from the text.
///
/// No network, no API key: the bytes are the SHA-256 digest of
/// `"{voice}\n{text}"`, so the same input always yields the same clip and two
/// different inputs (or voices) yield different clips. Empty text is rejected
/// with [`ArgoError::Schema`], mirroring how a real adapter guards input.
///
/// # Examples
///
/// ```
/// use argo_voice_io::{MockTts, TextToSpeech, VoiceId};
/// # async fn demo() -> argo_types::ArgoResult<()> {
/// let tts = MockTts::new();
/// let a = tts.synthesize("hello", &VoiceId::new("portia")).await?;
/// let b = tts.synthesize("hello", &VoiceId::new("portia")).await?;
/// assert_eq!(a, b); // deterministic
/// # Ok(()) }
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct MockTts;

impl MockTts {
    /// Build a mock text-to-speech adapter.
    ///
    /// # Examples
    ///
    /// ```
    /// use argo_voice_io::MockTts;
    /// let _tts = MockTts::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// The deterministic bytes this adapter would produce for `text`/`voice`.
    ///
    /// Exposed so the round-trip mock STT ([`MockStt::transcribing`]) can pair
    /// with it without duplicating the derivation.
    ///
    /// # Examples
    ///
    /// ```
    /// use argo_voice_io::{MockTts, VoiceId};
    /// let bytes = MockTts::bytes_for("hi", &VoiceId::new("v"));
    /// assert_eq!(bytes.len(), 32);
    /// ```
    #[must_use]
    pub fn bytes_for(text: &str, voice: &VoiceId) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(voice.as_str().as_bytes());
        hasher.update(b"\n");
        hasher.update(text.as_bytes());
        hasher.finalize().to_vec()
    }
}

#[async_trait]
impl TextToSpeech for MockTts {
    async fn synthesize(&self, text: &str, voice: &VoiceId) -> ArgoResult<AudioClip> {
        if text.trim().is_empty() {
            return Err(ArgoError::Schema("mock tts: empty text".into()));
        }
        Ok(AudioClip::new(MOCK_AUDIO_FORMAT, Self::bytes_for(text, voice)))
    }
}

/// How a [`MockStt`] decides what text to return.
#[derive(Debug, Clone)]
enum MockSttMode {
    /// Always return this exact transcript text.
    Canned(String),
    /// Derive a stable transcript from a hex digest of the audio bytes.
    Echo,
    /// Recover the exact text a [`MockTts`] clip was synthesized from.
    Transcribing { voice: VoiceId, text: String },
}

/// Deterministic [`SpeechToText`] with three test-friendly modes.
///
/// - [`MockStt::canned`] always returns a fixed string — handy when a test
///   wants to drive a known question into [`crate::voice_turn`].
/// - [`MockStt::echo`] returns a stable transcript derived from the audio
///   bytes (`"mock-transcript:<8 hex chars>"`), so distinct audio yields
///   distinct text without needing a real recognizer.
/// - [`MockStt::transcribing`] is the inverse of a [`MockTts`] clip: feed it
///   the bytes [`MockTts`] produced for some text and it returns that text,
///   enabling a true mock round-trip in tests.
///
/// # Examples
///
/// ```
/// use argo_voice_io::{MockStt, SpeechToText};
/// # async fn demo() -> argo_types::ArgoResult<()> {
/// let stt = MockStt::canned("what is skycot");
/// assert_eq!(stt.transcribe(b"ignored").await?.text(), "what is skycot");
/// # Ok(()) }
/// ```
#[derive(Debug, Clone)]
pub struct MockStt {
    mode: MockSttMode,
}

impl MockStt {
    /// A mock STT that always returns `text`, regardless of the audio.
    ///
    /// # Examples
    ///
    /// ```
    /// use argo_voice_io::{MockStt, SpeechToText};
    /// # async fn demo() -> argo_types::ArgoResult<()> {
    /// assert_eq!(MockStt::canned("hi").transcribe(b"x").await?.text(), "hi");
    /// # Ok(()) }
    /// ```
    #[must_use]
    pub fn canned(text: impl Into<String>) -> Self {
        Self {
            mode: MockSttMode::Canned(text.into()),
        }
    }

    /// A mock STT that derives a stable transcript from the audio bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use argo_voice_io::{MockStt, SpeechToText};
    /// # async fn demo() -> argo_types::ArgoResult<()> {
    /// let t = MockStt::echo().transcribe(b"abc").await?;
    /// assert!(t.text().starts_with("mock-transcript:"));
    /// # Ok(()) }
    /// ```
    #[must_use]
    pub fn echo() -> Self {
        Self {
            mode: MockSttMode::Echo,
        }
    }

    /// A mock STT that recovers the exact text a [`MockTts`] clip encoded.
    ///
    /// Pair it with [`MockTts::bytes_for`] using the same `voice`/`text` to get
    /// a deterministic synthesize-then-transcribe round-trip in tests.
    ///
    /// # Examples
    ///
    /// ```
    /// use argo_voice_io::{MockStt, MockTts, SpeechToText, VoiceId};
    /// # async fn demo() -> argo_types::ArgoResult<()> {
    /// let voice = VoiceId::new("portia");
    /// let bytes = MockTts::bytes_for("hello", &voice);
    /// let stt = MockStt::transcribing(voice, "hello");
    /// assert_eq!(stt.transcribe(&bytes).await?.text(), "hello");
    /// # Ok(()) }
    /// ```
    #[must_use]
    pub fn transcribing(voice: VoiceId, text: impl Into<String>) -> Self {
        Self {
            mode: MockSttMode::Transcribing {
                voice,
                text: text.into(),
            },
        }
    }
}

#[async_trait]
impl SpeechToText for MockStt {
    async fn transcribe(&self, audio: &[u8]) -> ArgoResult<Transcript> {
        if audio.is_empty() {
            return Err(ArgoError::Schema("mock stt: empty audio".into()));
        }
        match &self.mode {
            MockSttMode::Canned(text) => Ok(Transcript::with_confidence(text.clone(), 1.0)),
            MockSttMode::Echo => {
                let digest = Sha256::digest(audio);
                let short = hex::encode(&digest[..4]);
                Ok(Transcript::with_confidence(
                    format!("mock-transcript:{short}"),
                    1.0,
                ))
            }
            MockSttMode::Transcribing { voice, text } => {
                if audio == MockTts::bytes_for(text, voice).as_slice() {
                    Ok(Transcript::with_confidence(text.clone(), 1.0))
                } else {
                    Err(ArgoError::Provider(
                        "mock stt: audio does not match the expected mock clip".into(),
                    ))
                }
            }
        }
    }
}
