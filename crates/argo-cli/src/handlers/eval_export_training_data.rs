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
use super::eval_judged_history as jh;
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use serde::Serialize;
use serde_json::{json, Value};
use std::io::Write;
use std::path::Path;

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
    let (store, tenant_id) = jh::open_for_tenant(tenant).await?;
    let mut examples = collect_approval_examples(&store, tenant_id).await?;
    let from_approvals = examples.len();
    let from_retrains = append_retrain_examples(&store, tenant_id, &mut examples).await?;
    let (written, redacted) = write_jsonl(out, examples)?;
    emit(out, written, from_approvals, from_retrains, redacted, json_out)
}

/// Decided approval requests → "approve or reject this action?" pairs.
/// Reads via the shared judged-history layer (`eval_judged_history`),
/// shared with `argo eval generate-cases`, so the query lives once.
async fn collect_approval_examples(
    store: &SessionStore,
    tenant_id: TenantId,
) -> ArgoResult<Vec<TrainingExample>> {
    let rows = jh::decided_approvals(store, tenant_id).await?;
    let mut out = Vec::with_capacity(rows.len());
    for a in rows {
        let action = a.draft.unwrap_or_else(|| "(no draft recorded)".into());
        let completion = match a.status.as_str() {
            "granted" => "approve",
            "denied" => "reject",
            _ => "expire",
        };
        out.push(TrainingExample {
            source: "human_approval",
            worker_id: None,
            prompt: format!("A teammate proposed this action. Approve or reject:\n{action}"),
            completion: completion.to_string(),
            label: a.decision.unwrap_or(a.status),
            requested_by: Some(a.requested_by),
            decided_by: a.decided_by,
        });
    }
    Ok(out)
}

/// Committed retrains → labelled human-correction examples. Reads via
/// the shared judged-history layer (`eval_judged_history`).
async fn append_retrain_examples(
    store: &SessionStore,
    tenant_id: TenantId,
    out: &mut Vec<TrainingExample>,
) -> ArgoResult<usize> {
    let rows = jh::committed_retrains(store, tenant_id).await?;
    let added = rows.len();
    for c in rows {
        out.push(TrainingExample {
            source: "human_correction",
            worker_id: c.worker_id,
            prompt: "A human corrected this teammate's behaviour. Apply the correction going \
                     forward."
                .into(),
            completion: correction_text(&c.classification, c.structural_change.as_ref()),
            label: c.classification,
            requested_by: None,
            decided_by: None,
        });
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
        let (prompt, rp) = jh::redact(&ex.prompt, "training_export");
        let (completion, rc) = jh::redact(&ex.completion, "training_export");
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
