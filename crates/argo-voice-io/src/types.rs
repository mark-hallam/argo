//! Provider-agnostic value types shared by the STT and TTS traits.
//!
//! These are deliberately tiny and vendor-neutral: a [`VoiceId`] selects which
//! voice to speak in, a [`Transcript`] is the text a listener produced, and an
//! [`AudioClip`] is a self-describing blob of synthesized audio. No provider
//! type (ElevenLabs, Deepgram, OpenAI) leaks into these definitions.

use serde::{Deserialize, Serialize};

/// Identifier selecting which voice a [`crate::TextToSpeech`] adapter speaks in.
///
/// The string is provider-specific (an ElevenLabs voice id, an OpenAI voice
/// name, a cloned-brand-voice handle); this crate treats it as an opaque key
/// chosen by a voice manifest, never interpreting it.
///
/// # Examples
///
/// ```
/// use argo_voice_io::VoiceId;
/// let voice = VoiceId::new("portia");
/// assert_eq!(voice.as_str(), "portia");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VoiceId(String);

impl VoiceId {
    /// Build a voice id from any string-like value.
    ///
    /// # Examples
    ///
    /// ```
    /// use argo_voice_io::VoiceId;
    /// assert_eq!(VoiceId::new("grant").as_str(), "grant");
    /// ```
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Borrow the underlying provider voice key.
    ///
    /// # Examples
    ///
    /// ```
    /// use argo_voice_io::VoiceId;
    /// let voice = VoiceId::new("alloy");
    /// assert_eq!(voice.as_str(), "alloy");
    /// ```
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for VoiceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for VoiceId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for VoiceId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

/// Text a [`crate::SpeechToText`] adapter produced from spoken audio.
///
/// Carries the recognized `text` plus an optional `confidence` in `0.0..=1.0`
/// (mock and some real providers may omit it). The struct is intentionally
/// minimal; richer per-word timing is a V2 concern, not L1.
///
/// # Examples
///
/// ```
/// use argo_voice_io::Transcript;
/// let t = Transcript::new("hello there");
/// assert_eq!(t.text(), "hello there");
/// assert!(t.confidence().is_none());
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transcript {
    text: String,
    confidence: Option<f32>,
}

impl Transcript {
    /// Build a transcript with no confidence score.
    ///
    /// # Examples
    ///
    /// ```
    /// use argo_voice_io::Transcript;
    /// assert_eq!(Transcript::new("yes").text(), "yes");
    /// ```
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            confidence: None,
        }
    }

    /// Build a transcript with a confidence score in `0.0..=1.0`.
    ///
    /// Out-of-range values are clamped, so callers never observe an impossible
    /// confidence.
    ///
    /// # Examples
    ///
    /// ```
    /// use argo_voice_io::Transcript;
    /// let t = Transcript::with_confidence("ok", 1.5);
    /// assert_eq!(t.confidence(), Some(1.0));
    /// ```
    #[must_use]
    pub fn with_confidence(text: impl Into<String>, confidence: f32) -> Self {
        Self {
            text: text.into(),
            confidence: Some(confidence.clamp(0.0, 1.0)),
        }
    }

    /// Borrow the recognized text.
    ///
    /// # Examples
    ///
    /// ```
    /// use argo_voice_io::Transcript;
    /// assert_eq!(Transcript::new("hi").text(), "hi");
    /// ```
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Recognition confidence in `0.0..=1.0`, if the provider supplied one.
    ///
    /// # Examples
    ///
    /// ```
    /// use argo_voice_io::Transcript;
    /// assert_eq!(Transcript::with_confidence("hi", 0.9).confidence(), Some(0.9));
    /// ```
    #[must_use]
    pub fn confidence(&self) -> Option<f32> {
        self.confidence
    }
}

/// A self-describing blob of synthesized audio from a [`crate::TextToSpeech`].
///
/// Holds the raw `bytes` plus a free-form `format` tag (e.g. `"pcm_s16le_16k"`,
/// `"mp3_44100"`, or the mock's `"mock/raw"`). The format string is opaque to
/// this crate; consumers (gateway routes, the browser UI) interpret it.
///
/// # Examples
///
/// ```
/// use argo_voice_io::AudioClip;
/// let clip = AudioClip::new("mp3_44100", vec![1, 2, 3]);
/// assert_eq!(clip.format(), "mp3_44100");
/// assert_eq!(clip.len(), 3);
/// assert!(!clip.is_empty());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioClip {
    format: String,
    bytes: Vec<u8>,
}

impl AudioClip {
    /// Build an audio clip from a format tag and its bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use argo_voice_io::AudioClip;
    /// assert_eq!(AudioClip::new("wav", vec![0u8; 4]).len(), 4);
    /// ```
    #[must_use]
    pub fn new(format: impl Into<String>, bytes: Vec<u8>) -> Self {
        Self {
            format: format.into(),
            bytes,
        }
    }

    /// Borrow the opaque format tag.
    ///
    /// # Examples
    ///
    /// ```
    /// use argo_voice_io::AudioClip;
    /// assert_eq!(AudioClip::new("wav", vec![]).format(), "wav");
    /// ```
    #[must_use]
    pub fn format(&self) -> &str {
        &self.format
    }

    /// Borrow the raw audio bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use argo_voice_io::AudioClip;
    /// assert_eq!(AudioClip::new("wav", vec![7, 8]).bytes(), &[7, 8]);
    /// ```
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Number of audio bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use argo_voice_io::AudioClip;
    /// assert_eq!(AudioClip::new("wav", vec![1, 2, 3]).len(), 3);
    /// ```
    #[must_use]
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Whether the clip carries no audio bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use argo_voice_io::AudioClip;
    /// assert!(AudioClip::new("wav", vec![]).is_empty());
    /// ```
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Consume the clip and return its raw bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use argo_voice_io::AudioClip;
    /// assert_eq!(AudioClip::new("wav", vec![9]).into_bytes(), vec![9]);
    /// ```
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}
