//! Edge-case coverage for [`crate::loader`].
//!
//! Authored under brief `eval-argo-vertical-loader-coverage` to
//! deepen test surface beyond the happy-path cases in
//! `loader.rs`'s inline test module. Each test below targets a
//! specific failure mode the loader is supposed to handle
//! gracefully (collected error, not panic; deterministic order;
//! useful error message).
use super::{load_dir, LoadError};
use std::fs;
use std::path::Path;
use tempfile::tempdir;

const MIN_TOML: &str = "schema_version = \"1\"\n\n[vertical]\nid = \"{id}\"\nname = \"V {id}\"\nversion = \"0.1.0\"\n";

fn write_vertical(root: &Path, id: &str) {
    let dir = root.join(id);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("vertical.toml"), MIN_TOML.replace("{id}", id)).unwrap();
}

fn write_broken_vertical(root: &Path, subdir: &str, body: &str) {
    let dir = root.join(subdir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("vertical.toml"), body).unwrap();
}

/// `load_dir` accepts a plain file (not a directory) — should
/// surface that as a structured schema error, not a panic or a
/// silent empty load.
#[test]
fn rejects_when_input_path_is_a_regular_file() {
    let dir = tempdir().unwrap();
    let f = dir.path().join("im-a-file");
    fs::write(&f, "not a directory").unwrap();
    let err = load_dir(&f).expect_err("plain file must error");
    let msg = format!("{err}");
    assert!(msg.contains("is not a directory"), "got: {msg}");
}

/// Multiple broken verticals must all surface as `LoadError`s in
/// deterministic (sorted) subdir order — tests rely on this to
/// pin assertions on a specific index without flake.
#[test]
fn collects_multiple_errors_in_sorted_subdir_order() {
    let dir = tempdir().unwrap();
    // 3 broken verticals in non-alpha order; loader should sort
    // by path so we see zeta last regardless of fs traversal order.
    write_broken_vertical(dir.path(), "zeta", "not valid toml :::");
    write_broken_vertical(dir.path(), "alpha", "schema_version = \"99\"\n");
    write_broken_vertical(dir.path(), "mid", "schema_version = \"1\"\n");
    let (reg, errs) = load_dir(dir.path()).unwrap();
    assert_eq!(reg.len(), 0, "all three are broken");
    assert_eq!(errs.len(), 3, "every broken vertical collected");
    let names: Vec<String> = errs
        .iter()
        .map(|e: &LoadError| e.dir.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, vec!["alpha", "mid", "zeta"]);
}

/// One clean vertical + one broken vertical: the clean one still
/// lands in the registry, the broken one shows up in errors.
/// Locks the "isolation" promise from the module docstring:
/// "one malformed vertical doesn't blind the rest of the catalogue."
#[test]
fn clean_verticals_load_alongside_broken_ones() {
    let dir = tempdir().unwrap();
    write_vertical(dir.path(), "good");
    write_broken_vertical(dir.path(), "bad", "schema_version = \"\"\n");
    let (reg, errs) = load_dir(dir.path()).unwrap();
    assert_eq!(reg.len(), 1);
    assert!(reg.get("good").is_some());
    assert_eq!(errs.len(), 1);
    assert!(errs[0].dir.ends_with("bad"));
}

/// Duplicate id collision error must name BOTH the colliding id
/// and the path of the vertical that was loaded first — so an
/// operator can tell which file to fix without git-grepping.
#[test]
fn duplicate_id_error_names_first_loaded_root() {
    let dir = tempdir().unwrap();
    // Two subdirs both declare id="twin"; loader sorts dirs, so
    // "first wins" = "first" subdir.
    let one = dir.path().join("first");
    let two = dir.path().join("second");
    fs::create_dir_all(&one).unwrap();
    fs::create_dir_all(&two).unwrap();
    let body = MIN_TOML.replace("{id}", "twin");
    fs::write(one.join("vertical.toml"), &body).unwrap();
    fs::write(two.join("vertical.toml"), &body).unwrap();
    let (reg, errs) = load_dir(dir.path()).unwrap();
    assert_eq!(reg.len(), 1);
    assert_eq!(errs.len(), 1);
    let msg = format!("{}", errs[0].error);
    assert!(msg.contains("twin"), "got: {msg}");
    assert!(msg.contains("first"), "got: {msg}");
}

/// `vertical.toml` placed at the top level (not inside a subdir)
/// is invisible to the loader by design — the loader walks
/// IMMEDIATE subdirs and looks for `<subdir>/vertical.toml`.
/// Catches accidental flattening of the directory layout.
#[test]
fn top_level_manifest_is_ignored() {
    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("vertical.toml"),
        MIN_TOML.replace("{id}", "rootlevel"),
    )
    .unwrap();
    let (reg, errs) = load_dir(dir.path()).unwrap();
    assert_eq!(reg.len(), 0);
    assert!(errs.is_empty());
}

/// Subdir whose `vertical.toml` is actually a directory (so
/// `is_file()` returns false) is silently skipped — same path as
/// "no manifest at all", not a hard error.
#[test]
fn vertical_toml_that_is_a_directory_is_ignored() {
    let dir = tempdir().unwrap();
    let bad_sub = dir.path().join("nested");
    fs::create_dir_all(bad_sub.join("vertical.toml")).unwrap();
    let (reg, errs) = load_dir(dir.path()).unwrap();
    assert_eq!(reg.len(), 0);
    assert!(errs.is_empty());
}

/// Missing top-level `schema_version` is a parse error from the
/// `#[serde(deny_unknown_fields)]` shape. Surfaces a parse error
/// (collected per-vertical), not an unwrap.
#[test]
fn collects_missing_schema_version_as_parse_error() {
    let dir = tempdir().unwrap();
    write_broken_vertical(
        dir.path(),
        "noschema",
        "[vertical]\nid = \"x\"\nname = \"X\"\nversion = \"0.1.0\"\n",
    );
    let (reg, errs) = load_dir(dir.path()).unwrap();
    assert_eq!(reg.len(), 0);
    assert_eq!(errs.len(), 1);
    let msg = format!("{}", errs[0].error);
    assert!(msg.contains("schema_version"), "got: {msg}");
}
