//! `argo skill rate <name> --vote <up|down> [--version <v>] [--comment "..."]`
//! handler (Stream `mission-skill-ratings-publisher`).
//!
//! Looks up the active skill row by name (or the explicit version
//! when `--version` is given), parses the publisher namespace out of
//! `provenance.source` (`published:<grade>:<publisher>`), and writes
//! the rating row via [`argo_store::skill_rating::record_rating`].
//! Non-published source → explicit "only published skills can be
//! rated" error, mirroring the brief's contract.

use argo_skill::SkillStore;
use argo_store::skill_rating::{record_rating, SkillVote};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use std::sync::Arc;

use crate::home::{db_path, ensure_dir, resolve_home};

/// Parse `provenance.source` into the publisher namespace string.
///
/// Marketplace sources are of the form `published:<grade>:<publisher>`
/// where `<grade>` is the trust grade label (`verified` / `community`)
/// and `<publisher>` is the tenant/vendor namespace. Anything else
/// returns `None`, signalling "not a published skill".
#[must_use]
pub(crate) fn publisher_from_source(source: &str) -> Option<String> {
    let rest = source.strip_prefix("published:")?;
    let (_grade, publisher) = rest.split_once(':')?;
    if publisher.is_empty() {
        None
    } else {
        Some(publisher.to_string())
    }
}

pub async fn run(
    name: &str,
    version: Option<&str>,
    vote: &str,
    comment: Option<&str>,
    json: bool,
) -> ArgoResult<()> {
    let vote_parsed = match vote.to_ascii_lowercase().as_str() {
        "up" => SkillVote::Up,
        "down" => SkillVote::Down,
        other => {
            return Err(ArgoError::Provider(format!(
                "invalid --vote '{other}'; expected 'up' or 'down'"
            )))
        }
    };
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = Arc::new(SessionStore::open(db_path(&home)).await?);
    store.migrate().await?;
    let workspace_name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let workspace_root = cwd.to_string_lossy();
    let (tenant_id, _wsid) = store
        .ensure_workspace(workspace_name, &workspace_root)
        .await?;
    let skills = SkillStore::new(store.clone());
    let skill = match version {
        Some(v) => find_version(&skills, name, v).await?,
        None => skills.find_active_by_name(name).await?.ok_or_else(|| {
            ArgoError::Provider(format!("no active skill named '{name}'"))
        })?,
    };
    let publisher = publisher_from_source(&skill.frontmatter.provenance.source)
        .ok_or_else(|| {
            ArgoError::Provider(
                "only published skills can be rated (source must be \
                 `published:<grade>:<publisher>`)"
                    .into(),
            )
        })?;
    record_rating(
        store.pool(),
        skill.id,
        &skill.frontmatter.version,
        &publisher,
        tenant_id,
        vote_parsed,
        comment,
    )
    .await?;
    if json {
        let payload = serde_json::json!({
            "ok": true,
            "skill_id": skill.id.0.to_string(),
            "skill_name": skill.frontmatter.name,
            "skill_version": skill.frontmatter.version,
            "publisher_tenant": publisher,
            "vote": vote_parsed.as_str(),
        });
        println!("{}", serde_json::to_string(&payload).unwrap_or_default());
    } else {
        println!(
            "recorded {} vote on '{}' v{} (publisher '{}')",
            vote_parsed.as_str(),
            skill.frontmatter.name,
            skill.frontmatter.version,
            publisher
        );
    }
    Ok(())
}

async fn find_version(
    skills: &SkillStore,
    name: &str,
    version: &str,
) -> ArgoResult<argo_skill::Skill> {
    let rows = skills.versions_for_name(name).await?;
    rows.into_iter()
        .find(|s| s.frontmatter.version == version)
        .ok_or_else(|| {
            ArgoError::Provider(format!(
                "no skill '{name}' at version '{version}' in this workspace"
            ))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publisher_from_verified() {
        assert_eq!(
            publisher_from_source("published:verified:acme"),
            Some("acme".into())
        );
    }

    #[test]
    fn publisher_from_community() {
        assert_eq!(
            publisher_from_source("published:community:bob"),
            Some("bob".into())
        );
    }

    #[test]
    fn publisher_none_for_builtin() {
        assert_eq!(publisher_from_source("builtin"), None);
        assert_eq!(publisher_from_source("user"), None);
        assert_eq!(publisher_from_source("forge"), None);
        assert_eq!(publisher_from_source("bundle:retail"), None);
        assert_eq!(publisher_from_source("learned"), None);
    }

    #[test]
    fn publisher_none_for_empty_after_grade() {
        assert_eq!(publisher_from_source("published:verified:"), None);
        assert_eq!(publisher_from_source("published:"), None);
        assert_eq!(publisher_from_source("published"), None);
    }
}
