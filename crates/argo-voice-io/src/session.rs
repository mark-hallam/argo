//! Voice session lifecycle.

use crate::manifest::VoiceManifest;
use crate::provider::{
    SpeakHandle, SpeakOptions, SpeakStatus, TranscriptEvent, VoiceProviderRegistry, VoiceTransport,
};
use crate::turn_taking::{Speaker, TurnEvent, TurnState, TurnTakingPolicy};
use argo_types::{ArgoError, ArgoResult, SessionId, TenantId};
use futures::{stream, Stream};

/// A single agent-side voice session.
///
/// # Examples
///
/// ```
/// use argo_voice_io::{VoiceManifest, VoiceSession};
/// # async fn demo() -> argo_types::ArgoResult<()> {
/// let session = VoiceSession::open(&VoiceManifest::stub(), argo_types::TenantId::new()).await?;
/// assert_eq!(session.provider_name(), "stub");
/// # Ok(()) }
/// ```
pub struct VoiceSession {
    session_id: SessionId,
    tenant_id: TenantId,
    provider_name: String,
    transport: Box<dyn VoiceTransport>,
    turn_taking: TurnTakingPolicy,
    turn_state: TurnState,
    turn_events: Vec<TurnEvent>,
    current_speech: Option<SpeakHandle>,
}

impl VoiceSession {
    /// Open using the default registry, which currently contains only `stub`.
    ///
    /// # Examples
    ///
    /// ```
    /// use argo_voice_io::{VoiceManifest, VoiceSession};
    /// # async fn demo() -> argo_types::ArgoResult<()> {
    /// let session = VoiceSession::open(&VoiceManifest::stub(), argo_types::TenantId::new()).await?;
    /// assert!(session.session_id().to_string().len() > 10);
    /// # Ok(()) }
    /// ```
    pub async fn open(manifest: &VoiceManifest, tenant_id: TenantId) -> ArgoResult<Self> {
        Self::open_with_registry(manifest, tenant_id, &VoiceProviderRegistry::with_stub()).await
    }

    /// Open using an explicit provider registry.
    ///
    /// # Examples
    ///
    /// ```
    /// use argo_voice_io::{VoiceManifest, VoiceProviderRegistry, VoiceSession};
    /// # async fn demo() -> argo_types::ArgoResult<()> {
    /// let registry = VoiceProviderRegistry::with_stub();
    /// let session = VoiceSession::open_with_registry(&VoiceManifest::stub(), argo_types::TenantId::new(), &registry).await?;
    /// assert_eq!(session.provider_name(), "stub");
    /// # Ok(()) }
    /// ```
    pub async fn open_with_registry(
        manifest: &VoiceManifest,
        tenant_id: TenantId,
        registry: &VoiceProviderRegistry,
    ) -> ArgoResult<Self> {
        manifest.validate_provider(registry)?;
        let provider = registry.resolve(&manifest.provider).ok_or_else(|| {
            ArgoError::Schema(format!(
                "voice provider '{}' disappeared",
                manifest.provider
            ))
        })?;
        let transport = provider.open(&manifest.provider_config()).await?;
        Ok(Self {
            session_id: SessionId::new(),
            tenant_id,
            provider_name: manifest.provider.clone(),
            transport,
            turn_taking: manifest.turn_taking,
            turn_state: TurnState::Idle,
            turn_events: Vec::new(),
            current_speech: None,
        })
    }

    /// Close the voice session.
    ///
    /// # Examples
    ///
    /// ```
    /// use argo_voice_io::{VoiceManifest, VoiceSession};
    /// # async fn demo() -> argo_types::ArgoResult<()> {
    /// VoiceSession::open(&VoiceManifest::stub(), argo_types::TenantId::new()).await?.close().await?;
    /// # Ok(()) }
    /// ```
    pub async fn close(self) -> ArgoResult<()> {
        Ok(())
    }

