//! Unit tests for the structured fields on [`VoiceProfile`]
//! (Stream `mission-voice-profile-richer-fields`). Sibling file to
//! `types_role.rs` so the production file stays under the 500-LOC
//! ceiling, following the `store_skill_tests` pattern.
#![cfg(test)]

use crate::{Formality, Tone, VoiceProfile};

#[test]
fn legacy_preamble_only_composes_verbatim() {
    let voice = VoiceProfile {
        preamble: "Be concise.".into(),
        ..Default::default()
    };
    assert_eq!(voice.compose_preamble(), "Be concise.");
}

#[test]
fn tone_warm_prepends_instruction() {
    let voice = VoiceProfile {
        preamble: "Be Maya.".into(),
        tone: Tone::Warm,
        ..Default::default()
    };
    let composed = voice.compose_preamble();
    assert_eq!(
        composed,
        "Speak with a warm, empathetic tone.\n\nBe Maya."
    );
}

#[test]
fn formality_executive_prepends_instruction() {
    let voice = VoiceProfile {
        preamble: "Brief the operator.".into(),
        formality: Formality::Executive,
        ..Default::default()
    };
    let composed = voice.compose_preamble();
    assert_eq!(
        composed,
        "Use a concise executive-briefing register.\n\nBrief the operator."
    );
}

#[test]
fn lexicon_appends_vocabulary_hints() {
    let voice = VoiceProfile {
        preamble: "Be precise.".into(),
        lexicon: vec!["MTTR".into(), "DAU".into()],
        ..Default::default()
    };
    let composed = voice.compose_preamble();
    assert_eq!(
        composed,
        "Vocabulary you may use: MTTR, DAU.\n\nBe precise."
    );
}

#[test]
fn forbidden_phrases_appends_negation() {
    let voice = VoiceProfile {
        preamble: "Be measured.".into(),
        forbidden_phrases: vec!["synergy".into(), "leverage".into()],
        ..Default::default()
    };
    let composed = voice.compose_preamble();
    assert_eq!(
        composed,
        "Never use these phrases: synergy, leverage.\n\nBe measured."
    );
}

#[test]
fn all_fields_compose_deterministic_order() {
    let voice = VoiceProfile {
        preamble: "Be Maya.".into(),
        tone: Tone::Direct,
        formality: Formality::Professional,
        lexicon: vec!["MTTR".into()],
        forbidden_phrases: vec!["synergy".into()],
    };
    let composed = voice.compose_preamble();
    assert_eq!(
        composed,
        concat!(
            "Speak directly and concisely.\n\n",
            "Use a professional workplace register.\n\n",
            "Vocabulary you may use: MTTR.\n\n",
            "Never use these phrases: synergy.\n\n",
            "Be Maya."
        )
    );
}

#[test]
fn default_tone_neutral() {
    assert_eq!(Tone::default(), Tone::Neutral);
    let voice = VoiceProfile {
        preamble: "P".into(),
        ..Default::default()
    };
    assert_eq!(voice.tone, Tone::Neutral);
    assert_eq!(voice.compose_preamble(), "P");
}

#[test]
fn default_formality_conversational() {
    assert_eq!(Formality::default(), Formality::Conversational);
    let voice = VoiceProfile {
        preamble: "P".into(),
        ..Default::default()
    };
    assert_eq!(voice.formality, Formality::Conversational);
    assert_eq!(voice.compose_preamble(), "P");
}

#[test]
fn legacy_serialised_persona_round_trips() {
    // Existing personas serialised before the structured fields existed
    // (just `{ "preamble": "..." }`) must deserialise unchanged.
    let json = r#"{"preamble":"You are Maya."}"#;
    let voice: VoiceProfile =
        serde_json::from_str(json).expect("legacy preamble JSON must parse");
    assert_eq!(voice.preamble, "You are Maya.");
    assert_eq!(voice.tone, Tone::Neutral);
    assert_eq!(voice.formality, Formality::Conversational);
    assert!(voice.lexicon.is_empty());
    assert!(voice.forbidden_phrases.is_empty());
    assert_eq!(voice.compose_preamble(), "You are Maya.");
}
