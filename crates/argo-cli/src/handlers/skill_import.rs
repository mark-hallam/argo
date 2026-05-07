//! `argo skill import` and `argo skill review import`.
use super::skill_export::{body_hash, render_skill_md, PortableSkill};
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_skill::{parse_skill_md, PrincipalOverride, SkillArtifact, SkillFrontmatter, SkillStore};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, Lock};
use flate2::read::GzDecoder;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::Arc;

pub async fn import_artifact(
    default_skills: &SkillStore,
    path: &str,
    rename: Option<&str>,
    workspace: Option<&str>,
    dry_run: bool,
    json: bool,
) -> ArgoResult<()> {
    let artifact = read_skill_artifact(Path::new(path))?;
    let target = artifact.frontmatter_for_import(rename);
    if let Some(workspace) = workspace {
        let skills = skill_store_for_workspace(Path::new(workspace)).await?;
        return import_into(&skills, path, artifact, target, dry_run, json).await;
    }
    import_into(default_skills, path, artifact, target, dry_run, json).await
}

async fn import_into(
    skills: &SkillStore,
    path: &str,
    artifact: SkillArtifact,
    frontmatter: SkillFrontmatter,
    dry_run: bool,
    json: bool,
) -> ArgoResult<()> {
    if frontmatter.name.trim().is_empty() {
        return Err(ArgoError::Provider(
            "skill import: empty target name".into(),
        ));
    }
    if skills
        .find_active_by_name(&frontmatter.name)
        .await?
        .is_some()
    {
        return Err(ArgoError::Provider(format!(
            "skill '{}' already installed; pass --name <new-name> to rename",
            frontmatter.name
        )));
    }
    if dry_run {
        if json {
            return print_json(&serde_json::json!({
                "ok": true,
                "dry_run": true,
                "artifact": artifact_json(path, &artifact),
                "would_import": skill_json(None, &frontmatter),
            }));
        }
        println!(
            "would import {} v{} from {}",
            frontmatter.name, frontmatter.version, path
        );
        return Ok(());
    }
    let requested_lock = frontmatter.lock;
    let raw = render_skill_md(&frontmatter, &artifact.body)?;
    let installed = skills.install(&raw, None).await?;
    if requested_lock != Lock::None {
        skills
            .set_lock(installed.id, requested_lock, PrincipalOverride::Force)
            .await?;
    }
    if json {
        return print_json(&serde_json::json!({
            "ok": true,
            "dry_run": false,
            "artifact": artifact_json(path, &artifact),
            "skill": skill_json(Some(&installed.id.0.to_string()), &frontmatter),
        }));
    }
    println!(
        "imported skill '{}' v{} (id={})",
        frontmatter.name, frontmatter.version, installed.id.0
    );
    Ok(())
}

async fn skill_store_for_workspace(workspace: &Path) -> ArgoResult<SkillStore> {
    let home = resolve_home(workspace)?;
    ensure_dir(&home)?;
    let store = Arc::new(SessionStore::open(&db_path(&home)).await?);
    store.migrate().await?;
    Ok(SkillStore::new(store))
}

fn read_skill_artifact(path: &Path) -> ArgoResult<SkillArtifact> {
    let file = File::open(path)
        .map_err(|e| ArgoError::Provider(format!("open {}: {e}", path.display())))?;
    let decoder = GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);
    let mut manifest = None;
    let mut body = None;
    let mut sha256 = None;
    for entry in archive
        .entries()
        .map_err(|e| ArgoError::Provider(format!("read skill artifact entries: {e}")))?
    {
        let mut entry =
            entry.map_err(|e| ArgoError::Provider(format!("read skill artifact entry: {e}")))?;
        if entry.header().entry_type().is_dir() {
            continue;
        }
        let path = entry
            .path()
            .map_err(|e| ArgoError::Provider(format!("skill artifact path: {e}")))?
            .to_string_lossy()
            .replace('\\', "/");
        let mut bytes = Vec::new();
        entry
            .read_to_end(&mut bytes)
            .map_err(|e| ArgoError::Provider(format!("read {path}: {e}")))?;
        match path.as_str() {
            argo_skill::SKILL_ARTIFACT_MANIFEST => manifest = Some(bytes),
            argo_skill::SKILL_ARTIFACT_BODY => body = Some(bytes),
            argo_skill::SKILL_ARTIFACT_SHA256 => sha256 = Some(bytes),
            other => {
                return Err(ArgoError::Schema(format!(
                    "unexpected skill artifact entry '{other}'"
                )));
            }
        }
    }
    SkillArtifact::from_archive_parts(
        &required_entry(manifest, argo_skill::SKILL_ARTIFACT_MANIFEST)?,
        &required_entry(body, argo_skill::SKILL_ARTIFACT_BODY)?,
        &required_entry(sha256, argo_skill::SKILL_ARTIFACT_SHA256)?,
    )
}

fn required_entry(bytes: Option<Vec<u8>>, name: &str) -> ArgoResult<Vec<u8>> {
    bytes.ok_or_else(|| ArgoError::Schema(format!("skill artifact missing {name}")))
}

fn artifact_json(path: &str, artifact: &SkillArtifact) -> serde_json::Value {
    serde_json::json!({
        "path": path,
        "body_sha256": &artifact.manifest.body_sha256,
        "signature": &artifact.manifest.signature,
    })
}

fn skill_json(id: Option<&str>, frontmatter: &SkillFrontmatter) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "name": &frontmatter.name,
        "version": &frontmatter.version,
        "description": &frontmatter.description,
        "lock": frontmatter.lock,
    })
}

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

fn print_json<T: Serialize>(value: &T) -> ArgoResult<()> {
    let body =
        serde_json::to_string(value).map_err(|e| ArgoError::Provider(format!("json: {e}")))?;
    println!("{body}");
    Ok(())
}
