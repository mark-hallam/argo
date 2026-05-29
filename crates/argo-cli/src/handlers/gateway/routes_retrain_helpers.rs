//! Support helpers for `routes_retrain` — split out for the 500-LOC
//! ceiling. Contains the keyword classifier, conflict scan,
//! before-snapshot capture, memory-fact apply/revert, and the
//! audit-table queries that hydrate `RetrainRow` for `undo` /
//! `reverse-test`.

use super::{bad_request, server_error, Resp, WorkerCtx, COMMIT_KEY, UNDO_KEY};
use argo_identity::MemoryScope;
use argo_memory::{MemoryFile, MemoryStore, PrincipalOverride};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

pub(super) const FACT_TOMBSTONE: &str = "\u{0007}skycot-fact-tombstone\u{0007}";

pub(super) struct ClassifyOutcome {
    pub(super) classification: String,
    pub(super) plain_english: String,
    pub(super) structural_change: Value,
    pub(super) needs_generalization: Option<Value>,
}
pub(super) fn classify_input(input: &str) -> ClassifyOutcome {
    let lower = input.to_ascii_lowercase();
    if let Some((from, to)) = detect_vocabulary_patch(&lower) {
        return outcome(
            "vocabulary_patch",
            format!("Replace '{from}' with '{to}'."),
            json!({ "from": from, "to": to }),
            None,
        );
    }
    if let Some(banned) = detect_action_ban(&lower) {
        return outcome(
            "action_ban",
            format!("Ban action '{banned}'."),
            json!({ "action_type": banned }),
            None,
        );
    }
    if matches_any(
        &lower,
        &[
            "always ",
            "must ",
            "must not ",
            "before ",
            "require ",
            "only when ",
        ],
    ) {
        return outcome(
            "policy_rule",
            format!("Add policy rule: {}", trim_for_display(input)),
            json!({ "predicate_json": { "raw": input }, "action_type": guess_action_type(&lower) }),
            None,
        );
    }
    if matches_any(
        &lower,
        &["you got it wrong", "next time", "incorrect", "wrong answer"],
    ) {
        return outcome(
            "skill_correction",
            format!("Correct skill output: {}", trim_for_display(input)),
            json!({ "note": input }),
            Some(json!({
                "question": "Apply to all similar inputs or only this exact case?",
                "options": ["all similar", "this case only"],
            })),
        );
    }
    outcome(
        "memory_fact",
        format!("Remember: {}", trim_for_display(input)),
        json!({ "text": input }),
        None,
    )
}
fn outcome(c: &str, p: String, s: Value, g: Option<Value>) -> ClassifyOutcome {
    ClassifyOutcome {
        classification: c.into(),
        plain_english: p,
        structural_change: s,
        needs_generalization: g,
    }
}
fn matches_any(lower: &str, keys: &[&str]) -> bool {
    keys.iter().any(|k| lower.contains(k))
}
fn detect_vocabulary_patch(lower: &str) -> Option<(String, String)> {
    let idx = lower.find(" instead of ")?;
    let head = &lower[..idx];
    let new_word = head.rsplit_once(' ').map_or(head, |(_, w)| w);
    let tail = &lower[idx + " instead of ".len()..];
    let old_word = tail.split([' ', '.', ',']).next().unwrap_or(tail);
    (!new_word.is_empty() && !old_word.is_empty())
        .then(|| (old_word.to_string(), new_word.to_string()))
}

fn detect_action_ban(lower: &str) -> Option<String> {
    for prefix in ["stop doing ", "don't ever ", "never "] {
        if let Some(idx) = lower.find(prefix) {
            let tail = &lower[idx + prefix.len()..];
            let first = tail.split([' ', '.', ',']).next().unwrap_or(tail);
            if !first.is_empty() {
                return Some(first.to_string());
            }
        }
    }
    None
}

fn guess_action_type(lower: &str) -> String {
    for kw in [
        "send_email",
        "send_sms",
        "send",
        "post",
        "pay",
        "refund",
        "schedule",
    ] {
        if lower.contains(kw) {
            return kw.into();
        }
    }
    "unspecified".into()
}

