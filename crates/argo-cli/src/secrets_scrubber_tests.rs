//! Unit tests for [`crate::secrets_scrubber`]. Covers the six
//! patterns enumerated in the brief plus the env-var opt-out.

use std::io::Write as IoWrite;
use std::sync::{Arc, Mutex};

use tracing_subscriber::layer::SubscriberExt;

use crate::secrets_scrubber::{enabled_from_env, scrub, SecretsScrubberLayer};

#[derive(Clone)]
struct SharedBuf(Arc<Mutex<Vec<u8>>>);

impl SharedBuf {
    fn new() -> Self {
        Self(Arc::new(Mutex::new(Vec::new())))
    }

    fn snapshot(&self) -> String {
        let g = self.0.lock().unwrap();
        String::from_utf8(g.clone()).unwrap()
    }
}

impl IoWrite for SharedBuf {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn capture_with_layer(enabled: bool, emit: impl FnOnce()) -> String {
    let buf = SharedBuf::new();
    let layer = SecretsScrubberLayer::with_writer_and_enabled(Box::new(buf.clone()), enabled);
    let subscriber = tracing_subscriber::registry().with(layer);
    tracing::subscriber::with_default(subscriber, emit);
    buf.snapshot()
}

#[test]
fn bearer_token_in_event_message_is_scrubbed() {
    let out = capture_with_layer(true, || {
        tracing::error!("auth failed Bearer abc123XYZ_token-value");
    });
    assert!(
        out.contains("<scrubbed:bearer>"),
        "expected bearer redaction, got: {out}"
    );
    assert!(
        !out.contains("abc123XYZ_token-value"),
        "raw bearer leaked: {out}"
    );
}

#[test]
fn api_key_field_value_is_scrubbed() {
    let out = capture_with_layer(true, || {
        tracing::error!(api_key = "sk_live_abcdef0123456789", "outbound call");
    });
    assert!(
        !out.contains("sk_live_abcdef0123456789"),
        "raw api_key leaked: {out}"
    );
    // The structured field carries a literal value; the scrubber's
    // text-pattern set fires on `api_key=...` shape inside the
    // formatted line.
    assert!(
        out.contains("<scrubbed:apikey>"),
        "expected apikey redaction, got: {out}"
    );
}

#[test]
fn password_field_value_is_scrubbed() {
    let out = capture_with_layer(true, || {
        tracing::error!(password = "hunter2-correct-horse", "login");
    });
    assert!(
        !out.contains("hunter2-correct-horse"),
        "raw password leaked: {out}"
    );
    assert!(
        out.contains("<scrubbed:password>"),
        "expected password redaction, got: {out}"
    );
}

#[test]
fn jwt_shape_is_scrubbed() {
    let raw = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjMifQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
    let s = scrub(raw);
    assert_eq!(s, "<scrubbed:jwt>", "scrub() on raw JWT");

    let out = capture_with_layer(true, || {
        tracing::error!("session token = eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjMifQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c");
    });
    assert!(
        out.contains("<scrubbed:jwt>"),
        "expected jwt redaction, got: {out}"
    );
    assert!(!out.contains("SflKxwRJSMeKKF"), "raw JWT leaked: {out}");
}

#[test]
fn innocuous_string_with_word_token_is_not_scrubbed() {
    // "token" appearing in prose (no high-entropy value) must not
    // trigger redaction — over-scrubbing destroys debug usefulness.
    let s = scrub("requested a new token from the server");
    assert_eq!(
        s, "requested a new token from the server",
        "innocuous prose was over-scrubbed: {s}"
    );

    let out = capture_with_layer(true, || {
        tracing::error!("requested a new token from the server");
    });
    assert!(
        out.contains("requested a new token from the server"),
        "innocuous message lost: {out}"
    );
    assert!(!out.contains("<scrubbed:"), "false-positive redaction: {out}");
}

#[test]
fn env_opt_out_disables_scrubbing() {
    // Pure-function check on the env resolver.
    assert!(!enabled_from_env(Some("0")), "ARGO_TRACING_SCRUB=0 should disable");
    assert!(enabled_from_env(None), "unset env should keep enabled");
    assert!(enabled_from_env(Some("1")), "any non-zero value should keep enabled");
    assert!(enabled_from_env(Some("yes")), "non-numeric should keep enabled");

    // End-to-end: a layer constructed in disabled mode passes the
    // bearer through verbatim.
    let raw = "Bearer abc123XYZ_token-value";
    let out = capture_with_layer(false, || {
        tracing::error!("{}", raw);
    });
    assert!(
        out.contains("abc123XYZ_token-value"),
        "disabled layer should pass through, got: {out}"
    );
    assert!(
        !out.contains("<scrubbed:"),
        "disabled layer must not scrub: {out}"
    );
}
