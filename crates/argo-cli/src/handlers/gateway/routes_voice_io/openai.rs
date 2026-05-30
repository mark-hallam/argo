//! Real OpenAI speech adapters behind the `argo-voice-io` traits.
//!
//! Two discrete REST adapters — no Realtime socket (that is a later
//! latency upgrade, DEC-021):
//!
//! - [`OpenAiStt`] implements [`SpeechToText`] via
//!   `POST https://api.openai.com/v1/audio/transcriptions` (multipart,
//!   `whisper-1`).
//! - [`OpenAiTts`] implements [`TextToSpeech`] via
//!   `POST https://api.openai.com/v1/audio/speech` (JSON, `tts-1`).
//!
//! Both read the key from `OPENAI_API_KEY` via
//! [`argo_config::cred::get_unscoped`] (tenant-unscoped: a gateway-process
//! provider secret, like the other transport call sites) and store it in a
//! [`Redacted`] wrapper so it can never leak through `Debug`/serialization.
//! A single [`reqwest::Client`] is built once per adapter.
//!
//! NO THEATER: when a key is present these make real OpenAI calls; there is
//! no canned/fake response path. With no key, construction fails with
//! [`ArgoError::Provider`] (so the selector falls back / surfaces a 503),
//! and the key-gated integration tests early-return.

use argo_types::{ArgoError, ArgoResult, Redacted};
use argo_voice_io::{AudioClip, SpeechToText, TextToSpeech, Transcript, VoiceId};
use async_trait::async_trait;

/// Env var holding the OpenAI key. Read tenant-unscoped (gateway-process
/// secret), mirroring the provider-transport call sites in `judge_eval`.
pub(super) const OPENAI_KEY_ENV: &str = "OPENAI_API_KEY";

const TRANSCRIPTIONS_URL: &str = "https://api.openai.com/v1/audio/transcriptions";
const SPEECH_URL: &str = "https://api.openai.com/v1/audio/speech";

/// Whisper model used for transcription. Discrete (non-Realtime) per DEC-021.
const STT_MODEL: &str = "whisper-1";
/// TTS model used for synthesis.
const TTS_MODEL: &str = "tts-1";
/// We request mp3 from OpenAI; this is the matching MIME for the [`AudioClip`].
const TTS_MIME: &str = "audio/mpeg";
/// Filename + MIME for the uploaded audio part. mp3 is the broadest-support
/// container OpenAI accepts and what the browser push-to-talk layer (V3) will
/// produce; whisper sniffs the real codec from the bytes regardless.
const UPLOAD_FILENAME: &str = "audio.mp3";
const UPLOAD_MIME: &str = "audio/mpeg";

/// The OpenAI voice Skycot's brand voices map onto. `tts-1` accepts a
/// **fixed enum** of vendor voices (`alloy`/`ash`/`coral`/`echo`/`fable`/
/// `nova`/`onyx`/`sage`/`shimmer`); a brand id like `portia` is a 400. So
/// we translate: a Skycot brand voice → its assigned OpenAI voice here,
/// and an already-valid OpenAI voice id passes through unchanged. This
/// keeps "voice as a manifest parameter" (DEC-021) while making the real
/// vendor call valid. Portia → `shimmer` (warm, the demo Pitch Agent).
const PORTIA_OPENAI_VOICE: &str = "shimmer";

/// The set of voice ids `tts-1` accepts verbatim. Anything outside this set
/// (e.g. a Skycot brand name) is mapped to a real voice before the call.
const OPENAI_TTS_VOICES: &[&str] = &[
    "alloy", "ash", "coral", "echo", "fable", "nova", "onyx", "sage", "shimmer",
];

/// Translate a request [`VoiceId`] to a vendor-valid OpenAI voice.
///
/// - An id already in [`OPENAI_TTS_VOICES`] (case-insensitive) passes
///   through lower-cased.
/// - The Skycot brand voice `portia` maps to [`PORTIA_OPENAI_VOICE`].
/// - Any other unknown id falls back to [`PORTIA_OPENAI_VOICE`] rather than
///   sending a value OpenAI will reject with a 400 — the demo voice is the
///   safe default for an unrecognised brand name.
fn map_voice(voice: &VoiceId) -> &'static str {
    let id = voice.as_str().trim().to_ascii_lowercase();
    if let Some(found) = OPENAI_TTS_VOICES.iter().find(|v| ***v == id) {
        return found;
    }
    PORTIA_OPENAI_VOICE
}

