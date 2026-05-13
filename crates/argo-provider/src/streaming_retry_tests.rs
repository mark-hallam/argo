//! Unit tests for the streaming-aware retry wrapper. Covers the
//! seven acceptance scenarios from `mission-streaming-aware-retry`:
//! clean retry stitches output; retries exhausted yields
//! `StreamingRetryExhausted` with partial content; non-retryable
//! error short-circuits; retry request carries the assistant prefix;
//! exponential backoff sequence; open-time failure still retries;
//! disabled config is a pass-through.
//!
//! We use a controllable `AttemptFactory` rather than a network mock
//! (mockito isn't in the workspace) — the wrapper's contract is at
//! the [`Transport`]-stream boundary, not the wire.
use super::*;
use crate::transport::StopReason;
use argo_types::{ArgoError, ContentBlock, Message, MessageId, MessageRole, SessionId};
use futures::stream;
use std::sync::Mutex;

/// Build a vanilla `ProviderRequest` with a single user message.
fn make_req() -> ProviderRequest {
    ProviderRequest {
        model: "test-model".into(),
        messages: vec![Message {
            id: MessageId::new(),
            session_id: SessionId::new(),
            role: MessageRole::User,
            content: vec![ContentBlock::Text {
                text: "Once upon a time".into(),
            }],
            tool_calls: vec![],
            tool_call_id: None,
            reasoning: None,
            created_at: chrono::Utc::now(),
            provider_meta: serde_json::Value::Null,
        }],
        tools: vec![],
        system: None,
        sampling: Default::default(),
    }
}

/// One scripted attempt: the deltas to emit, plus an optional
/// terminal error after them.
struct ScriptedAttempt {
    deltas: Vec<ProviderResponseDelta>,
    terminal_err: Option<ArgoError>,
    /// Fail at `complete_stream` open time rather than emitting any
    /// chunks at all.
    open_err: Option<ArgoError>,
}

impl ScriptedAttempt {
    fn ok(deltas: Vec<ProviderResponseDelta>) -> Self {
        Self {
            deltas,
            terminal_err: None,
            open_err: None,
        }
    }
    fn fail_after(deltas: Vec<ProviderResponseDelta>, err: ArgoError) -> Self {
        Self {
            deltas,
            terminal_err: Some(err),
            open_err: None,
        }
    }
    fn open_failure(err: ArgoError) -> Self {
        Self {
            deltas: vec![],
            terminal_err: None,
            open_err: Some(err),
        }
    }
}

/// A factory that pops the next scripted attempt off a queue. Also
/// records the `ProviderRequest` it was called with so the tests can
/// assert about the continuation prefix.
#[derive(Default)]
struct Recorder {
    received: Vec<ProviderRequest>,
}

fn scripted_factory(
    script: Vec<ScriptedAttempt>,
    recorder: Arc<Mutex<Recorder>>,
) -> AttemptFactory {
    let queue = Arc::new(Mutex::new(script.into_iter().collect::<Vec<_>>()));
    Arc::new(move |req: ProviderRequest| {
        let queue = queue.clone();
        let recorder = recorder.clone();
        Box::pin(async move {
            recorder.lock().unwrap().received.push(req);
            let next = {
                let mut q = queue.lock().unwrap();
                if q.is_empty() {
                    None
                } else {
                    Some(q.remove(0))
                }
            };
            let next = next.ok_or_else(|| {
                ArgoError::Provider("test: script exhausted (unexpected extra attempt)".into())
            })?;
            if let Some(open_err) = next.open_err {
                return Err(open_err);
            }
            let mut items: Vec<ArgoResult<ProviderResponseDelta>> =
                next.deltas.into_iter().map(Ok).collect();
            if let Some(err) = next.terminal_err {
                items.push(Err(err));
            }
            let s = stream::iter(items).boxed();
            Ok(s)
        })
    })
}

fn fast_config(max_attempts: u32) -> RetryConfig {
    // 1ms base keeps total test wall-clock under 50ms even for the
    // 3-retry exhaustion case (1 + 2 + 4 + 8 = 15ms of sleep).
    RetryConfig {
        enabled: true,
        max_attempts,
        backoff_initial_ms: 1,
    }
}

#[tokio::test]
async fn clean_retry_stitches_attempt_1_and_2() {
    let recorder = Arc::new(Mutex::new(Recorder::default()));
    let script = vec![
        ScriptedAttempt::fail_after(
            vec![
                ProviderResponseDelta::TextChunk("Hello, ".into()),
                ProviderResponseDelta::TextChunk("wor".into()),
            ],
            ArgoError::Provider("anthropic HTTP 503: service unavailable".into()),
        ),
        ScriptedAttempt::ok(vec![
            ProviderResponseDelta::TextChunk("ld!".into()),
            ProviderResponseDelta::FinishReason(StopReason::EndTurn),
        ]),
    ];
    let factory = scripted_factory(script, recorder.clone());
    let mut s = retry_streaming(make_req(), fast_config(3), factory);

    let mut chunks: Vec<String> = Vec::new();
    let mut finish_seen = false;
    while let Some(item) = s.next().await {
        match item.unwrap() {
            ProviderResponseDelta::TextChunk(c) => chunks.push(c),
            ProviderResponseDelta::FinishReason(_) => finish_seen = true,
            _ => {}
        }
    }
    assert_eq!(chunks, vec!["Hello, ", "wor", "ld!"]);
    assert!(finish_seen);
    assert_eq!(recorder.lock().unwrap().received.len(), 2);
}