fn trim_for_display(input: &str) -> String {
    let s = input.trim();
    if s.len() <= 120 {
        s.into()
    } else {
        format!("{}…", s.chars().take(117).collect::<String>())
    }
}
pub(super) async fn find_conflicts(
    store: &Arc<SessionStore>,
    ctx: &WorkerCtx,
    outcome: &ClassifyOutcome,
    input: &str,
) -> ArgoResult<Vec<Value>> {
    if !matches!(
        outcome.classification.as_str(),
        "memory_fact" | "vocabulary_patch"
    ) {
        return Ok(vec![]);
    }
    let lower = input.to_ascii_lowercase();
    let words: Vec<&str> = lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() >= 4)
        .take(8)
        .collect();
    if words.is_empty() {
        return Ok(vec![]);
    }
    let ns = &ctx.worker.memory_namespace;
    let rows: Vec<(Vec<u8>, String)> = sqlx::query_as(
        "SELECT id, body FROM memory_entry \
         WHERE tenant_id = ? AND worker_id = ? AND memory_scope = ? \
           AND file = ? AND superseded_by IS NULL AND body != ? \
         ORDER BY learned_at DESC LIMIT 100",
    )
    .bind(ns.tenant_id.0.as_bytes().as_slice())
    .bind(ns.worker_id.0.as_bytes().as_slice())
    .bind(scope_label(ns.scope))
    .bind(MemoryFile::Memory.filename())
    .bind(FACT_TOMBSTONE)
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("conflict scan: {e}")))?;
    let mut conflicts = Vec::new();
    for (id_bytes, body) in rows {
        let lb = body.to_ascii_lowercase();
        let overlaps = words.iter().filter(|w| lb.contains(*w)).count();
        if overlaps >= 2 {
            let id = Uuid::from_slice(&id_bytes)
                .map_err(|e| ArgoError::Provider(format!("fact id: {e}")))?;
            conflicts.push(json!({ "fact_id": id.to_string(), "existing_text": body, "overlap_words": overlaps }));
            if conflicts.len() >= 5 {
                break;
            }
        }
    }
    Ok(conflicts)
}

pub(super) fn capture_snapshot(ctx: &WorkerCtx, classification: &str, sc: &Value) -> Value {
    match classification {
        "memory_fact" => json!({ "kind": "memory_fact" }),
        "policy_rule" => json!({ "kind": "policy_rule", "rule_intent": sc }),
        "voice_profile_edit" => json!({
            "kind": "voice_profile_edit",
            "persona": serde_json::to_value(&ctx.worker.persona).unwrap_or(Value::Null),
        }),
        "vocabulary_patch" => json!({ "kind": "vocabulary_patch" }),
        "action_ban" => json!({
            "kind": "action_ban",
            "action_type": sc.get("action_type").cloned().unwrap_or(Value::Null),
        }),
        _ => Value::Null,
    }
}
pub(super) async fn apply_memory_fact(
    store: &Arc<SessionStore>,
    ctx: &WorkerCtx,
    sc: &Value,
) -> Result<(), Resp> {
    let text = sc
        .get("text")
        .and_then(Value::as_str)
        .ok_or_else(|| bad_request("memory_fact requires `structural_change.text`"))?
        .trim()
        .to_string();
    if text.is_empty() {
        return Err(bad_request("memory_fact text is empty"));
    }
    let workspace = crate::home::cwd_workspace(store)
        .await
        .map_err(|e| server_error(&format!("cwd_workspace: {e}")))?;
    MemoryStore::new(Arc::clone(store))
        .append_in_namespace(
            &ctx.worker.memory_namespace,
            workspace,
            MemoryFile::Memory,
            text,
            None,
        )
        .await
        .map(|_| ())
        .map_err(|e| server_error(&format!("memory append: {e}")))
}
pub(super) async fn revert_memory_fact(
    store: &Arc<SessionStore>,
    ctx: &WorkerCtx,
    row: &RetrainRow,
) -> Result<(), Resp> {
    let Some(text) = row.structural_change.get("text").and_then(Value::as_str) else {
        return Ok(());
    };
    let bytes: Option<Vec<u8>> = sqlx::query_scalar(
        "SELECT id FROM memory_entry \
         WHERE tenant_id = ? AND worker_id = ? AND memory_scope = ? \
           AND body = ? AND superseded_by IS NULL \
         ORDER BY learned_at DESC LIMIT 1",
    )
    .bind(ctx.tenant.0.as_bytes().as_slice())
    .bind(ctx.worker.id.0.as_bytes().as_slice())
    .bind(scope_label(ctx.worker.memory_namespace.scope))
    .bind(text)
    .fetch_optional(store.pool())
    .await
    .map_err(|e| server_error(&format!("fact lookup: {e}")))?;
    let Some(b) = bytes else { return Ok(()) };
    let id = Uuid::from_slice(&b).map_err(|e| server_error(&format!("fact id: {e}")))?;
    MemoryStore::new(Arc::clone(store))
        .supersede(id, FACT_TOMBSTONE.into(), None, PrincipalOverride::Agent)
        .await
        .map(|_| ())
        .map_err(|e| server_error(&format!("fact supersede: {e}")))
}

