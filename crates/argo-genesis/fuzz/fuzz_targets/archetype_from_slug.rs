//! cargo-fuzz target — slug → `Archetype` lookup.
//!
//! `Archetype::from_slug` tolerates whitespace, uppercase, and `-`/`_`
//! drift in caller-supplied slugs (typically LLM output). Confirms:
//!
//! - the lookup never panics on adversarial UTF-8
//! - when a slug resolves, the round-trip back to the canonical slug
//!   form is stable
#![no_main]

use argo_genesis::Archetype;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let slug = String::from_utf8_lossy(data);
    if let Some(arch) = Archetype::from_slug(&slug) {
        let canonical = arch.slug();
        assert_eq!(
            Archetype::from_slug(canonical),
            Some(arch),
            "canonical slug must round-trip"
        );
    }
});
