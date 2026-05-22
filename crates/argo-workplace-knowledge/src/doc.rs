//! Head-doc CRUD for `workplace_knowledge_doc`.
//!
//! Each call binds `tenant_id` into the WHERE clause so cross-tenant
//! reads or writes cannot succeed even if a caller passes the wrong
//! `slug`. `update` is transactional: it appends a revision row, then
//! bumps `version` + `body_markdown` + `parent_revision_id` on the
//! head in the same SQLite transaction. `delete` removes only the
//! head row; revisions are preserved for audit.
//!
//! IDs are uuid v7 (monotonic + time-prefixed) per SKYCOT convention.
//! Timestamps are stored as i64 microseconds since the Unix epoch.

use argo_types::{ArgoError, ArgoResult, TenantId};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Row, Sqlite};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use crate::revision;

/// The five operator-locked categories for a workplace-knowledge doc
/// (per design doc §3.1). Persisted as lowercase kebab strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum KnowledgeCategory {
    /// Brand voice / tone / style guidance.
    Brand,
    /// Business policies (refund, escalation, SLA, …).
    BusinessRules,
    /// Compliance / legal / privacy obligations.
    Compliance,
    /// Internal operating procedures (not customer-facing).
    Internal,
    /// Frequently-asked questions surfaced to customers.
    Faq,
}

impl KnowledgeCategory {
    /// String form persisted in sqlite.
    #[must_use]
    pub fn as_db_str(self) -> &'static str {
        match self {
            Self::Brand => "brand",
            Self::BusinessRules => "business-rules",
            Self::Compliance => "compliance",
            Self::Internal => "internal",
            Self::Faq => "faq",
        }
    }

    /// Parse the stored sqlite string back into a typed category.
    /// Returns `None` for unknown values so callers can decide
    /// whether to surface them as `Invariant` or skip the row.
    #[must_use]
    pub fn from_db_str(s: &str) -> Option<Self> {
        match s {
            "brand" => Some(Self::Brand),
            "business-rules" => Some(Self::BusinessRules),
            "compliance" => Some(Self::Compliance),
            "internal" => Some(Self::Internal),
            "faq" => Some(Self::Faq),
            _ => None,
        }
    }
}

/// One head row from `workplace_knowledge_doc`. Returned by
/// [`create`], [`update`], [`get`], and [`list`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkplaceKnowledgeDoc {
    /// Stable row id (uuid v7 as TEXT).
    pub id: String,
    /// Tenant that owns this doc.
    pub tenant_id: String,
    /// URL-safe slug, unique per tenant.
    pub slug: String,
    /// Human-readable title.
    pub title: String,
    /// Operator-locked category.
    pub category: KnowledgeCategory,
    /// Markdown body of the current head version.
    pub body_markdown: String,
    /// Monotonic head version, starts at 1.
    pub version: i64,
    /// Id of the revision row that captured the prior body. `None`
    /// for v1 (no parent).
    pub parent_revision_id: Option<String>,
    /// Microseconds since the Unix epoch when the doc was first
    /// created.
    pub created_at_micros: i64,
    /// Principal handle that created the doc.
    pub created_by: String,
    /// Microseconds since the Unix epoch of the most recent update.
    pub updated_at_micros: i64,
    /// Principal handle that authored the most recent update.
    pub updated_by: String,
}

