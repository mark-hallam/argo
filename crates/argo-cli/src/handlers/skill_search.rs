//! `argo skill search` marketplace search handler.
//!
//! Two paths:
//!
//! - Default: in-memory marketplace search by query text + trust filter.
//! - `--vertical <id>` (Stream `claude-skill-search-by-vertical`): a
//!   store-side query against the `skill` table. With
//!   `--include-used-by-sessions` the search also walks
//!   `session_skill ↔ session` to surface skills used by sessions
//!   in the vertical.

use crate::home::{db_path, ensure_dir, resolve_home};
use argo_skill::marketplace::{parse_trust_grade, search, MarketplaceSkill, SearchOptions};
use argo_skill::TrustGrade;
use argo_store::skill_query::{distinct_skill_verticals, search_by_vertical, SkillVerticalRow};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};

#[allow(clippy::too_many_arguments)]
pub async fn run(
    query: &str,
    trust_min: Option<&str>,
    tenant: Option<&str>,
    limit: usize,
    json: bool,
    vertical: Option<&str>,
    include_used_by_sessions: bool,
) -> ArgoResult<()> {
    if let Some(vid) = vertical {
        return run_vertical(vid, limit, include_used_by_sessions, json).await;
    }
    let trust_min = trust_min
        .map(parse_trust_grade)
        .transpose()
        .map_err(ArgoError::Provider)?;
    let options = SearchOptions {
        trust_min,
        tenant: tenant.map(str::to_string),
        limit,
    };
    let rows = search(query, &options);
    if json {
        let serialized = serde_json::to_string(&rows)
            .map_err(|e| ArgoError::Provider(format!("serialize skill search: {e}")))?;
        println!("{serialized}");
        return Ok(());
    }
    render_table(&rows);
    Ok(())
}

async fn run_vertical(
    vertical_id: &str,
    limit: usize,
    indirect: bool,
    json: bool,
) -> ArgoResult<()> {
    let store = open_store().await?;
    let pool = store.pool();
    let limit_u32 = u32::try_from(limit.clamp(1, 500)).unwrap_or(500);
    let rows = search_by_vertical(pool, vertical_id, limit_u32, indirect).await?;
    if rows.is_empty() && !known_vertical(pool, vertical_id).await? {
        let valid = distinct_skill_verticals(pool).await.unwrap_or_default();
        let known = if valid.is_empty() {
            "(no skills currently tagged with a vertical)".to_string()
        } else {
            valid.join(", ")
        };
        return Err(ArgoError::Provider(format!(
            "vertical '{vertical_id}' has no skills (direct or indirect). Known skill vertical_ids: {known}"
        )));
    }
    if json {
        let serialized = serde_json::to_string(&rows)
            .map_err(|e| ArgoError::Provider(format!("serialize skill search: {e}")))?;
        println!("{serialized}");
        return Ok(());
    }
    render_vertical_table(&rows, indirect);
    Ok(())
}

async fn open_store() -> ArgoResult<SessionStore> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(db_path(&home)).await?;
    store.migrate().await?;
    Ok(store)
}

async fn known_vertical(pool: &sqlx::Pool<sqlx::Sqlite>, vertical_id: &str) -> ArgoResult<bool> {
    let verticals = distinct_skill_verticals(pool).await?;
    Ok(verticals.iter().any(|v| v == vertical_id))
}

fn render_table(rows: &[MarketplaceSkill]) {
    if rows.is_empty() {
        println!("(no marketplace skills matched)");
        return;
    }
    println!(
        "{:<30} {:<12} {:<20} {:<10} {:<20} description",
        "skill", "tenant", "trust", "version", "last_published"
    );
    for row in rows {
        println!(
            "{:<30} {:<12} {:<20} {:<10} {:<20} {}",
            row.name,
            row.tenant,
            trust_label(row.trust_grade),
            row.version,
            row.last_published_at,
            row.description
        );
    }
}

fn render_vertical_table(rows: &[SkillVerticalRow], indirect: bool) {
    if rows.is_empty() {
        println!("(no skills found in vertical)");
        return;
    }
    let activity_label = if indirect { "last_attached" } else { "created_at" };
    println!(
        "{:<36} {:<24} {:<10} {:<24} {}",
        "skill_id", "name", "version", "vertical_id", activity_label
    );
    for row in rows {
        println!(
            "{:<36} {:<24} {:<10} {:<24} {}",
            row.skill_id,
            row.skill_name,
            row.version,
            row.vertical_id.as_deref().unwrap_or("-"),
            row.last_activity_micros,
        );
    }
}

fn trust_label(grade: TrustGrade) -> &'static str {
    match grade {
        TrustGrade::Builtin => "builtin",
        TrustGrade::PublishedVerified => "published_verified",
        TrustGrade::Tenant => "tenant",
        TrustGrade::PublishedCommunity => "published_community",
        TrustGrade::LearnedUnreviewed => "learned_unreviewed",
    }
}
