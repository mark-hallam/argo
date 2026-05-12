//! cargo-fuzz target — multi-row chain-composition invariants.
//!
//! Treats the input as a sequence of `(kind_len: u8, kind, payload_len:
//! u8, payload)` records and folds them into an audit chain by piping
//! each row's `self_hash` into the next row's `parent_hash`. Confirms:
//!
//! - no panic on any byte sequence
//! - the chain is byte-for-byte reproducible on a second walk
//!
//! This catches integration-level bugs (off-by-one in length framing,
//! UTF-8 boundary slips) that the single-call hash target would miss.
#![no_main]

use argo_audit::compute_chain_hash;
use libfuzzer_sys::fuzz_target;

fn walk(data: &[u8]) -> Vec<[u8; 32]> {
    let mut parent = [0u8; 32];
    let mut hashes = Vec::new();
    let mut iter = data.iter().copied();
    while let Some(klen) = iter.next() {
        let kind_bytes: Vec<u8> = iter.by_ref().take(klen as usize).collect();
        if kind_bytes.len() < klen as usize {
            break;
        }
        let Some(plen) = iter.next() else { break };
        let payload_bytes: Vec<u8> = iter.by_ref().take(plen as usize).collect();
        if payload_bytes.len() < plen as usize {
            break;
        }
        let kind = String::from_utf8_lossy(&kind_bytes);
        let payload = String::from_utf8_lossy(&payload_bytes);
        let h = compute_chain_hash(&parent, &kind, &payload);
        hashes.push(h);
        parent = h;
    }
    hashes
}

fuzz_target!(|data: &[u8]| {
    let first = walk(data);
    let second = walk(data);
    assert_eq!(first, second, "chain walk must be reproducible");
});