/// Create a new doc at version 1 and return the head row.
///
/// Returns [`ArgoError::Schema`] if `slug`, `title`, or `created_by`
/// is empty. Returns [`ArgoError::Provider`] (mapped from the sqlx
/// UNIQUE-constraint error) when `(tenant_id, slug)` already exists.
pub async fn create(
    pool: &Pool<Sqlite>,
    tenant_id: &TenantId,
    slug: &str,
    title: &str,
    category: KnowledgeCategory,
    body_markdown: &str,
    created_by: &str,
) -> ArgoResult<WorkplaceKnowledgeDoc> {
    if slug.is_empty() {
        return Err(ArgoError::Schema(
            "workplace_knowledge_doc: slug must not be empty".into(),
        ));
    }
    if title.is_empty() {
        return Err(ArgoError::Schema(
            "workplace_knowledge_doc: title must not be empty".into(),
        ));
    }
    if created_by.is_empty() {
        return Err(ArgoError::Schema(
            "workplace_knowledge_doc: created_by must not be empty".into(),
        ));
    }

    let id = Uuid::now_v7().to_string();
    let tenant_str = tenant_id.to_string();
    let now = now_micros()?;

    sqlx::query(
        "INSERT INTO workplace_knowledge_doc \
            (id, tenant_id, slug, title, category, body_markdown, version, \
             parent_revision_id, created_at_micros, created_by, \
             updated_at_micros, updated_by) \
         VALUES (?, ?, ?, ?, ?, ?, 1, NULL, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&tenant_str)
    .bind(slug)
    .bind(title)
    .bind(category.as_db_str())
    .bind(body_markdown)
    .bind(now)
    .bind(created_by)
    .bind(now)
    .bind(created_by)
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("workplace_knowledge_doc insert: {e}")))?;

    Ok(WorkplaceKnowledgeDoc {
        id,
        tenant_id: tenant_str,
        slug: slug.to_string(),
        title: title.to_string(),
        category,
        body_markdown: body_markdown.to_string(),
        version: 1,
        parent_revision_id: None,
        created_at_micros: now,
        created_by: created_by.to_string(),
        updated_at_micros: now,
        updated_by: created_by.to_string(),
    })
}

/// Update the head doc in place: bump `version`, swap
/// `body_markdown`, and append a revision row capturing the prior
/// body. Returns the refreshed head.
///
/// Returns [`ArgoError::NotFound`] when no doc with that `(tenant,
/// slug)` exists. Returns [`ArgoError::Schema`] when `updated_by` is
/// empty.
pub async fn update(
    pool: &Pool<Sqlite>,
    tenant_id: &TenantId,
    slug: &str,
    new_body: &str,
    updated_by: &str,
    edit_reason: Option<&str>,
) -> ArgoResult<WorkplaceKnowledgeDoc> {
    if updated_by.is_empty() {
        return Err(ArgoError::Schema(
            "workplace_knowledge_doc: updated_by must not be empty".into(),
        ));
    }

    let tenant_str = tenant_id.to_string();
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| ArgoError::Provider(format!("workplace_knowledge_doc update tx: {e}")))?;

    let head = sqlx::query(
        "SELECT id, title, category, body_markdown, version, created_at_micros, created_by \
         FROM workplace_knowledge_doc \
         WHERE tenant_id = ? AND slug = ?",
    )
    .bind(&tenant_str)
    .bind(slug)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| ArgoError::Provider(format!("workplace_knowledge_doc update probe: {e}")))?;
    let head = head.ok_or_else(|| {
        ArgoError::NotFound(format!(
            "workplace_knowledge_doc: slug '{slug}' not found for tenant"
        ))
    })?;

    let doc_id: String = head
        .try_get("id")
        .map_err(|e| ArgoError::Provider(format!("workplace_knowledge_doc decode id: {e}")))?;
    let title: String = head
        .try_get("title")
        .map_err(|e| ArgoError::Provider(format!("workplace_knowledge_doc decode title: {e}")))?;
    let category_str: String = head.try_get("category").map_err(|e| {
        ArgoError::Provider(format!("workplace_knowledge_doc decode category: {e}"))
    })?;
    let category = KnowledgeCategory::from_db_str(&category_str).ok_or_else(|| {
        ArgoError::Invariant(format!(
            "workplace_knowledge_doc: unknown category '{category_str}' on slug '{slug}'"
        ))
    })?;
    let prior_body: String = head.try_get("body_markdown").map_err(|e| {
        ArgoError::Provider(format!(
            "workplace_knowledge_doc decode body_markdown: {e}"
        ))
    })?;
    let prior_version: i64 = head.try_get("version").map_err(|e| {
        ArgoError::Provider(format!("workplace_knowledge_doc decode version: {e}"))
    })?;
    let created_at_micros: i64 = head.try_get("created_at_micros").map_err(|e| {
        ArgoError::Provider(format!(
            "workplace_knowledge_doc decode created_at_micros: {e}"
        ))
    })?;
    let created_by: String = head.try_get("created_by").map_err(|e| {
        ArgoError::Provider(format!("workplace_knowledge_doc decode created_by: {e}"))
    })?;

    let now = now_micros()?;
    let revision_id = revision::write(
        &mut tx,
        revision::RevisionInsert {
            tenant_id: &tenant_str,
            doc_id: &doc_id,
            version: prior_version,
            body_markdown: &prior_body,
            diff_from_prior: None,
            edited_at_micros: now,
            edited_by: updated_by,
            edit_reason,
        },
    )
    .await?;

    let new_version = prior_version + 1;
    sqlx::query(
        "UPDATE workplace_knowledge_doc \
         SET body_markdown = ?, version = ?, parent_revision_id = ?, \
             updated_at_micros = ?, updated_by = ? \
         WHERE tenant_id = ? AND slug = ?",
    )
    .bind(new_body)
    .bind(new_version)
    .bind(&revision_id)
    .bind(now)
    .bind(updated_by)
    .bind(&tenant_str)
    .bind(slug)
    .execute(&mut *tx)
    .await
    .map_err(|e| ArgoError::Provider(format!("workplace_knowledge_doc update: {e}")))?;

    tx.commit()
        .await
        .map_err(|e| ArgoError::Provider(format!("workplace_knowledge_doc update commit: {e}")))?;

    Ok(WorkplaceKnowledgeDoc {
        id: doc_id,
        tenant_id: tenant_str,
        slug: slug.to_string(),
        title,
        category,
        body_markdown: new_body.to_string(),
        version: new_version,
        parent_revision_id: Some(revision_id),
        created_at_micros,
        created_by,
        updated_at_micros: now,
        updated_by: updated_by.to_string(),
    })
}

