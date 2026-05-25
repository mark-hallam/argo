//! Provider traits, transcript and speech handles, and registry.

use crate::manifest::ProviderConfig;
use argo_types::ArgoResult;
use futures::future::BoxFuture;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// A provider-emitted transcript event.
///
/// # Examples
///
/// ```
/// use argo_voice_io::TranscriptEvent;
/// let event = TranscriptEvent::Partial { text: "hello".into(), is_speaker_continuing: true };
/// assert!(matches!(event, TranscriptEvent::Partial { .. }));
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum TranscriptEvent {
    /// Interim transcript text.
    Partial {
        /// Transcript text.
        text: String,
        /// Whether the speaker is likely still talking.
        is_speaker_continuing: bool,
    },
    /// Final transcript text with the local completion instant.
    Final {
        /// Transcript text.
        text: String,
        /// Local instant when the utterance ended.
        ended_at: Instant,
    },
    /// Endpointing detected a silence interval.
    SilenceDetected {
        /// Silence duration in milliseconds.
        duration_ms: u32,
    },
    /// The user started speaking while the agent was speaking.
    UserInterrupted,
}

/// Options for one synthesized agent utterance.
///
/// # Examples
///
/// ```
/// use argo_voice_io::{SpeakOptions, SpeakPriority};
/// let opts = SpeakOptions::default();
/// assert_eq!(opts.priority, SpeakPriority::Normal);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpeakOptions {
    /// Provider-specific voice id override.
    pub voice_id: Option<String>,
    /// Whether a user utterance may interrupt this speech.
    pub interruptible: bool,
    /// Utterance priority.
    pub priority: SpeakPriority,
}

impl Default for SpeakOptions {
    fn default() -> Self {
        Self {
            voice_id: None,
            interruptible: true,
            priority: SpeakPriority::Normal,
        }
    }
}

/// Priority for synthesized speech.
///
/// # Examples
///
/// ```
/// use argo_voice_io::SpeakPriority;
/// assert_ne!(SpeakPriority::Normal, SpeakPriority::BargeOver);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpeakPriority {
    /// Normal, interruptible channel speech.
    Normal,
    /// Continue over user speech; reserved for rare safety messages.
    BargeOver,
}

/// Current status of a synthesized utterance.
///
/// # Examples
///
/// ```
/// use argo_voice_io::SpeakStatus;
/// assert_eq!(format!("{:?}", SpeakStatus::Speaking), "Speaking");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpeakStatus {
    /// Provider accepted the utterance and audio is in progress.
    Speaking,
    /// The utterance completed.
    Complete,
    /// The utterance was interrupted.
    Interrupted,
    /// The utterance failed.
    Failed,
}

/// Handle returned after a provider accepts synthesized speech.
///
/// # Examples
///
/// ```
/// use argo_voice_io::{SpeakHandle, SpeakStatus};
/// let handle = SpeakHandle::new("utt_1", SpeakStatus::Speaking);
/// assert_eq!(handle.utterance_id, "utt_1");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpeakHandle {
    /// Provider or platform utterance id.
    pub utterance_id: String,
    /// Last known utterance status.
    pub status: SpeakStatus,
}

impl SpeakHandle {
    /// Build a speech handle.
    ///
    /// # Examples
    ///
    /// ```
    /// use argo_voice_io::{SpeakHandle, SpeakStatus};
    /// assert_eq!(SpeakHandle::new("u", SpeakStatus::Complete).status, SpeakStatus::Complete);
    /// ```
    #[must_use]
    pub fn new(id: impl Into<String>, status: SpeakStatus) -> Self {
        Self {
            utterance_id: id.into(),
            status,
        }
    }
}

/// Provider capability hints used for routing and UX copy.
///
/// # Examples
///
/// ```
/// use argo_voice_io::VoiceCapabilities;
/// let caps = VoiceCapabilities::stub();
/// assert!(caps.supports_interrupt);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoiceCapabilities {
    /// Whether the provider supports cancelling active synthesis.
    pub supports_interrupt: bool,
    /// Whether the provider supports agent barge-in over user speech.
    pub supports_barge_in: bool,
    /// Typical first-audio latency in milliseconds.
    pub typical_first_audio_latency_ms: u32,
    /// Typical STT latency in milliseconds.
    pub typical_transcription_latency_ms: u32,
}

