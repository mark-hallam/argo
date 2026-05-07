//! Budget rollups over skills.
//!
//! The durable spend ledger is session-scoped. Skill attribution
//! follows the `session_skill` association table, so a session's
//! spend is attributed to each attached skill.
use argo_types::ArgoError;
use serde::Serialize;
use sqlx::{Pool, QueryBuilder, Row, Sqlite};

/// Ranked spend bucket for one skill.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SkillSpendRow {
    /// Skill UUID.
    pub skill_id: String,
    /// Skill display name.
    pub skill_name: String,
    /// Skill semver/version label.
    pub version: String,
    /// Optional vertical scope stamped on the skill row.
    pub vertical_id: Option<String>,
    /// Total spend in USD micros.
    pub spend_usd_micros: u64,
    /// Number of spend ledger rows in this bucket.
    pub charge_count: u64,
    /// Number of distinct sessions contributing spend.
    pub session_count: u64,
}

/// Per-day spend bucket for one skill name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SkillSpendDay {
    /// UTC day bucket, formatted `YYYY-MM-DD`.
    pub day: String,
    /// Skill name the history was requested for.
    pub skill_name: String,
    /// Total spend in USD micros.
    pub spend_usd_micros: u64,
    /// Number of spend ledger rows in this bucket.
    pub charge_count: u64,
}

/// Return top skills by attributed spend, descending.
pub async fn spend_by_skill_top_n(
    pool: &Pool<Sqlite>,
    since_micros: Option<i64>,
    vertical_id: Option<&str>,
    limit: u32,
) -> Result<Vec<SkillSpendRow>, ArgoError> {
    let mut qb = QueryBuilder::<Sqlite>::new(
        "SELECT s.id, s.name, s.version, s.vertical_id, \
         COALESCE(SUM(l.amount_usd_micros), 0) AS total, \
         COUNT(l.seq) AS charges, COUNT(DISTINCT l.session_id) AS sessions \
         FROM spend_ledger l \
         JOIN session_skill ss ON ss.session_id = l.session_id \
         JOIN skill s ON s.id = ss.skill_id WHERE 1=1",
    );
    if let Some(cutoff) = since_micros {
        qb.push(" AND l.charged_at_micros >= ").push_bind(cutoff);
    }
    if let Some(vertical) = vertical_id {
        qb.push(" AND s.vertical_id = ").push_bind(vertical);
    }
    qb.push(
        " GROUP BY s.id, s.name, s.version, s.vertical_id \
             ORDER BY total DESC, s.name ASC, s.version ASC LIMIT ",
    )
    .push_bind(i64::from(limit.clamp(1, 500)));

    let rows = qb
        .build()
        .fetch_all(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("spend_by_skill_top_n: {e}")))?;
    rows.into_iter().map(skill_row).collect()
}

/// Return daily spend history for one skill name, oldest first.
pub async fn spend_by_skill_history(
    pool: &Pool<Sqlite>,
    skill_name: &str,
    since_micros: Option<i64>,
    vertical_id: Option<&str>,
) -> Result<Vec<SkillSpendDay>, ArgoError> {
    let mut qb = QueryBuilder::<Sqlite>::new(
        "SELECT strftime('%Y-%m-%d', l.charged_at_micros / 1000000, 'unixepoch') AS day, \
         s.name, COALESCE(SUM(l.amount_usd_micros), 0) AS total, COUNT(l.seq) AS charges \
         FROM spend_ledger l \
         JOIN session_skill ss ON ss.session_id = l.session_id \
         JOIN skill s ON s.id = ss.skill_id WHERE s.name = ",
    );
    qb.push_bind(skill_name);
    if let Some(cutoff) = since_micros {
        qb.push(" AND l.charged_at_micros >= ").push_bind(cutoff);
    }
    if let Some(vertical) = vertical_id {
        qb.push(" AND s.vertical_id = ").push_bind(vertical);
    }
    qb.push(" GROUP BY day, s.name ORDER BY day ASC");

    let rows = qb
        .build()
        .fetch_all(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("spend_by_skill_history: {e}")))?;
    rows.into_iter().map(history_row).collect()
}

