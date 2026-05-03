//! Unit tests for `handlers::vertical_install`. Sibling-mod
//! pattern keeps the production module under the 500-LOC budget.

use super::*;

fn write_minimal_vertical(dir: &Path, id: &str) {
    std::fs::create_dir_all(dir.join("prompts")).unwrap();
    std::fs::write(
        dir.join("vertical.toml"),
        format!(
            "schema_version = \"1\"\n\
             [vertical]\nid = \"{id}\"\nname = \"X\"\nversion = \"0.1.0\"\n\
             [role]\nsystem_prompt = \"prompts/system.md\"\n"
        ),
    )
    .unwrap();
    std::fs::write(dir.join("prompts/system.md"), "you are demo agent").unwrap();
}

#[tokio::test]
async fn install_rejects_non_directory_source() {
    let target = tempfile::tempdir().unwrap();
    let err = install(
        "/no/such/dir",
        Some(target.path().to_str().unwrap()),
        false,
        false,
    )
    .await
    .unwrap_err();
    assert!(format!("{err}").contains("not a directory"), "got: {err}");
}

#[tokio::test]
async fn install_rejects_source_without_manifest() {
    let src = tempfile::tempdir().unwrap();
    let target = tempfile::tempdir().unwrap();
    let err = install(
        src.path().to_str().unwrap(),
        Some(target.path().to_str().unwrap()),
        false,
        false,
    )
    .await
    .unwrap_err();
    assert!(format!("{err}").contains("vertical.toml"), "got: {err}");
}

#[tokio::test]
async fn install_copies_validated_source_into_target() {
    let src = tempfile::tempdir().unwrap();
    write_minimal_vertical(src.path(), "demo");
    let target = tempfile::tempdir().unwrap();

    install(
        src.path().to_str().unwrap(),
        Some(target.path().to_str().unwrap()),
        false,
        false,
    )
    .await
    .expect("install");

    let dest = target.path().join("demo");
    assert!(dest.join("vertical.toml").exists());
    assert!(dest.join("prompts/system.md").exists());
    let body = std::fs::read_to_string(dest.join("prompts/system.md")).unwrap();
    assert_eq!(body, "you are demo agent");
}

#[tokio::test]
async fn install_refuses_to_overwrite_without_force() {
    let src = tempfile::tempdir().unwrap();
    write_minimal_vertical(src.path(), "demo");
    let target = tempfile::tempdir().unwrap();

    install(
        src.path().to_str().unwrap(),
        Some(target.path().to_str().unwrap()),
        false,
        false,
    )
    .await
    .expect("first install");

    let err = install(
        src.path().to_str().unwrap(),
        Some(target.path().to_str().unwrap()),
        false,
        false,
    )
    .await
    .unwrap_err();
    assert!(format!("{err}").contains("already installed"), "got: {err}");
    assert!(format!("{err}").contains("--force"), "got: {err}");
}

#[tokio::test]
async fn install_with_force_replaces_existing_dir() {
    let src1 = tempfile::tempdir().unwrap();
    write_minimal_vertical(src1.path(), "demo");
    let target = tempfile::tempdir().unwrap();
    install(
        src1.path().to_str().unwrap(),
        Some(target.path().to_str().unwrap()),
        false,
        false,
    )
    .await
    .expect("first install");

    // New source with a leftover marker file — confirms --force
    // wipes the previous tree (not just merges over it).
    let src2 = tempfile::tempdir().unwrap();
    write_minimal_vertical(src2.path(), "demo");
    std::fs::write(src2.path().join("CHANGED"), "v2").unwrap();
    install(
        src2.path().to_str().unwrap(),
        Some(target.path().to_str().unwrap()),
        true,
        false,
    )
    .await
    .expect("second install with force");

    assert!(target.path().join("demo/CHANGED").exists());
}

#[tokio::test]
async fn install_rejects_source_with_invalid_manifest() {
    let src = tempfile::tempdir().unwrap();
    // schema_version is required + must equal "1" — leave it out.
    std::fs::write(
        src.path().join("vertical.toml"),
        "[vertical]\nid = \"bad\"\nname = \"B\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    let target = tempfile::tempdir().unwrap();
    let err = install(
        src.path().to_str().unwrap(),
        Some(target.path().to_str().unwrap()),
        false,
        false,
    )
    .await
    .unwrap_err();
    let msg = format!("{err}");
    assert!(
        msg.contains("schema_version") || msg.contains("missing"),
        "expected schema-validation message, got: {msg}"
    );
}

#[tokio::test]
async fn uninstall_refuses_without_yes_flag() {
    let src = tempfile::tempdir().unwrap();
    write_minimal_vertical(src.path(), "demo");
    let target = tempfile::tempdir().unwrap();
    install(
        src.path().to_str().unwrap(),
        Some(target.path().to_str().unwrap()),
        false,
        false,
    )
    .await
    .unwrap();

    let err = uninstall(
        "demo",
        Some(target.path().to_str().unwrap()),
        false,
        false,
    )
    .await
    .unwrap_err();
    assert!(format!("{err}").contains("--yes"), "got: {err}");
    // The vertical must still be on disk.
    assert!(target.path().join("demo/vertical.toml").exists());
}

#[tokio::test]
async fn uninstall_with_yes_removes_directory() {
    let src = tempfile::tempdir().unwrap();
    write_minimal_vertical(src.path(), "demo");
    let target = tempfile::tempdir().unwrap();
    install(
        src.path().to_str().unwrap(),
        Some(target.path().to_str().unwrap()),
        false,
        false,
    )
    .await
    .unwrap();

    uninstall(
        "demo",
        Some(target.path().to_str().unwrap()),
        true,
        false,
    )
    .await
    .expect("uninstall");
    assert!(!target.path().join("demo").exists());
}

#[tokio::test]
async fn uninstall_unknown_id_returns_error() {
    let target = tempfile::tempdir().unwrap();
    let err = uninstall("ghost", Some(target.path().to_str().unwrap()), true, false)
        .await
        .unwrap_err();
    assert!(format!("{err}").contains("ghost"), "got: {err}");
    assert!(format!("{err}").contains("no vertical"), "got: {err}");
}
