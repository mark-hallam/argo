//! Skill marketplace publish helpers.
//!
//! The V2 marketplace publish path starts with a deterministic
//! filesystem-backed `mock://` registry. The layout is OCI-shaped:
//! tag pointers live under `oci/<skill>/<tag>/manifest.json`, while
//! immutable blobs live under `oci/blobs/sha256/<digest>.tar`.

use crate::{parse_skill_md, Skill};
use argo_types::{ArgoError, ArgoResult};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

/// A skill source ready to be packaged for marketplace publish.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillPublishSource {
    /// Skill package name.
    pub name: String,
    /// Source skill version.
    pub version: String,
    /// Human-readable description.
    pub description: String,
    /// Complete `SKILL.md` bytes as UTF-8 text.
    pub raw: String,
}

impl SkillPublishSource {
    /// Build a publish source from an active [`SkillStore`](crate::SkillStore) row.
    pub fn from_skill(skill: &Skill) -> ArgoResult<Self> {
        let yaml = serde_yaml::to_string(&skill.frontmatter)
            .map_err(|e| ArgoError::Provider(format!("serialize skill frontmatter: {e}")))?;
        let raw = format!("---\n{yaml}---\n{}", skill.body);
        Self::from_raw(&raw)
    }

    /// Parse a raw `SKILL.md` document into a publish source.
    pub fn from_raw(raw: &str) -> ArgoResult<Self> {
        let (front, _body) = parse_skill_md(raw)?;
        Ok(Self {
            name: front.name,
            version: front.version,
            description: front.description,
            raw: raw.to_string(),
        })
    }
}

/// Manifest record written for a published skill tag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillPublishRecord {
    /// Skill package name.
    pub name: String,
    /// Published tag.
    pub tag: String,
    /// Source skill version.
    pub version: String,
    /// Human-readable description.
    pub description: String,
    /// SHA-256 digest of the tar bundle.
    pub digest: String,
    /// Bundle size in bytes.
    pub bytes: usize,
    /// Relative manifest path under the mock registry root.
    pub manifest_path: String,
    /// Relative tag-local bundle path under the mock registry root.
    pub tag_bundle_path: String,
    /// Relative immutable blob path under the mock registry root.
    pub blob_path: String,
    /// Canonical mock OCI reference.
    pub oci_ref: String,
    /// RFC-3339 timestamp, or `DRY-RUN`.
    pub published_at: String,
    /// Whether this record came from a dry run.
    pub dry_run: bool,
}

/// One published tag for `argo skill tags`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillTag {
    /// Skill package name.
    pub name: String,
    /// Published tag.
    pub tag: String,
    /// SHA-256 digest of the tar bundle.
    pub digest: String,
    /// RFC-3339 publish timestamp.
    pub published_at: String,
    /// Canonical mock OCI reference.
    pub oci_ref: String,
}

/// Result from removing a mutable tag pointer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillUnpublishRecord {
    /// Skill package name.
    pub name: String,
    /// Removed tag.
    pub tag: String,
    /// Whether the tag directory existed and was removed.
    pub removed: bool,
    /// Digest that remains in the immutable blob store, if known.
    pub preserved_digest: Option<String>,
    /// Whether the blob still exists after tag removal.
    pub blob_preserved: bool,
}

/// Convert `mock://path` into the local mock registry root.
pub fn mock_registry_root(registry: &str) -> ArgoResult<PathBuf> {
    let Some(path) = registry.strip_prefix("mock://") else {
        return Err(ArgoError::Provider(format!(
            "only mock:// registries are supported for skill publish in this build: {registry}"
        )));
    };
    Ok(if path.is_empty() { "." } else { path }.into())
}

/// Publish a skill to the filesystem-backed mock OCI registry.
pub fn publish_mock_skill(
    root: &Path,
    source: &SkillPublishSource,
    tag: &str,
    dry_run: bool,
) -> ArgoResult<SkillPublishRecord> {
    validate_segment("skill name", &source.name)?;
    validate_segment("tag", tag)?;
    let tar = build_skill_tar(source, tag)?;
    let digest = sha256_hex(&tar);
    let record = publish_record(root, source, tag, &digest, tar.len(), dry_run);
    if dry_run {
        return Ok(record);
    }
    let blob_abs = root.join(&record.blob_path);
    let tag_bundle_abs = root.join(&record.tag_bundle_path);
    let manifest_abs = root.join(&record.manifest_path);
    write_file(&blob_abs, &tar)?;
    write_file(&tag_bundle_abs, &tar)?;
    let manifest = serde_json::to_vec_pretty(&record)
        .map_err(|e| ArgoError::Provider(format!("serialize skill publish manifest: {e}")))?;
    write_file(&manifest_abs, &manifest)?;
    Ok(record)
}

