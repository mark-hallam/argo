use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::process::Command;

fn write_vertical(root: &std::path::Path) {
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
}

fn unpack(path: &std::path::Path) -> BTreeMap<String, Vec<u8>> {
    let decoder = GzDecoder::new(File::open(path).unwrap());
    let mut archive = tar::Archive::new(decoder);
    let mut files = BTreeMap::new();
    for entry in archive.entries().unwrap() {
        let mut entry = entry.unwrap();
        let name = entry.path().unwrap().to_string_lossy().to_string();
        let mut bytes = Vec::new();
        std::io::Read::read_to_end(&mut entry, &mut bytes).unwrap();
        files.insert(name, bytes);
    }
    files
}

fn sha256(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

#[test]
fn genesis_export_writes_portable_bundle_with_matching_sha256() {
    let workspace = tempfile::tempdir().unwrap();
    let verticals = workspace.path().join("verticals");
    write_vertical(&verticals.join("sales-sdr"));
    let out_path = workspace.path().join("sales-sdr.argo-vertical.tar.gz");

    let out = Command::new(env!("CARGO_BIN_EXE_argo"))
        .args([
            "genesis",
            "export",
            "--vertical",
            "sales-sdr",
            "--output",
            out_path.to_str().unwrap(),
        ])
        .env("ARGO_VERTICALS_DIR", &verticals)
        .current_dir(workspace.path())
        .output()
        .expect("spawn argo");
    assert!(
        out.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let files = unpack(&out_path);
    assert_eq!(
        files.keys().cloned().collect::<Vec<_>>(),
        vec![
            "BUNDLE.toml",
            "eval-seed.toml",
            "memory-schema.toml",
            "skills/follow-up.md",
            "system-prompt.md",
            "vertical.toml",
        ]
    );
    let manifest: toml::Value =
        toml::from_str(std::str::from_utf8(files.get("BUNDLE.toml").unwrap()).unwrap()).unwrap();
    for file in manifest["files"].as_array().unwrap() {
        let path = file["path"].as_str().unwrap();
        let bytes = files.get(path).unwrap();
        assert_eq!(file["size"].as_integer().unwrap() as usize, bytes.len());
        assert_eq!(file["sha256"].as_str().unwrap(), sha256(bytes));
    }
}