pub(super) struct RetrainRow {
    pub(super) classification: String,
    pub(super) structural_change: Value,
    pub(super) applied_at: DateTime<Utc>,
    pub(super) is_undone: bool,
}

pub(super) async fn find_retrain_row(
    store: &Arc<SessionStore>,
    ctx: &WorkerCtx,
    retrain_id: Uuid,
) -> ArgoResult<Option<RetrainRow>> {
    let row: Option<(String, String, String)> = sqlx::query_as(
        "SELECT json_extract(payload_json, '$.classification'), \
                json_extract(payload_json, '$.structural_change'), \
                json_extract(payload_json, '$.applied_at') \
         FROM audit_record WHERE kind = 'http.request' \
           AND tenant_id = ? \
           AND json_extract(payload_json, '$.endpoint') = ? \
           AND json_extract(payload_json, '$.retrain_id') = ? \
           AND json_extract(payload_json, '$.worker_id') = ? \
         LIMIT 1",
    )
    .bind(ctx.tenant.to_string())
    .bind(COMMIT_KEY)
    .bind(retrain_id.to_string())
    .bind(ctx.worker.id.to_string())
    .fetch_optional(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("retrain row probe: {e}")))?;
    let Some((classification, sc_raw, applied_raw)) = row else {
        return Ok(None);
    };
    let structural_change: Value = serde_json::from_str(&sc_raw).unwrap_or(Value::Null);
    let applied_at = DateTime::parse_from_rfc3339(&applied_raw)
        .map_err(|e| ArgoError::Provider(format!("applied_at parse: {e}")))?
        .with_timezone(&Utc);
    let undone: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_record WHERE kind = 'http.request' \
           AND tenant_id = ? \
           AND json_extract(payload_json, '$.endpoint') = ? \
           AND json_extract(payload_json, '$.retrain_id') = ? \
           AND json_extract(payload_json, '$.worker_id') = ?",
    )
    .bind(ctx.tenant.to_string())
    .bind(UNDO_KEY)
    .bind(retrain_id.to_string())
    .bind(ctx.worker.id.to_string())
    .fetch_one(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("undo probe: {e}")))?;
    Ok(Some(RetrainRow {
        classification,
        structural_change,
        applied_at,
        is_undone: undone > 0,
    }))
}

pub(super) async fn build_reverse_samples(
    store: &Arc<SessionStore>,
    ctx: &WorkerCtx,
    row: &RetrainRow,
    sample_size: u32,
) -> Vec<Value> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT COALESCE(json_extract(payload_json, '$.tool_name'), ''), payload_json \
         FROM audit_record WHERE kind = 'tool_call' \
           AND tenant_id = ? \
           AND json_extract(payload_json, '$.worker_id') = ? \
         ORDER BY seq DESC LIMIT ?",
    )
    .bind(ctx.tenant.to_string())
    .bind(ctx.worker.id.to_string())
    .bind(i64::from(sample_size).max(1))
    .fetch_all(store.pool())
    .await
    .unwrap_or_default();
    rows.into_iter()
        .take(sample_size as usize)
        .map(|(tn, payload)| {
            let tool_name = if tn.is_empty() { "unknown".into() } else { tn };
            let outcome = match row.classification.as_str() {
                "action_ban" if tool_name.contains("send") || tool_name.contains("post") => {
                    "blocked"
                }
                "policy_rule" => "review",
                _ => "safe",
            };
            json!({
                "inbound_context": { "tool_name": tool_name, "raw_payload": payload },
                "before_output": { "action": "unchanged", "tool_name": tool_name },
                "after_output": { "action": outcome, "tool_name": tool_name },
                "outcome": outcome,
            })
        })
        .collect()
}

fn scope_label(scope: MemoryScope) -> &'static str {
    match scope {
        MemoryScope::WorkerSelf => "worker_self",
        MemoryScope::Role => "role",
        MemoryScope::SharedTeam => "shared_team",
    }
}

/// One committed retrain, hydrated from the audit log for the
/// `GET /retrain/recent` panel. `undone_at` is `Some` when a matching
/// `worker.retrain.undone` row exists for the same retrain id.
pub(super) struct RecentRetrain {
    pub(super) retrain_id: String,
    pub(super) classification: String,
    pub(super) structural_change: Value,
    pub(super) applied_at: DateTime<Utc>,
    pub(super) undone_at: Option<DateTime<Utc>>,
}

