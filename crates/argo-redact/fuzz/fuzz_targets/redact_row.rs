//! cargo-fuzz target — JSON-row redaction wrapper.
//!
//! Decodes arbitrary input as JSON; if the parse succeeds, feeds the
//! value through `redact_approval_row` under the strict default
//! policy. Catches type-shape edge cases (non-object roots, nested
//! arrays, embedded NUL bytes) that the plain-text scanner target
//! would not exercise.
#![no_main]

use argo_redact::{redact_approval_row, RedactionPolicy};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(s) = std::str::from_utf8(data) else { return };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(s) else { return };
    let policy = RedactionPolicy::default();
    let _ = redact_approval_row(value, &policy, None);
});
