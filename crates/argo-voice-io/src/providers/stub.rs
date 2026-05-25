//! Deterministic in-memory voice provider.

use crate::manifest::ProviderConfig;
use crate::provider::{
    SpeakHandle, SpeakOptions, SpeakStatus, TranscriptEvent, VoiceCapabilities, VoiceProvider,
    VoiceTransport,
};
use argo_types::{ArgoError, ArgoResult, SessionId};
use futures::future::{ready, BoxFuture};
use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::Instant;

/// In-memory provider for offline tests.
///
/// # Examples
///
/// ```
/// use argo_voice_io::{StubProvider, VoiceProvider};
/// assert_eq!(StubProvider::default().name(), "stub");
/// ```
pub struct StubProvider {
    script: Mutex<Vec<TranscriptEvent>>,
}

impl Default for StubProvider {
    fn default() -> Self {
        Self {
            script: Mutex::new(Vec::new()),
        }
    }
}

impl StubProvider {
    /// Build a provider whose transports start with scripted events.
    ///
    /// # Examples
    ///
    /// ```
    /// use argo_voice_io::{StubProvider, TranscriptEvent};
    /// let provider = StubProvider::with_script(vec![TranscriptEvent::UserInterrupted]);
    /// assert_eq!(provider.script_len(), 1);
    /// ```
    #[must_use]
    pub fn with_script(events: Vec<TranscriptEvent>) -> Self {
        Self {
            script: Mutex::new(events),
        }
    }

    /// Number of scripted events currently configured.
    ///
    /// # Examples
    ///
    /// ```
    /// use argo_voice_io::StubProvider;
    /// assert_eq!(StubProvider::default().script_len(), 0);
    /// ```
    #[must_use]
    pub fn script_len(&self) -> usize {
        match self.script.lock() {
            Ok(script) => script.len(),
            Err(_) => 0,
        }
    }

    fn transport(&self) -> ArgoResult<StubTransport> {
        let script = self
            .script
            .lock()
            .map_err(|_| ArgoError::Invariant("stub provider script lock poisoned".into()))?
            .clone();
        Ok(StubTransport {
            transcripts: VecDeque::from(script),
            synth_count: 0,
            cancel_count: 0,
            speaking: false,
        })
    }
}

impl VoiceProvider for StubProvider {
    fn name(&self) -> &'static str {
        "stub"
    }

    fn capabilities(&self) -> VoiceCapabilities {
        VoiceCapabilities::stub()
    }

    fn open<'a>(
        &'a self,
        _cfg: &'a ProviderConfig,
    ) -> BoxFuture<'a, ArgoResult<Box<dyn VoiceTransport>>> {
        Box::pin(async move { Ok(Box::new(self.transport()?) as Box<dyn VoiceTransport>) })
    }
}

struct StubTransport {
    transcripts: VecDeque<TranscriptEvent>,
    synth_count: u64,
    cancel_count: u64,
    speaking: bool,
}

impl VoiceTransport for StubTransport {
    fn push_audio<'a>(&'a mut self, pcm_frames: &'a [i16]) -> BoxFuture<'a, ArgoResult<()>> {
        if !pcm_frames.is_empty() {
            self.transcripts.push_back(TranscriptEvent::Partial {
                text: format!("stub heard {} frames", pcm_frames.len()),
                is_speaker_continuing: true,
            });
            self.transcripts.push_back(TranscriptEvent::Final {
                text: "stub final transcript".into(),
                ended_at: Instant::now(),
            });
        }
        Box::pin(ready(Ok(())))
    }

    fn poll_transcript(&mut self) -> Option<TranscriptEvent> {
        self.transcripts.pop_front()
    }

    fn synth<'a>(
        &'a mut self,
        _text: &'a str,
        _opts: &'a SpeakOptions,
    ) -> BoxFuture<'a, ArgoResult<SpeakHandle>> {
        self.synth_count = self.synth_count.saturating_add(1);
        self.speaking = true;
        let id = format!("stub-{}-{}", SessionId::new(), self.synth_count);
        Box::pin(ready(Ok(SpeakHandle::new(id, SpeakStatus::Speaking))))
    }

    fn cancel_synth<'a>(&'a mut self) -> BoxFuture<'a, ArgoResult<()>> {
        self.cancel_count = self.cancel_count.saturating_add(1);
        self.speaking = false;
        Box::pin(ready(Ok(())))
    }
}