#[tokio::test]
async fn retries_exhausted_emits_streaming_retry_exhausted_with_partial_content() {
    let recorder = Arc::new(Mutex::new(Recorder::default()));
    let make_attempt = || {
        ScriptedAttempt::fail_after(
            vec![ProviderResponseDelta::TextChunk("partial".into())],
            ArgoError::Provider("anthropic HTTP 502: bad gateway".into()),
        )
    };
    let script = vec![
        make_attempt(),
        make_attempt(),
        make_attempt(),
        make_attempt(),
    ];
    let factory = scripted_factory(script, recorder.clone());
    let mut s = retry_streaming(make_req(), fast_config(3), factory);

    let mut texts: Vec<String> = Vec::new();
    let mut terminal: Option<ArgoError> = None;
    while let Some(item) = s.next().await {
        match item {
            Ok(ProviderResponseDelta::TextChunk(c)) => texts.push(c),
            Ok(_) => {}
            Err(e) => terminal = Some(e),
        }
    }
    // Each attempt yielded "partial" before the 5xx; the consumer
    // sees all four (live forwarding). The buffer accumulates them
    // and surfaces as `partial_content`.
    assert_eq!(texts, vec!["partial", "partial", "partial", "partial"]);
    let exhausted = terminal.expect("final retry must yield Err");
    match exhausted {
        ArgoError::StreamingRetryExhausted { partial_content } => {
            assert_eq!(partial_content, "partialpartialpartialpartial");
        }
        other => panic!("expected StreamingRetryExhausted, got {other:?}"),
    }
    // 1 initial + 3 retries = 4 attempts.
    assert_eq!(recorder.lock().unwrap().received.len(), 4);
}

#[tokio::test]
async fn non_retryable_error_passes_through_untouched() {
    let recorder = Arc::new(Mutex::new(Recorder::default()));
    let script = vec![ScriptedAttempt::fail_after(
        vec![ProviderResponseDelta::TextChunk("hi".into())],
        // 401 is a fatal auth failure — must not retry, must not
        // get rewritten to `StreamingRetryExhausted`.
        ArgoError::Provider("anthropic HTTP 401: bad api key".into()),
    )];
    let factory = scripted_factory(script, recorder.clone());
    let mut s = retry_streaming(make_req(), fast_config(3), factory);

    let mut got: Vec<ArgoResult<ProviderResponseDelta>> = Vec::new();
    while let Some(item) = s.next().await {
        got.push(item);
    }
    assert_eq!(got.len(), 2, "one TextChunk + one terminal Err");
    let err = got.pop().unwrap().unwrap_err();
    match err {
        ArgoError::Provider(msg) => assert!(msg.contains("HTTP 401")),
        other => panic!("expected pass-through Provider err, got {other:?}"),
    }
    assert_eq!(recorder.lock().unwrap().received.len(), 1);
}

#[tokio::test]
async fn retry_request_includes_assistant_prefix() {
    let recorder = Arc::new(Mutex::new(Recorder::default()));
    let script = vec![
        ScriptedAttempt::fail_after(
            vec![
                ProviderResponseDelta::TextChunk("The cat ".into()),
                ProviderResponseDelta::TextChunk("sat on the ".into()),
            ],
            ArgoError::Provider("anthropic HTTP 500: server error".into()),
        ),
        ScriptedAttempt::ok(vec![
            ProviderResponseDelta::TextChunk("mat.".into()),
            ProviderResponseDelta::FinishReason(StopReason::EndTurn),
        ]),
    ];
    let factory = scripted_factory(script, recorder.clone());
    let mut s = retry_streaming(make_req(), fast_config(3), factory);
    while s.next().await.is_some() {}

    let received = recorder.lock().unwrap().received.clone();
    assert_eq!(received.len(), 2);
    // First attempt: just the user message, no prefix.
    assert_eq!(received[0].messages.len(), 1);
    assert_eq!(received[0].messages[0].role, MessageRole::User);
    // Second attempt: original user message + assistant prefix.
    assert_eq!(received[1].messages.len(), 2);
    let tail = received[1].messages.last().unwrap();
    assert_eq!(tail.role, MessageRole::Assistant);
    match &tail.content[0] {
        ContentBlock::Text { text } => assert_eq!(text, "The cat sat on the "),
        other => panic!("expected text prefix, got {other:?}"),
    }
}

