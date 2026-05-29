//! Unit tests for the L1 trait layer, mock adapters, and `voice_turn`.

use crate::{
    voice_turn, AudioClip, MockStt, MockTts, SpeechToText, TextToSpeech, Transcript, VoiceId,
    VoiceManifest, MOCK_AUDIO_FORMAT,
};
use argo_types::ArgoError;

// ----- value types -------------------------------------------------------

#[test]
fn transcript_clamps_confidence_into_unit_range() {
    assert_eq!(Transcript::with_confidence("x", 1.7).confidence(), Some(1.0));
    assert_eq!(Transcript::with_confidence("x", -0.4).confidence(), Some(0.0));
    assert_eq!(Transcript::new("x").confidence(), None);
}

#[test]
fn audio_clip_reports_length_and_emptiness() {
    let clip = AudioClip::new("wav", vec![1, 2, 3]);
    assert_eq!(clip.len(), 3);
    assert!(!clip.is_empty());
    assert_eq!(clip.bytes(), &[1, 2, 3]);
    assert!(AudioClip::new("wav", vec![]).is_empty());
}

#[test]
fn voice_id_round_trips_through_str() {
    let voice: VoiceId = "grant".into();
    assert_eq!(voice.as_str(), "grant");
    assert_eq!(voice.to_string(), "grant");
}

// ----- MockTts ------------------------------------------------------------

#[tokio::test]
async fn mock_tts_is_deterministic_for_same_input() {
    let tts = MockTts::new();
    let voice = VoiceId::new("portia");
    let a = tts.synthesize("hello world", &voice).await.expect("synth a");
    let b = tts.synthesize("hello world", &voice).await.expect("synth b");
    assert_eq!(a, b);
    assert_eq!(a.format(), MOCK_AUDIO_FORMAT);
    assert_eq!(a.len(), 32); // sha-256 digest
}

#[tokio::test]
async fn mock_tts_differs_by_text_and_by_voice() {
    let tts = MockTts::new();
    let portia = VoiceId::new("portia");
    let grant = VoiceId::new("grant");
    let same_text_diff_voice = tts.synthesize("hi", &grant).await.expect("synth");
    let base = tts.synthesize("hi", &portia).await.expect("synth");
    let diff_text_same_voice = tts.synthesize("bye", &portia).await.expect("synth");
    assert_ne!(base, same_text_diff_voice);
    assert_ne!(base, diff_text_same_voice);
}

#[tokio::test]
async fn mock_tts_rejects_empty_text() {
    let err = MockTts::new()
        .synthesize("   ", &VoiceId::new("v"))
        .await
        .expect_err("empty text rejected");
    assert!(matches!(err, ArgoError::Schema(_)));
}

// ----- MockStt ------------------------------------------------------------

#[tokio::test]
async fn mock_stt_canned_ignores_audio_and_returns_fixed_text() {
    let stt = MockStt::canned("what is skycot");
    assert_eq!(
        stt.transcribe(b"anything").await.expect("transcribe").text(),
        "what is skycot"
    );
}

#[tokio::test]
async fn mock_stt_echo_is_stable_and_distinct_per_audio() {
    let stt = MockStt::echo();
    let a = stt.transcribe(b"abc").await.expect("a");
    let a2 = stt.transcribe(b"abc").await.expect("a2");
    let b = stt.transcribe(b"xyz").await.expect("b");
    assert_eq!(a, a2);
    assert_ne!(a.text(), b.text());
    assert!(a.text().starts_with("mock-transcript:"));
}

#[tokio::test]
async fn mock_stt_rejects_empty_audio() {
    let err = MockStt::echo()
        .transcribe(&[])
        .await
        .expect_err("empty audio rejected");
    assert!(matches!(err, ArgoError::Schema(_)));
}

#[tokio::test]
async fn mock_round_trip_recovers_synthesized_text() {
    let voice = VoiceId::new("portia");
    let tts = MockTts::new();
    let clip = tts.synthesize("hello there", &voice).await.expect("synth");

    let stt = MockStt::transcribing(voice.clone(), "hello there");
    let heard = stt.transcribe(clip.bytes()).await.expect("transcribe");
    assert_eq!(heard.text(), "hello there");

    // Wrong bytes for the configured text/voice -> provider error.
    let err = stt
        .transcribe(b"not the clip")
        .await
        .expect_err("mismatched audio rejected");
    assert!(matches!(err, ArgoError::Provider(_)));
}

