//! Stream `claude-data-classification` — sqlx CRUD over the
//! `data_classification_tag` table.
//!
//! The typed [`argo_identity::classification::DataClass`] enum sits
//! one layer up; this module accepts class strings (`"pii"`,
//! `"phi"`, ...) so we don't need to depend on `argo-identity`
//! from `argo-store` (which would invert the dep graph).
//!
//! # Schema
//!
//! See `migrations/26__data_classification.sql`. Primary key is
//! `(record_kind, record_id, class)` so a record can carry multiple
//! tags, but no class duplicates per record.

use argo_types::{ArgoError, ArgoResult};
use chrono::{TimeZone, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

/// One row of `data_classification_tag`. Strings stay raw at this
/// layer — typed parsing is the caller's job.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassificationTag {
    /// Logical record kind (e.g. `"memory_entry"`).
    pub record_kind: String,
    /// Record id (caller-defined string).
    pub record_id: String,
    /// Class string (`"pii"` / `"phi"` / ...).
    pub class: String,
    /// Microseconds since epoch.
    pub tagged_at_micros: i64,
    /// Optional actor id.
    pub tagged_by: Option<String>,
}

/// Insert (or replace if duplicate) a classification tag. Returns
/// the `tagged_at_micros` written (always `now`). Schema is owned
/// by `migrations/v31.rs`; callers must run `SessionStore::migrate`
/// before use.
pub async fn insert_tag(
    pool: &SqlitePool,
    record_kind: &str,
    record_id: &str,
    class: &str,
    tagged_by: Option<&str>,
) -> ArgoResult<i64> {
    let now = Utc::now().timestamp_micros();
    sqlx::query(
        "INSERT OR REPLACE INTO data_classification_tag \
         (record_kind, record_id, class, tagged_at_micros, tagged_by) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(record_kind)
    .bind(record_id)
    .bind(class)
    .bind(now)
    .bind(tagged_by)
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("classification insert: {e}")))?;
    Ok(now)
}

/// List tags. All filters are optional; `limit` caps the result.
/// Rows are returned newest-first.
pub async fn list_tags(
    pool: &SqlitePool,
    class: Option<&str>,
    record_kind: Option<&str>,
    tagged_by: Option<&str>,
    limit: u32,
) -> ArgoResult<Vec<ClassificationTag>> {
    let class_filter = class.unwrap_or("");
    let kind_filter = record_kind.unwrap_or("");
    let actor_filter = tagged_by.unwrap_or("");
    let limit_i64 = i64::from(limit);
    let rows = sqlx::query_as::<_, (String, String, String, i64, Option<String>)>(
        "SELECT record_kind, record_id, class, tagged_at_micros, tagged_by \
         FROM data_classification_tag \
         WHERE (?1 = '' OR class = ?1) \
           AND (?2 = '' OR record_kind = ?2) \
           AND (?3 = '' OR tagged_by = ?3) \
         ORDER BY tagged_at_micros DESC \
         LIMIT ?4",
    )
    .bind(class_filter)
    .bind(kind_filter)
    .bind(actor_filter)
    .bind(limit_i64)
    .fetch_all(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("classification list: {e}")))?;
    Ok(rows
        .into_iter()
        .map(|(record_kind, record_id, class, tagged_at_micros, tagged_by)| ClassificationTag {
            record_kind,
            record_id,
            class,
            tagged_at_micros,
            tagged_by,
        })
        .collect())
}

/// Fetch every class tagged on one record. Empty vec if untagged.
pub async fn classes_for_record(
    pool: &SqlitePool,
    record_kind: &str,
    record_id: &str,
) -> ArgoResult<Vec<String>> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT class FROM data_classification_tag \
         WHERE record_kind = ? AND record_id = ? \
         ORDER BY class ASC",
    )
    .bind(record_kind)
    .bind(record_id)
    .fetch_all(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("classes for record: {e}")))?;
    Ok(rows.into_iter().map(|(c,)| c).collect())
}

/// Whether a record carries at least one of the given classes.
pub async fn record_has_any_class(
    pool: &SqlitePool,
    record_kind: &str,
    record_id: &str,
    classes: &[&str],
) -> ArgoResult<bool> {
    if classes.is_empty() {
        return Ok(false);
    }
    let existing = classes_for_record(pool, record_kind, record_id).await?;
    Ok(existing.iter().any(|c| classes.contains(&c.as_str())))
}

/// Delete tags older than `cutoff_micros` for the given class.
/// Returns the number of rows removed. Used by retention sweeps.
pub async fn prune_tags_older_than(
    pool: &SqlitePool,
    class: &str,
    cutoff_micros: i64,
) -> ArgoResult<u64> {
    let result = sqlx::query(
        "DELETE FROM data_classification_tag \
         WHERE class = ? AND tagged_at_micros < ?",
    )
    .bind(class)
    .bind(cutoff_micros)
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("classification prune: {e}")))?;
    Ok(result.rows_affected())
}

