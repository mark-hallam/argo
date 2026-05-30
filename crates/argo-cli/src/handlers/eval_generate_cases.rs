//! `argo eval generate-cases` — turn a tenant's real *judged* history
//! into `argo-eval` regression cases (the self-improvement loop's
//! Step 1b).
//!
//! For every **human-rejected** approval, emit a case asserting the
//! same proposed action must be **refused or deferred**; for every
//! **committed retrain**, emit a case asserting the **corrected
//! behaviour**. The generated JSONL loads + runs in the existing eval
//! harness (`argo eval validate <file>` / `argo_eval::load_jsonl`), so
//! judged-bad behaviour becomes a test the worker is held to.
//!
//! Read-only over existing stores — **no new table, no migration**.
//! Reuses the shared judged-history reader (`eval_judged_history`) that
//! also backs `argo eval export-training-data`, and the same
//! `pii_detect::scan_and_maybe_redact` boundary, so a generated case
//! never leaks PII the audit layer would have caught. Design doc:
//! `docs/skycot/design/2026-05-29-gap-closure-plans/03-self-improvement-training-export.md`.
use super::eval_judged_history as jh;
use argo_eval::{generate_cases, Correction, JudgedInputs, RejectedAction};
use argo_types::{ArgoError, ArgoResult};
use serde_json::json;
use std::path::Path;

/// CLI entry: read the tenant's judged history, build the (redacted)
/// generator inputs, generate cases, write JSONL, print a summary.
pub async fn run(tenant: &str, out: &Path, json_out: bool) -> ArgoResult<()> {
    let (store, tenant_id) = jh::open_for_tenant(tenant).await?;

    let approvals = jh::decided_approvals(&store, tenant_id).await?;
    let retrains = jh::committed_retrains(&store, tenant_id).await?;

    let mut redacted = 0usize;
    let inputs = build_inputs(approvals, retrains, &mut redacted);
    let from_rejected = inputs.rejected.len();
    let from_corrections = inputs.corrections.len();

    let cases = generate_cases(&inputs);
    let lines = serialize(&cases)?;
    let written = jh::write_lines(out, &lines)?;

    emit(
        out,
        written,
        from_rejected,
        from_corrections,
        redacted,
        json_out,
    )
}

/// Turn read rows into the generator's neutral, PII-redacted inputs.
/// Only *rejected* approvals carry a learnable "must not do this"
/// signal, so approved rows are dropped here (not in the generator).
fn build_inputs(
    approvals: Vec<jh::DecidedApproval>,
    retrains: Vec<jh::CommittedRetrain>,
    redacted: &mut usize,
) -> JudgedInputs {
    let rejected = approvals
        .into_iter()
        .filter(jh::DecidedApproval::was_refused)
        .map(|a| {
            let draft = a.draft.unwrap_or_else(|| "(no draft recorded)".into());
            let (draft, r1) = jh::redact(&draft, "eval_case_gen");
            let decision = a.decision.map(|d| {
                let (d, r) = jh::redact(&d, "eval_case_gen");
                *redacted += usize::from(r);
                d
            });
            *redacted += usize::from(r1);
            RejectedAction { draft, decision }
        })
        .collect();

    let corrections = retrains
        .into_iter()
        .map(|c| Correction {
            worker_id: c.worker_id,
            classification: c.classification,
            structural_change: c
                .structural_change
                .map(|sc| redact_structural_change(sc, redacted)),
        })
        .collect();

    JudgedInputs {
        rejected,
        corrections,
    }
}

/// PII-redact the free-text leaves of a `structural_change` object
/// (the values the generator reads: `text`, `note`, `from`, `to`,
/// `predicate_json.raw`). Non-string / structural fields pass through.
fn redact_structural_change(sc: serde_json::Value, redacted: &mut usize) -> serde_json::Value {
    fn walk(v: &mut serde_json::Value, redacted: &mut usize) {
        match v {
            serde_json::Value::String(s) => {
                let (clean, was) = jh::redact(s, "eval_case_gen");
                *redacted += usize::from(was);
                *s = clean;
            }
            serde_json::Value::Object(map) => {
                for (_, child) in map.iter_mut() {
                    walk(child, redacted);
                }
            }
            serde_json::Value::Array(items) => {
                for child in items.iter_mut() {
                    walk(child, redacted);
                }
            }
            _ => {}
        }
    }
    let mut sc = sc;
    walk(&mut sc, redacted);
    sc
}

/// Serialize each case to one JSONL line, matching the on-disk task
/// format `argo_eval::load_jsonl` reads.
fn serialize(cases: &[argo_eval::EvalTask]) -> ArgoResult<Vec<String>> {
    cases
        .iter()
        .map(|c| {
            serde_json::to_string(c)
                .map_err(|e| ArgoError::Provider(format!("serialize eval case: {e}")))
        })
        .collect()
}

fn emit(
    out: &Path,
    written: usize,
    from_rejected: usize,
    from_corrections: usize,
    redacted: usize,
    json_out: bool,
) -> ArgoResult<()> {
    if json_out {
        let body = serde_json::to_string(&json!({
            "ok": true,
            "out": out.display().to_string(),
            "cases": written,
            "from_rejected_approvals": from_rejected,
            "from_committed_retrains": from_corrections,
            "fields_redacted": redacted,
        }))
        .map_err(|e| ArgoError::Provider(format!("serialize summary: {e}")))?;
        println!("{body}");
    } else {
        println!("generated {written} eval case(s) to {}", out.display());
        println!("  from rejected approvals: {from_rejected}");
        println!("  from committed retrains: {from_corrections}");
        println!("  fields redacted:         {redacted}");
        if written == 0 {
            println!("  (no rejected approvals or committed retrains for this tenant)");
        } else {
            println!("  run them: argo eval validate {}", out.display());
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "eval_generate_cases_tests.rs"]
mod tests;
