#![allow(missing_docs)]
use crate::{SessionStore, StoreResult};
use argo_types::ArgoError;
use serde::Serialize;
use serde_json::{json, Value};
use sqlx::Row;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct GdprCounts {
    pub memory_rows: u64,
    pub audit_records: u64,
    pub skill_candidates: u64,
    pub notify_channels: u64,
    pub worker_operator_refs: u64,
    pub session_payloads: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct GdprRecord {
    pub table: String,
    pub id: String,
    pub payload: Value,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct GdprPlan {
    pub counts: GdprCounts,
    pub records: Vec<GdprRecord>,
}

impl SessionStore {
    pub async fn gdpr_plan(&self, subject: &str) -> StoreResult<GdprPlan> {
        let subject = normalize_subject(subject);
        let mut plan = GdprPlan::default();
        collect_memory(self, &subject, &mut plan).await?;
        collect_audit(self, &subject, &mut plan).await?;
        collect_skill_candidates(self, &subject, &mut plan).await?;
        collect_notify_channels(self, &subject, &mut plan).await?;
        collect_worker_refs(self, &subject, &mut plan).await?;
        collect_session_payloads(self, &subject, &mut plan).await?;
        Ok(plan)
    }

    pub async fn gdpr_apply_non_audit(
        &self,
        subject: &str,
        replacement: &str,
    ) -> StoreResult<GdprCounts> {
        let subject = normalize_subject(subject);
        let plan = self.gdpr_plan(&subject).await?;
        delete_memory(self, &subject).await?;
        delete_skill_candidates(self, &subject).await?;
        update_notify_channels(self, &subject).await?;
        redact_worker_refs(self, &subject, replacement).await?;
        redact_session_payloads(self, &subject, replacement).await?;
        Ok(GdprCounts {
            audit_records: 0,
            ..plan.counts
        })
    }
}

#[must_use]
pub fn gdpr_redact_payload(input: &str, subject: &str, replacement: &str) -> String {
    match serde_json::from_str::<Value>(input) {
        Ok(mut value) => {
            redact_value(&mut value, subject, replacement);
            serde_json::to_string(&value).unwrap_or_else(|_| redact_text(input, subject, replacement))
        }
        Err(_) => redact_text(input, subject, replacement),
    }
}

fn normalize_subject(subject: &str) -> String { subject.trim().to_ascii_lowercase() }

async fn has_table(store: &SessionStore, table: &str) -> StoreResult<bool> {
    let found: Option<i64> = sqlx::query_scalar("SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?")
        .bind(table)
        .fetch_optional(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("gdpr schema probe: {e}")))?;
    Ok(found.is_some())
}

async fn collect_memory(store: &SessionStore, subject: &str, plan: &mut GdprPlan) -> StoreResult<()> {
    let rows = sqlx::query(
        "SELECT id, file, body, learned_at FROM memory_entry
         WHERE json_valid(body)
           AND lower(COALESCE(json_extract(body, '$.author'), '')) = ?",
    )
    .bind(subject)
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("gdpr memory plan: {e}")))?;
    plan.counts.memory_rows = rows.len() as u64;
    for r in rows {
        plan.records.push(record(
            "memory_entry",
            blob_id(&r.get::<Vec<u8>, _>("id")),
            json!({"file": r.get::<String, _>("file"), "body": r.get::<String, _>("body"), "learned_at": r.get::<i64, _>("learned_at")}),
        ));
    }
    Ok(())
}

async fn collect_audit(store: &SessionStore, subject: &str, plan: &mut GdprPlan) -> StoreResult<()> {
    let rows = sqlx::query(
        "SELECT seq, kind, payload_json, created_at FROM audit_record
         WHERE instr(lower(payload_json), ?) > 0 ORDER BY seq ASC",
    )
    .bind(subject)
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("gdpr audit plan: {e}")))?;
    plan.counts.audit_records = rows.len() as u64;
    for r in rows {
        plan.records.push(record(
            "audit_record",
            r.get::<i64, _>("seq").to_string(),
            json!({"kind": r.get::<String, _>("kind"), "payload_json": r.get::<String, _>("payload_json"), "created_at": r.get::<i64, _>("created_at")}),
        ));
    }
    Ok(())
}

async fn collect_skill_candidates(
    store: &SessionStore,
    subject: &str,
    plan: &mut GdprPlan,
) -> StoreResult<()> {
    let rows = sqlx::query(
        "SELECT id, name, version, body, frontmatter FROM skill
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
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("gdpr skill plan: {e}")))?;
    plan.counts.skill_candidates = rows.len() as u64;
    for r in rows {
        plan.records.push(record(
            "skill_candidate",
            blob_id(&r.get::<Vec<u8>, _>("id")),
            json!({"name": r.get::<String, _>("name"), "version": r.get::<String, _>("version"), "body": r.get::<String, _>("body"), "frontmatter": r.get::<String, _>("frontmatter")}),
        ));
    }
    Ok(())
}

async fn collect_notify_channels(
    store: &SessionStore,
    subject: &str,
    plan: &mut GdprPlan,
) -> StoreResult<()> {
    if !has_table(store, "worker").await? { return Ok(()); }
    let rows = sqlx::query(
        "SELECT id, channels_json FROM worker WHERE instr(lower(channels_json), ?) > 0",
    )
    .bind(subject)
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("gdpr channels plan: {e}")))?;
    for r in rows {
        let channels: String = r.get("channels_json");
        let removed = count_subject_channels(&channels, subject);
        if removed > 0 {
            plan.counts.notify_channels += removed;
            plan.records.push(record(
                "notify_channel",
                r.get::<String, _>("id"),
                json!({"channels_json": channels, "removed": removed}),
            ));
        }
    }
    Ok(())
}

