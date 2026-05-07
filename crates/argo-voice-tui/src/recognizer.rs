//! [`Recognizer`] trait + [`MockRecognizer`].
//!
//! The recognizer abstracts speech-to-text: real impl is whisper.cpp
//! (gated behind the `whisper-native` feature; see
//! [`crate::recognizer_whisper`]); test impl is a scriptable mock
//! that hands back canned strings in order.
//!
//! Trait is sync — voice frames arrive at human speech cadence,
//! not network speed, so blocking-on-thread is fine. The runner
//! drives the recognizer from a `tokio::task::spawn_blocking` when
//! integrated.

use thiserror::Error;

/// Recognizer-layer errors. Most real-world failures (no model,
/// mic permission denied, decode failure) surface as
/// [`RecognizerError::Backend`] with a descriptive message; the
/// CLI handler renders these to stderr and falls back to the TUI
/// list view.
#[derive(Debug, Error)]
pub enum RecognizerError {
    /// Backend-specific failure (whisper.cpp init, model load,
    /// audio decode, etc.). String wraps the upstream error so the
    /// CLI can surface it verbatim.
    #[error("voice backend error: {0}")]
    Backend(String),
    /// The mock has nothing left to play back. Real recognizers do
    /// not produce this.
    #[error("mock recognizer exhausted")]
    Exhausted,
}

/// Speech-to-text contract.
///
/// `transcribe_next` blocks until the next utterance has been
/// captured + transcribed, then returns the recognized text.
/// Returning `Ok(String::new())` is allowed — the runner treats
/// empty output as "no speech detected, keep listening".
pub trait Recognizer {
    /// Block until the next utterance is recognized; return its
    /// text (already approximately punctuation-stripped — but
    /// callers should still feed it through [`crate::grammar::tokens`]
    /// before matching).
    fn transcribe_next(&mut self) -> Result<String, RecognizerError>;
}

/// Test-mode recognizer that hands back a fixed playlist of
/// utterances in order. Useful for unit tests of [`crate::runner`]
/// and for the `cargo run -p argo-cli -- voice --mock-script ...`
/// developer hook so a contributor without a mic can drive the loop.
#[derive(Debug, Clone)]
pub struct MockRecognizer {
    queue: std::collections::VecDeque<String>,
}

impl MockRecognizer {
    /// New mock seeded with `script` — calls to `transcribe_next`
    /// drain the queue front-to-back.
    pub fn new<I, S>(script: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            queue: script.into_iter().map(Into::into).collect(),
        }
    }

    /// Queue one extra utterance at the back. Used by tests that
    /// inject confirmation responses after the runner emits the
    /// safety prompt.
    pub fn push(&mut self, utterance: impl Into<String>) {
        self.queue.push_back(utterance.into());
    }

    /// Number of utterances still queued.
    pub fn remaining(&self) -> usize {
        self.queue.len()
    }
}

impl Recognizer for MockRecognizer {
    fn transcribe_next(&mut self) -> Result<String, RecognizerError> {
        self.queue.pop_front().ok_or(RecognizerError::Exhausted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_plays_back_in_order() {
        let mut r = MockRecognizer::new(["argo approve maya", "argo quit"]);
        assert_eq!(r.transcribe_next().unwrap(), "argo approve maya");
        assert_eq!(r.transcribe_next().unwrap(), "argo quit");
    }

    #[test]
    fn mock_exhausts_after_drain() {
        let mut r = MockRecognizer::new(["argo next"]);
        let _ = r.transcribe_next().unwrap();
        assert!(matches!(
            r.transcribe_next(),
            Err(RecognizerError::Exhausted)
        ));
    }

    #[test]
    fn mock_push_appends() {
        let mut r = MockRecognizer::new(["argo next"]);
        r.push("argo quit");
        assert_eq!(r.remaining(), 2);
    }

    #[test]
    fn backend_error_renders_message() {
        let e = RecognizerError::Backend("model file missing".into());
        let s = format!("{e}");
        assert!(s.contains("model file missing"));
    }
}
