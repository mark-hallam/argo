//! Tests for `isolation::check_isolation`. Sibling-mod pattern
//! keeps the production module under the 500-LOC budget.
#![cfg(test)]

use super::*;
use crate::manifest::parse;

fn fixture_root() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

#[test]
fn minimal_manifest_passes_isolation() {
    let dir = fixture_root();
    let m = parse(
        "schema_version = \"1\"\n\
         [vertical]\nid = \"demo\"\nname = \"D\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    check_isolation(&m, dir.path()).expect("minimal manifest passes");
}

#[test]
fn rejects_role_system_prompt_with_parent_traversal() {
    let dir = fixture_root();
    let m = parse(
        "schema_version = \"1\"\n\
         [vertical]\nid = \"demo\"\nname = \"D\"\nversion = \"0.1.0\"\n\
         [role]\nsystem_prompt = \"../other/prompts/system.md\"\n",
    )
    .unwrap();
    let err = check_isolation(&m, dir.path()).unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("[role].system_prompt"), "got: {msg}");
    assert!(msg.contains("`..`"), "got: {msg}");
}

#[test]
fn rejects_absolute_system_prompt() {
    let dir = fixture_root();
    let m = parse(
        "schema_version = \"1\"\n\
         [vertical]\nid = \"demo\"\nname = \"D\"\nversion = \"0.1.0\"\n\
         [role]\nsystem_prompt = \"/etc/passwd\"\n",
    )
    .unwrap();
    let err = check_isolation(&m, dir.path()).unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("absolute"), "got: {msg}");
}

#[test]
fn rejects_workflows_files_with_parent_traversal() {
    let dir = fixture_root();
    let m = parse(
        "schema_version = \"1\"\n\
         [vertical]\nid = \"demo\"\nname = \"D\"\nversion = \"0.1.0\"\n\
         [workflows]\nfiles = [\"workflows/ok.toml\", \"../other/leaked.toml\"]\n",
    )
    .unwrap();
    let err = check_isolation(&m, dir.path()).unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("[workflows].files"), "got: {msg}");
}

#[test]
fn rejects_skills_bundle_with_parent_traversal() {
    let dir = fixture_root();
    let m = parse(
        "schema_version = \"1\"\n\
         [vertical]\nid = \"demo\"\nname = \"D\"\nversion = \"0.1.0\"\n\
         [skills]\nbundle = \"../other/skills/\"\n",
    )
    .unwrap();
    let err = check_isolation(&m, dir.path()).unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("[skills].bundle"), "got: {msg}");
}

#[test]
fn rejects_memory_schema_with_parent_traversal() {
    let dir = fixture_root();
    let m = parse(
        "schema_version = \"1\"\n\
         [vertical]\nid = \"demo\"\nname = \"D\"\nversion = \"0.1.0\"\n\
         [memory]\nschema = \"../other/memory.schema.toml\"\n",
    )
    .unwrap();
    let err = check_isolation(&m, dir.path()).unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("[memory].schema"), "got: {msg}");
}

#[test]
fn rejects_memory_seed_with_parent_traversal() {
    let dir = fixture_root();
    let m = parse(
        "schema_version = \"1\"\n\
         [vertical]\nid = \"demo\"\nname = \"D\"\nversion = \"0.1.0\"\n\
         [memory]\nseed = \"../other/seed/\"\n",
    )
    .unwrap();
    let err = check_isolation(&m, dir.path()).unwrap_err();
    assert!(format!("{err}").contains("[memory].seed"));
}

#[test]
fn rejects_eval_suite_with_parent_traversal() {
    let dir = fixture_root();
    let m = parse(
        "schema_version = \"1\"\n\
         [vertical]\nid = \"demo\"\nname = \"D\"\nversion = \"0.1.0\"\n\
         [evaluation]\nsuites = [\"../other/eval.jsonl\"]\n",
    )
    .unwrap();
    let err = check_isolation(&m, dir.path()).unwrap_err();
    assert!(format!("{err}").contains("[evaluation].suites"));
}

#[test]
fn rejects_role_persona_with_parent_traversal() {
    let dir = fixture_root();
    let m = parse(
        "schema_version = \"1\"\n\
         [vertical]\nid = \"demo\"\nname = \"D\"\nversion = \"0.1.0\"\n\
         [role]\nsystem_prompt = \"prompts/system.md\"\npersona = \"../leak.md\"\n",
    )
    .unwrap();
    let err = check_isolation(&m, dir.path()).unwrap_err();
    assert!(format!("{err}").contains("[role].persona"));
}

#[test]
fn rejects_path_with_dot_dot_in_middle() {
    // Catches more sneaky `subdir/../../other` patterns.
    let dir = fixture_root();
    let m = parse(
        "schema_version = \"1\"\n\
         [vertical]\nid = \"demo\"\nname = \"D\"\nversion = \"0.1.0\"\n\
         [role]\nsystem_prompt = \"prompts/sub/../../escape.md\"\n",
    )
    .unwrap();
    let err = check_isolation(&m, dir.path()).unwrap_err();
    assert!(format!("{err}").contains("`..`"));
}

#[test]
fn accepts_legitimate_subdir_paths() {
    let dir = fixture_root();
    // Ordinary paths in subdirs are fine.
    let m = parse(
        "schema_version = \"1\"\n\
         [vertical]\nid = \"demo\"\nname = \"D\"\nversion = \"0.1.0\"\n\
         [role]\nsystem_prompt = \"prompts/system.md\"\npersona = \"prompts/persona.md\"\n\
         [memory]\nschema = \"memory.schema.toml\"\nseed = \"memory/seed/\"\n\
         [skills]\nbundle = \"skills/\"\n\
         [workflows]\nfiles = [\"workflows/main.workflow.toml\"]\n\
         [evaluation]\nsuites = [\"eval/main.jsonl\", \"eval/extra.jsonl\"]\n",
    )
    .unwrap();
    check_isolation(&m, dir.path()).expect("legitimate subdir paths must pass");
}
