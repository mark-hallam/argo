//! `argo skill refine` — supersede an active skill with a refinement hint.
use super::skill::body_hash;
use argo_skill::{PrincipalOverride, SkillStore};
use argo_types::{ArgoError, ArgoResult};

pub(super) async fn refine(
    skills: &SkillStore,
    name: &str,
    hint: &str,
    version_override: Option<&str>,
) -> ArgoResult<()> {
    let trimmed = hint.trim();
    if trimmed.is_empty() {
        return Err(ArgoError::Provider("skill refine: empty --hint".into()));
    }
    let active = skills
        .find_active_by_name(name)
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no active skill named '{name}'")))?;
    let new_version = version_override
        .map(String::from)
        .unwrap_or_else(|| bump_version(&active.frontmatter.version));
    let new_body = format!("{}\n\n## Refinement\n{}\n", active.body.trim_end(), trimmed);
    let raw = format!(
        "---\nname: {name}\nversion: \"{new_version}\"\ndescription: {desc}\n\
         lock: none\nprovenance:\n  source: refined-from-{prev}\n  hash: sha256-{hash}\n---\n{body}",
        desc = active.frontmatter.description,
        prev = active.frontmatter.version,
        hash = body_hash(&new_body),
        body = new_body,
    );
    let new_skill = skills
        .supersede(&raw, None, PrincipalOverride::Force)
        .await?;
    println!(
        "argo skill refine: {name} v{} → v{} (skill_id {})",
        active.frontmatter.version, new_version, new_skill.id.0
    );
    println!("  hint:    {trimmed}");
    println!(
        "  next chat will pick this up via the active-skills system prompt."
    );
    Ok(())
}

/// Bump the trailing numeric segment of a semver-ish version
/// (`"0.1"` → `"0.2"`, `"1.5.3"` → `"1.5.4"`). Falls back to
/// appending `-refined` for non-numeric tails.
fn bump_version(v: &str) -> String {
    let mut parts: Vec<String> = v.split('.').map(String::from).collect();
    if let Some(last) = parts.last_mut() {
        if let Ok(n) = last.parse::<u64>() {
            *last = (n + 1).to_string();
            return parts.join(".");
        }
    }
    format!("{v}-refined")
}
