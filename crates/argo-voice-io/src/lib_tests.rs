use crate::{
    SpeakOptions, Speaker, StubProvider, TranscriptEvent, TurnEvent, VoiceManifest,
    VoiceProviderRegistry, VoiceSession,
};
use argo_types::TenantId;
use futures::StreamExt;
use std::sync::Arc;

#[tokio::test]
async fn open_and_close_session() {
    let session = VoiceSession::open(&VoiceManifest::stub(), TenantId::new())
        .await
        .expect("stub session opens");
    assert_eq!(session.provider_name(), "stub");
    session.close().await.expect("close is ok");
}

#[tokio::test]
async fn push_audio_yields_partial_and_final_transcripts() {
    let mut session = VoiceSession::open(&VoiceManifest::stub(), TenantId::new())
        .await
        .expect("stub session opens");
    session.push_audio(&[1, 2, 3]).await.expect("push audio");
    let events: Vec<_> = session
        .transcript_stream()
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .map(|r| r.expect("event ok"))
        .collect();
    assert!(matches!(events[0], TranscriptEvent::Partial { .. }));
    assert!(matches!(events[1], TranscriptEvent::Final { .. }));
}

#[tokio::test]
async fn speak_and_interrupt_mid_utterance() {
    let mut session = VoiceSession::open(&VoiceManifest::stub(), TenantId::new())
        .await
        .expect("stub session opens");
    let handle = session
        .speak("hello", SpeakOptions::default())
        .await
        .expect("speak");
    assert!(handle.utterance_id.starts_with("stub-"));
    assert!(session.is_agent_speaking());
    session.interrupt().await.expect("interrupt");
    assert!(!session.is_agent_speaking());
}

#[tokio::test]
async fn user_interruption_auto_detects_agent_interrupt() {
    let mut registry = VoiceProviderRegistry::default();
    registry.register(Arc::new(StubProvider::with_script(vec![
        TranscriptEvent::UserInterrupted,
    ])));
    let mut session =
        VoiceSession::open_with_registry(&VoiceManifest::stub(), TenantId::new(), &registry)
            .await
            .expect("stub session opens");
    session
        .speak("pitch line", SpeakOptions::default())
        .await
        .expect("speak");
    let events: Vec<_> = session.transcript_stream().collect().await;
    assert_eq!(events.len(), 1);
    assert!(!session.is_agent_speaking());
    assert!(session
        .drain_turn_events()
        .contains(&TurnEvent::InterruptionDetected {
            who_was_speaking: Speaker::Agent,
        },));
}

#[tokio::test]
async fn tenant_id_is_per_session_and_does_not_cross_talk() {
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();
    let mut a = VoiceSession::open(&VoiceManifest::stub(), tenant_a)
        .await
        .expect("session a");
    let mut b = VoiceSession::open(&VoiceManifest::stub(), tenant_b)
        .await
        .expect("session b");
    a.push_audio(&[1]).await.expect("push a");
    assert_eq!(a.tenant_id(), tenant_a);
    assert_eq!(b.tenant_id(), tenant_b);
    assert_ne!(a.session_id(), b.session_id());
    assert_eq!(b.transcript_stream().count().await, 0);
    assert_eq!(a.transcript_stream().count().await, 2);
}

#[test]
fn manifest_loader_accepts_valid_toml() {
    let manifest = VoiceManifest::from_toml_str(
        "id='stub'\nprovider='stub'\n[stt]\nlanguage='en-AU'\ninterim_results=true\n[tts]\nvoice_id='demo'\nspeed=1.0\n[turn_taking]\nendpoint_silence_ms=600\nallow_user_interrupt=true\nallow_agent_barge=false",
    )
    .expect("valid toml");
    assert_eq!(manifest.provider, "stub");
}

#[test]
fn manifest_loader_rejects_invalid_toml() {
    assert!(VoiceManifest::from_toml_str("id = ").is_err());
}

#[test]
fn provider_registry_resolves_known_provider() {
    assert!(VoiceProviderRegistry::with_stub().resolve("stub").is_some());
}

#[test]
fn provider_registry_rejects_unknown_provider() {
    assert!(VoiceProviderRegistry::with_stub()
        .resolve("missing")
        .is_none());
}

#[test]
fn manifest_validation_rejects_unknown_provider() {
    let mut manifest = VoiceManifest::stub();
    manifest.provider = "cartesia".into();
    assert!(manifest
        .validate_provider(&VoiceProviderRegistry::with_stub())
        .is_err());
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
    }
}
