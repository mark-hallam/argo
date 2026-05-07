//! `argo skill export-all`.
use super::skill_export::export_skill_to_path;
use argo_skill::{Skill, SkillStore};
use argo_types::{ArgoError, ArgoResult};
use serde::Serialize;
use std::path::{Path, PathBuf};

pub async fn run(
    skills: &SkillStore,
    output_dir: &str,
    vertical: Option<&str>,
    json: bool,
) -> ArgoResult<()> {
    let dir = Path::new(output_dir);
    std::fs::create_dir_all(dir)
        .map_err(|e| ArgoError::Provider(format!("create {}: {e}", dir.display())))?;
    let rows = skills_for_export(skills, vertical).await?;
    let mut artifacts = Vec::new();
    for skill in rows {
        let path = dir.join(artifact_name(&skill));
        artifacts.push(export_skill_to_path(&skill, &path)?);
    }
    if json {
        return print_json(&serde_json::json!({
            "ok": true,
            "count": artifacts.len(),
            "vertical": vertical,
            "artifacts": artifacts,
        }));
    }
    println!(
        "exported {} skill artifact(s) to {}",
        artifacts.len(),
        dir.display()
    );
    for artifact in artifacts {
        println!(
            "{} v{} -> {}",
            artifact.name, artifact.version, artifact.path
        );
    }
    Ok(())
}

async fn skills_for_export(skills: &SkillStore, vertical: Option<&str>) -> ArgoResult<Vec<Skill>> {
    match vertical {
        Some(id) => skills.list_skills_bundled_by_vertical(id).await,
        None => skills.list_active().await,
    }
}

fn artifact_name(skill: &Skill) -> PathBuf {
    let name = clean_component(&skill.frontmatter.name);
    let version = clean_component(&skill.frontmatter.version);
    PathBuf::from(format!("{name}-{version}.skill.tar.gz"))
}

fn clean_component(value: &str) -> String {
    let cleaned: String = value
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') {
                c
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = cleaned.trim_matches('-');
    if trimmed.is_empty() {
        "skill".into()
    } else {
        trimmed.into()
    }
}

fn print_json<T: Serialize>(value: &T) -> ArgoResult<()> {
    let body =
        serde_json::to_string(value).map_err(|e| ArgoError::Provider(format!("json: {e}")))?;
    println!("{body}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_skill::{Provenance, SkillFrontmatter};
    use argo_types::{CapabilitySet, Lock, SkillId};
    use chrono::Utc;

    #[test]
    fn artifact_name_sanitizes_skill_and_version() {
        let skill = Skill {
            id: SkillId::new(),
            frontmatter: SkillFrontmatter {
                name: "name with spaces".into(),
                version: "1/2".into(),
                description: "d".into(),
                capabilities: CapabilitySet::empty(),
                provenance: Provenance {
                    source: "test".into(),
                    hash: "sha256-x".into(),
                },
                lock: Lock::None,
            },
            body: String::new(),
            hash: [0; 32],
            learned_in: None,
            superseded_by: None,
            created_at: Utc::now(),
            vertical_id: None,
            lifecycle_state: argo_skill::LifecycleState::Active,
        };
        assert_eq!(
            artifact_name(&skill).to_string_lossy(),
            "name-with-spaces-1-2.skill.tar.gz"
        );
    }
}
