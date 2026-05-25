//! Compile-time guardrail: refuse to build if `migrations/` contains
//! two files sharing the same numeric slot prefix. Also emits a static
//! `MIGRATION_FILES` list consumed by
//! `src/migrations/duplicate_slot_validator.rs` as a runtime safety net.
//! Note: this is the slot-collision corpus, not the schema-apply path;
//! see `src/migrations/README.md` for the schema-apply story.

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let migrations_dir = manifest_dir.join("migrations");
    println!("cargo:rerun-if-changed={}", migrations_dir.display());

    let mut files: Vec<String> = fs::read_dir(&migrations_dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", migrations_dir.display()))
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".sql"))
        .collect();
    // Sort by numeric slot prefix so the embedded list iterates in
    // schema-apply order; lexicographic sort would put "100__" before
    // "12__" and break the monotonic-order regression test.
    files.sort_by_key(|name| {
        name.split_once("__")
            .and_then(|(prefix, _)| prefix.parse::<u32>().ok())
            .unwrap_or(u32::MAX)
    });

    let mut by_slot: BTreeMap<u32, Vec<&String>> = BTreeMap::new();
    for file in &files {
        if let Some((prefix, _)) = file.split_once("__") {
            if let Ok(slot) = prefix.parse::<u32>() {
                by_slot.entry(slot).or_default().push(file);
            }
        }
    }

    let duplicates: Vec<(&u32, &Vec<&String>)> = by_slot
        .iter()
        .filter(|(_, group)| group.len() > 1)
        .collect();
    if !duplicates.is_empty() {
        for (slot, group) in &duplicates {
            eprintln!(
                "argo-store build.rs: duplicate migration slot {slot}: {}",
                group
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        panic!(
            "argo-store build.rs: {} duplicate migration slot group(s) detected; pick the next free slot",
            duplicates.len()
        );
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let dest = out_dir.join("migration_files.rs");
    let mut body = String::from(
        "/// Filenames of every `.sql` in `crates/argo-store/migrations/`,\n\
         /// captured at build time so the runtime duplicate-slot validator\n\
         /// can run without filesystem access.\n\
         pub(crate) const MIGRATION_FILES: &[&str] = &[\n",
    );
    for file in &files {
        body.push_str(&format!("    \"{file}\",\n"));
    }
    body.push_str("];\n");
    fs::write(&dest, body).unwrap_or_else(|e| panic!("write {}: {e}", dest.display()));
}
