//! `argo eval export-training-data` — export a tenant's real *judged*
//! history as SFT-style JSONL training data:
//!
//! - **human approval decisions** (`approval_request`, decided rows) →
//!   "a teammate proposed this action; a human approved/rejected it"
//!   pairs, and
//! - **committed retrains** (`worker.retrain.committed` audit rows) →
//!   labelled human-correction examples.
//!
//! Read-only over existing stores — **no new table, no migration**.
//! Closes competitive gap #3 (self-improvement / feedback-mining) and
//! pairs with the tamper-evident audit moat: "every action is provable,
//! and you can learn from the record." Design doc:
//! `docs/skycot/design/2026-05-29-gap-closure-plans/03-self-improvement-training-export.md`.
//!
//! Free-text fields pass through `pii_detect::scan_and_maybe_redact`
//! (respects `ARGO_PII_AUTOREDACT`, exactly like the audit sink) so an
//! export never leaks PII the audit layer would have caught.
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_security::pii_detect::scan_and_maybe_redact;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use serde::Serialize;
use serde_json::{json, Value};
use std::borrow::Cow;
use std::io::Write;
use std::path::Path;

const RETRAIN_COMMIT_ENDPOINT: &str = "worker.retrain.committed";

/// One JSONL training example. Two sources share the struct: a human
/// approval decision (`source = "human_approval"`) and a committed
/// retrain correction (`source = "human_correction"`).
#[derive(Serialize)]
struct TrainingExample {
    source: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    worker_id: Option<String>,
    prompt: String,
    completion: String,
    label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    requested_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    decided_by: Option<String>,
}

/// CLI entry: parse the tenant, open the live store, collect examples
/// from approvals + retrains, write JSONL, print a summary.
pub async fn run(tenant: &str, out: &Path, json_out: bool) -> ArgoResult<()> {
    let tenant_id = parse_tenant(tenant)?;
    let store = open_store().await?;
    let mut examples = collect_approval_examples(&store, tenant_id).await?;
    let from_approvals = examples.len();
    let from_retrains = append_retrain_examples(&store, tenant_id, &mut examples).await?;
    let (written, redacted) = write_jsonl(out, examples)?;
    emit(out, written, from_approvals, from_retrains, redacted, json_out)
}

fn parse_tenant(tenant: &str) -> ArgoResult<TenantId> {
    uuid::Uuid::parse_str(tenant)
        .map(TenantId)
        .map_err(|e| ArgoError::Schema(format!("invalid tenant id '{tenant}': {e}")))
}

async fn open_store() -> ArgoResult<SessionStore> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(db_path(&home)).await?;
    store.migrate().await?;
    Ok(store)
}

/// Decided approval requests → "approve or reject this action?" pairs.
/// `approval_request.tenant_id` is a BLOB column (unlike `audit_record`).
async fn collect_approval_examples(
    store: &SessionStore,
    tenant_id: TenantId,
) -> ArgoResult<Vec<TrainingExample>> {
    type Row = (Option<String>, String, Option<String>, String, Option<String>);
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT draft_content, status, decision, requested_by, decided_by \
         FROM approval_request \
         WHERE tenant_id = ? AND status != 'pending' \
         ORDER BY decided_at DESC",
    )
    .bind(tenant_id.0.as_bytes().as_slice())
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("list decided approvals: {e}")))?;
    let mut out = Vec::with_capacity(rows.len());
    for (draft, status, decision, requested_by, decided_by) in rows {
        let action = draft.unwrap_or_else(|| "(no draft recorded)".into());
        let completion = match status.as_str() {
            "granted" => "approve",
            "denied" => "reject",
            _ => "expire",
        };
        out.push(TrainingExample {
            source: "human_approval",
            worker_id: None,
            prompt: format!("A teammate proposed this action. Approve or reject:\n{action}"),
            completion: completion.to_string(),
            label: decision.unwrap_or(status),
            requested_by: Some(requested_by),
            decided_by,
        });
    }
    Ok(out)
}

