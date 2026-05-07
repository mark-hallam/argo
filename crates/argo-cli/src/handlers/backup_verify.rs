//! `argo backup verify`.

use super::backup_create::{
    payload_sha256, sha256_hex, ArchivePayload, BackupFile, BackupManifest, MANIFEST_PATH,
};
use argo_types::{ArgoError, ArgoResult};
use flate2::read::GzDecoder;
use serde::Serialize;
use std::fs::File;
use std::path::{Component, Path};

#[derive(Debug, Clone)]
pub(crate) struct ArchiveFile {
    pub(crate) path: String,
    pub(crate) bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub(crate) struct BackupInspection {
    pub(crate) manifest: BackupManifest,
    pub(crate) files: Vec<ArchiveFile>,
}

#[derive(Debug, Serialize)]
struct VerifyOut {
    path: String,
    sha256: String,
    files: usize,
    include_credentials: bool,
}

pub async fn run(path: &Path, json: bool) -> ArgoResult<()> {
    let inspection = verify_backup(path)?;
    let out = VerifyOut {
        path: path.display().to_string(),
        sha256: inspection.manifest.sha256.clone(),
        files: inspection.files.len(),
        include_credentials: inspection.manifest.include_credentials,
    };
    if json {
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({"ok": true, "backup_verify": out}))
                .map_err(|e| ArgoError::Provider(format!("backup verify JSON: {e}")))?
        );
    } else {
        println!(
            "backup ok: {} ({} files, sha256 {})",
            out.path, out.files, out.sha256
        );
    }
    Ok(())
}

pub(crate) fn verify_backup(path: &Path) -> ArgoResult<BackupInspection> {
    let inspection = inspect_archive(path)?;
    validate_manifest(&inspection)?;
    Ok(inspection)
}

pub(crate) fn inspect_archive(path: &Path) -> ArgoResult<BackupInspection> {
    let file = File::open(path)?;
    let decoder = GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);
    let mut manifest = None;
    let mut files = Vec::new();
    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry
            .path()
            .map_err(|e| ArgoError::Provider(format!("backup entry path: {e}")))?
            .to_string_lossy()
            .to_string();
        let mut bytes = Vec::new();
        std::io::Read::read_to_end(&mut entry, &mut bytes)?;
        if path == MANIFEST_PATH {
            manifest = Some(
                serde_json::from_slice::<BackupManifest>(&bytes)
                    .map_err(|e| ArgoError::Provider(format!("backup manifest: {e}")))?,
            );
        } else {
            validate_archive_path(&path)?;
            files.push(ArchiveFile { path, bytes });
        }
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    let manifest = manifest.ok_or_else(|| {
        ArgoError::Provider(format!("backup archive missing {MANIFEST_PATH}"))
    })?;
    Ok(BackupInspection { manifest, files })
}

fn validate_manifest(inspection: &BackupInspection) -> ArgoResult<()> {
    if inspection.manifest.schema_version != 1 {
        return Err(ArgoError::Provider(format!(
            "unsupported backup schema_version {}",
            inspection.manifest.schema_version
        )));
    }
    let payloads: Vec<ArchivePayload> = inspection
        .files
        .iter()
        .map(|file| ArchivePayload {
            path: file.path.clone(),
            bytes: file.bytes.clone(),
        })
        .collect();
    let actual_sha = payload_sha256(&payloads);
    if actual_sha != inspection.manifest.sha256 {
        return Err(ArgoError::Provider(format!(
            "backup sha256 mismatch: manifest {} actual {}",
            inspection.manifest.sha256, actual_sha
        )));
    }
    let actual_files: Vec<BackupFile> = inspection
        .files
        .iter()
        .map(|file| BackupFile {
            path: file.path.clone(),
            bytes: file.bytes.len() as u64,
            sha256: sha256_hex(&file.bytes),
        })
        .collect();
    if actual_files != inspection.manifest.files {
        return Err(ArgoError::Provider(
            "backup manifest file list does not match archive contents".to_string(),
        ));
    }
    Ok(())
}

pub(crate) fn validate_archive_path(path: &str) -> ArgoResult<()> {
    let rel = Path::new(path);
    let mut components = rel.components();
    match components.next() {
        Some(Component::Normal(root)) if root == ".argo" => {}
        _ => {
            return Err(ArgoError::Provider(format!(
                "backup entry must be under .argo/: {path}"
            )));
        }
    }
    for component in components {
        match component {
            Component::Normal(_) => {}
            _ => {
                return Err(ArgoError::Provider(format!(
                    "unsafe backup entry path: {path}"
                )));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::write::GzEncoder;
    use flate2::Compression;

    #[test]
    fn payload_hash_depends_on_path_and_bytes() {
        let first = vec![ArchivePayload {
            path: ".argo/a".to_string(),
            bytes: b"one".to_vec(),
        }];
        let second = vec![ArchivePayload {
            path: ".argo/b".to_string(),
            bytes: b"one".to_vec(),
        }];

        assert_ne!(payload_sha256(&first), payload_sha256(&second));
    }

    #[test]
    fn inspect_archive_rejects_missing_manifest() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("missing-manifest.tar.gz");
        let file = File::create(&path).unwrap();
        let encoder = GzEncoder::new(file, Compression::default());
        let mut builder = tar::Builder::new(encoder);
        let mut header = tar::Header::new_gnu();
        header.set_size(5);
        header.set_mode(0o600);
        header.set_cksum();
        builder
            .append_data(&mut header, ".argo/state.db", &b"state"[..])
            .unwrap();
        let encoder = builder.into_inner().unwrap();
        encoder.finish().unwrap();

        let err = inspect_archive(&path).unwrap_err().to_string();

        assert!(err.contains("manifest.json"));
    }

    #[test]
    fn validate_archive_path_rejects_parent_escape() {
        let err = validate_archive_path(".argo/../state.db")
            .unwrap_err()
            .to_string();

        assert!(err.contains("unsafe backup entry path"));
    }
}