impl VoiceCapabilities {
    /// Capability profile for the deterministic stub provider.
    ///
    /// # Examples
    ///
    /// ```
    /// use argo_voice_io::VoiceCapabilities;
    /// assert_eq!(VoiceCapabilities::stub().typical_first_audio_latency_ms, 1);
    /// ```
    #[must_use]
    pub fn stub() -> Self {
        Self {
            supports_interrupt: true,
            supports_barge_in: false,
            typical_first_audio_latency_ms: 1,
            typical_transcription_latency_ms: 1,
        }
    }
}

/// Trait implemented by every voice provider plugin or test stub.
///
/// # Examples
///
/// ```
/// use argo_voice_io::{StubProvider, VoiceProvider};
/// let provider = StubProvider::default();
/// assert_eq!(provider.name(), "stub");
/// ```
pub trait VoiceProvider: Send + Sync {
    /// Stable provider registry name.
    fn name(&self) -> &'static str;

    /// Capability hints for the provider.
    fn capabilities(&self) -> VoiceCapabilities;

    /// Open a provider-specific voice transport.
    fn open<'a>(
        &'a self,
        cfg: &'a ProviderConfig,
    ) -> BoxFuture<'a, ArgoResult<Box<dyn VoiceTransport>>>;
}

/// Provider-specific session transport.
///
/// # Examples
///
/// ```
/// use argo_voice_io::{VoiceManifest, VoiceProviderRegistry};
/// let registry = VoiceProviderRegistry::with_stub();
/// assert!(registry.resolve(&VoiceManifest::stub().provider).is_some());
/// ```
pub trait VoiceTransport: Send + Sync {
    /// Push signed 16-bit PCM audio frames into STT.
    fn push_audio<'a>(&'a mut self, pcm_frames: &'a [i16]) -> BoxFuture<'a, ArgoResult<()>>;

    /// Poll one queued transcript event, if available.
    fn poll_transcript(&mut self) -> Option<TranscriptEvent>;

    /// Synthesize one text utterance.
    fn synth<'a>(
        &'a mut self,
        text: &'a str,
        opts: &'a SpeakOptions,
    ) -> BoxFuture<'a, ArgoResult<SpeakHandle>>;

    /// Cancel active synthesis, if any.
    fn cancel_synth<'a>(&'a mut self) -> BoxFuture<'a, ArgoResult<()>>;
}

/// Voice provider registry.
///
/// # Examples
///
/// ```
/// use argo_voice_io::VoiceProviderRegistry;
/// let registry = VoiceProviderRegistry::with_stub();
/// assert!(registry.resolve("stub").is_some());
/// ```
#[derive(Clone, Default)]
pub struct VoiceProviderRegistry {
    by_name: HashMap<String, Arc<dyn VoiceProvider>>,
}

impl VoiceProviderRegistry {
    /// Build a registry containing only the deterministic stub provider.
    ///
    /// # Examples
    ///
    /// ```
    /// use argo_voice_io::VoiceProviderRegistry;
    /// assert!(VoiceProviderRegistry::with_stub().resolve("stub").is_some());
    /// ```
    #[must_use]
    pub fn with_stub() -> Self {
        let mut registry = Self::default();
        registry.register(Arc::new(crate::providers::stub::StubProvider::default()));
        registry
    }

    /// Register a provider by its stable name.
    ///
    /// # Examples
    ///
    /// ```
    /// use argo_voice_io::{StubProvider, VoiceProviderRegistry};
    /// use std::sync::Arc;
    /// let mut registry = VoiceProviderRegistry::default();
    /// registry.register(Arc::new(StubProvider::default()));
    /// assert!(registry.resolve("stub").is_some());
    /// ```
    pub fn register(&mut self, provider: Arc<dyn VoiceProvider>) {
        self.by_name.insert(provider.name().to_owned(), provider);
    }

    /// Resolve a provider by name.
    ///
    /// # Examples
    ///
    /// ```
    /// use argo_voice_io::VoiceProviderRegistry;
    /// assert!(VoiceProviderRegistry::default().resolve("missing").is_none());
    /// ```
    #[must_use]
    pub fn resolve(&self, name: &str) -> Option<Arc<dyn VoiceProvider>> {
        self.by_name.get(name).cloned()
    }
}
