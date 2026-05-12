//! cargo-fuzz target — `compute_chain_hash` parser/format boundary.
//!
//! Splits the input into a `parent_hash[32]`, a `kind` slice, and a
//! `payload` slice (both interpreted as lossy UTF-8). Confirms:
//!
//! - the function never panics on adversarial byte sequences
//! - the hash is deterministic when called twice on the same input
//!
//! Audit-chain bytes are signed and can't be replayed; a panic in the
//! hasher would either DoS the audit-write path or leave a chain
//! partially extended.
#![no_main]

use argo_audit::compute_chain_hash;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.len() < 33 {
        return;
    }
    let (parent_raw, rest) = data.split_at(32);
    let mut parent = [0u8; 32];
    parent.copy_from_slice(parent_raw);
    // First byte after the parent hash chooses the split between
    // `kind` and `payload` so the fuzzer can explore both lengths.
    let split = (rest[0] as usize).min(rest.len().saturating_sub(1));
    let (kind_raw, payload_raw) = rest[1..].split_at(split);
    let kind = String::from_utf8_lossy(kind_raw);
    let payload = String::from_utf8_lossy(payload_raw);
    let h1 = compute_chain_hash(&parent, &kind, &payload);
    let h2 = compute_chain_hash(&parent, &kind, &payload);
    assert_eq!(h1, h2, "compute_chain_hash must be deterministic");
});
