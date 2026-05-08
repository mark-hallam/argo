//! `argo skill rollback` - promote a prior eligible version.

use argo_audit::{AuditKind, AuditSink};
use argo_skill::{LifecycleState, Skill, SkillStore};
use argo_types::{ArgoError, ArgoResult};
use serde_json::json;

/// Roll back a skill to a prior version, or preview the transition.
pub async fn run(
    skills: &SkillStore,
    name: &str,
    to_version: &str,
    dry_run: bool,
    reason: Option<&str>,
    json_out: bool,
) -> ArgoResult<()> {
    let current = active_skill(skills, name).await?;
    let target = target_version(skills, name, to_version).await?;
    ensure_eligible(&target)?;
    let from_version = current.frontmatter.version.clone();

    if dry_run {
        print_result(json_out, true, name, &from_version, to_version);
        return Ok(());
    }

    let promoted = skills.promote(name, to_version).await?;
    write_audit(skills, &current, &promoted, reason).await?;
    print_result(json_out, false, name, &from_version, to_version);
    Ok(())
}

async fn active_skill(skills: &SkillStore, name: &str) -> ArgoResult<Skill> {
    skills
        .find_active_by_name(name)
        .await?
        .ok_or_else(|| ArgoError::Schema(format!("unknown skill '{name}'")))
}

async fn target_version(skills: &SkillStore, name: &str, version: &str) -> ArgoResult<Skill> {
    let versions = skills.versions_for_name(name).await?;
    if versions.is_empty() {
        return Err(ArgoError::Schema(format!("unknown skill '{name}'")));
    }
    versions
        .into_iter()
        .find(|s| s.frontmatter.version == version)
        .ok_or_else(|| ArgoError::Schema(format!("skill '{name}' has no version '{version}'")))
}

fn ensure_eligible(target: &Skill) -> ArgoResult<()> {
    match target.lifecycle_state {
        LifecycleState::Archived | LifecycleState::Rejected => Err(ArgoError::Schema(format!(
            "cannot rollback to {} version '{}'",
            target.lifecycle_state.label(),
            target.frontmatter.version
        ))),
        _ => Ok(()),
    }
}

async fn write_audit(
    skills: &SkillStore,
    from: &Skill,
    to: &Skill,
    reason: Option<&str>,
) -> ArgoResult<()> {
    let payload = json!({
        "name": to.frontmatter.name,
        "from_skill_id": from.id.0.to_string(),
        "from_version": from.frontmatter.version,
        "to_skill_id": to.id.0.to_string(),
        "to_version": to.frontmatter.version,
        "reason": reason,
    });
    AuditSink::new(skills.store_arc())
        .write(
            to.learned_in.or(from.learned_in),
            AuditKind::SkillRolledBack,
            payload,
        )
        .await
}

fn print_result(json_out: bool, dry_run: bool, name: &str, from_version: &str, to_version: &str) {
    if json_out {
        let payload = json!({
            "ok": true,
            "dry_run": dry_run,
            "name": name,
            "from_version": from_version,
            "to_version": to_version,
        });
        println!("{}", serde_json::to_string(&payload).unwrap_or_default());
    } else if dry_run {
        println!("would rollback {name}: {from_version} -> {to_version}");
    } else {
        println!("rolled back {name}: {from_version} -> {to_version}");
    }
}
