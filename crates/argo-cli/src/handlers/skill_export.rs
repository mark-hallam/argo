//! `argo skill export` and `argo skill review export`.
use argo_skill::{
    skill_body_sha256, Skill, SkillArtifact, SkillArtifactMetadata, SkillFrontmatter, SkillStore,
    TrustGrade, SKILL_ARTIFACT_BODY, SKILL_ARTIFACT_MANIFEST, SKILL_ARTIFACT_METADATA,
    SKILL_ARTIFACT_SHA256,
};
use argo_types::{ArgoError, ArgoResult, Lock, SkillId};
use flate2::write::GzEncoder;
use flate2::Compression;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct PortableSkill {
    pub(crate) frontmatter: SkillFrontmatter,
    pub(crate) body: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ExportedArtifact {
    pub(crate) name: String,
    pub(crate) version: String,
    pub(crate) path: String,
    pub(crate) body_sha256: String,
    pub(crate) artifact_sha256: String,
    pub(crate) metadata: SkillArtifactMetadata,
}

pub async fn export_one(
    skills: &SkillStore,
    selector: &str,
    output: &str,
    json: bool,
) -> ArgoResult<()> {
    let skill = lookup_by_selector(skills, selector).await?;
    let exported = export_skill_to_path(&skill, Path::new(output))?;
    if json {
        return print_json(&serde_json::json!({
            "ok": true,
            "skill": {
                "name": exported.name,
                "version": exported.version,
            },
            "artifact": {
                "path": exported.path,
                "sha256": exported.artifact_sha256,
                "body_sha256": exported.body_sha256,
                "metadata": exported.metadata,
            }
        }));
    }
    println!(
        "exported {} v{} to {}",
        exported.name, exported.version, exported.path
    );
    Ok(())
}

pub(crate) fn export_skill_to_path(skill: &Skill, output: &Path) -> ArgoResult<ExportedArtifact> {
    if let Some(parent) = output.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)
            .map_err(|e| ArgoError::Provider(format!("create {}: {e}", parent.display())))?;
    }
    let mut artifact = SkillArtifact::new(skill.frontmatter.clone(), skill.body.clone());
    let metadata = metadata_for_skill(skill, &artifact.manifest.body_sha256);
    artifact = artifact.with_metadata(metadata.clone());
    artifact.verify()?;
    write_skill_artifact(output, &artifact)?;
    let artifact_sha256 = file_sha256(output)?;
    Ok(ExportedArtifact {
        name: skill.frontmatter.name.clone(),
        version: skill.frontmatter.version.clone(),
        path: output.display().to_string(),
        body_sha256: artifact.manifest.body_sha256,
        artifact_sha256,
        metadata,
    })
}

pub async fn run(
    skills: &SkillStore,
    skill_id: &str,
    output: &str,
    format: &str,
) -> ArgoResult<()> {
    let skill_id = parse_skill_id(skill_id)?;
    let skill = lookup_by_id(skills, skill_id).await?;
    let portable = PortableSkill {
        frontmatter: skill.frontmatter,
        body: skill.body,
    };
    let bytes = match format {
        "md" => render_skill_md(&portable.frontmatter, &portable.body)?.into_bytes(),
        "json" => serde_json::to_vec_pretty(&portable)
            .map_err(|e| ArgoError::Provider(format!("serialize skill json: {e}")))?,
        "toml" => toml::to_string_pretty(&portable)
            .map_err(|e| ArgoError::Provider(format!("serialize skill toml: {e}")))?
            .into_bytes(),
        other => {
            return Err(ArgoError::Provider(format!(
                "unknown --format '{other}' - use md|toml|json"
            )));
        }
    };
    std::fs::write(output, bytes)
        .map_err(|e| ArgoError::Provider(format!("write {output}: {e}")))?;
    println!("exported skill {} to {output}", skill_id.0);
    Ok(())
}

pub(crate) fn render_skill_md(front: &SkillFrontmatter, body: &str) -> ArgoResult<String> {
    let mut out = String::from("---\n");
    out.push_str("name: ");
    out.push_str(&yaml_string(&front.name)?);
    out.push('\n');
    out.push_str("version: ");
    out.push_str(&yaml_string(&front.version)?);
    out.push('\n');
    out.push_str("description: ");
    out.push_str(&yaml_string(&front.description)?);
    out.push('\n');
    if !front.capabilities.0.is_empty() {
        out.push_str("capabilities: ");
        out.push_str(
            &serde_json::to_string(&front.capabilities)
                .map_err(|e| ArgoError::Provider(format!("serialize capabilities: {e}")))?,
        );
        out.push('\n');
    }
    out.push_str("lock: ");
    out.push_str(lock_label(front.lock));
    out.push('\n');
    out.push_str("provenance:\n  source: ");
    out.push_str(&yaml_string(&front.provenance.source)?);
    out.push_str("\n  hash: ");
    out.push_str(&yaml_string(&front.provenance.hash)?);
    out.push_str("\n---\n");
    out.push_str(body);
    Ok(out)
}

pub(crate) fn body_hash(body: &str) -> String {
    skill_body_sha256(body)
}

fn yaml_string(value: &str) -> ArgoResult<String> {
    serde_json::to_string(value)
        .map_err(|e| ArgoError::Provider(format!("serialize yaml string: {e}")))
}

fn lock_label(lock: Lock) -> &'static str {
    match lock {
        Lock::None => "none",
        Lock::UserProtected => "user-protected",
        Lock::ContentLocked => "content-locked",
    }
}

fn parse_skill_id(raw: &str) -> ArgoResult<SkillId> {
    Uuid::parse_str(raw)
        .map(SkillId)
        .map_err(|e| ArgoError::Schema(format!("invalid skill id '{raw}': {e}")))
}