    /// Session id minted for this voice session.
    ///
    /// # Examples
    ///
    /// ```
    /// use argo_voice_io::{VoiceManifest, VoiceSession};
    /// # async fn demo() -> argo_types::ArgoResult<()> {
    /// let session = VoiceSession::open(&VoiceManifest::stub(), argo_types::TenantId::new()).await?;
    /// let _ = session.session_id();
    /// # Ok(()) }
    /// ```
    #[must_use]
    pub fn session_id(&self) -> SessionId {
        self.session_id
    }

    /// Tenant id supplied by the channel adapter.
    ///
    /// # Examples
    ///
    /// ```
    /// use argo_voice_io::{VoiceManifest, VoiceSession};
    /// # async fn demo() -> argo_types::ArgoResult<()> {
    /// let tenant = argo_types::TenantId::new();
    /// let session = VoiceSession::open(&VoiceManifest::stub(), tenant).await?;
    /// assert_eq!(session.tenant_id(), tenant);
    /// # Ok(()) }
    /// ```
    #[must_use]
    pub fn tenant_id(&self) -> TenantId {
        self.tenant_id
    }

    /// Provider registry key backing this session.
    ///
    /// # Examples
    ///
    /// ```
    /// use argo_voice_io::{VoiceManifest, VoiceSession};
    /// # async fn demo() -> argo_types::ArgoResult<()> {
    /// assert_eq!(VoiceSession::open(&VoiceManifest::stub(), argo_types::TenantId::new()).await?.provider_name(), "stub");
    /// # Ok(()) }
    /// ```
    #[must_use]
    pub fn provider_name(&self) -> &str {
        &self.provider_name
    }

    /// Push user audio into the provider.
    ///
    /// # Examples
    ///
    /// ```
    /// use argo_voice_io::{VoiceManifest, VoiceSession};
    /// # async fn demo() -> argo_types::ArgoResult<()> {
    /// let mut session = VoiceSession::open(&VoiceManifest::stub(), argo_types::TenantId::new()).await?;
    /// session.push_audio(&[1, 2, 3]).await?;
    /// # Ok(()) }
    /// ```
    pub async fn push_audio(&mut self, pcm_frames: &[i16]) -> ArgoResult<()> {
        self.transport.push_audio(pcm_frames).await
    }

