//! `argo skill review import` offline skill distribution.
use super::skill_export::{body_hash, render_skill_md, PortableSkill};
use argo_skill::{parse_skill_md, PrincipalOverride, SkillFrontmatter, SkillStore};
use argo_types::{ArgoError, ArgoResult, Lock};
use sha2::{Digest, Sha256};
use std::path::Path;

pub async fn run(skills: &SkillStore, path: &str, lock: Option<&str>) -> ArgoResult<()> {
    let raw = std::fs::read_to_string(path)
        .map_err(|e| ArgoError::Provider(format!("read {path}: {e}")))?;
    let imported_source = format!("imported:{}", sha256_hex(raw.as_bytes()));
    reject_import_collision(skills, &imported_source).await?;

    let (mut frontmatter, body) = read_portable(path, &raw)?;
    let requested_lock = match lock {
        Some(raw) => parse_lock(raw)?,
        None => frontmatter.lock,
    };
    frontmatter.provenance.source = imported_source;
    frontmatter.provenance.hash = body_hash(&body);
    frontmatter.lock = requested_lock;

    let canonical = render_skill_md(&frontmatter, &body)?;
    let installed = skills.install(&canonical, None).await?;
    if requested_lock != Lock::None {
        skills
            .set_lock(installed.id, requested_lock, PrincipalOverride::Force)
            .await?;
    }
    println!(
        "imported skill '{}' v{} (id={})",
        installed.frontmatter.name, installed.frontmatter.version, installed.id.0
    );
    Ok(())
}

fn read_portable(path: &str, raw: &str) -> ArgoResult<(SkillFrontmatter, String)> {
    match Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("json") => {
            let portable: PortableSkill = serde_json::from_str(raw)
                .map_err(|e| ArgoError::Provider(format!("parse skill json: {e}")))?;
            Ok((portable.frontmatter, portable.body))
        }
        Some("toml") => {
            let portable: PortableSkill = toml::from_str(raw)
                .map_err(|e| ArgoError::Provider(format!("parse skill toml: {e}")))?;
            Ok((portable.frontmatter, portable.body))
        }
        _ => parse_skill_md(raw),
    }
}

async fn reject_import_collision(skills: &SkillStore, imported_source: &str) -> ArgoResult<()> {
    if let Some(existing) = skills
        .list_all()
        .await?
        .into_iter()
        .find(|skill| skill.frontmatter.provenance.source == imported_source)
    {
        return Err(ArgoError::Provider(format!(
            "import hash collision: {} already imported as '{}' v{} ({})",
            imported_source, existing.frontmatter.name, existing.frontmatter.version, existing.id.0
        )));
    }
    Ok(())
}

fn parse_lock(raw: &str) -> ArgoResult<Lock> {
    match raw {
        "none" => Ok(Lock::None),
        "user-protected" | "user_protected" => Ok(Lock::UserProtected),
        "content-locked" | "content_locked" => Ok(Lock::ContentLocked),
        other => Err(ArgoError::Provider(format!(
            "unknown --lock '{other}' - use none|user_protected|content_locked"
        ))),
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex::encode(h.finalize())
}
