//! Unit tests for the charter-dialogue renderer + question table
//! (Stream `claude-genesis-charter-conversation`). Sibling test
//! file so `charter_dialogue.rs` stays small.
use super::{
    dialogue_to_charter, name_to_persona_slug, CharterDialogue, DialogueError, QUESTIONS,
};

fn maya_dialogue() -> CharterDialogue {
    CharterDialogue {
        role: "marketing intern".into(),
        name: "Maya".into(),
        industry: "B2B SaaS".into(),
        duties: vec![
            "Track competitor product launches".into(),
            "Draft tweets and LinkedIn posts".into(),
            "Respond to inbound demo requests".into(),
        ],
        channels: vec!["slack-dm".into(), "email".into()],
        spend_cap_per_day_usd: 50.0,
    }
}

#[test]
fn dialogue_to_charter_renders_all_fields_in_expected_order() {
    let d = maya_dialogue();
    let s = dialogue_to_charter(&d);
    assert!(s.starts_with("Be my marketing intern, named Maya, working in B2B SaaS."));
    assert!(s.contains("1. Track competitor product launches"));
    assert!(s.contains("2. Draft tweets and LinkedIn posts"));
    assert!(s.contains("3. Respond to inbound demo requests"));
    assert!(s.contains("For approvals, use these channels: slack-dm, email."));
    assert!(s.contains("Daily spend cap: $50.00 USD."));
}

#[test]
fn dialogue_to_charter_omits_industry_when_blank() {
    let mut d = maya_dialogue();
    d.industry = "".into();
    let s = dialogue_to_charter(&d);
    assert!(s.starts_with("Be my marketing intern, named Maya."));
    assert!(!s.contains("working in"));
}

#[test]
fn dialogue_to_charter_drops_blank_duty_lines() {
    let mut d = maya_dialogue();
    d.duties = vec![
        "First duty".into(),
        "  ".into(), // whitespace-only — should be dropped
        "Second duty".into(),
    ];
    let s = dialogue_to_charter(&d);
    assert!(s.contains("1. First duty"));
    assert!(s.contains("2. Second duty"));
    assert!(!s.contains("3."));
}

#[test]
fn validate_rejects_missing_required_fields() {
    let mut d = maya_dialogue();
    d.role = "".into();
    assert_eq!(d.validate().unwrap_err(), DialogueError::MissingField("role"));

    let mut d = maya_dialogue();
    d.name = "  ".into();
    assert_eq!(d.validate().unwrap_err(), DialogueError::MissingField("name"));

    let mut d = maya_dialogue();
    d.duties = vec!["".into(), "  ".into()];
    assert_eq!(
        d.validate().unwrap_err(),
        DialogueError::MissingField("duties")
    );

    let mut d = maya_dialogue();
    d.channels = vec!["".into()];
    assert_eq!(
        d.validate().unwrap_err(),
        DialogueError::MissingField("channels")
    );
}

#[test]
fn validate_rejects_zero_or_negative_spend_cap() {
    let mut d = maya_dialogue();
    d.spend_cap_per_day_usd = 0.0;
    assert_eq!(
        d.validate().unwrap_err(),
        DialogueError::InvalidSpendCap(0.0)
    );

    let mut d = maya_dialogue();
    d.spend_cap_per_day_usd = -1.0;
    assert!(matches!(
        d.validate().unwrap_err(),
        DialogueError::InvalidSpendCap(_)
    ));

    let mut d = maya_dialogue();
    d.spend_cap_per_day_usd = f64::NAN;
    assert!(matches!(
        d.validate().unwrap_err(),
        DialogueError::InvalidSpendCap(_)
    ));
}

#[test]
fn validate_passes_for_well_formed_dialogue() {
    assert!(maya_dialogue().validate().is_ok());
}

#[test]
fn name_to_persona_slug_lowercases_and_trims_punctuation() {
    assert_eq!(name_to_persona_slug("Maya"), "maya");
    assert_eq!(name_to_persona_slug("Sam J."), "sam-j");
    assert_eq!(name_to_persona_slug("Maria_Garcia"), "maria-garcia");
    assert_eq!(name_to_persona_slug("  Riley  "), "riley");
    // emoji + punctuation gets dropped
    assert_eq!(name_to_persona_slug("Alex! 🚀"), "alex");
    // entirely empty / non-alphanumeric falls back to "worker"
    assert_eq!(name_to_persona_slug(""), "worker");
    assert_eq!(name_to_persona_slug("@@@"), "worker");
}

#[test]
fn questions_table_covers_every_dialogue_field() {
    let keys: Vec<&str> = QUESTIONS.iter().map(|q| q.key).collect();
    assert_eq!(
        keys,
        vec!["role", "name", "industry", "duties", "channels", "spend_cap"]
    );
    // The brief calls for "5+" prompts; we ship 6.
    assert_eq!(QUESTIONS.len(), 6);
}

#[test]
fn questions_table_marks_duties_as_multi_and_industry_optional() {
    let by_key: std::collections::HashMap<&str, &super::Question> =
        QUESTIONS.iter().map(|q| (q.key, q)).collect();
    assert!(by_key["duties"].multi);
    assert!(by_key["duties"].required);
    assert!(!by_key["industry"].required);
    assert!(by_key["role"].required);
    assert!(by_key["name"].required);
}

#[test]
fn json_roundtrip_preserves_every_field() {
    // The cli handler uses serde_yaml for `--non-interactive`
    // YAML loading; here we exercise the serde derives via the
    // already-available serde_json so this crate doesn't pick up
    // a new YAML dep just for tests.
    let d = maya_dialogue();
    let json = serde_json::to_string(&d).expect("serialize");
    let back: CharterDialogue = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(d, back);
}

#[test]
fn deserialize_supplies_industry_default_when_omitted() {
    // industry has #[serde(default)] — operators can omit it
    // entirely from --non-interactive YAML / JSON. The yaml-side
    // load is tested in argo-cli; this test pins the
    // serde-default behaviour at the type definition.
    let json = r#"{"role":"marketing intern","name":"Maya","duties":["a"],"channels":["slack"],"spend_cap_per_day_usd":50}"#;
    let d: CharterDialogue = serde_json::from_str(json).expect("deserialize");
    assert_eq!(d.industry, "");
}
