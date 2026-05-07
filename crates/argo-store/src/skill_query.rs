//! Skill discovery queries scoped to a vertical (Stream
//! `claude-skill-search-by-vertical`).
//!
//! Two modes:
//!
//! - **direct**: rows whose own `skill.vertical_id` matches the
//!   given vertical id (the normal case — a skill written for one
//!   vertical's workers).
//! - **indirect**: rows whose skill has been attached (via
//!   `session_skill`) to at least one session whose
//!   `session.vertical_id` matches. Surfaces skills that workers
//!   in the vertical have actually used regardless of how the skill
//!   row tagged itself.
//!
//! Both modes return [`SkillVerticalRow`] sorted newest-activity-first
//! and limited to `limit` rows.

use argo_types::{ArgoError, ArgoResult};
use serde::Serialize;
use sqlx::{Pool, Row, Sqlite};

use crate::models::uuid_from_blob;

/// One skill row returned by [`search_by_vertical`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SkillVerticalRow {
    /// UUIDv7 of the `skill` row.
    pub skill_id: String,
    /// `skill.name` display string.
    pub skill_name: String,
    /// `skill.version` semver/version label.
    pub version: String,
    /// `skill.vertical_id` as stamped on the row (always populated
    /// for direct mode; `Some` from the join for indirect mode).
    pub vertical_id: Option<String>,
    /// Direct mode: the skill row's `created_at` (UTC micros).
    /// Indirect mode: the most recent `session_skill.attached_at`
    /// for this skill across sessions in the vertical.
    pub last_activity_micros: i64,
}

/// Return all skills associated with `vertical_id`.
///
/// `indirect = false` → look at `skill.vertical_id` directly.
/// `indirect = true`  → look at sessions whose `session.vertical_id`
/// matches and surface skills attached to them.
///
/// `limit` is clamped to `[1, 500]`.
pub async fn search_by_vertical(
    pool: &Pool<Sqlite>,
    vertical_id: &str,
    limit: u32,
    indirect: bool,
) -> ArgoResult<Vec<SkillVerticalRow>> {
    let limit = i64::from(limit.clamp(1, 500));
    if indirect {
        search_indirect(pool, vertical_id, limit).await
    } else {
        search_direct(pool, vertical_id, limit).await
    }
}

/// Return the distinct set of `skill.vertical_id` values currently
/// stored — handy for the CLI's "vertical-not-found, here are valid
/// ids" error message.
pub async fn distinct_skill_verticals(pool: &Pool<Sqlite>) -> ArgoResult<Vec<String>> {
    let rows = sqlx::query(
        "SELECT DISTINCT vertical_id FROM skill \
         WHERE vertical_id IS NOT NULL ORDER BY vertical_id",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("distinct_skill_verticals: {e}")))?;
    Ok(rows
        .into_iter()
        .filter_map(|row| row.try_get::<String, _>("vertical_id").ok())
        .collect())
}

async fn search_direct(
    pool: &Pool<Sqlite>,
    vertical_id: &str,
    limit: i64,
) -> ArgoResult<Vec<SkillVerticalRow>> {
    let rows = sqlx::query(
        "SELECT id, name, version, vertical_id, created_at \
         FROM skill \
         WHERE vertical_id = ? \
         ORDER BY created_at DESC, name ASC, version ASC \
         LIMIT ?",
    )
    .bind(vertical_id)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("skill_search direct: {e}")))?;
    rows.into_iter().map(direct_row).collect()
}

async fn search_indirect(
    pool: &Pool<Sqlite>,
    vertical_id: &str,
    limit: i64,
) -> ArgoResult<Vec<SkillVerticalRow>> {
    let rows = sqlx::query(
        "SELECT s.id, s.name, s.version, s.vertical_id, \
                MAX(ss.attached_at) AS last_attached \
         FROM skill s \
         JOIN session_skill ss ON ss.skill_id = s.id \
         JOIN session sess ON sess.id = ss.session_id \
         WHERE sess.vertical_id = ? \
         GROUP BY s.id, s.name, s.version, s.vertical_id \
         ORDER BY last_attached DESC, s.name ASC, s.version ASC \
         LIMIT ?",
    )
    .bind(vertical_id)
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("skill_search indirect: {e}")))?;
    rows.into_iter().map(indirect_row).collect()
}

fn direct_row(row: sqlx::sqlite::SqliteRow) -> ArgoResult<SkillVerticalRow> {
    let id_blob: Vec<u8> = row.try_get("id")
        .map_err(|e| ArgoError::Provider(format!("skill.id: {e}")))?;
    Ok(SkillVerticalRow {
        skill_id: uuid_from_blob(&id_blob).to_string(),
        skill_name: row.try_get("name")
            .map_err(|e| ArgoError::Provider(format!("skill.name: {e}")))?,
        version: row.try_get("version")
            .map_err(|e| ArgoError::Provider(format!("skill.version: {e}")))?,
        vertical_id: row.try_get("vertical_id").ok(),
        last_activity_micros: row.try_get("created_at")
            .map_err(|e| ArgoError::Provider(format!("skill.created_at: {e}")))?,
    })
}