/// Soft delete: remove the head row only. Revisions are preserved
/// so the audit trail remains walkable. Returns `true` if a row was
/// removed, `false` if the slug did not exist for this tenant.
///
/// Audit emission (the `WorkplaceKnowledgeDocDeleted` row) is the
/// caller's responsibility — this layer is the substrate, not the
/// audit boundary (see brief: kind variants live in the sibling
/// `mission-workplace-knowledge-audit-kinds` brief).
pub async fn delete(
    pool: &Pool<Sqlite>,
    tenant_id: &TenantId,
    slug: &str,
) -> ArgoResult<bool> {
    let tenant_str = tenant_id.to_string();
    let res = sqlx::query(
        "DELETE FROM workplace_knowledge_doc WHERE tenant_id = ? AND slug = ?",
    )
    .bind(&tenant_str)
    .bind(slug)
    .execute(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("workplace_knowledge_doc delete: {e}")))?;
    Ok(res.rows_affected() > 0)
}

/// Read the head doc for `(tenant, slug)`. Returns `Ok(None)` if no
/// matching row exists (slug missing OR slug exists in a different
/// tenant — cross-tenant guard).
pub async fn get(
    pool: &Pool<Sqlite>,
    tenant_id: &TenantId,
    slug: &str,
) -> ArgoResult<Option<WorkplaceKnowledgeDoc>> {
    let tenant_str = tenant_id.to_string();
    let row = sqlx::query(SELECT_HEAD_COLUMNS)
        .bind(&tenant_str)
        .bind(slug)
        .fetch_optional(pool)
        .await
        .map_err(|e| ArgoError::Provider(format!("workplace_knowledge_doc get: {e}")))?;
    row.map(decode_head_row).transpose()
}