async fn lookup_by_id(skills: &SkillStore, target: SkillId) -> ArgoResult<Skill> {
    skills
        .list_all()
        .await?
        .into_iter()
        .find(|skill| skill.id == target)
        .ok_or_else(|| ArgoError::Provider(format!("no skill with id '{}'", target.0)))
}

async fn lookup_by_selector(skills: &SkillStore, selector: &str) -> ArgoResult<Skill> {
    let (name, version) = parse_selector(selector)?;
    if let Some(version) = version {
        return skills
            .versions_for_name(name)
            .await?
            .into_iter()
            .find(|skill| skill.frontmatter.version == version)
            .ok_or_else(|| {
                ArgoError::Provider(format!("no skill named '{name}' at version '{version}'"))
            });
    }
    skills
        .find_active_by_name(name)
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no active skill named '{name}'")))
}

fn parse_selector(selector: &str) -> ArgoResult<(&str, Option<&str>)> {
    if selector.trim().is_empty() {
        return Err(ArgoError::Provider("skill export: empty skill name".into()));
    }
    match selector.rsplit_once(':') {
        Some((name, version)) if !name.is_empty() && !version.is_empty() => {
            Ok((name, Some(version)))
        }
        Some(_) => Err(ArgoError::Provider(format!(
            "invalid skill selector '{selector}' - use name or name:version"
        ))),
        None => Ok((selector, None)),
    }
}

fn write_skill_artifact(path: &Path, artifact: &SkillArtifact) -> ArgoResult<()> {
    let file = File::create(path)
        .map_err(|e| ArgoError::Provider(format!("create {}: {e}", path.display())))?;
    let encoder = GzEncoder::new(file, Compression::default());
    let mut builder = tar::Builder::new(encoder);
    append_bytes(
        &mut builder,
        SKILL_ARTIFACT_MANIFEST,
        &artifact.manifest_json()?,
    )?;
    append_bytes(&mut builder, SKILL_ARTIFACT_BODY, artifact.body.as_bytes())?;
    append_bytes(
        &mut builder,
        SKILL_ARTIFACT_SHA256,
        artifact.sha256_text().as_bytes(),
    )?;
    if let Some(metadata) = artifact.metadata_json()? {
        append_bytes(&mut builder, SKILL_ARTIFACT_METADATA, &metadata)?;
    }
    let encoder = builder
        .into_inner()
        .map_err(|e| ArgoError::Provider(format!("finish skill tar: {e}")))?;
    encoder
        .finish()
        .map_err(|e| ArgoError::Provider(format!("finish skill gzip: {e}")))?;
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
    builder
        .append_data(&mut header, path, bytes)
        .map_err(|e| ArgoError::Provider(format!("append {path}: {e}")))?;
    Ok(())
}

fn file_sha256(path: &Path) -> ArgoResult<String> {
    let bytes = std::fs::read(path)
        .map_err(|e| ArgoError::Provider(format!("read {}: {e}", path.display())))?;
    let mut h = Sha256::new();
    h.update(&bytes);
    Ok(format!("sha256-{}", hex::encode(h.finalize())))
}

fn print_json<T: Serialize>(value: &T) -> ArgoResult<()> {
    let body =
        serde_json::to_string(value).map_err(|e| ArgoError::Provider(format!("json: {e}")))?;
    println!("{body}");
    Ok(())
}

fn metadata_for_skill(skill: &Skill, body_sha256: &str) -> SkillArtifactMetadata {
    SkillArtifactMetadata::new(
        trust_grade_label(TrustGrade::from_source(
            &skill.frontmatter.provenance.source,
        )),
        skill.vertical_id.clone(),
        skill.learned_in.map(|sid| sid.0.to_string()),
        chrono::Utc::now().to_rfc3339(),
        env!("CARGO_PKG_VERSION").to_string(),
        body_sha256.to_string(),
    )
}

fn trust_grade_label(grade: TrustGrade) -> String {
    serde_json::to_value(grade)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_else(|| "learned_unreviewed".to_string())
}

#[cfg(test)]
mod tests {
    use super::metadata_for_skill;
    use argo_skill::{LifecycleState, Provenance, Skill, SkillFrontmatter};
    use argo_types::{CapabilitySet, Lock, SessionId, SkillId};
    use chrono::Utc;
    use uuid::Uuid;

    #[test]
    fn metadata_for_skill_includes_vertical_and_learned_session() {
        let learned = SessionId(Uuid::parse_str("018f55b7-89ab-7cde-8123-456789abcdef").unwrap());
        let skill = Skill {
            id: SkillId(Uuid::parse_str("018f55b7-89ab-7cde-8123-456789abcdee").unwrap()),
            frontmatter: SkillFrontmatter {
                name: "demo".into(),
                version: "1.0".into(),
                description: "demo".into(),
                capabilities: CapabilitySet::empty(),
                provenance: Provenance {
                    source: "learned".into(),
                    hash: "sha256-x".into(),
                },
                lock: Lock::None,
                evals: Vec::new(),
            },
            body: "body".into(),
            hash: [0; 32],
            learned_in: Some(learned),
            superseded_by: None,
            created_at: Utc::now(),
            vertical_id: Some("finance-ar".into()),
            lifecycle_state: LifecycleState::Active,
        };
        let metadata = metadata_for_skill(&skill, "sha256-body");
        let learned_id = learned.0.to_string();
        assert_eq!(metadata.trust_grade, "learned_unreviewed");
        assert_eq!(metadata.vertical_id.as_deref(), Some("finance-ar"));
        assert_eq!(metadata.learned_in.as_deref(), Some(learned_id.as_str()));
        assert_eq!(metadata.body_sha256, "sha256-body");
        assert!(!metadata.exporter_version.is_empty());
    }
}