/// Pretty-print `tagged_at_micros` as RFC-3339 for logs / CLI text.
#[must_use]
pub fn micros_to_rfc3339(micros: i64) -> String {
    Utc.timestamp_micros(micros)
        .single()
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_else(|| micros.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SessionStore;

    async fn fresh_pool() -> SqlitePool {
        // The data_classification_tag table now lives in
        // `migrations/v31.rs`; opening a SessionStore + migrate gives
        // us the table without including raw SQL.
        let store = SessionStore::open(":memory:").await.unwrap();
        store.migrate().await.unwrap();
        store.pool().clone()
    }

    #[tokio::test]
    async fn insert_and_list_round_trips() {
        let pool = fresh_pool().await;
        insert_tag(&pool, "memory_entry", "m-1", "pii", Some("op-1"))
            .await
            .unwrap();
        let rows = list_tags(&pool, None, None, None, 100).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].record_kind, "memory_entry");
        assert_eq!(rows[0].record_id, "m-1");
        assert_eq!(rows[0].class, "pii");
        assert_eq!(rows[0].tagged_by.as_deref(), Some("op-1"));
    }

    #[tokio::test]
    async fn list_filters_combine() {
        let pool = fresh_pool().await;
        insert_tag(&pool, "memory_entry", "m-1", "pii", Some("op-1"))
            .await
            .unwrap();
        insert_tag(&pool, "audit_record", "a-1", "pii", Some("op-2"))
            .await
            .unwrap();
        insert_tag(&pool, "memory_entry", "m-2", "secret", None)
            .await
            .unwrap();
        let pii_only = list_tags(&pool, Some("pii"), None, None, 100).await.unwrap();
        assert_eq!(pii_only.len(), 2);
        let memory_pii = list_tags(&pool, Some("pii"), Some("memory_entry"), None, 100)
            .await
            .unwrap();
        assert_eq!(memory_pii.len(), 1);
        assert_eq!(memory_pii[0].record_id, "m-1");
        let by_op2 = list_tags(&pool, None, None, Some("op-2"), 100).await.unwrap();
        assert_eq!(by_op2.len(), 1);
        assert_eq!(by_op2[0].class, "pii");
    }

    #[tokio::test]
    async fn replace_on_duplicate_keeps_one_row() {
        let pool = fresh_pool().await;
        insert_tag(&pool, "memory_entry", "m-1", "pii", Some("op-1"))
            .await
            .unwrap();
        insert_tag(&pool, "memory_entry", "m-1", "pii", Some("op-2"))
            .await
            .unwrap();
        let rows = list_tags(&pool, None, None, None, 100).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].tagged_by.as_deref(), Some("op-2"));
    }

    #[tokio::test]
    async fn classes_for_record_returns_all() {
        let pool = fresh_pool().await;
        insert_tag(&pool, "memory_entry", "m-1", "pii", None).await.unwrap();
        insert_tag(&pool, "memory_entry", "m-1", "secret", None)
            .await
            .unwrap();
        insert_tag(&pool, "memory_entry", "m-2", "phi", None).await.unwrap();
        let classes = classes_for_record(&pool, "memory_entry", "m-1")
            .await
            .unwrap();
        assert_eq!(classes, vec!["pii".to_string(), "secret".to_string()]);
        let none = classes_for_record(&pool, "memory_entry", "m-missing")
            .await
            .unwrap();
        assert!(none.is_empty());
    }

    #[tokio::test]
    async fn record_has_any_class_matches() {
        let pool = fresh_pool().await;
        insert_tag(&pool, "memory_entry", "m-1", "pii", None).await.unwrap();
        assert!(
            record_has_any_class(&pool, "memory_entry", "m-1", &["pii", "phi"])
                .await
                .unwrap()
        );
        assert!(
            !record_has_any_class(&pool, "memory_entry", "m-1", &["secret"])
                .await
                .unwrap()
        );
        assert!(
            !record_has_any_class(&pool, "memory_entry", "m-1", &[])
                .await
                .unwrap()
        );
    }

    #[tokio::test]
    async fn prune_removes_old_tags() {
        let pool = fresh_pool().await;
        // Manually insert with backdated tagged_at to simulate old rows.
        sqlx::query(
            "INSERT INTO data_classification_tag \
             (record_kind, record_id, class, tagged_at_micros, tagged_by) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind("memory_entry")
        .bind("m-old")
        .bind("pii")
        .bind(1_000_i64)
        .bind::<Option<&str>>(None)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO data_classification_tag \
             (record_kind, record_id, class, tagged_at_micros, tagged_by) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind("memory_entry")
        .bind("m-new")
        .bind("pii")
        .bind(99_999_999_999_999_i64)
        .bind::<Option<&str>>(None)
        .execute(&pool)
        .await
        .unwrap();
        let removed = prune_tags_older_than(&pool, "pii", 1_000_000).await.unwrap();
        assert_eq!(removed, 1);
        let remaining = list_tags(&pool, None, None, None, 100).await.unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].record_id, "m-new");
    }
}