/// Read the OpenAI key from the environment/cred store.
///
/// `tenant-unscoped`: the gateway voice loop is a process-level concern
/// (like the webhook HMAC / provider keys), so it reads the unscoped key —
/// the same pattern as the provider transports in `handlers/judge_eval.rs`.
fn read_key() -> ArgoResult<Redacted<String>> {
    // tenant-unscoped: gateway-process provider secret
    let key = argo_config::cred::get_unscoped(OPENAI_KEY_ENV)
        .map_err(|_| ArgoError::Provider(format!("{OPENAI_KEY_ENV} not set")))?;
    if key.trim().is_empty() {
        return Err(ArgoError::Provider(format!("{OPENAI_KEY_ENV} is empty")));
    }
    Ok(Redacted::new(key))
}

/// Build the shared HTTP client once. Kept tiny so both adapters share it.
fn build_client() -> ArgoResult<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent(concat!("argo-voice-openai/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| ArgoError::Provider(format!("openai voice http client: {e}")))
}

/// Real OpenAI speech-to-text adapter (`whisper-1` transcriptions).
#[derive(Clone)]
pub(super) struct OpenAiStt {
    key: Redacted<String>,
    http: reqwest::Client,
}

impl OpenAiStt {
    /// Build the adapter, reading `OPENAI_API_KEY` and constructing the
    /// shared client once.
    ///
    /// # Errors
    /// [`ArgoError::Provider`] when the key is absent/empty or the HTTP
    /// client cannot be built.
    pub(super) fn new() -> ArgoResult<Self> {
        Ok(Self {
            key: read_key()?,
            http: build_client()?,
        })
    }
}

#[async_trait]
impl SpeechToText for OpenAiStt {
    async fn transcribe(&self, audio: &[u8]) -> ArgoResult<Transcript> {
        if audio.is_empty() {
            return Err(ArgoError::Schema("openai stt: empty audio".into()));
        }
        let part = reqwest::multipart::Part::bytes(audio.to_vec())
            .file_name(UPLOAD_FILENAME)
            .mime_str(UPLOAD_MIME)
            .map_err(|e| ArgoError::Provider(format!("openai stt: multipart mime: {e}")))?;
        let form = reqwest::multipart::Form::new()
            .text("model", STT_MODEL)
            .part("file", part);
        let resp = self
            .http
            .post(TRANSCRIPTIONS_URL)
            .bearer_auth(self.key.expose())
            .multipart(form)
            .send()
            .await
            .map_err(|e| ArgoError::Provider(format!("openai stt: POST: {e}")))?;
        let status = resp.status();
        let value: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ArgoError::Provider(format!("openai stt: decode: {e}")))?;
        if !status.is_success() {
            return Err(ArgoError::Provider(format!(
                "openai stt: HTTP {status}: {value}"
            )));
        }
        let text = value
            .get("text")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ArgoError::Provider("openai stt: response missing `text`".into()))?;
        Ok(Transcript::new(text))
    }
}

/// Real OpenAI text-to-speech adapter (`tts-1` speech synthesis).
#[derive(Clone)]
pub(super) struct OpenAiTts {
    key: Redacted<String>,
    http: reqwest::Client,
}

impl OpenAiTts {
    /// Build the adapter, reading `OPENAI_API_KEY` and constructing the
    /// shared client once.
    ///
    /// # Errors
    /// [`ArgoError::Provider`] when the key is absent/empty or the HTTP
    /// client cannot be built.
    pub(super) fn new() -> ArgoResult<Self> {
        Ok(Self {
            key: read_key()?,
            http: build_client()?,
        })
    }
}

