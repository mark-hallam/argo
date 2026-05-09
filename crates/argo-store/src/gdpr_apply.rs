#![allow(missing_docs)]
use crate::gdpr::{contains_subject, gdpr_redact_payload, has_table, hexish, parse_array};
use crate::{SessionStore, StoreResult};
use argo_types::ArgoError;
use sqlx::Row;

pub(crate) async fn delete_memory(store: &SessionStore, subject: &str) -> StoreResult<()> {
    sqlx::query(
        "DELETE FROM memory_entry
         WHERE json_valid(body)
           AND lower(COALESCE(json_extract(body, '$.author'), '')) = ?",
    )
    .bind(subject)
    .execute(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("gdpr memory delete: {e}")))?;
    Ok(())
}

pub(crate) async fn delete_skill_candidates(store: &SessionStore, subject: &str) -> StoreResult<()> {
    sqlx::query(
        "DELETE FROM skill
         WHERE lifecycle_state = 'pending_review'
           AND ((json_valid(body) AND lower(COALESCE(json_extract(body, '$.proposed_by'), '')) = ?)
             OR (json_valid(frontmatter) AND lower(COALESCE(json_extract(frontmatter, '$.proposed_by'), '')) = ?)
             OR instr(lower(body), ?) > 0
             OR instr(lower(frontmatter), ?) > 0)",
    )
    .bind(subject)
    .bind(subject)
    .bind(subject)
    .bind(subject)
    .execute(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("gdpr skill delete: {e}")))?;
    Ok(())
}

pub(crate) async fn update_notify_channels(store: &SessionStore, subject: &str) -> StoreResult<()> {
    if !has_table(store, "worker").await? { return Ok(()); }
    let rows = sqlx::query("SELECT id, channels_json FROM worker WHERE instr(lower(channels_json), ?) > 0")
        .bind(subject)
        .fetch_all(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("gdpr channel read: {e}")))?;
    for r in rows {
        let id: String = r.get("id");
        let channels: String = r.get("channels_json");
        if let Some(next) = remove_subject_channels(&channels, subject) {
            sqlx::query("UPDATE worker SET channels_json = ? WHERE id = ?")
                .bind(next)
                .bind(id)
                .execute(store.pool())
                .await
                .map_err(|e| ArgoError::Provider(format!("gdpr channel update: {e}")))?;
        }
    }
    Ok(())
}

pub(crate) async fn redact_worker_refs(
    store: &SessionStore,
    subject: &str,
    replacement: &str,
) -> StoreResult<()> {
    if !has_table(store, "worker").await? { return Ok(()); }
    let rows = sqlx::query("SELECT id, auto_approval_json FROM worker")
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("gdpr worker read: {e}")))?;
    for r in rows {
        let wid: String = r.get("id");
        update_text(store, "worker", "auto_approval_json", "id", &wid, r.get("auto_approval_json"), subject, replacement).await?;
    }
    Ok(())
}

pub(crate) async fn redact_session_payloads(
    store: &SessionStore,
    subject: &str,
    replacement: &str,
) -> StoreResult<()> {
    update_payload_table(store, subject, replacement, "session", &["title", "system_prompt", "summary"]).await?;
    update_payload_table(store, subject, replacement, "message", &["content_json", "tool_calls_json", "provider_meta"]).await?;
    update_payload_table(store, subject, replacement, "tool_call", &["arguments_json", "result_json"]).await?;
    update_payload_table(store, subject, replacement, "handoff", &["to", "expected_outcome", "payload"]).await
}

async fn update_payload_table(
    store: &SessionStore,
    subject: &str,
    replacement: &str,
    table: &'static str,
    columns: &[&'static str],
) -> StoreResult<()> {
    if !has_table(store, table).await? { return Ok(()); }
    let id_col = "id";
    let sql = format!("SELECT * FROM {table}");
    let rows = sqlx::query(&sql)
        .fetch_all(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("gdpr {table} read: {e}")))?;
    for r in rows {
        let blob = matches!(table, "session" | "message" | "tool_call");
        let id = if blob {
            hexish(&r.get::<Vec<u8>, _>(id_col))
        } else {
            r.get::<String, _>(id_col)
        };
        let pred = if blob { "lower(hex(id))" } else { "id" };
        for col in columns {
            if let Ok(Some(value)) = r.try_get::<Option<String>, _>(*col) {
                update_text(store, table, col, pred, &id, value, subject, replacement).await?;
            } else if let Ok(value) = r.try_get::<String, _>(*col) {
                update_text(store, table, col, pred, &id, value, subject, replacement).await?;
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn update_text(
    store: &SessionStore,
    table: &'static str,
    column: &'static str,
    id_predicate: &'static str,
    id: &str,
    value: String,
    subject: &str,
    replacement: &str,
) -> StoreResult<()> {
    if !contains_subject(&value, subject) {
        return Ok(());
    }
    let next = gdpr_redact_payload(&value, subject, replacement);
    let sql = format!("UPDATE {table} SET \"{column}\" = ? WHERE {id_predicate} = ?");
    sqlx::query(&sql)
        .bind(next)
        .bind(id)
        .execute(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("gdpr {table}.{column} update: {e}")))?;
    Ok(())
}

fn remove_subject_channels(input: &str, subject: &str) -> Option<String> {
    let mut items = parse_array(input)?;
    let before = items.len();
    items.retain(|v| {
        !v.get("target")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|target| target.eq_ignore_ascii_case(subject))
    });
    (items.len() != before).then(|| serde_json::to_string(&items).unwrap_or_else(|_| "[]".into()))
}