// ----- voice_turn composition --------------------------------------------

#[tokio::test]
async fn voice_turn_wires_stt_then_answer_then_tts() {
    let stt = MockStt::canned("what does skycot do");
    let tts = MockTts::new();
    let voice = VoiceId::new("portia");
    let outcome = voice_turn(&stt, &tts, &voice, b"question-audio", |heard| async move {
        Ok::<_, ArgoError>(format!("you asked: {}", heard.text()))
    })
    .await
    .expect("voice turn");

    assert_eq!(outcome.heard.text(), "what does skycot do");
    assert_eq!(outcome.answer_text, "you asked: what does skycot do");
    // The spoken clip is exactly what MockTts would produce for the answer.
    let expected = AudioClip::new(MOCK_AUDIO_FORMAT, MockTts::bytes_for(&outcome.answer_text, &voice));
    assert_eq!(outcome.spoken, expected);
}

#[tokio::test]
async fn voice_turn_propagates_answer_error_without_synthesizing() {
    let stt = MockStt::canned("trigger");
    let tts = MockTts::new();
    let voice = VoiceId::new("portia");
    let err = voice_turn(&stt, &tts, &voice, b"audio", |_heard| async move {
        Err::<String, _>(ArgoError::Provider("brain refused".into()))
    })
    .await
    .expect_err("answer error propagates");
    assert!(matches!(err, ArgoError::Provider(m) if m == "brain refused"));
}

#[tokio::test]
async fn voice_turn_short_circuits_on_stt_error() {
    let stt = MockStt::echo();
    let tts = MockTts::new();
    let voice = VoiceId::new("portia");
    // Empty audio fails at the STT stage; the answer closure must never run.
    let err = voice_turn(&stt, &tts, &voice, &[], |_heard| async move {
        panic!("answer must not run when STT fails");
        #[allow(unreachable_code)]
        Ok::<_, ArgoError>(String::new())
    })
    .await
    .expect_err("stt error short-circuits");
    assert!(matches!(err, ArgoError::Schema(_)));
}

#[tokio::test]
async fn voice_turn_works_through_trait_objects() {
    // Prove the helper accepts `dyn` adapters (the gateway-route shape at V2).
    let stt: Box<dyn SpeechToText> = Box::new(MockStt::canned("hi"));
    let tts: Box<dyn TextToSpeech> = Box::new(MockTts::new());
    let voice = VoiceId::new("v");
    let outcome = voice_turn(stt.as_ref(), tts.as_ref(), &voice, b"a", |h| async move {
        Ok::<_, ArgoError>(h.text().to_uppercase())
    })
    .await
    .expect("turn via trait objects");
    assert_eq!(outcome.answer_text, "HI");
}

// ----- manifest -----------------------------------------------------------

#[test]
fn manifest_loader_accepts_valid_toml() {
    let manifest = VoiceManifest::from_toml_str(
        "id='m'\nprovider='mock'\n[stt]\nlanguage='en-AU'\ninterim_results=true\n[tts]\nvoice_id='demo'\nspeed=1.0",
    )
    .expect("valid toml");
    assert_eq!(manifest.provider, "mock");
    assert_eq!(manifest.tts_voice().map(|v| v.to_string()).as_deref(), Some("demo"));
}

#[test]
fn manifest_loader_rejects_invalid_toml() {
    assert!(VoiceManifest::from_toml_str("id = ").is_err());
}

#[test]
fn example_manifests_parse_cleanly() {
    for path in [
        "../../voice-manifests/openai-realtime.toml",
        "../../voice-manifests/cartesia.toml",
        "../../voice-manifests/elevenlabs-deepgram.toml",
    ] {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
        let manifest = VoiceManifest::from_path(path).expect("example manifest parses");
        assert!(!manifest.provider.is_empty());
        assert!(manifest.tts_voice().is_some());
    }
}