/// List mock registry tags for a skill.
pub fn list_mock_skill_tags(root: &Path, name: &str, limit: usize) -> ArgoResult<Vec<SkillTag>> {
    validate_segment("skill name", name)?;
    let dir = root.join("oci").join(name);
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut rows = Vec::new();
    for entry in
        fs::read_dir(&dir).map_err(|e| ArgoError::Io(format!("read {}: {e}", dir.display())))?
    {
        let entry = entry.map_err(|e| ArgoError::Io(format!("read tag dir: {e}")))?;
        let manifest = entry.path().join("manifest.json");
        if manifest.exists() {
            rows.push(tag_from_manifest(&manifest)?);
        }
    }
    rows.sort_by(|a, b| {
        b.published_at
            .cmp(&a.published_at)
            .then_with(|| a.tag.cmp(&b.tag))
    });
    rows.truncate(limit);
    Ok(rows)
}

/// Remove a mock registry tag while preserving the immutable blob.
pub fn unpublish_mock_skill_tag(root: &Path, reference: &str) -> ArgoResult<SkillUnpublishRecord> {
    let (name, tag) = split_skill_ref(reference)?;
    let tag_dir = root.join("oci").join(&name).join(&tag);
    let manifest = tag_dir.join("manifest.json");
    let record = if manifest.exists() {
        Some(read_record(&manifest)?)
    } else {
        None
    };
    if tag_dir.exists() {
        fs::remove_dir_all(&tag_dir)
            .map_err(|e| ArgoError::Io(format!("remove {}: {e}", tag_dir.display())))?;
    }
    let blob_preserved = record
        .as_ref()
        .is_some_and(|r| root.join(&r.blob_path).exists());
    Ok(SkillUnpublishRecord {
        name,
        tag,
        removed: record.is_some(),
        preserved_digest: record.map(|r| r.digest),
        blob_preserved,
    })
}

fn publish_record(
    root: &Path,
    source: &SkillPublishSource,
    tag: &str,
    digest: &str,
    bytes: usize,
    dry_run: bool,
) -> SkillPublishRecord {
    let manifest_path = format!("oci/{}/{tag}/manifest.json", source.name);
    let tag_bundle_path = format!("oci/{}/{tag}/bundle.tar", source.name);
    let blob_path = format!("oci/blobs/sha256/{digest}.tar");
    SkillPublishRecord {
        name: source.name.clone(),
        tag: tag.to_string(),
        version: source.version.clone(),
        description: source.description.clone(),
        digest: digest.to_string(),
        bytes,
        manifest_path,
        tag_bundle_path,
        blob_path,
        oci_ref: format!("mock://{}/oci/{}:{tag}", root.display(), source.name),
        published_at: if dry_run {
            "DRY-RUN".into()
        } else {
            Utc::now().to_rfc3339()
        },
        dry_run,
    }
}

fn tag_from_manifest(path: &Path) -> ArgoResult<SkillTag> {
    let record = read_record(path)?;
    Ok(SkillTag {
        name: record.name,
        tag: record.tag,
        digest: record.digest,
        published_at: record.published_at,
        oci_ref: record.oci_ref,
    })
}

fn read_record(path: &Path) -> ArgoResult<SkillPublishRecord> {
    let body = fs::read_to_string(path)
        .map_err(|e| ArgoError::Io(format!("read {}: {e}", path.display())))?;
    serde_json::from_str(&body)
        .map_err(|e| ArgoError::Provider(format!("parse {}: {e}", path.display())))
}

fn split_skill_ref(reference: &str) -> ArgoResult<(String, String)> {
    let (name, tag) = reference.rsplit_once(':').ok_or_else(|| {
        ArgoError::Schema(format!("skill ref '{reference}' must be <name>:<tag>"))
    })?;
    validate_segment("skill name", name)?;
    validate_segment("tag", tag)?;
    Ok((name.to_string(), tag.to_string()))
}

fn validate_segment(label: &str, value: &str) -> ArgoResult<()> {
    if value.is_empty() || value.contains('/') || value.contains('\\') || value.contains("..") {
        return Err(ArgoError::Schema(format!(
            "{label} '{value}' must be a non-empty path segment"
        )));
    }
    Ok(())
}

fn write_file(path: &Path, bytes: &[u8]) -> ArgoResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| ArgoError::Io(format!("create {}: {e}", parent.display())))?;
    }
    fs::write(path, bytes).map_err(|e| ArgoError::Io(format!("write {}: {e}", path.display())))
}

