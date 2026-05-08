//! `argo genesis export` portable vertical bundle writer.

use argo_types::{ArgoError, ArgoResult};
use argo_vertical::{LoadedVertical, VerticalManifest};
use flate2::write::GzEncoder;
use flate2::Compression;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::path::{Component, Path, PathBuf};

const BUNDLE_MANIFEST: &str = "BUNDLE.toml";

#[derive(Debug, Clone)]
pub struct GenesisExportArgs {
    pub vertical: String,
    pub output: Option<PathBuf>,
    pub include_skills: bool,
    pub manifest_only: bool,
    pub force: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct BundleManifest {
    pub(crate) schema_version: u16,
    pub(crate) vertical_id: String,
    pub(crate) vertical_name: String,
    pub(crate) vertical_version: String,
    pub(crate) argo_version: String,
    pub(crate) files: Vec<BundleFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct BundleFile {
    pub(crate) path: String,
    pub(crate) size: u64,
    pub(crate) sha256: String,
}

#[derive(Debug, Clone)]
struct BundleEntry {
    meta: BundleFile,
    bytes: Vec<u8>,
}

#[derive(Debug, Clone, Serialize)]
struct ExportOutcome {
    path: String,
    sha256: String,
    files: usize,
}

pub fn handle_export(args: GenesisExportArgs) -> ArgoResult<()> {
    let verticals_dir = argo_vertical::default_dir();
    let loaded = load_vertical(&verticals_dir, &args.vertical)?;
    let output = output_path(&args.vertical, args.output.as_deref(), args.manifest_only);
    let outcome = export_loaded(
        &loaded,
        &output,
        args.include_skills,
        args.manifest_only,
        args.force,
    )?;
    println!(
        "exported {} to {} ({} files, sha256 {})",
        args.vertical, outcome.path, outcome.files, outcome.sha256
    );
    Ok(())
}

fn load_vertical(verticals_dir: &Path, vertical: &str) -> ArgoResult<LoadedVertical> {
    let (registry, _errors) = argo_vertical::load_dir(verticals_dir)?;
    registry.get(vertical).cloned().ok_or_else(|| {
        ArgoError::Provider(format!(
            "vertical '{vertical}' not found in {}",
            verticals_dir.display()
        ))
    })
}

fn export_loaded(
    loaded: &LoadedVertical,
    output: &Path,
    include_skills: bool,
    manifest_only: bool,
    force: bool,
) -> ArgoResult<ExportOutcome> {
    if output.exists() && !force {
        return Err(ArgoError::Schema(format!(
            "output already exists: {} (use --force to overwrite)",
            output.display()
        )));
    }
    if let Some(parent) = output.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    let entries = collect_entries(loaded, include_skills)?;
    let manifest = build_manifest(loaded, &entries);
    let manifest_bytes = toml::to_string_pretty(&manifest)
        .map_err(|e| ArgoError::Schema(format!("bundle manifest TOML: {e}")))?
        .into_bytes();
    if manifest_only {
        fs::write(output, &manifest_bytes)?;
    } else {
        write_archive(output, &manifest_bytes, &entries)?;
    }
    let sha256 = sha256_bytes(&fs::read(output)?);
    Ok(ExportOutcome {
        path: output.display().to_string(),
        sha256,
        files: entries.len(),
    })
}

fn output_path(vertical: &str, output: Option<&Path>, manifest_only: bool) -> PathBuf {
    match output {
        Some(path) => path.to_path_buf(),
        None if manifest_only => PathBuf::from(BUNDLE_MANIFEST),
        None => PathBuf::from(format!("{vertical}.argo-vertical.tar.gz")),
    }
}

fn build_manifest(loaded: &LoadedVertical, entries: &[BundleEntry]) -> BundleManifest {
    BundleManifest {
        schema_version: 1,
        vertical_id: loaded.manifest.vertical.id.clone(),
        vertical_name: loaded.manifest.vertical.name.clone(),
        vertical_version: loaded.manifest.vertical.version.clone(),
        argo_version: env!("CARGO_PKG_VERSION").to_string(),
        files: entries.iter().map(|entry| entry.meta.clone()).collect(),
    }
}

fn collect_entries(loaded: &LoadedVertical, include_skills: bool) -> ArgoResult<Vec<BundleEntry>> {
    let mut paths = vec![PathBuf::from("vertical.toml")];
    collect_manifest_paths(&loaded.root, &loaded.manifest, &mut paths)?;
    if include_skills {
        if let Some(skills) = &loaded.manifest.skills {
            push_ref_path(&loaded.root, &skills.bundle, &mut paths)?;
        } else {
            let skills_dir = loaded.root.join("skills");
            if skills_dir.is_dir() {
                collect_dir(&loaded.root, &skills_dir, &mut paths)?;
            }
        }
    }
    paths.sort();
    paths.dedup();
    paths
        .into_iter()
        .map(|path| read_entry(&loaded.root, path))
        .collect()
}

fn collect_manifest_paths(
    root: &Path,
    manifest: &VerticalManifest,
    paths: &mut Vec<PathBuf>,
) -> ArgoResult<()> {
    if let Some(role) = &manifest.role {
        push_ref_path(root, &role.system_prompt, paths)?;
        if let Some(persona) = &role.persona {
            push_ref_path(root, persona, paths)?;
        }
    }
    if let Some(memory) = &manifest.memory {
        if let Some(schema) = &memory.schema {
            push_ref_path(root, schema, paths)?;
        }
        if let Some(seed) = &memory.seed {
            push_ref_path(root, seed, paths)?;
        }
    }
    if let Some(eval) = &manifest.evaluation {
        for suite in &eval.suites {
            push_ref_path(root, suite, paths)?;
        }
    }
    if let Some(workflows) = &manifest.workflows {
        for file in &workflows.files {
            push_ref_path(root, file, paths)?;
        }
    }
    if let Some(actions) = &manifest.actions {
        for action in actions.actions.values() {
            if let Some(schema) = &action.payload_schema {
                push_ref_path(root, schema, paths)?;
            }
        }
    }
    Ok(())
}

fn push_ref_path(root: &Path, raw: &str, paths: &mut Vec<PathBuf>) -> ArgoResult<()> {
    let rel = safe_rel_path(raw)?;
    let abs = root.join(&rel);
    if abs.is_dir() {
        collect_dir(root, &abs, paths)
    } else {
        paths.push(rel);
        Ok(())
    }
}

fn collect_dir(root: &Path, dir: &Path, paths: &mut Vec<PathBuf>) -> ArgoResult<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            collect_dir(root, &path, paths)?;
        } else if entry.file_type()?.is_file() {
            paths.push(
                path.strip_prefix(root)
                    .map_err(|e| ArgoError::Provider(format!("bundle path: {e}")))?
                    .to_path_buf(),
            );
        }
    }
    Ok(())
}