/// Lists a worker's committed retrains newest-first (capped at
/// `limit`), joining each to its undo marker when present. Reads the
/// audit log — the canonical retrain history written by `commit` /
/// `undo` — so no parallel `retrain_record` table is introduced.
pub(super) async fn recent_retrain_rows(
    store: &Arc<SessionStore>,
    ctx: &WorkerCtx,
    limit: i64,
) -> ArgoResult<Vec<RecentRetrain>> {
    type OptRow = (Option<String>, Option<String>, Option<String>, Option<String>);
    let commits: Vec<OptRow> = sqlx::query_as(
        "SELECT json_extract(payload_json, '$.retrain_id'), \
                json_extract(payload_json, '$.classification'), \
                json_extract(payload_json, '$.structural_change'), \
                json_extract(payload_json, '$.applied_at') \
         FROM audit_record WHERE kind = 'http.request' \
           AND tenant_id = ? \
           AND json_extract(payload_json, '$.endpoint') = ? \
           AND json_extract(payload_json, '$.worker_id') = ? \
         ORDER BY seq DESC LIMIT ?",
    )
    .bind(ctx.tenant.to_string())
    .bind(COMMIT_KEY)
    .bind(ctx.worker.id.to_string())
    .bind(limit.clamp(1, 100))
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("recent retrain probe: {e}")))?;

    let undos: Vec<(Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT json_extract(payload_json, '$.retrain_id'), \
                json_extract(payload_json, '$.undone_at') \
         FROM audit_record WHERE kind = 'http.request' \
           AND tenant_id = ? \
           AND json_extract(payload_json, '$.endpoint') = ? \
           AND json_extract(payload_json, '$.worker_id') = ?",
    )
    .bind(ctx.tenant.to_string())
    .bind(UNDO_KEY)
    .bind(ctx.worker.id.to_string())
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("recent undo probe: {e}")))?;

    let undo_map: std::collections::HashMap<String, Option<DateTime<Utc>>> = undos
        .into_iter()
        .filter_map(|(rid, ts)| rid.map(|r| (r, parse_rfc3339(ts.as_deref()))))
        .collect();

    let mut out = Vec::with_capacity(commits.len());
    for (rid, class, sc_raw, applied_raw) in commits {
        let (Some(retrain_id), Some(classification), Some(applied_raw)) = (rid, class, applied_raw)
        else {
            continue;
        };
        let Some(applied_at) = parse_rfc3339(Some(&applied_raw)) else {
            continue;
        };
        let structural_change = sc_raw
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or(Value::Null);
        let undone_at = undo_map.get(&retrain_id).copied().flatten();
        out.push(RecentRetrain {
            retrain_id,
            classification,
            structural_change,
            applied_at,
            undone_at,
        });
    }
    Ok(out)
}

fn parse_rfc3339(raw: Option<&str>) -> Option<DateTime<Utc>> {
    raw.and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|d| d.with_timezone(&Utc))
}

/// Maps the internal retrain classification onto the SPA's
/// `RetrainClassification` enum (`memory` / `policy` / `skill` /
/// `vocabulary` / `action-ban`). Unknown classes fall back to
/// `memory` so the panel always renders.
pub(super) fn map_classification(internal: &str) -> &'static str {
    match internal {
        "policy_rule" => "policy",
        "skill_correction" => "skill",
        "vocabulary_patch" => "vocabulary",
        "action_ban" => "action-ban",
        _ => "memory",
    }
}

/// Derives the one-line summary the `RecentChangesPanel` renders, from
/// the committed classification + its structural change.
pub(super) fn retrain_summary(classification: &str, sc: &Value) -> String {
    let field = |k: &str| sc.get(k).and_then(Value::as_str).unwrap_or("").trim();
    match classification {
        "vocabulary_patch" => {
            let (from, to) = (field("from"), field("to"));
            if from.is_empty() || to.is_empty() {
                "Updated her wording.".into()
            } else {
                format!("Now says \u{201c}{to}\u{201d} instead of \u{201c}{from}\u{201d}.")
            }
        }
        "action_ban" => {
            let action = field("action_type");
            if action.is_empty() {
                "Banned an action.".into()
            } else {
                format!("Banned action \u{201c}{action}\u{201d}.")
            }
        }
        "policy_rule" => {
            let raw = sc
                .get("predicate_json")
                .and_then(|p| p.get("raw"))
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim();
            if raw.is_empty() {
                "Added a policy rule.".into()
            } else {
                format!("New rule: {}", trim_for_display(raw))
            }
        }
        "skill_correction" => {
            let note = field("note");
            if note.is_empty() {
                "Corrected a skill.".into()
            } else {
                format!("Skill correction: {}", trim_for_display(note))
            }
        }
        _ => {
            let text = field("text");
            if text.is_empty() {
                "Learned a new fact.".into()
            } else {
                format!("Remembered: {}", trim_for_display(text))
            }
        }
    }
}
