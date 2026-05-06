//! `argo skill review export` offline skill distribution.
use argo_skill::{Skill, SkillFrontmatter, SkillStore};
use argo_types::{ArgoError, ArgoResult, Lock, SkillId};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct PortableSkill {
    pub(crate) frontmatter: SkillFrontmatter,
    pub(crate) body: String,
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
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(body.as_bytes());
    format!("sha256-{}", hex::encode(h.finalize()))
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
        .map_err(|e| ArgoError::Provider(format!("invalid skill id '{raw}': {e}")))
}

async fn lookup_by_id(skills: &SkillStore, target: SkillId) -> ArgoResult<Skill> {
    skills
        .list_all()
        .await?
        .into_iter()
        .find(|skill| skill.id == target)
        .ok_or_else(|| ArgoError::Provider(format!("no skill with id '{}'", target.0)))
}
