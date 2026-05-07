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
pub const SKILL_ARTIFACT_BODY: &str = "body.md";
/// Path of the checksum sidecar inside a `.skill.tar.gz` artifact.
pub const SKILL_ARTIFACT_SHA256: &str = "sha256.txt";

/// Signed metadata stored as `manifest.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillArtifactManifest {
    /// Schema version for forward-compatible import.
    pub schema_version: u16,
    /// Original skill frontmatter.
    pub frontmatter: SkillFrontmatter,
    /// `sha256-<hex>` of `body.md`.
    pub body_sha256: String,
    /// Deterministic signature over schema, frontmatter, and body hash.
    pub signature: String,
}

/// A portable skill artifact after archive extraction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillArtifact {
    /// Signed artifact manifest.
    pub manifest: SkillArtifactManifest,
    /// Raw skill body from `body.md`.
    pub body: String,
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
        }
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
        let artifact = Self { manifest, body };
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

    /// Render the checksum sidecar.
    #[must_use]
    pub fn sha256_text(&self) -> String {
        format!("{}  {SKILL_ARTIFACT_BODY}\n", self.manifest.body_sha256)
    }

    /// Verify schema version, body checksum, and metadata signature.
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
    fn import_frontmatter_can_rename() {
        let artifact = SkillArtifact::new(frontmatter(), "body".into());
        let renamed = artifact.frontmatter_for_import(Some("renamed"));
        assert_eq!(renamed.name, "renamed");
        assert_eq!(renamed.version, "1.0");
    }
}
