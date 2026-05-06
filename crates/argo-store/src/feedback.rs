//! Sqlx CRUD helpers for `operator_feedback` (Stream
//! `claude-operator-feedback-channel`). Thin typed wrapper
//! over the SQL — the higher-level scoring functions in
//! `argo-skill::success_metric` consume the resolved vote.

use chrono::{TimeZone, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};
use std::str::FromStr;

/// Operator vote on a single action. Stored as `'up'` / `'down'`
/// in sqlite; this enum is the typed surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Vote {
    /// Operator approved this action.
    Up,
    /// Operator rejected this action.
    Down,
}

impl Vote {
    /// String form persisted in sqlite.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Up => "up",
            Self::Down => "down",
        }
    }

}

impl FromStr for Vote {
    type Err = FeedbackSqlError;

    /// Parse from sqlite text. Errors on any unknown value
    /// rather than guessing — keeps the column to a closed set.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "up" => Ok(Self::Up),
            "down" => Ok(Self::Down),
            other => Err(FeedbackSqlError::Backend(format!(
                "unknown vote value '{other}'"
            ))),
        }
    }
}

/// One row in the `operator_feedback` table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeedbackRow {
    /// Auto-assigned row id.
    pub id: i64,
    /// Action id this feedback is about.
    pub action_id: String,
    /// up / down.
    pub vote: Vote,
    /// Operator comment (`""` if omitted).
    pub comment: String,
    /// Microseconds since epoch.
    pub given_at_micros: i64,
    /// Optional worker id (filterable).
    pub worker_id: Option<String>,
}

/// Errors surfaced by the feedback helpers.
#[derive(Debug, thiserror::Error)]
pub enum FeedbackSqlError {
    /// Underlying sqlx / parse error.
    #[error("feedback backend: {0}")]
    Backend(String),
}

impl From<sqlx::Error> for FeedbackSqlError {
    fn from(e: sqlx::Error) -> Self {
        Self::Backend(format!("sqlx: {e}"))
    }
}

/// Insert one feedback row. Returns the assigned `id`.
pub async fn insert_feedback(
    pool: &Pool<Sqlite>,
    action_id: &str,
    vote: Vote,
    comment: &str,
    worker_id: Option<&str>,
) -> Result<i64, FeedbackSqlError> {
    let now = Utc::now().timestamp_micros();
    let res = sqlx::query(
        "INSERT INTO operator_feedback \
         (action_id, vote, comment, given_at, worker_id) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(action_id)
    .bind(vote.as_str())
    .bind(comment)
    .bind(now)
    .bind(worker_id)
    .execute(pool)
    .await?;
    Ok(res.last_insert_rowid())
}

/// Fetch every feedback row, optionally filtered by `worker_id`
/// and / or `vote`. Newest first.
pub async fn list_feedback(
    pool: &Pool<Sqlite>,
    worker_id: Option<&str>,
    vote: Option<Vote>,
    limit: u32,
) -> Result<Vec<FeedbackRow>, FeedbackSqlError> {
    let lim = limit.clamp(1, 1000) as i64;
    let rows: Vec<(i64, String, String, String, i64, Option<String>)> = match (worker_id, vote) {
        (Some(w), Some(v)) => sqlx::query_as(
            "SELECT id, action_id, vote, comment, given_at, worker_id \
             FROM operator_feedback WHERE worker_id = ? AND vote = ? \
             ORDER BY given_at DESC LIMIT ?",
        )
        .bind(w)
        .bind(v.as_str())
        .bind(lim)
        .fetch_all(pool)
        .await?,
        (Some(w), None) => sqlx::query_as(
            "SELECT id, action_id, vote, comment, given_at, worker_id \
             FROM operator_feedback WHERE worker_id = ? \
             ORDER BY given_at DESC LIMIT ?",
        )
        .bind(w)
        .bind(lim)
        .fetch_all(pool)
        .await?,
        (None, Some(v)) => sqlx::query_as(
            "SELECT id, action_id, vote, comment, given_at, worker_id \
             FROM operator_feedback WHERE vote = ? \
             ORDER BY given_at DESC LIMIT ?",
        )
        .bind(v.as_str())
        .bind(lim)
        .fetch_all(pool)
        .await?,
        (None, None) => sqlx::query_as(
            "SELECT id, action_id, vote, comment, given_at, worker_id \
             FROM operator_feedback \
             ORDER BY given_at DESC LIMIT ?",
        )
        .bind(lim)
        .fetch_all(pool)
        .await?,
    };
    rows.into_iter()
        .map(|(id, act, v, c, ts, w)| {
            Ok(FeedbackRow {
                id,
                action_id: act,
                vote: Vote::from_str(&v)?,
                comment: c,
                given_at_micros: ts,
                worker_id: w,
            })
        })
        .collect()
}

/// Most-recent vote for one action, if any.
pub async fn latest_vote_for_action(
    pool: &Pool<Sqlite>,
    action_id: &str,
) -> Result<Option<Vote>, FeedbackSqlError> {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT vote FROM operator_feedback \
         WHERE action_id = ? ORDER BY given_at DESC LIMIT 1",
    )
    .bind(action_id)
    .fetch_optional(pool)
    .await?;
    row.map(|(v,)| Vote::from_str(&v)).transpose()
}

