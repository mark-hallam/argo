//! Shared helpers for the `argo feedback ...` handlers.

use crate::home::{db_path, ensure_dir, resolve_home};
use argo_skill::SkillStore;
use argo_store::skill_rating::{record_rating, SkillVote};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use std::sync::Arc;

use super::skill_rate::publisher_from_source;

const SCHEMA_SQL: &str =
    include_str!("../../../argo-store/migrations/24__operator_feedback.sql");

pub(super) async fn open_store() -> ArgoResult<Arc<SessionStore>> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    Ok(Arc::new(store))
}

/// Best-effort cross-write into `skill_rating` after an `argo feedback
/// up|down --skill <name>` call. Returns `true` when a row was written
/// (skill is marketplace-installed), `false` when the skill is not
/// marketplace-installed (no rating row, no error). Errors propagate
/// only on hard failures (skill name unknown, store I/O failure).
///
/// Composes with [`open_store`]: callers pass the same `Arc<SessionStore>`
/// they already opened for `operator_feedback`, so the second write
/// shares the pool and migration state.
pub(super) async fn maybe_record_skill_rating(
    store: &Arc<SessionStore>,
    skill_name: &str,
    vote: SkillVote,
    comment: Option<&str>,
) -> ArgoResult<bool> {
    let skills = SkillStore::new(store.clone());
    let skill = skills
        .find_active_by_name(skill_name)
        .await?
        .ok_or_else(|| ArgoError::Provider(format!("no active skill named '{skill_name}'")))?;
    let Some(publisher) = publisher_from_source(&skill.frontmatter.provenance.source) else {
        return Ok(false);
    };
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let workspace_name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let workspace_root = cwd.to_string_lossy();
    let (tenant_id, _wsid) = store
        .ensure_workspace(workspace_name, &workspace_root)
        .await?;
    record_rating(
        store.pool(),
        skill.id,
        &skill.frontmatter.version,
        &publisher,
        tenant_id,
        vote,
        comment,
    )
    .await?;
    Ok(true)
}

pub(super) async fn ensure_feedback_schema(store: &SessionStore) -> ArgoResult<()> {
    for stmt in SCHEMA_SQL.split_terminator(';') {
        let sql = stmt.trim();
        if sql.is_empty() {
            continue;
        }
        sqlx::query(sql)
            .execute(store.pool())
            .await
            .map_err(|e| ArgoError::Provider(format!("feedback schema: {e}")))?;
    }
    Ok(())
}
