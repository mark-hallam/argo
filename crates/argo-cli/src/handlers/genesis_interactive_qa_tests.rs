//! Unit tests for the genesis interactive Q&A loop. Sibling test
//! file so the host module stays focused on the loop logic.
use super::{apply_answer, prompt_dialogue, LineReader};
use argo_genesis::CharterDialogue;
use argo_types::ArgoResult;

/// Tiny in-memory line reader for tests. Pops lines off a queue;
/// returns `None` at queue exhaustion to model EOF.
struct MockReader {
    lines: std::collections::VecDeque<String>,
}

impl MockReader {
    fn new(lines: &[&str]) -> Self {
        Self {
            lines: lines.iter().map(|s| s.to_string()).collect(),
        }
    }
}

impl LineReader for MockReader {
    fn read_line(&mut self) -> ArgoResult<Option<String>> {
        Ok(self.lines.pop_front())
    }
}

#[test]
fn happy_path_captures_every_field() {
    // Mirrors the example flow in
    // docs/build-plan/streams/claude-genesis-charter-conversation.md.
    let mut reader = MockReader::new(&[
        "marketing intern",
        "Maya",
        "B2B SaaS",
        // duties: 3 lines + blank to terminate
        "Track competitor product launches",
        "Draft tweets and LinkedIn posts",
        "Respond to inbound demo requests",
        "",
        "slack-dm, email",
        "50",
    ]);
    let mut writer: Vec<u8> = Vec::new();
    let dialogue = prompt_dialogue(&mut reader, &mut writer).expect("loop returns");
    assert_eq!(dialogue.role, "marketing intern");
    assert_eq!(dialogue.name, "Maya");
    assert_eq!(dialogue.industry, "B2B SaaS");
    assert_eq!(dialogue.duties.len(), 3);
    assert_eq!(dialogue.channels, vec!["slack-dm", "email"]);
    assert!((dialogue.spend_cap_per_day_usd - 50.0).abs() < f64::EPSILON);
}

#[test]
fn industry_can_be_skipped_with_blank_line() {
    let mut reader = MockReader::new(&[
        "support agent", "Sam", "", // industry skipped
        "answer tickets", "",
        "email", "10",
    ]);
    let mut writer: Vec<u8> = Vec::new();
    let dialogue = prompt_dialogue(&mut reader, &mut writer).expect("loop");
    assert_eq!(dialogue.industry, "");
    assert_eq!(dialogue.duties, vec!["answer tickets"]);
}

#[test]
fn empty_required_answer_reprompts_until_filled() {
    // role is required — supply blank then a real answer.
    let mut reader = MockReader::new(&[
        "", "  ", "marketing intern",
        "Maya",
        "",
        "ship copy", "",
        "slack",
        "5",
    ]);
    let mut writer: Vec<u8> = Vec::new();
    let dialogue = prompt_dialogue(&mut reader, &mut writer).expect("loop");
    assert_eq!(dialogue.role, "marketing intern");
    let echoed = String::from_utf8(writer).expect("utf-8");
    assert!(echoed.contains("(required — please answer)"));
}

#[test]
fn dollar_sign_prefix_on_spend_cap_is_tolerated() {
    let mut reader = MockReader::new(&[
        "marketing intern",
        "Maya",
        "",
        "ship copy", "",
        "slack",
        "$25.50",
    ]);
    let mut writer: Vec<u8> = Vec::new();
    let dialogue = prompt_dialogue(&mut reader, &mut writer).expect("loop");
    assert!((dialogue.spend_cap_per_day_usd - 25.5).abs() < f64::EPSILON);
}

#[test]
fn non_numeric_spend_cap_returns_error() {
    let mut reader = MockReader::new(&[
        "marketing intern",
        "Maya",
        "",
        "ship copy", "",
        "slack",
        "fifty",
    ]);
    let mut writer: Vec<u8> = Vec::new();
    let err = prompt_dialogue(&mut reader, &mut writer).expect_err("non-numeric");
    assert!(err.to_string().contains("spend cap"));
}

#[test]
fn channels_split_on_commas_and_trim_whitespace() {
    let mut d = CharterDialogue {
        role: "x".into(),
        name: "x".into(),
        industry: String::new(),
        duties: Vec::new(),
        channels: Vec::new(),
        spend_cap_per_day_usd: 0.0,
    };
    apply_answer(
        "channels",
        "  slack-dm , email,  push  ",
        &["  slack-dm , email,  push  ".into()],
        &mut d,
    )
    .expect("apply");
    assert_eq!(d.channels, vec!["slack-dm", "email", "push"]);
}

#[test]
fn duties_filter_blank_lines() {
    let mut d = CharterDialogue {
        role: "x".into(),
        name: "x".into(),
        industry: String::new(),
        duties: Vec::new(),
        channels: Vec::new(),
        spend_cap_per_day_usd: 0.0,
    };
    apply_answer(
        "duties",
        "first\n\nsecond\n",
        &[
            "first".into(),
            "  ".into(),
            "second".into(),
            "".into(),
        ],
        &mut d,
    )
    .expect("apply");
    assert_eq!(d.duties, vec!["first", "second"]);
}

#[test]
fn unknown_question_key_returns_error() {
    let mut d = CharterDialogue {
        role: String::new(),
        name: String::new(),
        industry: String::new(),
        duties: Vec::new(),
        channels: Vec::new(),
        spend_cap_per_day_usd: 0.0,
    };
    let err = apply_answer("nope", "x", &["x".into()], &mut d).expect_err("unknown");
    assert!(err.to_string().contains("unknown question key"));
}

#[test]
fn writer_records_introductory_banner() {
    let mut reader = MockReader::new(&[
        "x", "y", "", "z", "", "slack", "1",
    ]);
    let mut writer: Vec<u8> = Vec::new();
    let _ = prompt_dialogue(&mut reader, &mut writer).expect("loop");
    let echoed = String::from_utf8(writer).expect("utf-8");
    assert!(echoed.starts_with("argo genesis interactive"));
    assert!(echoed.contains("--output"));
}
