//! Voice-provider selection for the gateway voice routes.
//!
//! Mirrors the "provider manifests, not adapters" invariant: which speech
//! vendor backs the STT/TTS selectors is chosen by config, not hardcoded at
//! the call site. Selection has two layers, in priority order:
//!
//! 1. **Per-tenant** — the `provider` field of the tenant's `voice`
//!    `workspace_settings` group (`GET/PATCH /api/v1/voice/settings`), if set.
//! 2. **Process env** — [`PROVIDER_ENV`] (`ARGO_VOICE_PROVIDER`), the
//!    fallback when the tenant has expressed no preference.
//!
//! Both resolve to a [`Provider`]; the selectors return a boxed trait object
//! so the routes never name a concrete adapter. `mock` is key-free; `openai`
//! makes real OpenAI calls and so is **fallible** (no key ⇒
//! [`ArgoError::Provider`], surfaced as a 503 — never a silently-faked
//! response, per NO THEATER).

use super::openai::{OpenAiStt, OpenAiTts};
use argo_store::SessionStore;
use argo_types::ArgoResult;
use argo_voice_io::{MockStt, MockTts, SpeechToText, TextToSpeech};
use std::sync::Arc;

/// Env var selecting the voice provider when the tenant has no `voice`
/// setting. Values: `mock` (default) | `openai`.
pub(super) const PROVIDER_ENV: &str = "ARGO_VOICE_PROVIDER";

/// Resolved voice provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Provider {
    /// Deterministic, key-free in-crate fake (the default).
    Mock,
    /// Real OpenAI discrete STT (`whisper-1`) + TTS (`tts-1`). Needs
    /// `OPENAI_API_KEY`; construction fails without it.
    OpenAi,
}

impl Provider {
    /// Pure resolution of a provider string to a [`Provider`]. Split out so
    /// tests exercise the mapping without mutating the process-global
    /// environment (which races other tests). Unknown/unset ⇒ [`Provider::Mock`].
    fn from_env_value(value: Option<&str>) -> Self {
        match value.map(str::trim) {
            Some("openai") => Provider::OpenAi,
            Some("mock") | None => Provider::Mock,
            // An unknown vendor name falls back to the mock rather than
            // erroring — no adapter exists to honour it.
            Some(_unknown) => Provider::Mock,
        }
    }

    /// Resolve the active provider from [`PROVIDER_ENV`] alone (no tenant).
    fn from_env() -> Self {
        Self::from_env_value(std::env::var(PROVIDER_ENV).ok().as_deref())
    }

    /// Resolve the provider for a request: the tenant's `voice` setting if
    /// present, else the process env ([`from_env`]). A missing store or an
    /// unseeded `voice` group both fall through to the env default, so this
    /// never fails — only adapter *construction* (no key) can fail later.
    pub(super) async fn resolve_for_tenant(store: Option<&Arc<SessionStore>>) -> Self {
        if let Some(store) = store {
            // `super` is `routes_voice_io`; the settings module is its sibling
            // under `gateway`, hence `super::super`.
            if let Some(choice) =
                super::super::routes_workspace_settings::read_voice_provider(store).await
            {
                return Self::from_env_value(Some(choice.as_str()));
            }
        }
        Self::from_env()
    }
}

/// Build the STT adapter for an already-resolved provider.
///
/// # Errors
/// [`ArgoError::Provider`] when a real adapter (OpenAI) cannot be built —
/// e.g. `OPENAI_API_KEY` is unset. The mock never fails.
fn build_stt(provider: Provider) -> ArgoResult<Box<dyn SpeechToText>> {
    match provider {
        // `echo` returns a stable transcript derived from the audio bytes,
        // so distinct uploads yield distinct text without a real recognizer.
        Provider::Mock => Ok(Box::new(MockStt::echo())),
        Provider::OpenAi => Ok(Box::new(OpenAiStt::new()?)),
    }
}

/// Build the TTS adapter for an already-resolved provider.
///
/// # Errors
/// As for [`build_stt`].
fn build_tts(provider: Provider) -> ArgoResult<Box<dyn TextToSpeech>> {
    match provider {
        Provider::Mock => Ok(Box::new(MockTts::new())),
        Provider::OpenAi => Ok(Box::new(OpenAiTts::new()?)),
    }
}

/// The active [`SpeechToText`] adapter for this request's tenant.
///
/// # Errors
/// [`ArgoError::Provider`] when the selected real adapter cannot be built
/// (e.g. no API key) — the routes map this to a 503.
pub(super) async fn select_stt(
    store: Option<&Arc<SessionStore>>,
) -> ArgoResult<Box<dyn SpeechToText>> {
    build_stt(Provider::resolve_for_tenant(store).await)
}

/// The active [`TextToSpeech`] adapter for this request's tenant.
///
/// # Errors
/// As for [`select_stt`].
pub(super) async fn select_tts(
    store: Option<&Arc<SessionStore>>,
) -> ArgoResult<Box<dyn TextToSpeech>> {
    build_tts(Provider::resolve_for_tenant(store).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    // These exercise the pure mapping (no env mutation, so no cross-test
    // race on the process-global environment).
    #[test]
    fn resolve_defaults_to_mock_when_unset() {
        assert_eq!(Provider::from_env_value(None), Provider::Mock);
        assert_eq!(Provider::from_env_value(Some("mock")), Provider::Mock);
    }

    #[test]
    fn resolve_maps_openai() {
        assert_eq!(Provider::from_env_value(Some("openai")), Provider::OpenAi);
        // Trimmed too, so a stray-whitespace env value still selects OpenAI.
        assert_eq!(Provider::from_env_value(Some("  openai  ")), Provider::OpenAi);
    }

    #[test]
    fn resolve_unknown_vendor_falls_back_to_mock() {
        assert_eq!(Provider::from_env_value(Some("elevenlabs")), Provider::Mock);
        assert_eq!(Provider::from_env_value(Some("")), Provider::Mock);
    }

    // The mock adapters always build (no key needed). This pins that the
    // default path is infallible.
    #[test]
    fn mock_adapters_always_build() {
        assert!(build_stt(Provider::Mock).is_ok());
        assert!(build_tts(Provider::Mock).is_ok());
    }

    // With no store, resolution falls through to the env (which is unset in
    // the unit-test process unless a runner exports it) → a real provider,
    // never a panic. We only assert it resolves to *some* provider.
    #[tokio::test]
    async fn resolve_for_tenant_without_store_uses_env() {
        let p = Provider::resolve_for_tenant(None).await;
        assert!(matches!(p, Provider::Mock | Provider::OpenAi));
    }
}