/// Pretty-print a microseconds timestamp as RFC-3339.
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
    use sqlx::sqlite::SqlitePoolOptions;

    async fn fresh_pool() -> Pool<Sqlite> {
        let pool = SqlitePoolOptions::new()
            .connect(":memory:")
            .await
            .unwrap();
        for stmt in include_str!("../migrations/24__operator_feedback.sql")
            .split_terminator(';')
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            sqlx::query(stmt).execute(&pool).await.unwrap();
        }
        pool
    }

    #[tokio::test]
    async fn vote_string_round_trips() {
        assert_eq!(Vote::Up.as_str(), "up");
        assert_eq!(Vote::Down.as_str(), "down");
        assert_eq!(Vote::from_str("up").unwrap(), Vote::Up);
        assert_eq!(Vote::from_str("down").unwrap(), Vote::Down);
        assert!(Vote::from_str("maybe").is_err());
    }

    #[tokio::test]
    async fn insert_and_list_returns_inserted_row() {
        let pool = fresh_pool().await;
        let id = insert_feedback(&pool, "act-1", Vote::Up, "great", Some("wkr-1"))
            .await
            .unwrap();
        assert!(id > 0);
        let rows = list_feedback(&pool, None, None, 100).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].action_id, "act-1");
        assert_eq!(rows[0].vote, Vote::Up);
        assert_eq!(rows[0].comment, "great");
        assert_eq!(rows[0].worker_id.as_deref(), Some("wkr-1"));
    }

    #[tokio::test]
    async fn list_filter_by_worker_and_vote_combine() {
        let pool = fresh_pool().await;
        insert_feedback(&pool, "a1", Vote::Up, "", Some("w1")).await.unwrap();
        insert_feedback(&pool, "a2", Vote::Down, "", Some("w1")).await.unwrap();
        insert_feedback(&pool, "a3", Vote::Up, "", Some("w2")).await.unwrap();
        let rows = list_feedback(&pool, Some("w1"), Some(Vote::Down), 100)
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].action_id, "a2");
    }

    #[tokio::test]
    async fn latest_vote_for_action_returns_most_recent() {
        let pool = fresh_pool().await;
        insert_feedback(&pool, "act-1", Vote::Up, "", None).await.unwrap();
        // Sleep is unreliable in tests; use a wide-enough margin
        // by inserting a second row — its given_at is naturally later.
        insert_feedback(&pool, "act-1", Vote::Down, "changed mind", None)
            .await
            .unwrap();
        let v = latest_vote_for_action(&pool, "act-1").await.unwrap();
        assert_eq!(v, Some(Vote::Down));
    }

    #[tokio::test]
    async fn latest_vote_for_unknown_action_returns_none() {
        let pool = fresh_pool().await;
        let v = latest_vote_for_action(&pool, "nope").await.unwrap();
        assert_eq!(v, None);
    }
}
