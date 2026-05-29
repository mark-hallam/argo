//! Voice-provider selection for the gateway voice routes.
//!
//! Mirrors the "provider manifests, not adapters" invariant: which speech
//! vendor backs [`select_stt`] / [`select_tts`] is chosen by config
//! ([`PROVIDER_ENV`]), not hardcoded at the call site. The selector returns
//! a boxed trait object so a real adapter can drop in behind the same seam
//! without touching the routes.
//!
//! BLOCKED: real STT/TTS adapter needs operator API key — so every value of
//! [`PROVIDER_ENV`] currently resolves to the deterministic, key-free mock
//! ([`MockStt`] / [`MockTts`]). When a vendor key lands, add the real
//! adapter arm here (gated on the provider string) and nothing upstream
//! changes.

use argo_voice_io::{MockStt, MockTts, SpeechToText, TextToSpeech};

/// Env var selecting the voice provider. Reserved for the real-adapter
/// swap; today any value (or its absence) resolves to [`Provider::Mock`].
pub(super) const PROVIDER_ENV: &str = "ARGO_VOICE_PROVIDER";

/// Resolved voice provider. Only [`Provider::Mock`] is wired today; real
/// vendors are BLOCKED on an operator API key (see module docs).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Provider {
    /// Deterministic, key-free in-crate fake — the only wired provider.
    Mock,
}

impl Provider {
    /// Resolve the active provider from [`PROVIDER_ENV`].
    ///
    /// Unknown/unset values fall back to [`Provider::Mock`] rather than
    /// erroring, because no real adapter exists yet to honour them.
    pub(super) fn resolve() -> Self {
        Self::from_env_value(std::env::var(PROVIDER_ENV).ok().as_deref())
    }

    /// Pure resolution of a provider string to a [`Provider`]. Split from
    /// [`resolve`] so tests exercise the mapping without mutating the
    /// process-global environment (which races other tests).
    fn from_env_value(value: Option<&str>) -> Self {
        match value.map(str::trim) {
            // BLOCKED: real STT/TTS adapter needs operator API key. Each
            // named vendor gets its own arm here once its adapter lands,
            // e.g. `Some("elevenlabs") => Provider::ElevenLabs`. Until then
            // every value (including a vendor name an operator might set
            // early) deliberately resolves to the mock.
            Some("mock") | None => Provider::Mock,
            Some(_unwired_vendor) => Provider::Mock,
        }
    }
}

/// The active [`SpeechToText`] adapter. Boxed so a real adapter swaps in by
/// config without changing the route signatures.
pub(super) fn select_stt() -> Box<dyn SpeechToText> {
    match Provider::resolve() {
        // `echo` returns a stable transcript derived from the audio bytes,
        // so distinct uploads yield distinct text without a real recognizer.
        Provider::Mock => Box::new(MockStt::echo()),
    }
}

/// The active [`TextToSpeech`] adapter. Boxed so a real adapter swaps in by
/// config without changing the route signatures.
pub(super) fn select_tts() -> Box<dyn TextToSpeech> {
    match Provider::resolve() {
        Provider::Mock => Box::new(MockTts::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // These exercise the pure mapping (no env mutation, so no cross-test
    // race on the process-global environment).
    #[test]
    fn resolve_defaults_to_mock_when_unset() {
        assert_eq!(Provider::from_env_value(None), Provider::Mock);
    }

    #[test]
    fn resolve_falls_back_to_mock_for_unknown_provider() {
        // No real adapter exists yet, so even a named vendor resolves to
        // Mock (the BLOCKED seam). Pin this so the fallback is intentional.
        assert_eq!(Provider::from_env_value(Some("elevenlabs")), Provider::Mock);
        assert_eq!(Provider::from_env_value(Some("  openai  ")), Provider::Mock);
    }
}
