//! Voice manifest schema and loader.
//!
//! A [`VoiceManifest`] is the vendor-selection config (TOML, not Rust — per the
//! "provider manifests, not adapters" invariant): which provider to use, which
//! voice to speak in, and the language to listen in. The real adapter that a
//! `provider` key resolves to is wired at V2 once a vendor API key exists; L1
//! only parses and validates the manifest shape.

use crate::types::VoiceId;
use argo_types::{ArgoError, ArgoResult};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Provider-neutral voice manifest.
///
/// # Examples
///
/// ```
/// use argo_voice_io::VoiceManifest;
/// let manifest = VoiceManifest::from_toml_str(
///     "id='stub'\nprovider='mock'\n[stt]\nlanguage='en-AU'\ninterim_results=true\n[tts]\nvoice_id='demo'\nspeed=1.0",
/// )
/// .unwrap();
/// assert_eq!(manifest.provider, "mock");
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoiceManifest {
    /// Stable manifest id selected by a vertical or tenant.
    pub id: String,
    /// Provider registry key (resolved to a real adapter at V2).
    pub provider: String,
    /// Speech-to-text configuration.
    pub stt: SttConfig,
    /// Text-to-speech configuration.
    pub tts: TtsConfig,
}

impl VoiceManifest {
    /// Parse a TOML manifest from a string.
    ///
    /// # Errors
    ///
    /// Returns [`ArgoError::Schema`] when the TOML is malformed or missing a
    /// required field.
    ///
    /// # Examples
    ///
    /// ```
    /// use argo_voice_io::VoiceManifest;
    /// assert!(VoiceManifest::from_toml_str("not toml").is_err());
    /// ```
    pub fn from_toml_str(raw: &str) -> ArgoResult<Self> {
        toml::from_str(raw).map_err(|e| ArgoError::Schema(format!("voice manifest: {e}")))
    }

    /// Load and parse a TOML manifest from disk.
    ///
    /// # Errors
    ///
    /// Returns [`ArgoError::Io`] when the file cannot be read, or
    /// [`ArgoError::Schema`] when its contents are malformed.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use argo_voice_io::VoiceManifest;
    /// let manifest = VoiceManifest::from_path("voice-manifests/openai-realtime.toml")?;
    /// # Ok::<(), argo_types::ArgoError>(())
    /// ```
    pub fn from_path(path: impl AsRef<Path>) -> ArgoResult<Self> {
        let raw = std::fs::read_to_string(path.as_ref())
            .map_err(|e| ArgoError::Io(format!("read voice manifest: {e}")))?;
        Self::from_toml_str(&raw)
    }

    /// The TTS voice this manifest selects, as a [`VoiceId`], if any.
    ///
    /// This is the value a [`crate::TextToSpeech`] adapter is driven with.
    ///
    /// # Examples
    ///
    /// ```
    /// use argo_voice_io::VoiceManifest;
    /// let manifest = VoiceManifest::mock();
    /// assert_eq!(manifest.tts_voice().map(|v| v.to_string()).as_deref(), Some("mock-voice"));
    /// ```
    #[must_use]
    pub fn tts_voice(&self) -> Option<VoiceId> {
        self.tts.voice_id.as_deref().map(VoiceId::new)
    }

    /// Deterministic mock manifest used by tests.
    ///
    /// # Examples
    ///
    /// ```
    /// use argo_voice_io::VoiceManifest;
    /// assert_eq!(VoiceManifest::mock().provider, "mock");
    /// ```
    #[must_use]
    pub fn mock() -> Self {
        Self {
            id: "mock".into(),
            provider: "mock".into(),
            stt: SttConfig {
                language: "en-AU".into(),
                interim_results: true,
            },
            tts: TtsConfig {
                voice_id: Some("mock-voice".into()),
                speed: 1.0,
            },
        }
    }
}

impl std::str::FromStr for VoiceManifest {
    type Err = ArgoError;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        Self::from_toml_str(raw)
    }
}

/// Speech-to-text manifest settings.
///
/// # Examples
///
/// ```
/// use argo_voice_io::SttConfig;
/// let cfg = SttConfig { language: "en-AU".into(), interim_results: true };
/// assert!(cfg.interim_results);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SttConfig {
    /// BCP-47 language tag requested from the provider.
    pub language: String,
    /// Whether interim transcript events should be emitted (a V2 hint).
    pub interim_results: bool,
}

/// Text-to-speech manifest settings.
///
/// # Examples
///
/// ```
/// use argo_voice_io::TtsConfig;
/// let cfg = TtsConfig { voice_id: Some("alloy".into()), speed: 1.0 };
/// assert_eq!(cfg.voice_id.as_deref(), Some("alloy"));
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TtsConfig {
    /// Provider-specific voice id (becomes a [`VoiceId`] via [`VoiceManifest::tts_voice`]).
    pub voice_id: Option<String>,
    /// Speech speed multiplier (a V2 hint).
    pub speed: f32,
}
