//! Runtime safety net: refuse to start the migration runner if any
//! two `migrations/*.sql` files share the same numeric slot prefix.
//!
//! The primary guardrail is [`build.rs`], which fails the build under
//! the same condition. This module re-validates at runtime against the
//! `MIGRATION_FILES` list embedded by `build.rs` so the panic happens
//! before sqlx applies anything if the binary somehow shipped with a
//! stale embedded list (e.g. someone bypassed `build.rs` cleanly).
//!
//! [`build.rs`]: ../../../build.rs

use std::collections::BTreeMap;

use argo_types::ArgoError;

include!(concat!(env!("OUT_DIR"), "/migration_files.rs"));

pub(super) fn validate_no_duplicate_slots() -> Result<(), ArgoError> {
    detect_duplicate_slots(MIGRATION_FILES)
}

fn detect_duplicate_slots<S: AsRef<str>>(files: &[S]) -> Result<(), ArgoError> {
    let mut by_slot: BTreeMap<u32, Vec<&str>> = BTreeMap::new();
    for file in files {
        let file = file.as_ref();
        let Some((prefix, _)) = file.split_once("__") else {
            continue;
        };
        let Ok(slot) = prefix.parse::<u32>() else {
            continue;
        };
        by_slot.entry(slot).or_default().push(file);
    }
    let dupes: Vec<String> = by_slot
        .into_iter()
        .filter(|(_, group)| group.len() > 1)
        .map(|(slot, group)| format!("slot {slot}: {}", group.join(", ")))
        .collect();
    if dupes.is_empty() {
        return Ok(());
    }
    Err(ArgoError::Invariant(format!(
        "duplicate migration slot(s) detected pre-apply: {}",
        dupes.join("; ")
    )))
}

#[cfg(test)]
mod tests {
    use super::{detect_duplicate_slots, validate_no_duplicate_slots, MIGRATION_FILES};

    #[test]
    fn embedded_migrations_have_unique_slots() {
        validate_no_duplicate_slots()
            .expect("post-renumber migrations/ must be slot-monotonic");
    }

    #[test]
    fn migrations_list_is_monotonic_after_split() {
        let mut prev: Option<u32> = None;
        for file in MIGRATION_FILES {
            let (prefix, _) = file
                .split_once("__")
                .unwrap_or_else(|| panic!("non-slotted migration file: {file}"));
            let slot: u32 = prefix
                .parse()
                .unwrap_or_else(|_| panic!("non-numeric slot in {file}"));
            if let Some(prev_slot) = prev {
                assert!(
                    slot > prev_slot,
                    "non-monotonic slot ordering: {prev_slot} then {slot} ({file})"
                );
            }
            prev = Some(slot);
        }
    }

    #[test]
    fn simulated_duplicate_is_reported() {
        let files = vec!["78__one.sql", "78__two.sql", "79__three.sql"];
        let err = detect_duplicate_slots(&files).expect_err("duplicate must fail");
        let msg = err.to_string();
        assert!(
            msg.contains("slot 78") && msg.contains("78__one.sql") && msg.contains("78__two.sql"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn unique_slots_pass() {
        let files = vec!["1__a.sql", "2__b.sql", "3__c.sql"];
        detect_duplicate_slots(&files).expect("unique slots accepted");
    }

    #[test]
    fn non_slot_files_ignored() {
        let files = vec!["readme.sql", "1__a.sql"];
        detect_duplicate_slots(&files).expect("non-slot files ignored");
    }
}