/// List every head doc for a tenant, ordered by `category ASC, slug
/// ASC` so the context-injection hot path gets a stable order without
/// caller-side sort.
pub async fn list(
    pool: &Pool<Sqlite>,
    tenant_id: &TenantId,
) -> ArgoResult<Vec<WorkplaceKnowledgeDoc>> {
    let tenant_str = tenant_id.to_string();
    let rows = sqlx::query(
        "SELECT id, tenant_id, slug, title, category, body_markdown, version, \
                parent_revision_id, created_at_micros, created_by, \
                updated_at_micros, updated_by \
         FROM workplace_knowledge_doc \
         WHERE tenant_id = ? \
         ORDER BY category ASC, slug ASC",
    )
    .bind(&tenant_str)
    .fetch_all(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("workplace_knowledge_doc list: {e}")))?;
    rows.into_iter().map(decode_head_row).collect()
}

/// `MAX(version)` across the tenant — a cache-key probe used by the
/// context-injection hot path so it can skip the full
/// `list` when the body hasn't moved. Returns 0 when the tenant has
/// no docs at all.
pub async fn max_version_for_tenant(
    pool: &Pool<Sqlite>,
    tenant_id: &TenantId,
) -> ArgoResult<i64> {
    let tenant_str = tenant_id.to_string();
    let max: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(version), 0) FROM workplace_knowledge_doc WHERE tenant_id = ?",
    )
    .bind(&tenant_str)
    .fetch_one(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("workplace_knowledge_doc max_version: {e}")))?;
    Ok(max)
}

const SELECT_HEAD_COLUMNS: &str = "SELECT id, tenant_id, slug, title, category, body_markdown, \
            version, parent_revision_id, created_at_micros, created_by, \
            updated_at_micros, updated_by \
     FROM workplace_knowledge_doc \
     WHERE tenant_id = ? AND slug = ?";

fn decode_head_row(row: sqlx::sqlite::SqliteRow) -> ArgoResult<WorkplaceKnowledgeDoc> {
    let category_str: String = row.try_get("category").map_err(|e| {
        ArgoError::Provider(format!("workplace_knowledge_doc decode category: {e}"))
    })?;
    let category = KnowledgeCategory::from_db_str(&category_str).ok_or_else(|| {
        ArgoError::Invariant(format!(
            "workplace_knowledge_doc: unknown category '{category_str}'"
        ))
    })?;
    Ok(WorkplaceKnowledgeDoc {
        id: row
            .try_get("id")
            .map_err(|e| ArgoError::Provider(format!("decode id: {e}")))?,
        tenant_id: row
            .try_get("tenant_id")
            .map_err(|e| ArgoError::Provider(format!("decode tenant_id: {e}")))?,
        slug: row
            .try_get("slug")
            .map_err(|e| ArgoError::Provider(format!("decode slug: {e}")))?,
        title: row
            .try_get("title")
            .map_err(|e| ArgoError::Provider(format!("decode title: {e}")))?,
        category,
        body_markdown: row
            .try_get("body_markdown")
            .map_err(|e| ArgoError::Provider(format!("decode body_markdown: {e}")))?,
        version: row
            .try_get("version")
            .map_err(|e| ArgoError::Provider(format!("decode version: {e}")))?,
        parent_revision_id: row
            .try_get("parent_revision_id")
            .map_err(|e| ArgoError::Provider(format!("decode parent_revision_id: {e}")))?,
        created_at_micros: row
            .try_get("created_at_micros")
            .map_err(|e| ArgoError::Provider(format!("decode created_at_micros: {e}")))?,
        created_by: row
            .try_get("created_by")
            .map_err(|e| ArgoError::Provider(format!("decode created_by: {e}")))?,
        updated_at_micros: row
            .try_get("updated_at_micros")
            .map_err(|e| ArgoError::Provider(format!("decode updated_at_micros: {e}")))?,
        updated_by: row
            .try_get("updated_by")
            .map_err(|e| ArgoError::Provider(format!("decode updated_by: {e}")))?,
    })
}

fn now_micros() -> ArgoResult<i64> {
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| ArgoError::Invariant(format!("system clock before unix epoch: {e}")))?;
    i64::try_from(dur.as_micros()).map_err(|_| {
        ArgoError::Invariant("system clock past i64 microsecond range".into())
    })
}

#[cfg(test)]
#[path = "doc_tests.rs"]
mod doc_tests;