#[tokio::test]
async fn backoff_doubles_per_retry() {
    use std::time::Instant;
    let recorder = Arc::new(Mutex::new(Recorder::default()));
    let script = vec![
        ScriptedAttempt::fail_after(vec![], ArgoError::Provider("anthropic HTTP 500: e1".into())),
        ScriptedAttempt::fail_after(vec![], ArgoError::Provider("anthropic HTTP 500: e2".into())),
        ScriptedAttempt::fail_after(vec![], ArgoError::Provider("anthropic HTTP 500: e3".into())),
        ScriptedAttempt::ok(vec![ProviderResponseDelta::FinishReason(
            StopReason::EndTurn,
        )]),
    ];
    // 50ms base × {1, 2, 4} = 350ms total of sleep.
    let cfg = RetryConfig {
        enabled: true,
        max_attempts: 3,
        backoff_initial_ms: 50,
    };
    let factory = scripted_factory(script, recorder.clone());
    let start = Instant::now();
    let mut s = retry_streaming(make_req(), cfg, factory);
    while s.next().await.is_some() {}
    let elapsed = start.elapsed();
    // Floor: 50 + 100 + 200 = 350ms. Allow generous upper bound for
    // CI jitter without making the test brittle.
    assert!(
        elapsed.as_millis() >= 340,
        "backoffs too short: {elapsed:?}"
    );
    assert!(
        elapsed.as_millis() < 2_000,
        "backoffs took too long: {elapsed:?}"
    );
}

#[tokio::test]
async fn open_failure_on_first_attempt_still_retries() {
    let recorder = Arc::new(Mutex::new(Recorder::default()));
    let script = vec![
        ScriptedAttempt::open_failure(ArgoError::Provider("reqwest send: connect refused".into())),
        ScriptedAttempt::ok(vec![
            ProviderResponseDelta::TextChunk("recovered".into()),
            ProviderResponseDelta::FinishReason(StopReason::EndTurn),
        ]),
    ];
    let factory = scripted_factory(script, recorder.clone());
    let mut s = retry_streaming(make_req(), fast_config(3), factory);

    let mut texts: Vec<String> = Vec::new();
    while let Some(item) = s.next().await {
        if let Ok(ProviderResponseDelta::TextChunk(c)) = item {
            texts.push(c);
        }
    }
    assert_eq!(texts, vec!["recovered"]);
    assert_eq!(recorder.lock().unwrap().received.len(), 2);
    // The retry's request should have NO assistant prefix because
    // nothing was buffered on the failed open.
    let retry_req = &recorder.lock().unwrap().received[1];
    assert_eq!(retry_req.messages.len(), 1);
}

#[tokio::test]
async fn disabled_config_is_passthrough() {
    let recorder = Arc::new(Mutex::new(Recorder::default()));
    let script = vec![ScriptedAttempt::fail_after(
        vec![ProviderResponseDelta::TextChunk("partial".into())],
        ArgoError::Provider("anthropic HTTP 500: would-be-retryable".into()),
    )];
    let cfg = RetryConfig {
        enabled: false,
        max_attempts: 3,
        backoff_initial_ms: 1,
    };
    let factory = scripted_factory(script, recorder.clone());
    let mut s = retry_streaming(make_req(), cfg, factory);

    let mut texts: Vec<String> = Vec::new();
    let mut terminal: Option<ArgoError> = None;
    while let Some(item) = s.next().await {
        match item {
            Ok(ProviderResponseDelta::TextChunk(c)) => texts.push(c),
            Ok(_) => {}
            Err(e) => terminal = Some(e),
        }
    }
    assert_eq!(texts, vec!["partial"]);
    // Disabled: error surfaces unchanged, no `StreamingRetryExhausted`.
    match terminal.expect("error should still propagate") {
        ArgoError::Provider(msg) => assert!(msg.contains("HTTP 500")),
        other => panic!("expected pass-through Provider err, got {other:?}"),
    }
    // Only one attempt — no retries when disabled.
    assert_eq!(recorder.lock().unwrap().received.len(), 1);
}

#[test]
fn classifier_is_retryable_matches_brief() {
    assert!(is_retryable(&ArgoError::Provider(
        "anthropic HTTP 500: server error".into()
    )));
    assert!(is_retryable(&ArgoError::Provider(
        "openai HTTP 503: backend".into()
    )));
    assert!(is_retryable(&ArgoError::Provider(
        "openai HTTP 429: slow down".into()
    )));
    assert!(is_retryable(&ArgoError::Provider(
        "reqwest send: timeout".into()
    )));
    assert!(is_retryable(&ArgoError::Provider(
        "anthropic SSE chunk: connection reset".into()
    )));
    // Non-retryable shapes:
    assert!(!is_retryable(&ArgoError::Provider(
        "anthropic HTTP 401: bad key".into()
    )));
    assert!(!is_retryable(&ArgoError::Provider(
        "anthropic HTTP 400: bad request".into()
    )));
    assert!(!is_retryable(&ArgoError::Schema("missing field".into())));
}

#[test]
fn backoff_caps_at_thirty_seconds() {
    // Big attempt index should still saturate at 30s.
    let d = compute_backoff(1_000, 20);
    assert!(d.as_millis() <= 30_000, "uncapped: {:?}", d);
}
