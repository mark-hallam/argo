//! Portable offline skill artifact metadata and verification.
use crate::SkillFrontmatter;
use argo_types::{ArgoError, ArgoResult};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Artifact schema version supported by this crate.
pub const SKILL_ARTIFACT_SCHEMA_VERSION: u16 = 1;
/// Path of the JSON manifest inside a `.skill.tar.gz` artifact.
pub const SKILL_ARTIFACT_MANIFEST: &str = "manifest.json";
/// Path of the Markdown body inside a `.skill.tar.gz` artifact.
pub const SKILL_ARTIFACT_BODY: &str = "skill.md";
/// Legacy Markdown body path accepted for v1 `.skill.tar.gz` artifacts.
pub const SKILL_ARTIFACT_BODY_LEGACY: &str = "body.md";
/// Path of the checksum sidecar inside a `.skill.tar.gz` artifact.
pub const SKILL_ARTIFACT_SHA256: &str = "sha256.txt";
/// Path of the optional distribution metadata sidecar.
pub const SKILL_ARTIFACT_METADATA: &str = "metadata.json";
/// Stable metadata format string for v2 portable skill artifacts.
pub const SKILL_ARTIFACT_METADATA_FORMAT: &str = "skycot.skill.v2";

/// Signed metadata stored as `manifest.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillArtifactManifest {
    /// Schema version for forward-compatible import.
    pub schema_version: u16,
    /// Original skill frontmatter.
    pub frontmatter: SkillFrontmatter,
    /// `sha256-<hex>` of the exported skill body.
    pub body_sha256: String,
    /// Deterministic signature over schema, frontmatter, and body hash.
    pub signature: String,
}

/// Operator-facing distribution metadata stored as `metadata.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillArtifactMetadata {
    /// Stable artifact format identifier.
    pub artifact_format: String,
    /// Trust grade derived from skill provenance.
    pub trust_grade: String,
    /// Vertical that owns or bundled the skill, when known.
    pub vertical_id: Option<String>,
    /// Session that learned the skill, when known.
    pub learned_in: Option<String>,
    /// RFC-3339 export timestamp.
    pub exported_at: String,
    /// Exporting binary version.
    pub exporter_version: String,
    /// `sha256-<hex>` of the exported skill body.
    pub body_sha256: String,
}

impl SkillArtifactMetadata {
    /// Build metadata for a skill artifact export.
    #[must_use]
    pub fn new(
        trust_grade: String,
        vertical_id: Option<String>,
        learned_in: Option<String>,
        exported_at: String,
        exporter_version: String,
        body_sha256: String,
    ) -> Self {
        Self {
            artifact_format: SKILL_ARTIFACT_METADATA_FORMAT.to_string(),
            trust_grade,
            vertical_id,
            learned_in,
            exported_at,
            exporter_version,
            body_sha256,
        }
    }
}

/// A portable skill artifact after archive extraction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillArtifact {
    /// Signed artifact manifest.
    pub manifest: SkillArtifactManifest,
    /// Raw skill body from `skill.md`.
    pub body: String,
    /// Optional v2 distribution metadata sidecar.
    pub metadata: Option<SkillArtifactMetadata>,
}

impl SkillArtifact {
    /// Build an artifact from installed skill bytes.
    ///
    /// ```
    /// use argo_skill::{Provenance, SkillArtifact, SkillFrontmatter};
    /// use argo_types::{CapabilitySet, Lock};
    /// let frontmatter = SkillFrontmatter {
    ///     name: "demo".into(),
    ///     version: "0.1".into(),
    ///     description: "demo".into(),
    ///     capabilities: CapabilitySet::empty(),
    ///     provenance: Provenance { source: "test".into(), hash: "sha256-x".into() },
    ///     lock: Lock::None,
    ///     evals: Vec::new(),
    /// };
    /// let artifact = SkillArtifact::new(frontmatter, "body".into());
    /// assert!(artifact.verify().is_ok());
    /// ```
    #[must_use]
    pub fn new(frontmatter: SkillFrontmatter, body: String) -> Self {
        let body_sha256 = skill_body_sha256(&body);
        let signature = signature_for(SKILL_ARTIFACT_SCHEMA_VERSION, &frontmatter, &body_sha256);
        Self {
            manifest: SkillArtifactManifest {
                schema_version: SKILL_ARTIFACT_SCHEMA_VERSION,
                frontmatter,
                body_sha256,
                signature,
            },
            body,
            metadata: None,
        }
    }

    /// Attach optional distribution metadata to this artifact.
    #[must_use]
    pub fn with_metadata(mut self, metadata: SkillArtifactMetadata) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Parse the three required archive entries and verify them.
    ///
    /// # Errors
    /// Returns an error when JSON, UTF-8, checksum, or signature
    /// validation fails.
    pub fn from_archive_parts(
        manifest_bytes: &[u8],
        body_bytes: &[u8],
        sha256_bytes: &[u8],
    ) -> ArgoResult<Self> {
        Self::from_archive_parts_with_metadata(manifest_bytes, body_bytes, sha256_bytes, None)
    }

