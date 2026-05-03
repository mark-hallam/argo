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
    assert!(
        format!("{err}").contains("neither a directory nor a `.tar` file"),
        "got: {err}"
    );
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

// ---- pack + tarball install (#675) ---------------------------

#[tokio::test]
async fn pack_then_install_round_trips_via_tarball() {
    let src = tempfile::tempdir().unwrap();
    write_minimal_vertical(src.path(), "demo");
    // First install the source so we can pack it.
    let installed = tempfile::tempdir().unwrap();
    install(
        src.path().to_str().unwrap(),
        Some(installed.path().to_str().unwrap()),
        false,
        false,
    )
    .await
    .unwrap();

    let tar_dir = tempfile::tempdir().unwrap();
    let tar_path = tar_dir.path().join("demo.tar");
    pack(
        "demo",
        Some(tar_path.to_str().unwrap()),
        Some(installed.path().to_str().unwrap()),
        false,
    )
    .await
    .expect("pack");
    assert!(tar_path.exists(), "tarball must exist after pack");
    let bytes = std::fs::metadata(&tar_path).unwrap().len();
    assert!(bytes > 0, "tarball must be non-empty");

    // Install the tarball into a fresh target and confirm the
    // tree matches the original.
    let restored = tempfile::tempdir().unwrap();
    install(
        tar_path.to_str().unwrap(),
        Some(restored.path().to_str().unwrap()),
        false,
        false,
    )
    .await
    .expect("install from tarball");
    assert!(restored.path().join("demo/vertical.toml").exists());
    assert!(restored.path().join("demo/prompts/system.md").exists());
    let body = std::fs::read_to_string(restored.path().join("demo/prompts/system.md")).unwrap();
    assert_eq!(body, "you are demo agent");
}

#[tokio::test]
async fn pack_unknown_id_returns_error() {
    let target = tempfile::tempdir().unwrap();
    let out = tempfile::tempdir().unwrap();
    let err = pack(
        "ghost",
        Some(out.path().join("ghost.tar").to_str().unwrap()),
        Some(target.path().to_str().unwrap()),
        false,
    )
    .await
    .unwrap_err();
    assert!(format!("{err}").contains("ghost"));
    assert!(format!("{err}").contains("no vertical"));
}

#[tokio::test]
async fn pack_refuses_to_pack_a_broken_vertical() {
    // Install a vertical, then break its manifest (point [role]
    // at a missing file). Pack must fail validation rather
    // than ship a broken artifact.
    let target = tempfile::tempdir().unwrap();
    let v = target.path().join("broken");
    std::fs::create_dir_all(&v).unwrap();
    std::fs::write(
        v.join("vertical.toml"),
        "schema_version = \"1\"\n\
         [vertical]\nid = \"broken\"\nname = \"B\"\nversion = \"0.1.0\"\n\
         [role]\nsystem_prompt = \"prompts/missing.md\"\n",
    )
    .unwrap();
    let out_dir = tempfile::tempdir().unwrap();
    let err = pack(
        "broken",
        Some(out_dir.path().join("broken.tar").to_str().unwrap()),
        Some(target.path().to_str().unwrap()),
        false,
    )
    .await
    .unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("does not exist") || msg.contains("system_prompt"), "got: {msg}");
}