fn read_entry(root: &Path, rel: PathBuf) -> ArgoResult<BundleEntry> {
    let archive_path = archive_path(&rel)?;
    let path = root.join(&rel);
    let bytes =
        fs::read(&path).map_err(|e| ArgoError::Io(format!("read {}: {e}", path.display())))?;
    Ok(BundleEntry {
        meta: BundleFile {
            path: archive_path,
            size: bytes.len() as u64,
            sha256: sha256_bytes(&bytes),
        },
        bytes,
    })
}

fn safe_rel_path(raw: &str) -> ArgoResult<PathBuf> {
    let path = Path::new(raw);
    if path.is_absolute() {
        return Err(ArgoError::Provider(format!(
            "vertical bundle path must be relative: {raw}"
        )));
    }
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => out.push(part),
            _ => {
                return Err(ArgoError::Provider(format!(
                    "unsupported vertical bundle path: {raw}"
                )));
            }
        }
    }
    Ok(out)
}

fn archive_path(rel: &Path) -> ArgoResult<String> {
    let mut parts = Vec::new();
    for component in rel.components() {
        match component {
            Component::Normal(part) => parts.push(
                part.to_str()
                    .ok_or_else(|| {
                        ArgoError::Provider(format!("bundle path is not UTF-8: {}", rel.display()))
                    })?
                    .to_string(),
            ),
            _ => {
                return Err(ArgoError::Provider(format!(
                    "unsupported bundle path: {}",
                    rel.display()
                )));
            }
        }
    }
    Ok(parts.join("/"))
}

fn write_archive(path: &Path, manifest: &[u8], entries: &[BundleEntry]) -> ArgoResult<()> {
    let file = File::create(path)?;
    let encoder = GzEncoder::new(file, Compression::default());
    let mut builder = tar::Builder::new(encoder);
    append_bytes(&mut builder, BUNDLE_MANIFEST, manifest)?;
    for entry in entries {
        append_bytes(&mut builder, &entry.meta.path, &entry.bytes)?;
    }
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
    header.set_mtime(0);
    header.set_uid(0);
    header.set_gid(0);
    header.set_cksum();
    builder.append_data(&mut header, path, bytes)?;
    Ok(())
}

fn sha256_bytes(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex::encode(h.finalize())
}

#[cfg(test)]
#[path = "genesis_export_tests.rs"]
mod tests;
