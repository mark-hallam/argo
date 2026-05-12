//! cargo-fuzz target — `redact_text` regex scanner DoS canary.
//!
//! Feeds random byte sequences (interpreted as lossy UTF-8) through
//! the redaction scanner under the strict default policy. Confirms:
//!
//! - the scanner never panics, including on pathological inputs that
//!   could trip catastrophic regex backtracking
//! - calling twice on the same input is deterministic
//!
//! Catches both DOS-shaped regexes and UTF-8 boundary slips in the
//! string-splice path inside `redact_text`.
#![no_main]

use argo_redact::{redact_text, RedactionPolicy};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let text = String::from_utf8_lossy(data);
    let policy = RedactionPolicy::default();
    let (out1, hits1) = redact_text(&text, &policy);
    let (out2, hits2) = redact_text(&text, &policy);
    assert_eq!(out1, out2, "redact_text must be deterministic");
    assert_eq!(hits1, hits2, "category set must be deterministic");
});
