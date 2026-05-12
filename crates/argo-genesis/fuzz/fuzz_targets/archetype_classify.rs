//! cargo-fuzz target — `classify_charter` keyword classifier.
//!
//! Feeds random byte sequences (lossy UTF-8) into the offline charter
//! classifier. Confirms:
//!
//! - the classifier never panics on adversarial Unicode (including
//!   surrogate pairs, zero-width joiners, RTL marks)
//! - when classification succeeds, the variant is one of the 12 known
//!   archetypes (no out-of-bounds slug returned)
//!
//! The classifier is the entrypoint for operator-typed charters; a
//! panic here would crash the Genesis pipeline mid-`plan`.
#![no_main]

use argo_genesis::{classify_charter, Archetype};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let charter = String::from_utf8_lossy(data);
    if let Ok(archetype) = classify_charter(&charter) {
        assert!(
            Archetype::ALL.contains(&archetype),
            "classify_charter must return a known archetype"
        );
    }
});
