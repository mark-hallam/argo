//! Slot-collision guardrails for the versioned migration runner.
//!
//! The production runner uses the Rust `Migration { version, up, down }`
//! registry, not filesystem ordering. Raw SQL filenames now carry
//! strictly-monotonic numeric prefixes — historical duplicate slots
//! (19, 22, 23, 24, 26, 27, 28, 30, 45, 51, 75) were renumbered to
//! 78..103 (codex-migration-monotonic-renumber). The accompanying
//! file-slot duplicate check lives in [`duplicate_slot_validator`].
//!
//! [`duplicate_slot_validator`]: super::duplicate_slot_validator

use std::collections::BTreeSet;

use argo_types::ArgoError;

#[cfg(test)]
#[path = "slot_validator_tests.rs"]
mod slot_validator_tests;

/// Reject duplicate version numbers in the Rust runner registry.
pub(super) fn validate_registered_versions<I>(versions: I) -> Result<(), ArgoError>
where
    I: IntoIterator<Item = u32>,
{
    let mut seen = BTreeSet::new();
    let mut duplicates = Vec::new();
    for version in versions {
        if !seen.insert(version) {
            duplicates.push(version);
        }
    }
    if duplicates.is_empty() {
        return Ok(());
    }
    Err(ArgoError::Invariant(format!(
        "duplicate migration registry version(s): {}",
        duplicates
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    )))
}