    /// Parse archive entries, including optional `metadata.json`, and
    /// verify them.
    ///
    /// # Errors
    /// Returns an error when JSON, UTF-8, checksum, signature, or
    /// metadata validation fails.
    pub fn from_archive_parts_with_metadata(
        manifest_bytes: &[u8],
        body_bytes: &[u8],
        sha256_bytes: &[u8],
        metadata_bytes: Option<&[u8]>,
    ) -> ArgoResult<Self> {
        let manifest: SkillArtifactManifest = serde_json::from_slice(manifest_bytes)
            .map_err(|e| ArgoError::Schema(format!("skill artifact manifest JSON: {e}")))?;
        let body = String::from_utf8(body_bytes.to_vec())
            .map_err(|e| ArgoError::Schema(format!("skill artifact body UTF-8: {e}")))?;
        let declared = String::from_utf8(sha256_bytes.to_vec())
            .map_err(|e| ArgoError::Schema(format!("skill artifact sha256 UTF-8: {e}")))?;
        let sha = declared.split_whitespace().next().ok_or_else(|| {
            ArgoError::Schema("skill artifact sha256.txt must start with a hash".into())
        })?;
        if sha != manifest.body_sha256 {
            return Err(ArgoError::Schema(format!(
                "skill artifact sha256.txt declares {sha} but manifest declares {}",
                manifest.body_sha256
            )));
        }
        let metadata = match metadata_bytes {
            Some(bytes) => {
                let metadata: SkillArtifactMetadata = serde_json::from_slice(bytes)
                    .map_err(|e| ArgoError::Schema(format!("skill artifact metadata JSON: {e}")))?;
                if metadata.artifact_format != SKILL_ARTIFACT_METADATA_FORMAT {
                    return Err(ArgoError::Schema(format!(
                        "unsupported skill artifact metadata format {}",
                        metadata.artifact_format
                    )));
                }
                if metadata.body_sha256 != manifest.body_sha256 {
                    return Err(ArgoError::Schema(format!(
                        "skill artifact metadata body_sha256 declares {} but manifest declares {}",
                        metadata.body_sha256, manifest.body_sha256
                    )));
                }
                Some(metadata)
            }
            None => None,
        };
        let artifact = Self {
            manifest,
            body,
            metadata,
        };
        artifact.verify()?;
        Ok(artifact)
    }

    /// Serialize the manifest as stable JSON bytes.
    ///
    /// # Errors
    /// Returns an error when manifest serialization fails.
    pub fn manifest_json(&self) -> ArgoResult<Vec<u8>> {
        serde_json::to_vec_pretty(&self.manifest)
            .map_err(|e| ArgoError::Provider(format!("skill artifact manifest JSON: {e}")))
    }

    /// Serialize the optional metadata sidecar as stable JSON bytes.
    ///
    /// # Errors
    /// Returns an error when metadata serialization fails.
    pub fn metadata_json(&self) -> ArgoResult<Option<Vec<u8>>> {
        self.metadata
            .as_ref()
            .map(|metadata| {
                serde_json::to_vec_pretty(metadata)
                    .map_err(|e| ArgoError::Provider(format!("skill artifact metadata JSON: {e}")))
            })
            .transpose()
    }

    /// Render the checksum sidecar.
    #[must_use]
    pub fn sha256_text(&self) -> String {
        format!("{}  {SKILL_ARTIFACT_BODY}\n", self.manifest.body_sha256)
    }

    /// Verify schema version, body checksum, and manifest signature.
    ///
    /// # Errors
    /// Returns an error when the artifact is unsupported or tampered.
    pub fn verify(&self) -> ArgoResult<()> {
        if self.manifest.schema_version != SKILL_ARTIFACT_SCHEMA_VERSION {
            return Err(ArgoError::Schema(format!(
                "unsupported skill artifact schema_version {}",
                self.manifest.schema_version
            )));
        }
        let actual = skill_body_sha256(&self.body);
        if actual != self.manifest.body_sha256 {
            return Err(ArgoError::Schema(format!(
                "skill artifact body hash mismatch: manifest declares {} but body hashes to {actual}",
                self.manifest.body_sha256
            )));
        }
        let expected = signature_for(
            self.manifest.schema_version,
            &self.manifest.frontmatter,
            &self.manifest.body_sha256,
        );
        if expected != self.manifest.signature {
            return Err(ArgoError::Schema(
                "skill artifact signature mismatch".into(),
            ));
        }
        Ok(())
    }

    /// Return frontmatter, optionally replacing the skill name.
    #[must_use]
    pub fn frontmatter_for_import(&self, name: Option<&str>) -> SkillFrontmatter {
        let mut frontmatter = self.manifest.frontmatter.clone();
        if let Some(name) = name {
            frontmatter.name = name.to_string();
        }
        frontmatter
    }
}