fn build_skill_tar(source: &SkillPublishSource, tag: &str) -> ArgoResult<Vec<u8>> {
    let meta = serde_json::to_vec_pretty(&serde_json::json!({
        "schema_version": 1,
        "kind": "argo.skill",
        "name": source.name,
        "version": source.version,
        "tag": tag,
        "description": source.description,
    }))
    .map_err(|e| ArgoError::Provider(format!("serialize skill bundle metadata: {e}")))?;
    let mut out = Vec::new();
    append_tar_entry(&mut out, "SKILL.md", source.raw.as_bytes())?;
    append_tar_entry(&mut out, "skill-publish.json", &meta)?;
    out.extend_from_slice(&[0_u8; 1024]);
    Ok(out)
}

fn append_tar_entry(out: &mut Vec<u8>, path: &str, body: &[u8]) -> ArgoResult<()> {
    let header = tar_header(path, body.len() as u64)?;
    out.extend_from_slice(&header);
    out.extend_from_slice(body);
    let pad = (512 - (body.len() % 512)) % 512;
    out.resize(out.len() + pad, 0);
    Ok(())
}

fn tar_header(path: &str, size: u64) -> ArgoResult<[u8; 512]> {
    if path.len() > 100 {
        return Err(ArgoError::Schema(format!("tar path too long: {path}")));
    }
    let mut header = [0_u8; 512];
    header[..path.len()].copy_from_slice(path.as_bytes());
    write_octal(&mut header[100..108], 0o644);
    write_octal(&mut header[108..116], 0);
    write_octal(&mut header[116..124], 0);
    write_octal(&mut header[124..136], size);
    write_octal(&mut header[136..148], 0);
    header[148..156].fill(b' ');
    header[156] = b'0';
    header[257..263].copy_from_slice(b"ustar\0");
    header[263..265].copy_from_slice(b"00");
    let checksum: u32 = header.iter().map(|b| u32::from(*b)).sum();
    write_checksum(&mut header[148..156], checksum);
    Ok(header)
}

fn write_octal(slot: &mut [u8], value: u64) {
    let width = slot.len();
    let text = format!("{value:0width$o}", width = width - 1);
    slot[..width - 1].copy_from_slice(text.as_bytes());
    slot[width - 1] = 0;
}

fn write_checksum(slot: &mut [u8], value: u32) {
    let text = format!("{value:06o}\0 ");
    slot.copy_from_slice(text.as_bytes());
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex::encode(h.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source() -> SkillPublishSource {
        SkillPublishSource::from_raw(
            "---\nname: publish-me\nversion: \"0.1.0\"\ndescription: \"Publish me\"\nprovenance:\n  source: test\n  hash: sha256-test\n---\n# Body\n",
        )
        .unwrap()
    }

    fn tmp_root(label: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("argo-skill-publish-{label}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn mock_registry_root_accepts_mock_urls() {
        assert_eq!(
            mock_registry_root("mock://target").unwrap(),
            PathBuf::from("target")
        );
        assert!(mock_registry_root("https://registry.example").is_err());
    }

    #[test]
    fn publish_writes_tag_manifest_and_blob() {
        let root = tmp_root("publish");
        let record = publish_mock_skill(&root, &source(), "0.1.0", false).unwrap();
        assert!(root.join(&record.manifest_path).exists());
        assert!(root.join(&record.tag_bundle_path).exists());
        assert!(root.join(&record.blob_path).exists());
        assert!(record.bytes > 1024);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn dry_run_does_not_write_files() {
        let root = tmp_root("dry-run");
        let record = publish_mock_skill(&root, &source(), "dry", true).unwrap();
        assert!(record.dry_run);
        assert!(!root.join("oci").exists());
    }

    #[test]
    fn list_mock_tags_returns_newest_first_with_limit() {
        let root = tmp_root("tags");
        publish_mock_skill(&root, &source(), "0.1.0", false).unwrap();
        publish_mock_skill(&root, &source(), "0.2.0", false).unwrap();
        let rows = list_mock_skill_tags(&root, "publish-me", 1).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "publish-me");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn unpublish_removes_tag_but_preserves_blob() {
        let root = tmp_root("unpublish");
        let record = publish_mock_skill(&root, &source(), "0.1.0", false).unwrap();
        let out = unpublish_mock_skill_tag(&root, "publish-me:0.1.0").unwrap();
        assert!(out.removed);
        assert!(out.blob_preserved);
        assert!(!root.join("oci/publish-me/0.1.0").exists());
        assert!(!root.join(record.tag_bundle_path).exists());
        assert!(root.join(record.blob_path).exists());
        let _ = fs::remove_dir_all(root);
    }
}
