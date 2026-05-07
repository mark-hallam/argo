//! `argo backup create`.

use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, Utc};
use flate2::write::GzEncoder;
use flate2::Compression;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::path::{Component, Path, PathBuf};

pub(crate) const MANIFEST_PATH: &str = "manifest.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct BackupManifest {
    pub(crate) schema_version: u16,
    pub(crate) created_at: String,
    pub(crate) workspace: String,
    pub(crate) include_credentials: bool,
    pub(crate) sha256: String,
    pub(crate) files: Vec<BackupFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct BackupFile {
    pub(crate) path: String,
    pub(crate) bytes: u64,
    pub(crate) sha256: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct CreateBackupOut {
    pub(crate) path: String,
    pub(crate) bytes: u64,
    pub(crate) sha256: String,
    pub(crate) files: usize,
    pub(crate) include_credentials: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct ArchivePayload {
    pub(crate) path: String,
    pub(crate) bytes: Vec<u8>,
}

struct BackupEntry {
    meta: BackupFile,
    payload: ArchivePayload,
}

pub async fn run(output: Option<&Path>, include_credentials: bool, json: bool) -> ArgoResult<()> {
    let workspace = std::env::current_dir()
        .map_err(|e| ArgoError::Provider(format!("current_dir: {e}")))?;
    let out = create_backup(&workspace, output, include_credentials, Utc::now())?;
    if json {
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({"ok": true, "backup_create": out}))
                .map_err(|e| ArgoError::Provider(format!("backup create JSON: {e}")))?
        );
    } else {
        println!("created backup {} ({} files)", out.path, out.files);
    }
    Ok(())
}

pub(crate) fn create_backup(
    workspace: &Path,
    output: Option<&Path>,
    include_credentials: bool,
    created_at: DateTime<Utc>,
) -> ArgoResult<CreateBackupOut> {
    let argo_dir = workspace.join(".argo");
    fs::create_dir_all(&argo_dir)?;
    let output_path = output_path(workspace, output, created_at)?;
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let entries = collect_entries(&argo_dir, &output_path, include_credentials)?;
    let payloads: Vec<ArchivePayload> = entries.iter().map(|entry| entry.payload.clone()).collect();
    let sha256 = payload_sha256(&payloads);
    let manifest = BackupManifest {
        schema_version: 1,
        created_at: created_at.to_rfc3339(),
        workspace: workspace.display().to_string(),
        include_credentials,
        sha256: sha256.clone(),
        files: entries.iter().map(|entry| entry.meta.clone()).collect(),
    };
    write_archive(&output_path, &manifest, &payloads)?;
    let bytes = fs::metadata(&output_path)?.len();
    Ok(CreateBackupOut {
        path: output_path.display().to_string(),
        bytes,
        sha256,
        files: payloads.len(),
        include_credentials,
    })
}

pub(crate) fn payload_sha256(files: &[ArchivePayload]) -> String {
    let mut hasher = Sha256::new();
    for file in files {
        hasher.update(file.path.as_bytes());
        hasher.update([0]);
        hasher.update(&file.bytes);
        hasher.update([0]);
    }
    hex::encode(hasher.finalize())
}

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

fn output_path(
    workspace: &Path,
    output: Option<&Path>,
    created_at: DateTime<Utc>,
) -> ArgoResult<PathBuf> {
    match output {
        Some(path) if path.is_absolute() => Ok(path.to_path_buf()),
        Some(path) => Ok(workspace.join(path)),
        None => Ok(workspace
            .join(".argo")
            .join("backups")
            .join(format!("{}.tar.gz", created_at.format("%Y%m%dT%H%M%SZ")))),
    }
}

fn collect_entries(
    argo_dir: &Path,
    output_path: &Path,
    include_credentials: bool,
) -> ArgoResult<Vec<BackupEntry>> {
    let mut entries = Vec::new();
    collect_dir(argo_dir, argo_dir, output_path, include_credentials, &mut entries)?;
    entries.sort_by(|a, b| a.meta.path.cmp(&b.meta.path));
    Ok(entries)
}