/// Hash a skill body as `sha256-<hex>`.
///
/// ```
/// use argo_skill::skill_body_sha256;
/// let h = skill_body_sha256("hello");
/// assert!(h.starts_with("sha256-"));
/// assert_eq!(h, skill_body_sha256("hello"));
/// assert_ne!(h, skill_body_sha256("world"));
/// ```
#[must_use]
pub fn skill_body_sha256(body: &str) -> String {
    let mut h = Sha256::new();
    h.update(body.as_bytes());
    format!("sha256-{}", hex::encode(h.finalize()))
}

#[derive(Serialize)]
struct SignaturePayload<'a> {
    schema_version: u16,
    frontmatter: &'a SkillFrontmatter,
    body_sha256: &'a str,
}

fn signature_for(schema_version: u16, frontmatter: &SkillFrontmatter, body_sha256: &str) -> String {
    let payload = SignaturePayload {
        schema_version,
        frontmatter,
        body_sha256,
    };
    let bytes = serde_json::to_vec(&payload).unwrap_or_default();
    let mut h = Sha256::new();
    h.update(b"argo-skill-artifact-v1\n");
    h.update(bytes);
    format!("sha256-{}", hex::encode(h.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Provenance;
    use argo_types::{CapabilitySet, Lock};

    fn frontmatter() -> SkillFrontmatter {
        SkillFrontmatter {
            name: "offline".into(),
            version: "1.0".into(),
            description: "Offline".into(),
            capabilities: CapabilitySet::empty(),
            provenance: Provenance {
                source: "seed".into(),
                hash: "sha256-seed".into(),
            },
            lock: Lock::UserProtected,
            evals: Vec::new(),
        }
    }

    #[test]
    fn artifact_sets_body_hash() {
        let artifact = SkillArtifact::new(frontmatter(), "body".into());
        assert_eq!(artifact.manifest.body_sha256, skill_body_sha256("body"));
    }

    #[test]
    fn pristine_artifact_verifies() {
        let artifact = SkillArtifact::new(frontmatter(), "body".into());
        assert!(artifact.verify().is_ok());
    }

    #[test]
    fn body_tamper_fails_verification() {
        let mut artifact = SkillArtifact::new(frontmatter(), "body".into());
        artifact.body.push_str(" tampered");
        assert!(artifact.verify().is_err());
    }

    #[test]
    fn signature_tamper_fails_verification() {
        let mut artifact = SkillArtifact::new(frontmatter(), "body".into());
        artifact.manifest.frontmatter.description = "changed".into();
        assert!(artifact.verify().is_err());
    }

    #[test]
    fn archive_parts_require_matching_sha_sidecar() {
        let artifact = SkillArtifact::new(frontmatter(), "body".into());
        let manifest = artifact.manifest_json().unwrap();
        let err = SkillArtifact::from_archive_parts(&manifest, b"body", b"sha256-wrong  body.md\n")
            .unwrap_err();
        assert!(matches!(err, ArgoError::Schema(_)));
    }

    #[test]
    fn metadata_round_trips_from_archive_parts() {
        let artifact = SkillArtifact::new(frontmatter(), "body".into());
        let metadata = SkillArtifactMetadata::new(
            "tenant".into(),
            Some("finance-ar".into()),
            Some("018f55b7-89ab-7cde-8123-456789abcdef".into()),
            "2026-05-08T00:00:00Z".into(),
            "0.0.1".into(),
            artifact.manifest.body_sha256.clone(),
        );
        let artifact = artifact.with_metadata(metadata.clone());
        let parsed = SkillArtifact::from_archive_parts_with_metadata(
            &artifact.manifest_json().unwrap(),
            artifact.body.as_bytes(),
            artifact.sha256_text().as_bytes(),
            Some(&artifact.metadata_json().unwrap().unwrap()),
        )
        .unwrap();
        assert_eq!(parsed.metadata, Some(metadata));
    }

    #[test]
    fn metadata_hash_mismatch_fails_archive_parse() {
        let artifact = SkillArtifact::new(frontmatter(), "body".into());
        let metadata = SkillArtifactMetadata::new(
            "tenant".into(),
            None,
            None,
            "2026-05-08T00:00:00Z".into(),
            "0.0.1".into(),
            "sha256-wrong".into(),
        );
        let metadata = serde_json::to_vec(&metadata).unwrap();
        let err = SkillArtifact::from_archive_parts_with_metadata(
            &artifact.manifest_json().unwrap(),
            artifact.body.as_bytes(),
            artifact.sha256_text().as_bytes(),
            Some(&metadata),
        )
        .unwrap_err();
        assert!(matches!(err, ArgoError::Schema(_)));
    }

    #[test]
    fn import_frontmatter_can_rename() {
        let artifact = SkillArtifact::new(frontmatter(), "body".into());
        let renamed = artifact.frontmatter_for_import(Some("renamed"));
        assert_eq!(renamed.name, "renamed");
        assert_eq!(renamed.version, "1.0");
    }
}
