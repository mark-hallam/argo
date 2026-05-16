//! Handler tests for `argo skill pack lint`. Asserts the
//! formatter + exit-code classifier against tempdir fixtures —
//! the underlying lint checks are tested in
//! `argo_skill::pack_lint`.

use super::render;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

fn write(path: &Path, body: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    fs::write(path, body).expect("write file");
}

fn write_clean_pack(root: &Path) {
    write(
        &root.join("pack.toml"),
        "[pack]\nname='clean-pack'\nversion='1.0.0'\n\n[[verticals]]\nid='clean'\npath='.'\n",
    );
    write(&root.join("README.md"), "# clean-pack\n");
}

#[test]
fn clean_pack_exits_zero_text() {
    let tmp = tempdir().unwrap();
    write_clean_pack(tmp.path());
    let mut buf: Vec<u8> = Vec::new();
    let exit = render(&mut buf, tmp.path(), false, false, false).unwrap();
    assert_eq!(exit, 0, "{}", String::from_utf8_lossy(&buf));
    let out = String::from_utf8(buf).unwrap();
    assert!(out.contains("all checks passed"), "{out}");
}

#[test]
fn errors_exit_two_text() {
    let tmp = tempdir().unwrap();
    write(
        &tmp.path().join("pack.toml"),
        "[pack]\nname='broken-pack'\nversion='not-a-version'\n\n[[verticals]]\nid='x'\npath='.'\n",
    );
    write(&tmp.path().join("README.md"), "# broken\n");
    let mut buf: Vec<u8> = Vec::new();
    let exit = render(&mut buf, tmp.path(), false, false, false).unwrap();
    assert_eq!(exit, 2, "{}", String::from_utf8_lossy(&buf));
    let out = String::from_utf8(buf).unwrap();
    assert!(out.contains("ERROR"), "{out}");
    assert!(out.contains("semver"), "{out}");
}

#[test]
fn warnings_exit_one_text() {
    // pack.toml is fine, but README.md is missing → 1 warning only.
    let tmp = tempdir().unwrap();
    write(
        &tmp.path().join("pack.toml"),
        "[pack]\nname='warn-pack'\nversion='1.0.0'\n\n[[verticals]]\nid='x'\npath='.'\n",
    );
    let mut buf: Vec<u8> = Vec::new();
    let exit = render(&mut buf, tmp.path(), false, false, false).unwrap();
    assert_eq!(exit, 1, "{}", String::from_utf8_lossy(&buf));
}

#[test]
fn strict_escalates_warning_to_two() {
    let tmp = tempdir().unwrap();
    write(
        &tmp.path().join("pack.toml"),
        "[pack]\nname='warn-pack'\nversion='1.0.0'\n\n[[verticals]]\nid='x'\npath='.'\n",
    );
    let mut buf: Vec<u8> = Vec::new();
    let exit = render(&mut buf, tmp.path(), true, false, false).unwrap();
    assert_eq!(exit, 2);
}

#[test]
fn json_output_parses_and_includes_summary() {
    let tmp = tempdir().unwrap();
    write_clean_pack(tmp.path());
    let mut buf: Vec<u8> = Vec::new();
    let exit = render(&mut buf, tmp.path(), false, true, false).unwrap();
    assert_eq!(exit, 0);
    let value: serde_json::Value = serde_json::from_slice(&buf).expect("valid json");
    assert!(value.get("pack").is_some());
    assert!(value.get("checks").and_then(|c| c.as_array()).is_some());
    let summary = value.get("summary").expect("summary");
    assert_eq!(summary["errors"].as_u64(), Some(0));
    assert_eq!(summary["warnings"].as_u64(), Some(0));
}

#[test]
fn color_codes_only_when_tty() {
    let tmp = tempdir().unwrap();
    write(
        &tmp.path().join("pack.toml"),
        "[pack]\nname='warn-pack'\nversion='1.0.0'\n\n[[verticals]]\nid='x'\npath='.'\n",
    );
    let mut piped: Vec<u8> = Vec::new();
    render(&mut piped, tmp.path(), false, false, false).unwrap();
    let mut tty: Vec<u8> = Vec::new();
    render(&mut tty, tmp.path(), false, false, true).unwrap();
    let piped_s = String::from_utf8(piped).unwrap();
    let tty_s = String::from_utf8(tty).unwrap();
    assert!(!piped_s.contains("\x1b["), "piped output should be plain");
    assert!(tty_s.contains("\x1b["), "tty output should be colored");
}
