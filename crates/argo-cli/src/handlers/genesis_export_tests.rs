use super::*;
use flate2::read::GzDecoder;
use std::collections::BTreeMap;

fn fixture_vertical() -> (tempfile::TempDir, LoadedVertical) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("verticals/sales-sdr");
    fs::create_dir_all(root.join("skills")).unwrap();
    fs::write(
        root.join("vertical.toml"),
        r#"schema_version = "1"

[vertical]
id = "sales-sdr"
name = "Sales SDR"
version = "0.1.0"

[role]
system_prompt = "system-prompt.md"

[memory]
schema = "memory-schema.toml"

[evaluation]
suites = ["eval-seed.toml"]

[skills]
bundle = "skills"
"#,
    )
    .unwrap();
    fs::write(root.join("system-prompt.md"), "Sell carefully.\n").unwrap();
    fs::write(root.join("memory-schema.toml"), "schema_version = \"1\"\n").unwrap();
    fs::write(root.join("eval-seed.toml"), "# seeds\n").unwrap();
    fs::write(root.join("skills/follow-up.md"), "follow up\n").unwrap();
    let (registry, errors) = argo_vertical::load_dir(&dir.path().join("verticals")).unwrap();
    assert!(errors.is_empty(), "{errors:?}");
    let loaded = registry.get("sales-sdr").unwrap().clone();
    (dir, loaded)
}

fn unpack(path: &Path) -> BTreeMap<String, Vec<u8>> {
    let file = File::open(path).unwrap();
    let decoder = GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);
    let mut out = BTreeMap::new();
    for entry in archive.entries().unwrap() {
        let mut entry = entry.unwrap();
        let path = entry.path().unwrap().to_string_lossy().to_string();
        let mut bytes = Vec::new();
        std::io::Read::read_to_end(&mut entry, &mut bytes).unwrap();
        out.insert(path, bytes);
    }
    out
}

#[test]
fn export_fixture_vertical_produces_openable_tarball() {
    let (_dir, loaded) = fixture_vertical();
    let out = loaded.root.join("bundle.tar.gz");

    export_loaded(&loaded, &out, true, false, false).unwrap();
    let files = unpack(&out);

    assert!(files.contains_key(BUNDLE_MANIFEST));
    assert!(files.contains_key("vertical.toml"));
    assert!(files.contains_key("system-prompt.md"));
}

#[test]
fn bundle_manifest_has_sha256_and_size_for_every_payload() {
    let (_dir, loaded) = fixture_vertical();
    let out = loaded.root.join("bundle.tar.gz");

    export_loaded(&loaded, &out, true, false, false).unwrap();
    let files = unpack(&out);
    let raw = std::str::from_utf8(files.get(BUNDLE_MANIFEST).unwrap()).unwrap();
    let manifest: BundleManifest = toml::from_str(raw).unwrap();

    for file in manifest.files {
        let bytes = files.get(&file.path).unwrap();
        assert_eq!(file.size, bytes.len() as u64);
        assert_eq!(file.sha256, sha256_bytes(bytes));
    }
}

#[test]
fn manifest_only_writes_bundle_toml_only() {
    let dir = tempfile::tempdir().unwrap();
    let (_fixture, loaded) = fixture_vertical();
    let out = dir.path().join(BUNDLE_MANIFEST);

    export_loaded(&loaded, &out, true, true, false).unwrap();

    assert!(out.exists());
    assert!(toml::from_str::<BundleManifest>(&fs::read_to_string(&out).unwrap()).is_ok());
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
fn missing_vertical_returns_provider_error() {
    let dir = tempfile::tempdir().unwrap();

    let err = load_vertical(dir.path(), "missing").expect_err("missing vertical");

    assert!(matches!(err, ArgoError::Provider(_)));
}

#[test]
fn existing_output_path_refuses_overwrite_without_force() {
    let (_dir, loaded) = fixture_vertical();
    let out = loaded.root.join("bundle.tar.gz");
    fs::write(&out, "existing").unwrap();

    let err = export_loaded(&loaded, &out, true, false, false).expect_err("overwrite");

    assert!(matches!(err, ArgoError::Schema(_)));
}

#[test]
fn include_skills_false_omits_skills_dir() {
    let (_dir, loaded) = fixture_vertical();
    let out = loaded.root.join("bundle.tar.gz");

    export_loaded(&loaded, &out, false, false, false).unwrap();
    let files = unpack(&out);

    assert!(!files.keys().any(|path| path.starts_with("skills/")));
}
