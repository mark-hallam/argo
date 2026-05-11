//! Operator-to-publisher skill-rating store
//! (Stream `mission-skill-ratings-publisher`, MISSION_BACKLOG Section
//! 3 Claim 4 #80).
//!
//! Today operator thumbs (`argo feedback up|down`) influence a tenant's
//! local skill-learning loop but never propagate beyond the tenant
//! boundary. This module is the storage layer for the cross-tenant
//! wire: one row per (skill_id, skill_version, rater_tenant), keyed so
//! that re-voting replaces the prior row through `INSERT OR REPLACE`
//! on the composite primary key.
//!
//! Three surface functions, intentionally free-standing (mirror
//! [`super::feedback`] / [`super::persona_revision`]) so callers can
//! pass a `&Pool<Sqlite>` and stay away from the
//! `SessionStore::ensure_workspace` pattern when they already have a
//! pool in hand:
//!
//! * [`record_rating`] — write/replace a rating row. Comments are
//!   truncated to 500 chars at the boundary to keep abuse limited
//!   without a separate moderation surface.
//! * [`aggregate_ratings`] — `(up, down, score)` for one
//!   (skill_id, version). `score = up / (up + down)`, with `0.0`
//!   returned for the empty case (no division-by-zero).
//! * [`list_for_publisher`] — every [`RatingSummary`] for a publisher
//!   namespace, one row per (skill_id, version) with rolled-up counts.
//!   Used by the publisher dashboard query that ships in a follow-up.
//!
//! v1 uses a TEXT `publisher_tenant` column rather than the brief's
//! suggested BLOB because there is no first-class "publisher tenant
//! id" today — installed marketplace skills carry only a
//! `provenance.source` of the form `published:<grade>:<publisher>`,
//! and the namespace string after the trust grade is the only stable
//! handle. Migrating to a real `TenantId` once the publisher-side
//! schema lands is a future-work item; the wire here lets ratings
//! accumulate so that follow-up is purely additive.

use argo_types::{ArgoError, ArgoResult, SkillId, TenantId};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Row, Sqlite};

/// Operator vote on a skill — `Up` / `Down`. Stored as `"up"` /
/// `"down"` in sqlite (matching the existing `operator_feedback`
/// column convention so the two surfaces share a vocabulary).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillVote {
    /// Operator approved this skill.
    Up,
    /// Operator rejected this skill.
    Down,
}

impl SkillVote {
    /// String form persisted in sqlite.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Up => "up",
            Self::Down => "down",
        }
    }

    /// Parse the stored sqlite string back into a typed vote.
    ///
    /// Returns [`ArgoError::Provider`] on unknown values — the column
    /// has a CHECK constraint, so this should only fire on a corrupt
    /// or hand-edited DB.
    pub fn parse(s: &str) -> ArgoResult<Self> {
        match s {
            "up" => Ok(Self::Up),
            "down" => Ok(Self::Down),
            other => Err(ArgoError::Provider(format!(
                "skill_rating: unknown vote value '{other}'"
            ))),
        }
    }
}

/// Application-layer ceiling on `comment` length, mirrored at the
/// store boundary so callers don't need to enforce it themselves.
pub const MAX_COMMENT_LEN: usize = 500;

/// One rolled-up `(skill_id, version)` summary for the
/// publisher-dashboard view.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RatingSummary {
    /// Skill UUID, formatted as a string for cross-process transport.
    pub skill_id: String,
    /// Skill version label.
    pub skill_version: String,
    /// Publisher namespace (`provenance.source` suffix on installed
    /// marketplace skills).
    pub publisher_tenant: String,
    /// Number of up-votes across all rater tenants.
    pub up_count: u32,
    /// Number of down-votes across all rater tenants.
    pub down_count: u32,
    /// `up / (up + down)`; `0.0` when there are no votes.
    pub score: f32,
}

/// One full rating row — used by the per-skill `argo skill ratings`
/// surface to show recent comments alongside the rolled-up counts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RatingRow {
    /// Skill UUID, formatted as a string for cross-process transport.
    pub skill_id: String,
    /// Skill version label.
    pub skill_version: String,
    /// Publisher namespace (`provenance.source` suffix on installed
    /// marketplace skills).
    pub publisher_tenant: String,
    /// Rater's tenant id (UUID string).
    pub rater_tenant_id: String,
    /// Vote.
    pub vote: SkillVote,
    /// Microseconds since epoch when the vote was last written.
    pub rated_at_micros: i64,
    /// Operator comment, `None` when omitted at write time.
    pub comment: Option<String>,
}

