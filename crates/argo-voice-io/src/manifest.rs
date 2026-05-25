//! Voice manifest schema and loader.

use crate::provider::VoiceProviderRegistry;
use crate::turn_taking::TurnTakingPolicy;
use argo_types::{ArgoError, ArgoResult};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Provider-neutral voice manifest.
///
/// # Examples
///
/// ```
/// use argo_voice_io::VoiceManifest;
/// let manifest = VoiceManifest::from_toml_str("id='stub'\nprovider='stub'\n[stt]\nlanguage='en'\ninterim_results=true\n[tts]\nvoice_id='demo'\nspeed=1.0\n[turn_taking]\nendpoint_silence_ms=500\nallow_user_interrupt=true\nallow_agent_barge=false").unwrap();
/// assert_eq!(manifest.provider, "stub");
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoiceManifest {
    /// Stable manifest id selected by a vertical or tenant.
    pub id: String,
    /// Provider registry key.
    pub provider: String,
    /// Speech-to-text configuration.
    pub stt: SttConfig,
    /// Text-to-speech configuration.
    pub tts: TtsConfig,
    /// Turn-taking policy for this channel.
    pub turn_taking: TurnTakingPolicy,
}

impl VoiceManifest {
    /// Parse a TOML manifest from a string.
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

    /// Load a manifest and confirm its provider exists in `registry`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use argo_voice_io::{VoiceManifest, VoiceProviderRegistry};
    /// let registry = VoiceProviderRegistry::with_stub();
    /// let _ = VoiceManifest::from_path_with_registry("voice-manifests/stub.toml", &registry);
    /// ```
    pub fn from_path_with_registry(
        path: impl AsRef<Path>,
        registry: &VoiceProviderRegistry,
    ) -> ArgoResult<Self> {
        let manifest = Self::from_path(path)?;
        manifest.validate_provider(registry)?;
        Ok(manifest)
    }

    /// Validate that the provider key resolves in the registry.
    ///
    /// # Examples
    ///
    /// ```
    /// use argo_voice_io::{VoiceManifest, VoiceProviderRegistry};
    /// let mut manifest = VoiceManifest::stub();
    /// assert!(manifest.validate_provider(&VoiceProviderRegistry::with_stub()).is_ok());
    /// manifest.provider = "missing".into();
    /// assert!(manifest.validate_provider(&VoiceProviderRegistry::with_stub()).is_err());
    /// ```
    pub fn validate_provider(&self, registry: &VoiceProviderRegistry) -> ArgoResult<()> {
        if registry.resolve(&self.provider).is_some() {
            return Ok(());
        }
        Err(ArgoError::Schema(format!(
            "voice manifest provider '{}' is not registered",
            self.provider
        )))
    }

    /// Convert this manifest into provider-open configuration.
    ///
    /// # Examples
    ///
    /// ```
    /// use argo_voice_io::VoiceManifest;
    /// assert_eq!(VoiceManifest::stub().provider_config().provider, "stub");
    /// ```
    #[must_use]
    pub fn provider_config(&self) -> ProviderConfig {
        ProviderConfig {
            manifest_id: self.id.clone(),
            provider: self.provider.clone(),
            stt: self.stt.clone(),
            tts: self.tts.clone(),
        }
    }

    /// Deterministic stub manifest used by tests.
    ///
    /// # Examples
    ///
    /// ```
    /// use argo_voice_io::VoiceManifest;
    /// assert_eq!(VoiceManifest::stub().turn_taking.endpoint_silence_ms, 600);
    /// ```
    #[must_use]
    pub fn stub() -> Self {
        Self {
            id: "stub".into(),
            provider: "stub".into(),
            stt: SttConfig {
                language: "en-AU".into(),
                interim_results: true,
            },
            tts: TtsConfig {
                voice_id: Some("stub-voice".into()),
                speed: 1.0,
            },
            turn_taking: TurnTakingPolicy::default(),
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
    /// Whether interim transcript events should be emitted.
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
    /// Provider-specific voice id.
    pub voice_id: Option<String>,
    /// Speech speed multiplier.
    pub speed: f32,
}

/// Provider-open configuration derived from a manifest.
///
/// # Examples
///
/// ```
/// use argo_voice_io::VoiceManifest;
/// let cfg = VoiceManifest::stub().provider_config();
/// assert_eq!(cfg.manifest_id, "stub");
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ProviderConfig {
    /// Source manifest id.
    pub manifest_id: String,
    /// Provider registry key.
    pub provider: String,
    /// Speech-to-text settings.
    pub stt: SttConfig,
    /// Text-to-speech settings.
    pub tts: TtsConfig,
}
