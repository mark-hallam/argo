//! Unit tests for [`crate::context_truncate`]. Included via
//! `#[path]` from `context_truncate.rs` so symbols stay private to
//! the parent while the test file keeps the parent under the
//! 500-LOC ceiling. 8 tests per brief acceptance.

use super::*;
use crate::context_compress::TokenCounter;
use argo_types::{
    ArgoError, ContentBlock, Message, MessageId, MessageRole, SessionId, ToolCallId,
};

/// Word-count tokenizer kept deterministic without dragging in a
/// real tokenizer crate. Matches the helper used in
/// `context_compress_tests.rs`.
struct WordCounter;

impl TokenCounter for WordCounter {
    fn count_message(&self, msg: &Message) -> usize {
        msg.content
            .iter()
            .map(|c| match c {
                ContentBlock::Text { text } => self.count_text(text),
                _ => 0,
            })
            .sum::<usize>()
            .saturating_add(1) // role overhead
    }
    fn count_text(&self, text: &str) -> usize {
        text.split_whitespace().count()
    }
}

fn msg(role: MessageRole, session: SessionId, text: &str) -> Message {
    Message {
        id: MessageId::new(),
        session_id: session,
        role,
        content: vec![ContentBlock::Text { text: text.into() }],
        tool_calls: vec![],
        tool_call_id: None,
        reasoning: None,
        created_at: chrono::Utc::now(),
        provider_meta: serde_json::Value::Null,
    }
}

fn tool_msg(session: SessionId, text: &str) -> Message {
    let mut m = msg(MessageRole::Tool, session, text);
    m.tool_call_id = Some(ToolCallId::new());
    m
}

fn text_of(msg: &Message) -> String {
    match msg.content.first() {
        Some(ContentBlock::Text { text }) => text.clone(),
        _ => String::new(),
    }
}

#[test]
fn fits_already_is_a_passthrough() {
    // Acceptance: callers expect zero-cost passthrough when the
    // history is already small enough.
    let session = SessionId::new();
    let messages = vec![
        msg(MessageRole::System, session, "you are helpful"),
        msg(MessageRole::User, session, "hi"),
        msg(MessageRole::Assistant, session, "hello"),
    ];
    let result = truncate_to_fit(&messages, 1_000, &WordCounter).expect("fits");
    assert!(!result.truncated());
    assert_eq!(result.turns_dropped, 0);
    assert_eq!(result.intra_turn_truncations, 0);
    assert_eq!(result.view.len(), 3);
    assert_eq!(result.pre_token_count, result.post_token_count);
}

#[test]
fn drops_one_older_turn_fifo() {
    // Acceptance: "Older user/assistant turns (drop oldest first)".
    let session = SessionId::new();
    let messages = vec![
        msg(MessageRole::System, session, "you are helpful"),
        msg(
            MessageRole::User,
            session,
            "old question one two three four five",
        ),
        msg(MessageRole::Assistant, session, "old answer"),
        msg(MessageRole::User, session, "new question"),
        msg(MessageRole::Assistant, session, "new answer"),
    ];
    let pre: usize = messages.iter().map(|m| WordCounter.count_message(m)).sum();
    let cap = pre - 1; // force at least one drop
    let result = truncate_to_fit(&messages, cap, &WordCounter).expect("truncate");
    assert!(result.truncated());
    assert!(result.turns_dropped >= 1);
    assert!(result.post_token_count <= cap);
    // Last user must survive.
    let last_user_text = result
        .view
        .iter()
        .rev()
        .find(|m| matches!(m.role, MessageRole::User))
        .map(text_of);
    assert_eq!(last_user_text.as_deref(), Some("new question"));
    // System must survive.
    assert!(result.view.iter().any(|m| matches!(m.role, MessageRole::System)));
}

#[test]
fn drops_many_oldest_first_keeping_protected_set() {
    // Acceptance: "System prompt + last user + last assistant always
    // preserved verbatim". Drops are FIFO from the oldest end.
    let session = SessionId::new();
    let mut messages = vec![msg(MessageRole::System, session, "sys")];
    for i in 0..10 {
        messages.push(msg(
            MessageRole::User,
            session,
            &format!("u{i} payload payload payload"),
        ));
        messages.push(msg(
            MessageRole::Assistant,
            session,
            &format!("a{i} reply reply"),
        ));
    }
    let sys_count = WordCounter.count_message(&messages[0]);
    let last_user_count = WordCounter.count_message(&messages[messages.len() - 2]);
    let last_asst_count = WordCounter.count_message(&messages[messages.len() - 1]);
    let cap = sys_count + last_user_count + last_asst_count;

    let result = truncate_to_fit(&messages, cap, &WordCounter).expect("truncate");
    assert!(result.post_token_count <= cap);
    // 18 older messages (idx 1..=18) must be dropped — only sys, u9,
    // a9 remain.
    assert_eq!(result.turns_dropped, 18);
    assert_eq!(result.intra_turn_truncations, 0);
    let view_texts: Vec<String> = result.view.iter().map(text_of).collect();
    assert!(!view_texts.iter().any(|t| t.starts_with("u0 ")));
    assert!(!view_texts.iter().any(|t| t.starts_with("a0 ")));
    assert!(view_texts.iter().any(|t| t.starts_with("u9 ")));
    assert!(view_texts.iter().any(|t| t.starts_with("a9 ")));
    assert!(view_texts.iter().any(|t| t == "sys"));
}