/// Insert or replace one rating row keyed on
/// (`skill_id`, `skill_version`, `rater_tenant`). Re-voting from the
/// same rater on the same (skill, version) pair updates the row in
/// place, so the count surfaces stay one-vote-per-rater.
///
/// `comment` is truncated to [`MAX_COMMENT_LEN`] chars (counted by
/// UTF-8 char boundaries, not bytes) before insertion.
pub async fn record_rating(
    pool: &Pool<Sqlite>,
    skill_id: SkillId,
    skill_version: &str,
    publisher_tenant: &str,
    rater_tenant: TenantId,
    vote: SkillVote,
    comment: Option<&str>,
) -> ArgoResult<()> {
    if publisher_tenant.is_empty() {
        return Err(ArgoError::Provider(
            "skill_rating: publisher_tenant must not be empty".into(),
        ));
    }
    let trimmed = comment.map(truncate_comment);
    let rated_at = chrono::Utc::now().timestamp_micros();
    sqlx::query(
        "INSERT INTO skill_rating \
            (skill_id, skill_version, publisher_tenant, rater_tenant_id, vote, rated_at, comment) \
         VALUES (?, ?, ?, ?, ?, ?, ?) \
         ON CONFLICT(skill_id, skill_version, rater_tenant_id) DO UPDATE SET \
            publisher_tenant = excluded.publisher_tenant, \
            vote             = excluded.vote, \
            rated_at         = excluded.rated_at, \
            comment          = excluded.comment",
    )
    .bind(skill_id.0.as_bytes().as_slice())
    .bind(skill_version)
    .bind(publisher_tenant)
    .bind(rater_tenant.0.as_bytes().as_slice())
    .bind(vote.as_str())
    .bind(rated_at)
    .bind(trimmed.as_deref())
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("skill_rating record: {e}")))?;
    Ok(())
}

/// Aggregate `(up_count, down_count, score)` for one
/// (`skill_id`, `version`). `score = up / (up + down)`, with `0.0`
/// returned for the empty case so callers don't need to special-case
/// "no votes yet" division-by-zero.
pub async fn aggregate_ratings(
    pool: &Pool<Sqlite>,
    skill_id: SkillId,
    skill_version: &str,
) -> ArgoResult<(u32, u32, f32)> {
    let row = sqlx::query(
        "SELECT \
            SUM(CASE WHEN vote = 'up'   THEN 1 ELSE 0 END) AS up_count, \
            SUM(CASE WHEN vote = 'down' THEN 1 ELSE 0 END) AS down_count \
         FROM skill_rating WHERE skill_id = ? AND skill_version = ?",
    )
    .bind(skill_id.0.as_bytes().as_slice())
    .bind(skill_version)
    .fetch_one(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("skill_rating aggregate: {e}")))?;
    let up: Option<i64> = row.try_get("up_count").ok();
    let down: Option<i64> = row.try_get("down_count").ok();
    let up = u32::try_from(up.unwrap_or(0).max(0)).unwrap_or(0);
    let down = u32::try_from(down.unwrap_or(0).max(0)).unwrap_or(0);
    let total = up + down;
    let score = if total == 0 {
        0.0
    } else {
        f32::from(u16::try_from(up).unwrap_or(u16::MAX))
            / f32::from(u16::try_from(total).unwrap_or(u16::MAX))
    };
    Ok((up, down, score))
}

/// Return one [`RatingSummary`] per `(skill_id, skill_version)` that
/// the given publisher has accumulated votes for. Sorted by total
/// vote volume DESC so the most-rated skills surface first.
pub async fn list_for_publisher(
    pool: &Pool<Sqlite>,
    publisher_tenant: &str,
) -> ArgoResult<Vec<RatingSummary>> {
    let rows = sqlx::query(
        "SELECT \
            skill_id, skill_version, publisher_tenant, \
            SUM(CASE WHEN vote = 'up'   THEN 1 ELSE 0 END) AS up_count, \
            SUM(CASE WHEN vote = 'down' THEN 1 ELSE 0 END) AS down_count \
         FROM skill_rating WHERE publisher_tenant = ? \
         GROUP BY skill_id, skill_version, publisher_tenant \
         ORDER BY (up_count + down_count) DESC, skill_version ASC",
    )
    .bind(publisher_tenant)
    .fetch_all(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("skill_rating list_for_publisher: {e}")))?;
    rows.into_iter().map(decode_summary).collect()
}