/// Committed retrains → labelled human-correction examples. Audit's
/// `tenant_id` is a TEXT column (unlike `approval_request`).
async fn append_retrain_examples(
    store: &SessionStore,
    tenant_id: TenantId,
    out: &mut Vec<TrainingExample>,
) -> ArgoResult<usize> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT payload_json FROM audit_record \
         WHERE kind = 'http.request' AND tenant_id = ? \
           AND json_extract(payload_json, '$.endpoint') = ? \
         ORDER BY seq DESC",
    )
    .bind(tenant_id.to_string())
    .bind(RETRAIN_COMMIT_ENDPOINT)
    .fetch_all(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("list committed retrains: {e}")))?;
    let mut added = 0usize;
    for (payload,) in rows {
        let Ok(v) = serde_json::from_str::<Value>(&payload) else {
            continue;
        };
        let classification = v
            .get("classification")
            .and_then(Value::as_str)
            .unwrap_or("memory_fact");
        let worker_id = v
            .get("worker_id")
            .and_then(Value::as_str)
            .map(str::to_string);
        out.push(TrainingExample {
            source: "human_correction",
            worker_id,
            prompt: "A human corrected this teammate's behaviour. Apply the correction going \
                     forward."
                .into(),
            completion: correction_text(classification, v.get("structural_change")),
            label: classification.to_string(),
            requested_by: None,
            decided_by: None,
        });
        added += 1;
    }
    Ok(added)
}

/// Plain-English correction from a retrain's classification + change.
fn correction_text(classification: &str, sc: Option<&Value>) -> String {
    let field = |k: &str| {
        sc.and_then(|s| s.get(k))
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
    };
    match classification {
        "vocabulary_patch" => format!(
            "Say \u{201c}{}\u{201d} instead of \u{201c}{}\u{201d}.",
            field("to"),
            field("from")
        ),
        "action_ban" => format!("Never perform the action \u{201c}{}\u{201d}.", field("action_type")),
        "policy_rule" => {
            let raw = sc
                .and_then(|s| s.get("predicate_json"))
                .and_then(|p| p.get("raw"))
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim();
            format!("Follow this rule: {raw}")
        }
        "skill_correction" => format!("Skill correction: {}", field("note")),
        _ => format!("Remember: {}", field("text")),
    }
}

/// Write each example as one JSON line, PII-redacting the free-text
/// fields. Returns `(lines_written, fields_redacted)`.
fn write_jsonl(out: &Path, examples: Vec<TrainingExample>) -> ArgoResult<(usize, usize)> {
    let mut file = std::fs::File::create(out)
        .map_err(|e| ArgoError::Io(format!("create {}: {e}", out.display())))?;
    let mut written = 0usize;
    let mut redacted = 0usize;
    for mut ex in examples {
        let (prompt, rp) = redact(&ex.prompt);
        let (completion, rc) = redact(&ex.completion);
        redacted += usize::from(rp) + usize::from(rc);
        ex.prompt = prompt;
        ex.completion = completion;
        let line = serde_json::to_string(&ex)
            .map_err(|e| ArgoError::Provider(format!("serialize example: {e}")))?;
        writeln!(file, "{line}")
            .map_err(|e| ArgoError::Io(format!("write {}: {e}", out.display())))?;
        written += 1;
    }
    Ok((written, redacted))
}

/// `(possibly-redacted text, was_redacted)`. Owned ⇒ redaction applied.
fn redact(s: &str) -> (String, bool) {
    match scan_and_maybe_redact(s, "training_export") {
        Cow::Borrowed(_) => (s.to_string(), false),
        Cow::Owned(red) => (red, true),
    }
}

fn emit(
    out: &Path,
    written: usize,
    approvals: usize,
    retrains: usize,
    redacted: usize,
    json_out: bool,
) -> ArgoResult<()> {
    if json_out {
        let body = serde_json::to_string(&json!({
            "ok": true,
            "out": out.display().to_string(),
            "examples": written,
            "from_approvals": approvals,
            "from_retrains": retrains,
            "fields_redacted": redacted,
        }))
        .map_err(|e| ArgoError::Provider(format!("serialize summary: {e}")))?;
        println!("{body}");
    } else {
        println!("exported {written} training example(s) to {}", out.display());
        println!("  from approvals:  {approvals}");
        println!("  from retrains:   {retrains}");
        println!("  fields redacted: {redacted}");
        if written == 0 {
            println!("  (no decided approvals or committed retrains for this tenant)");
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "eval_export_training_data_tests.rs"]
mod tests;
