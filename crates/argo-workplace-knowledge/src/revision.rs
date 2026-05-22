//! Revision-history accessors for `workplace_knowledge_revision`.
//!
//! Append happens inside [`crate::doc::update`] via the
//! `pub(crate)` [`write`] helper — keeping the write path internal
//! prevents callers from inserting a revision without bumping the
//! head doc and vice versa. Read access is `pub` so a future gateway
//! brief can surface the diff timeline.

use argo_types::{ArgoError, ArgoResult, TenantId};
use serde::{Deserialize, Serialize};
use sqlx::{Row, Sqlite, Transaction};
use uuid::Uuid;

/// One row from `workplace_knowledge_revision`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkplaceKnowledgeRevision {
    /// Stable row id (uuid v7 as TEXT).
    pub id: String,
    /// Foreign key into `workplace_knowledge_doc.id`.
    pub doc_id: String,
    /// Tenant that owns the parent doc — redundant with the doc row
    /// but indexed here so cross-tenant guards work without a JOIN.
    pub tenant_id: String,
    /// Doc version this revision captured (i.e. the value of
    /// `workplace_knowledge_doc.version` *before* the update).
    pub version: i64,
    /// Body snapshot that was head when this revision was taken.
    pub body_markdown: String,
    /// Optional unified-diff snippet from the prior revision; the
    /// substrate stores callers' diffs verbatim and does not compute
    /// them itself.
    pub diff_from_prior: Option<String>,
    /// Microseconds since the Unix epoch the revision was written.
    pub edited_at_micros: i64,
    /// Principal handle that authored the edit which triggered this
    /// revision.
    pub edited_by: String,
    /// Optional caller-supplied reason ("clarified scope", "typo
    /// fix"). Surfaces in the edit timeline.
    pub edit_reason: Option<String>,
}

/// Bundle of borrowed fields for [`write`] — keeps the insert
/// helper at two arguments (transaction + payload) so it stays under
/// the `clippy::too_many_arguments` ceiling.
pub(crate) struct RevisionInsert<'a> {
    pub tenant_id: &'a str,
    pub doc_id: &'a str,
    pub version: i64,
    pub body_markdown: &'a str,
    pub diff_from_prior: Option<&'a str>,
    pub edited_at_micros: i64,
    pub edited_by: &'a str,
    pub edit_reason: Option<&'a str>,
}

/// Append a revision row inside an already-open transaction. Used by
/// [`crate::doc::update`] to snapshot the prior body atomically with
/// the head update.
///
/// Returns the freshly-generated revision id so the head row can
/// stamp it as `parent_revision_id`.
pub(crate) async fn write<'c>(
    tx: &mut Transaction<'c, Sqlite>,
    insert: RevisionInsert<'_>,
) -> ArgoResult<String> {
    let id = Uuid::now_v7().to_string();
    sqlx::query(
        "INSERT INTO workplace_knowledge_revision \
            (id, doc_id, tenant_id, version, body_markdown, diff_from_prior, \
             edited_at_micros, edited_by, edit_reason) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(insert.doc_id)
    .bind(insert.tenant_id)
    .bind(insert.version)
    .bind(insert.body_markdown)
    .bind(insert.diff_from_prior)
    .bind(insert.edited_at_micros)
    .bind(insert.edited_by)
    .bind(insert.edit_reason)
    .execute(&mut **tx)
    .await
    .map_err(|e| ArgoError::Provider(format!("workplace_knowledge_revision insert: {e}")))?;
    Ok(id)
}

/// List every revision for a doc, newest first (matches the
/// `idx_workplace_knowledge_revision_doc (doc_id, version DESC)`
/// index so the query plan stays cheap).
pub async fn list_for_doc(
    pool: &sqlx::Pool<Sqlite>,
    tenant_id: &TenantId,
    doc_id: &str,
) -> ArgoResult<Vec<WorkplaceKnowledgeRevision>> {
    let tenant_str = tenant_id.to_string();
    let rows = sqlx::query(
        "SELECT id, doc_id, tenant_id, version, body_markdown, diff_from_prior, \
                edited_at_micros, edited_by, edit_reason \
         FROM workplace_knowledge_revision \
         WHERE tenant_id = ? AND doc_id = ? \
         ORDER BY version DESC",
    )
    .bind(&tenant_str)
    .bind(doc_id)
    .fetch_all(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("workplace_knowledge_revision list: {e}")))?;
    rows.into_iter().map(decode_row).collect()
}

/// Fetch one revision by id, scoped to the tenant. Returns
/// `Ok(None)` when no matching row exists (id missing OR revision
/// belongs to a different tenant).
pub async fn get(
    pool: &sqlx::Pool<Sqlite>,
    tenant_id: &TenantId,
    revision_id: &str,
) -> ArgoResult<Option<WorkplaceKnowledgeRevision>> {
    let tenant_str = tenant_id.to_string();
    let row = sqlx::query(
        "SELECT id, doc_id, tenant_id, version, body_markdown, diff_from_prior, \
                edited_at_micros, edited_by, edit_reason \
         FROM workplace_knowledge_revision \
         WHERE tenant_id = ? AND id = ?",
    )
    .bind(&tenant_str)
    .bind(revision_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| ArgoError::Provider(format!("workplace_knowledge_revision get: {e}")))?;
    row.map(decode_row).transpose()
}

fn decode_row(row: sqlx::sqlite::SqliteRow) -> ArgoResult<WorkplaceKnowledgeRevision> {
    Ok(WorkplaceKnowledgeRevision {
        id: row
            .try_get("id")
            .map_err(|e| ArgoError::Provider(format!("decode id: {e}")))?,
        doc_id: row
            .try_get("doc_id")
            .map_err(|e| ArgoError::Provider(format!("decode doc_id: {e}")))?,
        tenant_id: row
            .try_get("tenant_id")
            .map_err(|e| ArgoError::Provider(format!("decode tenant_id: {e}")))?,
        version: row
            .try_get("version")
            .map_err(|e| ArgoError::Provider(format!("decode version: {e}")))?,
        body_markdown: row
            .try_get("body_markdown")
            .map_err(|e| ArgoError::Provider(format!("decode body_markdown: {e}")))?,
        diff_from_prior: row
            .try_get("diff_from_prior")
            .map_err(|e| ArgoError::Provider(format!("decode diff_from_prior: {e}")))?,
        edited_at_micros: row
            .try_get("edited_at_micros")
            .map_err(|e| ArgoError::Provider(format!("decode edited_at_micros: {e}")))?,
        edited_by: row
            .try_get("edited_by")
            .map_err(|e| ArgoError::Provider(format!("decode edited_by: {e}")))?,
        edit_reason: row
            .try_get("edit_reason")
            .map_err(|e| ArgoError::Provider(format!("decode edit_reason: {e}")))?,
    })
}

#[cfg(test)]
#[path = "revision_tests.rs"]
mod revision_tests;
