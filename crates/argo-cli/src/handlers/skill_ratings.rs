//! `argo skill ratings <name> [--version <v>] [--json]` handler
//! (Stream `mission-skill-ratings-publisher`).
//!
//! Aggregates up/down counts + score and surfaces the 10 most-recent
//! comments for one marketplace-installed skill. JSON envelope:
//! ```json
//! {
//!   "skill_id": "<uuid>",
//!   "skill_name": "...",
//!   "skill_version": "1.0.0",
//!   "publisher_tenant": "vendor",
//!   "up_count": 3, "down_count": 1, "score": 0.75,
//!   "recent_comments": [{ "vote": "up", "comment": "...", "rated_at_micros": 1234 }]
//! }
//! ```
//! The text form prints a one-line summary plus a comments block when
//! any are present.

use argo_skill::SkillStore;
use argo_store::skill_rating::{aggregate_ratings, recent_ratings};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use std::sync::Arc;

use crate::home::{db_path, ensure_dir, resolve_home};

const RECENT_COMMENT_LIMIT: u32 = 10;

pub async fn run(name: &str, version: Option<&str>, json: bool) -> ArgoResult<()> {
    let cwd =
        std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = Arc::new(SessionStore::open(db_path(&home)).await?);
    store.migrate().await?;
    let skills = SkillStore::new(store.clone());
    let skill = match version {
        Some(v) => find_version(&skills, name, v).await?,
        None => skills.find_active_by_name(name).await?.ok_or_else(|| {
            ArgoError::Provider(format!("no active skill named '{name}'"))
        })?,
    };
    let publisher = super::skill_rate::publisher_from_source(
        &skill.frontmatter.provenance.source,
    )
    .unwrap_or_default();
    let (up, down, score) =
        aggregate_ratings(store.pool(), skill.id, &skill.frontmatter.version).await?;
    let rows = recent_ratings(
        store.pool(),
        skill.id,
        &skill.frontmatter.version,
        RECENT_COMMENT_LIMIT,
    )
    .await?;
    if json {
        let comments: Vec<_> = rows
            .iter()
            .filter_map(|r| {
                r.comment.as_ref().map(|c| {
                    serde_json::json!({
                        "vote": r.vote.as_str(),
                        "comment": c,
                        "rated_at_micros": r.rated_at_micros,
                    })
                })
            })
            .collect();
        let payload = serde_json::json!({
            "skill_id": skill.id.0.to_string(),
            "skill_name": skill.frontmatter.name,
            "skill_version": skill.frontmatter.version,
            "publisher_tenant": publisher,
            "up_count": up,
            "down_count": down,
            "score": score,
            "recent_comments": comments,
        });
        println!("{}", serde_json::to_string(&payload).unwrap_or_default());
    } else {
        let publisher_label = if publisher.is_empty() {
            "(none — non-marketplace)".to_string()
        } else {
            publisher.clone()
        };
        println!(
            "{} v{} (publisher {publisher_label}): up={up} down={down} score={score:.3}",
            skill.frontmatter.name, skill.frontmatter.version
        );
        let with_comments: Vec<_> =
            rows.iter().filter(|r| r.comment.is_some()).collect();
        if !with_comments.is_empty() {
            println!("recent comments:");
            for r in with_comments {
                let c = r.comment.as_deref().unwrap_or("");
                println!("  [{}] {c}", r.vote.as_str());
            }
        }
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