fn skill_row(row: sqlx::sqlite::SqliteRow) -> Result<SkillSpendRow, ArgoError> {
    let id: Vec<u8> = row
        .try_get(0)
        .map_err(|e| ArgoError::Provider(format!("skill id: {e}")))?;
    Ok(SkillSpendRow {
        skill_id: uuid_or_hex(&id),
        skill_name: row
            .try_get(1)
            .map_err(|e| ArgoError::Provider(format!("skill name: {e}")))?,
        version: row
            .try_get(2)
            .map_err(|e| ArgoError::Provider(format!("skill version: {e}")))?,
        vertical_id: row
            .try_get(3)
            .map_err(|e| ArgoError::Provider(format!("skill vertical: {e}")))?,
        spend_usd_micros: nonnegative(row.try_get::<i64, _>(4), "skill total")?,
        charge_count: nonnegative(row.try_get::<i64, _>(5), "skill charges")?,
        session_count: nonnegative(row.try_get::<i64, _>(6), "skill sessions")?,
    })
}

fn history_row(row: sqlx::sqlite::SqliteRow) -> Result<SkillSpendDay, ArgoError> {
    Ok(SkillSpendDay {
        day: row
            .try_get(0)
            .map_err(|e| ArgoError::Provider(format!("history day: {e}")))?,
        skill_name: row
            .try_get(1)
            .map_err(|e| ArgoError::Provider(format!("history skill: {e}")))?,
        spend_usd_micros: nonnegative(row.try_get::<i64, _>(2), "history total")?,
        charge_count: nonnegative(row.try_get::<i64, _>(3), "history charges")?,
    })
}

fn nonnegative(value: Result<i64, sqlx::Error>, label: &str) -> Result<u64, ArgoError> {
    let value = value.map_err(|e| ArgoError::Provider(format!("{label}: {e}")))?;
    Ok(u64::try_from(value).unwrap_or(0))
}

