//! Unit tests for [`crate::context_compress`]. Included via
//! `#[path]` from `context_compress.rs` so symbols stay private to
//! the parent while the test file keeps the parent under the 500-LOC
//! ceiling.
//!
//! The brief calls for 7 tests; each pins one acceptance-criteria
//! row from `streams/mission-context-window-aware-compression.md`.

use super::*;
use argo_types::{ContentBlock, Message, MessageId, MessageRole, SessionId};

/// Word-count tokenizer used to keep tests deterministic without
/// dragging in a real tokenizer crate.
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
            // role overhead
            .saturating_add(1)
    }
    fn count_text(&self, text: &str) -> usize {
        text.split_whitespace().count()
    }
}

/// Always-succeeds summariser that emits a fixed string with a
/// caller-supplied token count.
struct FixedSummarizer {
    body: String,
    tokens: usize,
}

#[async_trait]
impl Summarizer for FixedSummarizer {
    async fn summarize(
        &self,
        _older: &[Message],
        _max_tokens: usize,
    ) -> Result<Summary, CompressionError> {
        Ok(Summary {
            text: self.body.clone(),
            token_count: self.tokens,
        })
    }
}

/// Summariser that always errors — used to pin the error-path test.
struct FailingSummarizer;

#[async_trait]
impl Summarizer for FailingSummarizer {
    async fn summarize(
        &self,
        _older: &[Message],
        _max_tokens: usize,
    ) -> Result<Summary, CompressionError> {
        Err(CompressionError::Summarizer("boom".into()))
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

/// Build a synthetic conversation: 1 system prompt + `n_pairs`
/// user/assistant pairs.
fn synth(n_pairs: usize) -> Vec<Message> {
    let session = SessionId::new();
    let mut out = vec![msg(MessageRole::System, session, "you are helpful")];
    for i in 0..n_pairs {
        out.push(msg(MessageRole::User, session, &format!("question {i}")));
        out.push(msg(MessageRole::Assistant, session, &format!("answer {i}")));
    }
    out
}

#[test]
fn trigger_pct_pins_threshold() {
    // Acceptance: "Compression triggers at 80% of model window
    // (configurable)". 80% of 100 = 80; at exactly 80 the trigger
    // fires, at 79 it does not.
    let config = CompressionConfig::default();
    assert!(should_compress(80, 100, &config));
    assert!(!should_compress(79, 100, &config));
    // Operator-aggressive 70% setting fires earlier.
    let aggressive = CompressionConfig {
        trigger_pct: 70,
        ..CompressionConfig::default()
    };
    assert!(should_compress(70, 100, &aggressive));
    assert!(!should_compress(69, 100, &aggressive));
}

#[test]
fn disabled_flag_and_unknown_window_short_circuit() {
    let config = CompressionConfig {
        enabled: false,
        ..CompressionConfig::default()
    };
    assert!(!should_compress(99_999, 100, &config));
    // Unknown window (caller signalling opt-out) never fires.
    let on = CompressionConfig::default();
    assert!(!should_compress(99_999, 0, &on));
}

#[test]
fn short_history_skips_compression_and_preserves_messages() {
    // Acceptance: "System prompt + last N turns always preserved
    // verbatim" — when there's nothing older than `keep_last_turns`
    // the input is returned unchanged with `turns_compressed = 0`.
    let messages = synth(3);
    let pre_len = messages.len();
    let config = CompressionConfig {
        keep_last_turns: 10,
        ..CompressionConfig::default()
    };
    let summarizer = FixedSummarizer {
        body: "should not be called".into(),
        tokens: 1,
    };
    let result = futures_block(compress(
        &messages,
        &config,
        &WordCounter,
        &summarizer,
    ))
    .expect("compress");
    assert!(!result.compressed());
    assert_eq!(result.turns_compressed, 0);
    assert_eq!(result.summary_length_tokens, 0);
    assert_eq!(result.view.len(), pre_len);
    assert_eq!(result.pre_token_count, result.post_token_count);
}

#[test]
fn partition_keeps_last_n_user_turns_after_system_prefix() {
    let messages = synth(5);
    let part = partition(&messages, 2);
    // System prefix is the leading system message.
    assert_eq!(part.system.len(), 1);
    assert!(matches!(part.system[0].role, MessageRole::System));
    // Recent slice begins with a User message (boundary lands on
    // the Nth-from-end user).
    assert!(matches!(part.recent.first().map(|m| m.role), Some(MessageRole::User)));
    // Recent slice carries exactly 2 user messages + their
    // assistant replies = 4 messages.
    let user_in_recent = part
        .recent
        .iter()
        .filter(|m| matches!(m.role, MessageRole::User))
        .count();
    assert_eq!(user_in_recent, 2);
    // Older slice carries the remaining 3 user/assistant pairs =
    // 6 messages.
    assert_eq!(part.older.len(), 6);
}

#[test]
fn compress_collapses_older_into_one_assistant_summary() {
    // Acceptance: "Compressed envelope is a valid message sequence
    // (no chain breaks)" + "Summary generated via same model".
    let messages = synth(5);
    let config = CompressionConfig {
        keep_last_turns: 2,
        ..CompressionConfig::default()
    };
    let summarizer = FixedSummarizer {
        body: "earlier discussion of design tradeoffs".into(),
        tokens: 6,
    };
    let result = futures_block(compress(
        &messages,
        &config,
        &WordCounter,
        &summarizer,
    ))
    .expect("compress");
    assert!(result.compressed());
    // view = system(1) + summary(1) + recent(2 user + 2 assistant) = 6
    assert_eq!(result.view.len(), 6);
    assert!(matches!(result.view[0].role, MessageRole::System));
    assert!(matches!(result.view[1].role, MessageRole::Assistant));
    // Summary turn carries <summary>…</summary> tag.
    let ContentBlock::Text { text } = &result.view[1].content[0] else {
        panic!("expected text content");
    };
    assert!(text.starts_with("<summary>"));
    assert!(text.ends_with("</summary>"));
    // Alternation downstream: summary(assistant) -> user -> assistant -> user -> assistant
    assert!(matches!(result.view[2].role, MessageRole::User));
    assert!(matches!(result.view[3].role, MessageRole::Assistant));
    assert!(matches!(result.view[4].role, MessageRole::User));
    assert!(matches!(result.view[5].role, MessageRole::Assistant));
    // turns_compressed = 3 user/assistant pairs collapsed = 6 messages
    assert_eq!(result.turns_compressed, 6);
    // Post-compression token count is strictly lower than pre.
    assert!(result.post_token_count < result.pre_token_count);
    assert_eq!(result.summary_length_tokens, 6);
}

#[test]
fn audit_payload_carries_documented_shape() {
    // Acceptance: "Audit row emitted with shape { pre, post,
    // summary_len, turns_compressed }".
    let result = CompressionResult {
        view: vec![],
        pre_token_count: 12_000,
        post_token_count: 4_000,
        turns_compressed: 18,
        summary_length_tokens: 432,
    };
    let payload = build_audit_payload(&result);
    assert_eq!(payload["pre_token_count"], 12_000);
    assert_eq!(payload["post_token_count"], 4_000);
    assert_eq!(payload["summary_length_tokens"], 432);
    assert_eq!(payload["turns_compressed"], 18);
    assert_eq!(AUDIT_KIND_LABEL, "runtime.context_compressed");
}

#[test]
fn invalid_config_rejected_and_summarizer_error_propagates() {
    let messages = synth(5);
    let counter = WordCounter;
    let bad = CompressionConfig {
        keep_last_turns: 0,
        ..CompressionConfig::default()
    };
    let ok_summarizer = FixedSummarizer {
        body: "x".into(),
        tokens: 1,
    };
    let err = futures_block(compress(&messages, &bad, &counter, &ok_summarizer))
        .expect_err("keep_last_turns=0 must reject");
    assert!(matches!(err, CompressionError::InvalidConfig(_)));

    let zero_summary = CompressionConfig {
        summary_max_tokens: 0,
        ..CompressionConfig::default()
    };
    let err = futures_block(compress(&messages, &zero_summary, &counter, &ok_summarizer))
        .expect_err("summary_max_tokens=0 must reject");
    assert!(matches!(err, CompressionError::InvalidConfig(_)));

    let config = CompressionConfig {
        keep_last_turns: 2,
        ..CompressionConfig::default()
    };
    let failing = FailingSummarizer;
    let err = futures_block(compress(&messages, &config, &counter, &failing))
        .expect_err("summariser error must propagate");
    assert!(matches!(err, CompressionError::Summarizer(s) if s.contains("boom")));
}

/// Block on a future inside a `#[test]` without dragging tokio's
/// `current_thread` runtime into the test. The compression flow
/// awaits a single summariser call per invocation; a busy-poll
/// driver with a no-op waker is sufficient and keeps the test crate
/// dependency surface small.
fn futures_block<F: std::future::Future>(future: F) -> F::Output {
    use std::pin::pin;
    use std::task::{Context, Poll, Wake, Waker};

    struct NoopWake;
    impl Wake for NoopWake {
        fn wake(self: std::sync::Arc<Self>) {}
    }
    let waker = Waker::from(std::sync::Arc::new(NoopWake));
    let mut cx = Context::from_waker(&waker);
    let mut future = pin!(future);
    loop {
        if let Poll::Ready(value) = future.as_mut().poll(&mut cx) {
            return value;
        }
    }
}