fn collect_dir(
    argo_dir: &Path,
    dir: &Path,
    output_path: &Path,
    include_credentials: bool,
    entries: &mut Vec<BackupEntry>,
) -> ArgoResult<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let rel = path
            .strip_prefix(argo_dir)
            .map_err(|e| ArgoError::Provider(format!("backup path: {e}")))?;
        if should_skip(rel, include_credentials) || path == output_path {
            continue;
        }
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            collect_dir(argo_dir, &path, output_path, include_credentials, entries)?;
        } else if file_type.is_file() {
            let bytes = fs::read(&path)?;
            let archive_path = archive_path(rel)?;
            entries.push(BackupEntry {
                meta: BackupFile {
                    path: archive_path.clone(),
                    bytes: bytes.len() as u64,
                    sha256: sha256_hex(&bytes),
                },
                payload: ArchivePayload {
                    path: archive_path,
                    bytes,
                },
            });
        }
    }
    Ok(())
}

fn should_skip(rel: &Path, include_credentials: bool) -> bool {
    let mut components = rel.components();
    matches!(components.next(), Some(Component::Normal(name)) if name == "backups")
        || (!include_credentials && rel == Path::new("credentials.toml"))
}

fn archive_path(rel: &Path) -> ArgoResult<String> {
    let mut parts = vec![".argo".to_string()];
    for component in rel.components() {
        match component {
            Component::Normal(value) => {
                let value = value.to_str().ok_or_else(|| {
                    ArgoError::Provider(format!("backup path is not UTF-8: {}", rel.display()))
                })?;
                parts.push(value.to_string());
            }
            _ => {
                return Err(ArgoError::Provider(format!(
                    "unsupported backup path: {}",
                    rel.display()
                )));
            }
        }
    }
    Ok(parts.join("/"))
}

fn write_archive(
    path: &Path,
    manifest: &BackupManifest,
    files: &[ArchivePayload],
) -> ArgoResult<()> {
    let file = File::create(path)?;
    let encoder = GzEncoder::new(file, Compression::default());
    let mut builder = tar::Builder::new(encoder);
    for payload in files {
        append_bytes(&mut builder, &payload.path, &payload.bytes)?;
    }
    let manifest_bytes = serde_json::to_vec_pretty(manifest)
        .map_err(|e| ArgoError::Provider(format!("backup manifest JSON: {e}")))?;
    append_bytes(&mut builder, MANIFEST_PATH, &manifest_bytes)?;
    let encoder = builder.into_inner()?;
    encoder.finish()?;
    Ok(())
}

fn append_bytes<W: std::io::Write>(
    builder: &mut tar::Builder<W>,
    path: &str,
    bytes: &[u8],
) -> ArgoResult<()> {
    let mut header = tar::Header::new_gnu();
    header.set_size(bytes.len() as u64);
    header.set_mode(0o600);
    header.set_cksum();
    builder.append_data(&mut header, path, bytes)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn collect_entries_excludes_credentials_and_backups_by_default() {
        let temp = tempfile::tempdir().unwrap();
        let argo = temp.path().join(".argo");
        fs::create_dir_all(argo.join("backups")).unwrap();
        fs::write(argo.join("state.db"), "state").unwrap();
        fs::write(argo.join("credentials.toml"), "secret").unwrap();
        fs::write(argo.join("backups").join("old.tar.gz"), "old").unwrap();

        let entries = collect_entries(&argo, &argo.join("backups/new.tar.gz"), false).unwrap();
        let paths: Vec<_> = entries.iter().map(|entry| entry.meta.path.as_str()).collect();

        assert_eq!(paths, vec![".argo/state.db"]);
    }

    #[test]
    fn collect_entries_includes_credentials_when_requested() {
        let temp = tempfile::tempdir().unwrap();
        let argo = temp.path().join(".argo");
        fs::create_dir_all(&argo).unwrap();
        fs::write(argo.join("credentials.toml"), "secret").unwrap();

        let entries = collect_entries(&argo, &argo.join("backup.tar.gz"), true).unwrap();

        assert_eq!(entries[0].meta.path, ".argo/credentials.toml");
    }

    #[test]
    fn create_backup_writes_default_path_under_backups() {
        let temp = tempfile::tempdir().unwrap();
        let created_at = Utc.with_ymd_and_hms(2026, 5, 7, 1, 2, 3).unwrap();

        let out = create_backup(temp.path(), None, false, created_at).unwrap();

        assert!(out.path.ends_with(".argo/backups/20260507T010203Z.tar.gz"));
    }
}
