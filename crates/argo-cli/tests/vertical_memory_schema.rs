//! Integration tests for memory-schema parsing + cross-validation
//! at vertical-load time (#663).
//!
//! Locks two contracts:
//!
//! 1. `argo vertical validate <id>` fails LOUDLY on a malformed
//!    memory schema — not silently at first memory write.
//! 2. `argo vertical show <id> --json` carries the typed
//!    `memory_entries` array so the gateway + scripting clients
//!    can introspect the contract without re-parsing.

mod common;

use common::argo_bin;
use std::path::Path;
use std::process::Command;

fn write_minimal_with_memory_schema(
    verticals_dir: &Path,
    id: &str,
    schema_body: &str,
) {
    let v = verticals_dir.join(id);
    std::fs::create_dir_all(&v).unwrap();
    std::fs::write(
        v.join("vertical.toml"),
        format!(
            "schema_version = \"1\"\n\
             [vertical]\nid = \"{id}\"\nname = \"X\"\nversion = \"0.1.0\"\n\
             [memory]\nschema = \"memory.schema.toml\"\n"
        ),
    )
    .unwrap();
    std::fs::write(v.join("memory.schema.toml"), schema_body).unwrap();
}

#[test]
fn cli_validate_fails_loudly_on_unknown_memory_entry_type() {
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    write_minimal_with_memory_schema(
        verticals.path(),
        "demo",
        "schema_version = \"1\"\n[[entry]]\nkey=\"x\"\ntype=\"telepathy\"\n",
    );

    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .args(["vertical", "validate", "demo", "--json"])
        .output()
        .expect("spawn");
    assert!(!out.status.success(), "validate must fail on bad schema");
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("telepathy"), "stderr: {stderr}");
}

#[test]
fn cli_validate_passes_on_well_formed_memory_schema() {
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    write_minimal_with_memory_schema(
        verticals.path(),
        "demo",
        "schema_version = \"1\"\n\
         [[entry]]\nkey=\"faqs\"\ntype=\"list-of-objects\"\npurpose=\"freq qns\"\n\
         [[entry]]\nkey=\"banlist\"\ntype=\"list-of-strings\"\n",
    );

    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .args(["vertical", "validate", "demo", "--json"])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "validate stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn cli_show_json_carries_typed_memory_entries() {
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    write_minimal_with_memory_schema(
        verticals.path(),
        "demo",
        "schema_version = \"1\"\n\
         [[entry]]\nkey=\"faqs\"\ntype=\"list-of-objects\"\npurpose=\"freq qns\"\n",
    );

    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .args(["vertical", "show", "demo", "--json"])
        .output()
        .expect("spawn");
    assert!(out.status.success());
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
    let entries = v["memory_entries"].as_array().expect("memory_entries");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["key"], "faqs");
    assert_eq!(entries[0]["type"], "list-of-objects");
    assert_eq!(entries[0]["purpose"], "freq qns");
}

#[test]
fn cli_show_json_omits_memory_entries_when_no_schema_declared() {
    let home = tempfile::tempdir().unwrap();
    let verticals = tempfile::tempdir().unwrap();
    let v = verticals.path().join("plain");
    std::fs::create_dir_all(&v).unwrap();
    std::fs::write(
        v.join("vertical.toml"),
        "schema_version = \"1\"\n\
         [vertical]\nid = \"plain\"\nname = \"P\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();

    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", verticals.path())
        .args(["vertical", "show", "plain", "--json"])
        .output()
        .expect("spawn");
    assert!(out.status.success());
    let val: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
    // Either absent or null is acceptable; both signal "no schema".
    let entries = &val["memory_entries"];
    assert!(
        entries.is_null() || entries.as_array().map(|a| a.is_empty()).unwrap_or(false),
        "got: {entries}"
    );
}

#[test]
fn cli_show_json_for_real_receptionist_carries_four_entries() {
    // Lock the shipped Receptionist memory schema. Future edits
    // that drop or rename an entry will fail this test in the
    // same PR.
    let repo_verticals = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .join("verticals");
    let home = tempfile::tempdir().unwrap();
    let out = Command::new(argo_bin())
        .env("ARGO_HOME", home.path())
        .env("ARGO_VERTICALS_DIR", &repo_verticals)
        .args(["vertical", "show", "receptionist", "--json"])
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
    let entries = v["memory_entries"]
        .as_array()
        .expect("receptionist must carry memory_entries");
    let keys: Vec<&str> = entries
        .iter()
        .map(|e| e["key"].as_str().unwrap())
        .collect();
    for required in [
        "recurring_visitors",
        "pending_followups",
        "learned_faqs",
        "out_of_scope_patterns",
    ] {
        assert!(
            keys.contains(&required),
            "receptionist memory schema missing entry '{required}'; got: {keys:?}"
        );
    }
}