fn indirect_row(row: sqlx::sqlite::SqliteRow) -> ArgoResult<SkillVerticalRow> {
    let id_blob: Vec<u8> = row.try_get("id")
        .map_err(|e| ArgoError::Provider(format!("skill.id: {e}")))?;
    Ok(SkillVerticalRow {
        skill_id: uuid_from_blob(&id_blob).to_string(),
        skill_name: row.try_get("name")
            .map_err(|e| ArgoError::Provider(format!("skill.name: {e}")))?,
        version: row.try_get("version")
            .map_err(|e| ArgoError::Provider(format!("skill.version: {e}")))?,
        vertical_id: row.try_get("vertical_id").ok(),
        last_activity_micros: row.try_get("last_attached")
            .map_err(|e| ArgoError::Provider(format!("session_skill.attached_at: {e}")))?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SessionStore;
    use argo_types::TenantId;
    use uuid::Uuid;

    async fn seed_store() -> SessionStore {
        let store = SessionStore::open(":memory:").await.unwrap();
        store.migrate().await.unwrap();
        store
    }

    async fn insert_skill(
        pool: &Pool<Sqlite>,
        id: Uuid,
        name: &str,
        version: &str,
        vertical_id: Option<&str>,
        created_at: i64,
    ) {
        sqlx::query(
            "INSERT INTO skill (id, name, version, body, frontmatter, source, hash, created_at, vertical_id) \
             VALUES (?, ?, ?, '', '', 'inline', randomblob(8), ?, ?)",
        )
        .bind(id.as_bytes().as_slice())
        .bind(name)
        .bind(version)
        .bind(created_at)
        .bind(vertical_id)
        .execute(pool)
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn direct_filters_by_vertical_and_orders_newest_first() {
        let store = seed_store().await;
        let pool = store.pool();
        let now = chrono::Utc::now().timestamp_micros();
        let v_a_old = Uuid::now_v7();
        let v_a_new = Uuid::now_v7();
        let v_b = Uuid::now_v7();
        insert_skill(pool, v_a_old, "alpha", "1.0", Some("vertical-a"), now - 1_000).await;
        insert_skill(pool, v_a_new, "beta", "1.0", Some("vertical-a"), now).await;
        insert_skill(pool, v_b, "gamma", "1.0", Some("vertical-b"), now).await;

        let rows = search_by_vertical(pool, "vertical-a", 10, false).await.unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].skill_name, "beta");
        assert_eq!(rows[1].skill_name, "alpha");
    }

    #[tokio::test]
    async fn direct_returns_empty_for_unknown_vertical() {
        let store = seed_store().await;
        let pool = store.pool();
        insert_skill(pool, Uuid::now_v7(), "alpha", "1.0", Some("vertical-a"), 0).await;
        let rows = search_by_vertical(pool, "vertical-z", 10, false).await.unwrap();
        assert!(rows.is_empty());
    }

    #[tokio::test]
    async fn direct_limit_clamps_returned_rows() {
        let store = seed_store().await;
        let pool = store.pool();
        for i in 0..5 {
            let name = format!("skill-{i}");
            insert_skill(pool, Uuid::now_v7(), &name, "1.0", Some("vertical-a"), i as i64).await;
        }
        let rows = search_by_vertical(pool, "vertical-a", 2, false).await.unwrap();
        assert_eq!(rows.len(), 2);
    }

    #[tokio::test]
    async fn distinct_lists_skill_vertical_ids() {
        let store = seed_store().await;
        let pool = store.pool();
        insert_skill(pool, Uuid::now_v7(), "a", "1", Some("v-a"), 0).await;
        insert_skill(pool, Uuid::now_v7(), "b", "1", Some("v-b"), 0).await;
        insert_skill(pool, Uuid::now_v7(), "c", "1", None, 0).await;
        let verticals = distinct_skill_verticals(pool).await.unwrap();
        assert_eq!(verticals, vec!["v-a".to_string(), "v-b".to_string()]);
    }

    #[tokio::test]
    async fn indirect_surfaces_attached_skills_via_session_vertical() {
        let store = seed_store().await;
        let pool = store.pool();
        let tenant = TenantId::new();
        let (_t, ws) = store
            .ensure_workspace("skill-search", "/tmp/skill-search-test")
            .await
            .unwrap();
        let _ = tenant;

        // Create one session bound to vertical-a, one to vertical-b.
        let session_a = argo_types::SessionId::new();
        let session_b = argo_types::SessionId::new();
        sqlx::query(
            "INSERT INTO session (id, tenant_id, workspace_id, model, provider, started_at, vertical_id) \
             VALUES (?, ?, ?, 'mock', 'mock', ?, ?)",
        )
        .bind(session_a.0.as_bytes().as_slice())
        .bind(_t.0.as_bytes().as_slice())
        .bind(ws.0.as_bytes().as_slice())
        .bind(0i64)
        .bind("vertical-a")
        .execute(pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO session (id, tenant_id, workspace_id, model, provider, started_at, vertical_id) \
             VALUES (?, ?, ?, 'mock', 'mock', ?, ?)",
        )
        .bind(session_b.0.as_bytes().as_slice())
        .bind(_t.0.as_bytes().as_slice())
        .bind(ws.0.as_bytes().as_slice())
        .bind(0i64)
        .bind("vertical-b")
        .execute(pool)
        .await
        .unwrap();

        // Skill bound to vertical-b but used by a session in vertical-a.
        let skill = Uuid::now_v7();
        insert_skill(pool, skill, "shared", "1.0", Some("vertical-b"), 0).await;
        sqlx::query(
            "INSERT INTO session_skill (session_id, skill_id, attached_at) VALUES (?, ?, ?)",
        )
        .bind(session_a.0.as_bytes().as_slice())
        .bind(skill.as_bytes().as_slice())
        .bind(123i64)
        .execute(pool)
        .await
        .unwrap();

        // Direct mode for vertical-a → empty (skill row says vertical-b).
        let direct = search_by_vertical(pool, "vertical-a", 10, false).await.unwrap();
        assert!(direct.is_empty());

        // Indirect mode for vertical-a → finds the shared skill.
        let indirect = search_by_vertical(pool, "vertical-a", 10, true).await.unwrap();
        assert_eq!(indirect.len(), 1);
        assert_eq!(indirect[0].skill_name, "shared");
        assert_eq!(indirect[0].last_activity_micros, 123);
    }
}