#[async_trait]
impl TextToSpeech for OpenAiTts {
    async fn synthesize(&self, text: &str, voice: &VoiceId) -> ArgoResult<AudioClip> {
        if text.trim().is_empty() {
            return Err(ArgoError::Schema("openai tts: empty text".into()));
        }
        let body = serde_json::json!({
            "model": TTS_MODEL,
            // Brand id (e.g. `portia`) → a vendor-valid OpenAI voice; an
            // already-valid OpenAI voice passes through. See `map_voice`.
            "voice": map_voice(voice),
            "input": text,
            "response_format": "mp3",
        });
        let resp = self
            .http
            .post(SPEECH_URL)
            .bearer_auth(self.key.expose())
            .json(&body)
            .send()
            .await
            .map_err(|e| ArgoError::Provider(format!("openai tts: POST: {e}")))?;
        let status = resp.status();
        if !status.is_success() {
            // Body on error is JSON; surface it for diagnosis.
            let detail = resp.text().await.unwrap_or_default();
            return Err(ArgoError::Provider(format!(
                "openai tts: HTTP {status}: {detail}"
            )));
        }
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| ArgoError::Provider(format!("openai tts: read body: {e}")))?;
        Ok(AudioClip::new(TTS_MIME, bytes.to_vec()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Compile-level proof the adapters satisfy the object-safe traits the
    // selector boxes them as. If a trait method signature drifts, this
    // stops compiling — no network, no key needed.
    #[test]
    fn adapters_are_trait_objects() {
        fn assert_stt<T: SpeechToText>() {}
        fn assert_tts<T: TextToSpeech>() {}
        assert_stt::<OpenAiStt>();
        assert_tts::<OpenAiTts>();
    }

    // No-key construction fails cleanly (Provider error), never panics —
    // proves the selector's fallback path is reachable. Only meaningful
    // when the env is actually unset; if a real key is present (CI with a
    // secret, or the operator's shell), `new()` succeeds, which is also
    // correct, so we accept either and only assert it does not panic.
    #[test]
    fn stt_new_does_not_panic_without_key() {
        let _ = OpenAiStt::new();
    }

    #[test]
    fn tts_new_does_not_panic_without_key() {
        let _ = OpenAiTts::new();
    }

    // The brand-voice → OpenAI-voice translation. No network: this is the
    // exact bug a live call hit — `tts-1` rejects `portia` with a 400, so
    // the route must map it to a real voice first.
    #[test]
    fn map_voice_translates_brand_and_passes_through_vendor() {
        // Skycot brand voice → its assigned OpenAI voice.
        assert_eq!(map_voice(&VoiceId::new("portia")), PORTIA_OPENAI_VOICE);
        // Already-valid OpenAI voices pass through (case-insensitive).
        assert_eq!(map_voice(&VoiceId::new("nova")), "nova");
        assert_eq!(map_voice(&VoiceId::new("Shimmer")), "shimmer");
        assert_eq!(map_voice(&VoiceId::new("  alloy  ")), "alloy");
        // Every mapped result is itself a value `tts-1` accepts.
        assert!(OPENAI_TTS_VOICES.contains(&map_voice(&VoiceId::new("portia"))));
        // An unknown brand id falls back to the demo voice, never an
        // invalid value that OpenAI would 400.
        assert_eq!(map_voice(&VoiceId::new("totally-unknown")), PORTIA_OPENAI_VOICE);
        assert!(OPENAI_TTS_VOICES.contains(&map_voice(&VoiceId::new("totally-unknown"))));
    }

    // --- Key-gated live integration tests ---------------------------------
    // These make REAL OpenAI calls. They early-return (skip) when
    // OPENAI_API_KEY is absent, so the suite passes without a key — they
    // are NOT a hard requirement for green. Run them with a key present to
    // exercise the real network path.

    fn key_present() -> bool {
        argo_config::cred::get_unscoped(OPENAI_KEY_ENV)
            .map(|k| !k.trim().is_empty())
            .unwrap_or(false)
    }

    #[tokio::test]
    async fn live_tts_then_stt_roundtrip_when_key_present() {
        if !key_present() {
            eprintln!("skip: {OPENAI_KEY_ENV} not set — live OpenAI voice test skipped");
            return;
        }
        let tts = OpenAiTts::new().expect("tts adapter with key");
        // Use the Skycot BRAND voice id on purpose: this proves `map_voice`
        // translates `portia` → a vendor-valid voice. The earlier `alloy`
        // here masked the live-only 400 the gateway hit with the default.
        let clip = tts
            .synthesize("Hello from Skycot.", &VoiceId::new("portia"))
            .await
            .expect("real tts synth");
        assert_eq!(clip.format(), TTS_MIME);
        assert!(!clip.is_empty(), "expected non-empty mp3 audio");

        let stt = OpenAiStt::new().expect("stt adapter with key");
        let transcript = stt
            .transcribe(clip.bytes())
            .await
            .expect("real stt transcribe");
        assert!(
            !transcript.text().trim().is_empty(),
            "expected non-empty transcript"
        );
    }
}
