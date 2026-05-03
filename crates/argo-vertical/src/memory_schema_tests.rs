//! Unit tests for `memory_schema`. Sibling-mod pattern keeps the
//! production module under the 500-LOC budget.

use super::*;

const MIN: &str = r#"
schema_version = "1"

[[entry]]
key  = "x"
type = "string"
"#;

#[test]
fn parses_minimal_schema() {
    let s = parse_memory_schema(MIN).unwrap();
    assert_eq!(s.entries.len(), 1);
    assert_eq!(s.entries[0].key, "x");
    assert_eq!(s.entries[0].type_, "string");
    assert_eq!(s.entries[0].purpose, "");
}

#[test]
fn parses_full_receptionist_style_schema() {
    let src = r#"
schema_version = "1"

[[entry]]
key  = "recurring_visitors"
type = "list-of-objects"
purpose = "People we've talked to before."

[[entry]]
key  = "out_of_scope_patterns"
type = "list-of-strings"
purpose = "Topics that always escalate."
"#;
    let s = parse_memory_schema(src).unwrap();
    assert_eq!(s.entries.len(), 2);
    assert_eq!(s.entries[0].type_, "list-of-objects");
    assert_eq!(s.entries[1].type_, "list-of-strings");
    assert!(!s.entries[1].purpose.is_empty());
}

#[test]
fn rejects_wrong_schema_version() {
    let bad = MIN.replace("\"1\"", "\"99\"");
    let err = parse_memory_schema(&bad).unwrap_err();
    assert!(format!("{err}").contains("schema_version '99'"));
}

#[test]
fn rejects_duplicate_keys() {
    let src = "schema_version = \"1\"\n\
               [[entry]]\nkey=\"x\"\ntype=\"string\"\n\
               [[entry]]\nkey=\"x\"\ntype=\"int\"\n";
    let err = parse_memory_schema(src).unwrap_err();
    assert!(format!("{err}").contains("duplicate entry key 'x'"));
}

#[test]
fn rejects_empty_key() {
    let src = "schema_version = \"1\"\n[[entry]]\nkey=\"\"\ntype=\"string\"\n";
    let err = parse_memory_schema(src).unwrap_err();
    assert!(format!("{err}").contains("empty `key`"));
}

#[test]
fn rejects_unknown_type() {
    let src = "schema_version = \"1\"\n[[entry]]\nkey=\"x\"\ntype=\"telepathic\"\n";
    let err = parse_memory_schema(src).unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("unknown type 'telepathic'"), "got: {msg}");
}

#[test]
fn accepts_every_documented_type() {
    for t in [
        "list-of-objects",
        "list-of-strings",
        "string",
        "object",
        "int",
        "float",
        "bool",
    ] {
        let src = format!(
            "schema_version = \"1\"\n[[entry]]\nkey=\"x\"\ntype=\"{t}\"\n"
        );
        parse_memory_schema(&src)
            .unwrap_or_else(|e| panic!("type {t} should parse, got: {e}"));
    }
}

#[test]
fn rejects_unknown_top_level_key() {
    let bad = format!("{MIN}\nbogus = 1\n");
    let err = parse_memory_schema(&bad).unwrap_err();
    assert!(format!("{err}").contains("memory.schema"));
}

#[test]
fn rejects_unknown_entry_key() {
    let src = "schema_version = \"1\"\n[[entry]]\nkey=\"x\"\ntype=\"string\"\nbogus=1\n";
    let err = parse_memory_schema(src).unwrap_err();
    assert!(format!("{err}").contains("memory.schema"));
}

#[test]
fn empty_entries_block_is_legal() {
    // A schema with no [[entry]] is valid (e.g. operator wants to
    // declare the manifest field but hasn't typed any rows yet).
    let s = parse_memory_schema("schema_version = \"1\"\n").unwrap();
    assert!(s.entries.is_empty());
}