/// Most-recent `limit` raw rating rows for one (`skill_id`,
/// `version`). Newest first; comments included verbatim.
pub async fn recent_ratings(
    pool: &Pool<Sqlite>,
    skill_id: SkillId,
    skill_version: &str,
    limit: u32,
) -> ArgoResult<Vec<RatingRow>> {
    let lim = i64::from(limit.clamp(1, 1000));
    let rows = sqlx::query(
        "SELECT skill_id, skill_version, publisher_tenant, rater_tenant_id, \
                vote, rated_at, comment \
         FROM skill_rating \
         WHERE skill_id = ? AND skill_version = ? \
         ORDER BY rated_at DESC LIMIT ?",
    )
    .bind(skill_id.0.as_bytes().as_slice())
    .bind(skill_version)
    .bind(lim)
    .fetch_all(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("skill_rating recent: {e}")))?;
    rows.into_iter().map(decode_row).collect()
}

fn decode_summary(row: sqlx::sqlite::SqliteRow) -> ArgoResult<RatingSummary> {
    let skill_id_bytes: Vec<u8> = row
        .try_get("skill_id")
        .map_err(|e| ArgoError::Provider(format!("skill_rating decode skill_id: {e}")))?;
    let version: String = row
        .try_get("skill_version")
        .map_err(|e| ArgoError::Provider(format!("skill_rating decode version: {e}")))?;
    let publisher: String = row
        .try_get("publisher_tenant")
        .map_err(|e| ArgoError::Provider(format!("skill_rating decode publisher: {e}")))?;
    let up: i64 = row.try_get("up_count").unwrap_or(0);
    let down: i64 = row.try_get("down_count").unwrap_or(0);
    let up = u32::try_from(up.max(0)).unwrap_or(0);
    let down = u32::try_from(down.max(0)).unwrap_or(0);
    let total = up + down;
    let score = if total == 0 {
        0.0
    } else {
        f32::from(u16::try_from(up).unwrap_or(u16::MAX))
            / f32::from(u16::try_from(total).unwrap_or(u16::MAX))
    };
    Ok(RatingSummary {
        skill_id: uuid_from_bytes(&skill_id_bytes)?,
        skill_version: version,
        publisher_tenant: publisher,
        up_count: up,
        down_count: down,
        score,
    })
}

fn decode_row(row: sqlx::sqlite::SqliteRow) -> ArgoResult<RatingRow> {
    let skill_id_bytes: Vec<u8> = row
        .try_get("skill_id")
        .map_err(|e| ArgoError::Provider(format!("skill_rating decode skill_id: {e}")))?;
    let rater_bytes: Vec<u8> = row
        .try_get("rater_tenant_id")
        .map_err(|e| ArgoError::Provider(format!("skill_rating decode rater_tenant: {e}")))?;
    let version: String = row
        .try_get("skill_version")
        .map_err(|e| ArgoError::Provider(format!("skill_rating decode version: {e}")))?;
    let publisher: String = row
        .try_get("publisher_tenant")
        .map_err(|e| ArgoError::Provider(format!("skill_rating decode publisher: {e}")))?;
    let vote_s: String = row
        .try_get("vote")
        .map_err(|e| ArgoError::Provider(format!("skill_rating decode vote: {e}")))?;
    let rated_at: i64 = row
        .try_get("rated_at")
        .map_err(|e| ArgoError::Provider(format!("skill_rating decode rated_at: {e}")))?;
    let comment: Option<String> = row.try_get("comment").ok();
    Ok(RatingRow {
        skill_id: uuid_from_bytes(&skill_id_bytes)?,
        skill_version: version,
        publisher_tenant: publisher,
        rater_tenant_id: uuid_from_bytes(&rater_bytes)?,
        vote: SkillVote::parse(&vote_s)?,
        rated_at_micros: rated_at,
        comment,
    })
}

fn uuid_from_bytes(bytes: &[u8]) -> ArgoResult<String> {
    let arr: [u8; 16] = bytes
        .try_into()
        .map_err(|_| ArgoError::Provider("skill_rating: skill_id bytes != 16".into()))?;
    Ok(uuid::Uuid::from_bytes(arr).to_string())
}

fn truncate_comment(s: &str) -> String {
    if s.chars().count() <= MAX_COMMENT_LEN {
        s.to_string()
    } else {
        s.chars().take(MAX_COMMENT_LEN).collect()
    }
}

#[cfg(test)]
#[path = "skill_rating_tests.rs"]
mod skill_rating_tests;