fn uuid_or_hex(raw: &[u8]) -> String {
    uuid::Uuid::from_slice(raw)
        .map(|id| id.to_string())
        .unwrap_or_else(|_| raw.iter().map(|byte| format!("{byte:02x}")).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SessionStore;
    use argo_types::{TenantId, WorkspaceId};
    use chrono::{TimeZone, Utc};
    use uuid::Uuid;

    async fn store() -> SessionStore {
        let store = SessionStore::open(":memory:").await.unwrap();
        store.migrate().await.unwrap();
        store
    }

    async fn workspace(store: &SessionStore) -> (TenantId, WorkspaceId) {
        let tenant = TenantId::new();
        let workspace = WorkspaceId::new();
        sqlx::query(
            "INSERT INTO workspace (id, tenant_id, name, root_path, created_at) \
             VALUES (?, ?, 't', '.', 1)",
        )
        .bind(workspace.0.as_bytes().as_slice())
        .bind(tenant.0.as_bytes().as_slice())
        .execute(store.pool())
        .await
        .unwrap();
        (tenant, workspace)
    }

    async fn skill(store: &SessionStore, name: &str, version: &str, vertical: &str) -> Uuid {
        let id = Uuid::now_v7();
        sqlx::query(
            "INSERT INTO skill (id, name, version, body, frontmatter, source, hash, created_at, vertical_id) \
             VALUES (?, ?, ?, 'body', '{}', 'test', ?, 1, ?)",
        )
        .bind(id.as_bytes().as_slice())
        .bind(name)
        .bind(version)
        .bind(vec![1_u8; 32])
        .bind(vertical)
        .execute(store.pool())
        .await
        .unwrap();
        id
    }

    async fn attach(store: &SessionStore, session: argo_types::SessionId, skill: Uuid) {
        sqlx::query(
            "INSERT INTO session_skill (session_id, skill_id, attached_at) VALUES (?, ?, 1)",
        )
        .bind(session.0.as_bytes().as_slice())
        .bind(skill.as_bytes().as_slice())
        .execute(store.pool())
        .await
        .unwrap();
    }

    fn micros(day: u32) -> i64 {
        Utc.with_ymd_and_hms(2026, 5, day, 12, 0, 0)
            .single()
            .unwrap()
            .timestamp_micros()
    }

    #[tokio::test]
    async fn top_n_ranks_skills_by_spend() {
        let store = store().await;
        let (tenant, workspace) = workspace(&store).await;
        let alpha = skill(&store, "alpha", "1.0.0", "v1").await;
        let beta = skill(&store, "beta", "1.0.0", "v1").await;
        let s1 = store
            .create_session(tenant, workspace, "m", "p", None, Some("v1"))
            .await
            .unwrap();
        let s2 = store
            .create_session(tenant, workspace, "m", "p", None, Some("v1"))
            .await
            .unwrap();
        attach(&store, s1, alpha).await;
        attach(&store, s2, beta).await;
        store
            .record_spend(tenant, s1, micros(1), 2_000_000, "p", "m")
            .await
            .unwrap();
        store
            .record_spend(tenant, s2, micros(1), 5_000_000, "p", "m")
            .await
            .unwrap();

        let rows = spend_by_skill_top_n(store.pool(), None, None, 10)
            .await
            .unwrap();

        assert_eq!(rows[0].skill_name, "beta");
        assert_eq!(rows[0].spend_usd_micros, 5_000_000);
        assert_eq!(rows[1].skill_name, "alpha");
    }

    #[tokio::test]
    async fn top_n_honors_since_cutoff() {
        let store = store().await;
        let (tenant, workspace) = workspace(&store).await;
        let alpha = skill(&store, "alpha", "1.0.0", "v1").await;
        let session = store
            .create_session(tenant, workspace, "m", "p", None, Some("v1"))
            .await
            .unwrap();
        attach(&store, session, alpha).await;
        store
            .record_spend(tenant, session, micros(1), 9_000_000, "p", "m")
            .await
            .unwrap();
        store
            .record_spend(tenant, session, micros(5), 1_000_000, "p", "m")
            .await
            .unwrap();

        let rows = spend_by_skill_top_n(store.pool(), Some(micros(4)), None, 10)
            .await
            .unwrap();

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].spend_usd_micros, 1_000_000);
        assert_eq!(rows[0].charge_count, 1);
    }

    #[tokio::test]
    async fn top_n_honors_vertical_filter() {
        let store = store().await;
        let (tenant, workspace) = workspace(&store).await;
        let alpha = skill(&store, "alpha", "1.0.0", "alpha-vertical").await;
        let beta = skill(&store, "beta", "1.0.0", "beta-vertical").await;
        let s1 = store
            .create_session(tenant, workspace, "m", "p", None, Some("alpha-vertical"))
            .await
            .unwrap();
        let s2 = store
            .create_session(tenant, workspace, "m", "p", None, Some("beta-vertical"))
            .await
            .unwrap();
        attach(&store, s1, alpha).await;
        attach(&store, s2, beta).await;
        store
            .record_spend(tenant, s1, micros(1), 2_000_000, "p", "m")
            .await
            .unwrap();
        store
            .record_spend(tenant, s2, micros(1), 5_000_000, "p", "m")
            .await
            .unwrap();

        let rows = spend_by_skill_top_n(store.pool(), None, Some("alpha-vertical"), 10)
            .await
            .unwrap();

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].skill_name, "alpha");
        assert_eq!(rows[0].vertical_id.as_deref(), Some("alpha-vertical"));
    }

    #[tokio::test]
    async fn history_groups_one_skill_by_utc_day() {
        let store = store().await;
        let (tenant, workspace) = workspace(&store).await;
        let alpha = skill(&store, "alpha", "1.0.0", "v1").await;
        let beta = skill(&store, "beta", "1.0.0", "v1").await;
        let s1 = store
            .create_session(tenant, workspace, "m", "p", None, Some("v1"))
            .await
            .unwrap();
        let s2 = store
            .create_session(tenant, workspace, "m", "p", None, Some("v1"))
            .await
            .unwrap();
        attach(&store, s1, alpha).await;
        attach(&store, s2, beta).await;
        store
            .record_spend(tenant, s1, micros(1), 2_000_000, "p", "m")
            .await
            .unwrap();
        store
            .record_spend(tenant, s1, micros(1) + 1, 3_000_000, "p", "m")
            .await
            .unwrap();
        store
            .record_spend(tenant, s1, micros(2), 4_000_000, "p", "m")
            .await
            .unwrap();
        store
            .record_spend(tenant, s2, micros(1), 7_000_000, "p", "m")
            .await
            .unwrap();

        let rows = spend_by_skill_history(store.pool(), "alpha", None, None)
            .await
            .unwrap();

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].day, "2026-05-01");
        assert_eq!(rows[0].spend_usd_micros, 5_000_000);
        assert_eq!(rows[0].charge_count, 2);
        assert_eq!(rows[1].day, "2026-05-02");
        assert_eq!(rows[1].spend_usd_micros, 4_000_000);
    }
}