async fn collect_worker_refs(
    store: &SessionStore,
    subject: &str,
    plan: &mut GdprPlan,
) -> StoreResult<()> {
    if !has_table(store, "worker").await? { return Ok(()); }
    let rows = sqlx::query("SELECT id, auto_approval_json FROM worker")
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("gdpr worker plan: {e}")))?;
    for r in rows {
        let auto: String = r.get("auto_approval_json");
        if contains_subject(&auto, subject) {
            plan.counts.worker_operator_refs += 1;
            plan.records.push(record(
                "worker",
                r.get::<String, _>("id"),
                json!({"auto_approval_json": auto}),
            ));
        }
    }
    Ok(())
}

async fn collect_session_payloads(
    store: &SessionStore,
    subject: &str,
    plan: &mut GdprPlan,
) -> StoreResult<()> {
    collect_text_rows(store, subject, plan, "session", &["title", "system_prompt", "summary"]).await?;
    collect_text_rows(store, subject, plan, "message", &["content_json", "tool_calls_json", "provider_meta"]).await?;
    collect_text_rows(store, subject, plan, "tool_call", &["arguments_json", "result_json"]).await?;
    collect_text_rows(store, subject, plan, "handoff", &["to", "expected_outcome", "payload"]).await?;
    Ok(())
}

async fn collect_text_rows(
    store: &SessionStore,
    subject: &str,
    plan: &mut GdprPlan,
    table: &'static str,
    columns: &[&'static str],
) -> StoreResult<()> {
    if !has_table(store, table).await? { return Ok(()); }
    let expr = columns
        .iter()
        .map(|c| format!("instr(lower(COALESCE(\"{c}\", '')), ?) > 0"))
        .collect::<Vec<_>>()
        .join(" OR ");
    let sql = format!("SELECT * FROM {table} WHERE {expr}");
    let mut q = sqlx::query(&sql);
    for _ in columns {
        q = q.bind(subject);
    }
    let rows = q
        .fetch_all(store.pool())
        .await
        .map_err(|e| ArgoError::Provider(format!("gdpr {table} plan: {e}")))?;
    plan.counts.session_payloads += rows.len() as u64;
    for r in rows {
        let id = if table == "session" || table == "message" || table == "tool_call" {
            blob_id(&r.get::<Vec<u8>, _>("id"))
        } else {
            r.get::<String, _>("id")
        };
        let mut payload = serde_json::Map::new();
        for c in columns {
            let v: Option<String> = r.try_get(*c).ok().flatten();
            payload.insert((*c).to_string(), json!(v));
        }
        plan.records.push(record(table, id, Value::Object(payload)));
    }
    Ok(())
}

async fn delete_memory(store: &SessionStore, subject: &str) -> StoreResult<()> {
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

async fn delete_skill_candidates(store: &SessionStore, subject: &str) -> StoreResult<()> {
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

async fn update_notify_channels(store: &SessionStore, subject: &str) -> StoreResult<()> {
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

async fn redact_worker_refs(
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

async fn redact_session_payloads(
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

fn count_subject_channels(input: &str, subject: &str) -> u64 {
    parse_array(input)
        .map(|items| {
            items
                .iter()
                .filter(|v| {
                    v.get("target")
                        .and_then(Value::as_str)
                        .is_some_and(|target| target.eq_ignore_ascii_case(subject))
                })
                .count() as u64
        })
        .unwrap_or(0)
}

fn remove_subject_channels(input: &str, subject: &str) -> Option<String> {
    let mut items = parse_array(input)?;
    let before = items.len();
    items.retain(|v| {
        !v.get("target")
            .and_then(Value::as_str)
            .is_some_and(|target| target.eq_ignore_ascii_case(subject))
    });
    (items.len() != before).then(|| serde_json::to_string(&items).unwrap_or_else(|_| "[]".into()))
}

fn parse_array(input: &str) -> Option<Vec<Value>> { serde_json::from_str::<Vec<Value>>(input).ok() }

fn contains_subject(value: &str, subject: &str) -> bool { value.to_ascii_lowercase().contains(subject) }

fn redact_value(value: &mut Value, subject: &str, replacement: &str) {
    match value {
        Value::String(s) => *s = redact_text(s, subject, replacement),
        Value::Array(items) => {
            for item in items {
                redact_value(item, subject, replacement);
            }
        }
        Value::Object(map) => {
            for item in map.values_mut() {
                redact_value(item, subject, replacement);
            }
        }
        _ => {}
    }
}

fn redact_text(input: &str, subject: &str, replacement: &str) -> String {
    let lower = input.to_ascii_lowercase();
    let mut out = String::new();
    let mut cursor = 0;
    while let Some(pos) = lower[cursor..].find(subject) {
        let start = cursor + pos;
        out.push_str(&input[cursor..start]);
        out.push_str(replacement);
        cursor = start + subject.len();
    }
    out.push_str(&input[cursor..]);
    out
}

fn blob_id(id: &[u8]) -> String { hexish(id) }

fn hexish(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

fn record(table: &str, id: String, payload: Value) -> GdprRecord { GdprRecord { table: table.into(), id, payload } }