    /// Stream currently available transcript events.
    ///
    /// # Examples
    ///
    /// ```
    /// use argo_voice_io::{VoiceManifest, VoiceSession};
    /// use futures::StreamExt;
    /// # async fn demo() -> argo_types::ArgoResult<()> {
    /// let mut session = VoiceSession::open(&VoiceManifest::stub(), argo_types::TenantId::new()).await?;
    /// session.push_audio(&[1]).await?;
    /// let events: Vec<_> = session.transcript_stream().collect().await;
    /// assert!(!events.is_empty());
    /// # Ok(()) }
    /// ```
    pub fn transcript_stream(&mut self) -> impl Stream<Item = ArgoResult<TranscriptEvent>> + '_ {
        stream::iter(self.drain_transcripts())
    }

    /// Synthesize agent speech.
    ///
    /// # Examples
    ///
    /// ```
    /// use argo_voice_io::{SpeakOptions, VoiceManifest, VoiceSession};
    /// # async fn demo() -> argo_types::ArgoResult<()> {
    /// let mut session = VoiceSession::open(&VoiceManifest::stub(), argo_types::TenantId::new()).await?;
    /// let handle = session.speak("hello", SpeakOptions::default()).await?;
    /// assert!(handle.utterance_id.starts_with("stub-"));
    /// # Ok(()) }
    /// ```
    pub async fn speak(&mut self, text: &str, opts: SpeakOptions) -> ArgoResult<SpeakHandle> {
        let handle = self.transport.synth(text, &opts).await?;
        self.turn_state = TurnState::AgentSpeaking;
        self.turn_events.push(TurnEvent::AgentBeganSpeaking);
        self.current_speech = Some(handle.clone());
        Ok(handle)
    }

    /// Interrupt current agent speech.
    ///
    /// # Examples
    ///
    /// ```
    /// use argo_voice_io::{SpeakOptions, VoiceManifest, VoiceSession};
    /// # async fn demo() -> argo_types::ArgoResult<()> {
    /// let mut session = VoiceSession::open(&VoiceManifest::stub(), argo_types::TenantId::new()).await?;
    /// let _ = session.speak("hello", SpeakOptions::default()).await?;
    /// session.interrupt().await?;
    /// assert!(!session.is_agent_speaking());
    /// # Ok(()) }
    /// ```
    pub async fn interrupt(&mut self) -> ArgoResult<()> {
        self.transport.cancel_synth().await?;
        self.finish_agent_speech(SpeakStatus::Interrupted);
        Ok(())
    }

    /// Whether the session currently considers the agent to be speaking.
    ///
    /// # Examples
    ///
    /// ```
    /// use argo_voice_io::{VoiceManifest, VoiceSession};
    /// # async fn demo() -> argo_types::ArgoResult<()> {
    /// let session = VoiceSession::open(&VoiceManifest::stub(), argo_types::TenantId::new()).await?;
    /// assert!(!session.is_agent_speaking());
    /// # Ok(()) }
    /// ```
    #[must_use]
    pub fn is_agent_speaking(&self) -> bool {
        self.turn_state == TurnState::AgentSpeaking
    }

    /// Drain accumulated turn-taking events.
    ///
    /// # Examples
    ///
    /// ```
    /// use argo_voice_io::{SpeakOptions, VoiceManifest, VoiceSession};
    /// # async fn demo() -> argo_types::ArgoResult<()> {
    /// let mut session = VoiceSession::open(&VoiceManifest::stub(), argo_types::TenantId::new()).await?;
    /// let _ = session.speak("hello", SpeakOptions::default()).await?;
    /// assert!(!session.drain_turn_events().is_empty());
    /// # Ok(()) }
    /// ```
    pub fn drain_turn_events(&mut self) -> Vec<TurnEvent> {
        std::mem::take(&mut self.turn_events)
    }

    fn drain_transcripts(&mut self) -> Vec<ArgoResult<TranscriptEvent>> {
        let mut out = Vec::new();
        while let Some(event) = self.transport.poll_transcript() {
            self.apply_transcript_turns(&event);
            out.push(Ok(event));
        }
        out
    }

    fn apply_transcript_turns(&mut self, event: &TranscriptEvent) {
        match event {
            TranscriptEvent::Partial { .. } => {
                if self.turn_state == TurnState::Idle {
                    self.turn_state = TurnState::UserSpeaking;
                    self.turn_events.push(TurnEvent::UserBeganSpeaking);
                }
            }
            TranscriptEvent::Final { .. } | TranscriptEvent::SilenceDetected { .. } => {
                if self.turn_state == TurnState::UserSpeaking {
                    self.turn_state = TurnState::Idle;
                    self.turn_events.push(TurnEvent::UserStoppedSpeaking);
                }
            }
            TranscriptEvent::UserInterrupted => self.apply_user_interrupt(),
        }
    }

    fn apply_user_interrupt(&mut self) {
        if self.turn_taking.allow_user_interrupt && self.turn_state == TurnState::AgentSpeaking {
            let speaker = self.turn_state.speaker().unwrap_or(Speaker::Agent);
            self.turn_events.push(TurnEvent::InterruptionDetected {
                who_was_speaking: speaker,
            });
            self.finish_agent_speech(SpeakStatus::Interrupted);
        }
    }

    fn finish_agent_speech(&mut self, status: SpeakStatus) {
        if let Some(handle) = self.current_speech.as_mut() {
            handle.status = status;
        }
        if self.turn_state == TurnState::AgentSpeaking {
            self.turn_state = TurnState::Idle;
            self.turn_events.push(TurnEvent::AgentFinishedSpeaking);
        }
    }
}