#[test]
fn intra_turn_truncates_giant_tool_output() {
    // Acceptance: "Intra-turn truncation triggers only when a single
    // turn > 50% of window" — and the marker matches the brief.
    let session = SessionId::new();
    let mut giant_text = String::new();
    for _ in 0..200 {
        giant_text.push_str("blob ");
    }
    let messages = vec![
        msg(MessageRole::System, session, "sys"),
        msg(MessageRole::User, session, "old"),
        tool_msg(session, &giant_text),
        msg(MessageRole::User, session, "new"),
        msg(MessageRole::Assistant, session, "new reply"),
    ];

    // Cap forces both a drop and an intra-turn truncation: the tool
    // message is protected (recent tool result) and on its own
    // exceeds 50% of the window.
    let result = truncate_to_fit(&messages, 50, &WordCounter).expect("truncate");
    assert!(result.truncated());
    assert!(result.intra_turn_truncations >= 1);
    assert!(result.post_token_count <= 50);

    let marker_present = result.view.iter().any(|m| {
        let t = text_of(m);
        t.starts_with("[...truncated") && t.contains("tokens of tool output...]")
    });
    assert!(marker_present, "intra-turn marker should appear in view");
}

#[test]
fn protects_recent_tool_results_drops_older_tool_results() {
    // Acceptance: "Tool-result turns from the last N turns (default
    // 3) — load-bearing for tool-call chains".
    let session = SessionId::new();
    let mut messages = vec![msg(MessageRole::System, session, "sys")];
    for i in 0..6 {
        messages.push(msg(MessageRole::User, session, &format!("u{i}")));
        messages.push(tool_msg(session, &format!("t{i} result data more data")));
        messages.push(msg(MessageRole::Assistant, session, &format!("a{i}")));
    }
    let pre: usize = messages.iter().map(|m| WordCounter.count_message(m)).sum();
    let cap = pre / 2;

    let result = truncate_to_fit(&messages, cap, &WordCounter).expect("truncate");
    assert!(result.post_token_count <= cap);

    let tool_texts: Vec<String> = result
        .view
        .iter()
        .filter(|m| matches!(m.role, MessageRole::Tool))
        .map(text_of)
        .collect();
    // Last 3 user-turn boundaries protect t3, t4, t5.
    assert!(tool_texts.iter().any(|t| t.starts_with("t3 ")));
    assert!(tool_texts.iter().any(|t| t.starts_with("t4 ")));
    assert!(tool_texts.iter().any(|t| t.starts_with("t5 ")));
    // Older tool results dropped.
    assert!(!tool_texts.iter().any(|t| t.starts_with("t0 ")));
}

#[test]
fn hard_fail_when_minimum_exceeds_window() {
    // Acceptance: "Return a typed `ArgoError::ContextWindowExhausted`
    // — caller must handle". Even after intra-turn truncation, the
    // marker plus the rest cannot fit if the cap is below the
    // marker's own footprint for sys + last user.
    let session = SessionId::new();
    let mut huge = String::new();
    for _ in 0..50 {
        huge.push_str("word ");
    }
    let messages = vec![
        msg(MessageRole::System, session, &huge),
        msg(MessageRole::User, session, &huge),
    ];

    let cap = 5;
    let err = truncate_to_fit(&messages, cap, &WordCounter).expect_err("must hard-fail");
    match err {
        ArgoError::ContextWindowExhausted {
            minimum_tokens,
            window_tokens,
        } => {
            assert!(minimum_tokens > window_tokens);
            assert_eq!(window_tokens, cap);
        }
        other => panic!("expected ContextWindowExhausted, got {other:?}"),
    }
}

#[test]
fn empty_history_is_a_zero_cost_passthrough() {
    // Edge: callers may pass an empty slice during session bootstrap;
    // the function must not panic and must report zero work.
    let result = truncate_to_fit(&[], 100, &WordCounter).expect("empty fits");
    assert_eq!(result.view.len(), 0);
    assert_eq!(result.pre_token_count, 0);
    assert_eq!(result.post_token_count, 0);
    assert_eq!(result.turns_dropped, 0);
    assert_eq!(result.intra_turn_truncations, 0);
    assert!(!result.truncated());
}

#[test]
fn audit_payload_and_label_match_brief() {
    // Acceptance: "Audit row per truncation (kind:
    // runtime.context_truncated) with { pre_token_count,
    // post_token_count, turns_dropped, intra_turn_truncations }".
    let result = TruncationResult {
        view: vec![],
        pre_token_count: 200_000,
        post_token_count: 180_000,
        turns_dropped: 12,
        intra_turn_truncations: 1,
    };
    let payload = build_audit_payload(&result);
    assert_eq!(payload["pre_token_count"], 200_000);
    assert_eq!(payload["post_token_count"], 180_000);
    assert_eq!(payload["turns_dropped"], 12);
    assert_eq!(payload["intra_turn_truncations"], 1);
    assert_eq!(AUDIT_KIND_LABEL, "runtime.context_truncated");
}
